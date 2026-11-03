//! Bounded local command execution. This is a process runner, not a sandbox.
//!
//! Commands retain the invoking user's filesystem and network authority. Only
//! explicitly named environment variables are inherited. Captures are private
//! files; the runner never interprets their contents or shell metacharacters.

use crate::plan::Invocation;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Why an invocation finished. A normal exit may still have a nonzero code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Termination {
    /// The process and both output streams completed.
    Completed,
    /// The executable could not be found.
    Unavailable,
    /// The executable could not be started for another reason.
    FailedToStart,
    /// The monotonic execution deadline expired.
    TimedOut,
    /// The caller requested cancellation.
    Interrupted,
    /// Capture was incomplete, exceeded its bound, or left descendants running.
    CaptureFailed,
}

/// Raw process observations; evaluator policy is applied separately.
#[derive(Debug)]
pub struct CapturedRun {
    /// Observed termination, not a gate verdict.
    pub termination: Termination,
    /// Exit code where the operating system supplied one.
    pub exit_code: Option<i32>,
    /// Terminating Unix signal, if any.
    pub signal: Option<i32>,
    /// Start time as Unix milliseconds.
    pub started_at_ms: u64,
    /// End time as Unix milliseconds.
    pub finished_at_ms: u64,
    /// Elapsed monotonic time, in milliseconds.
    pub elapsed_ms: u64,
    /// Exact retained standard output.
    pub stdout: PathBuf,
    /// Exact retained standard error.
    pub stderr: PathBuf,
    /// Non-secret explanation of an incomplete attempt.
    pub error: Option<String>,
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

/// Create a new private directory, refusing existing targets and symlink parents.
///
/// The caller may canonicalise its explicitly selected workspace root first.
/// These checks do not sandbox a concurrently hostile filesystem.
pub fn private_directory(path: &Path) -> io::Result<()> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut current = PathBuf::new();
    for component in absolute.components() {
        use std::path::Component;
        match component {
            Component::ParentDir => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "parent traversal in output path",
                ));
            }
            Component::CurDir => continue,
            _ => current.push(component),
        }
        if current == absolute {
            break;
        }
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => (),
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "output parent is not an ordinary directory",
                ));
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                let mut builder = fs::DirBuilder::new();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::DirBuilderExt;
                    builder.mode(0o700);
                }
                builder.create(&current)?;
            }
            Err(err) => return Err(err),
        }
    }
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(&absolute)
}

/// Exclusively create a private evidence file; existing files are never truncated.
pub fn private_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

#[cfg(unix)]
fn capture<R: Read + std::os::fd::AsFd + Send + 'static>(
    mut input: R,
    mut output: File,
    total: Arc<AtomicU64>,
    fault: Arc<AtomicU8>,
    stop: Arc<AtomicBool>,
    limit: u64,
) -> thread::JoinHandle<io::Result<()>> {
    thread::spawn(move || {
        let result = (|| {
            use nix::fcntl::{FcntlArg, OFlag, fcntl};
            let flags = fcntl(&input, FcntlArg::F_GETFL).map_err(io::Error::from)?;
            fcntl(
                &input,
                FcntlArg::F_SETFL(OFlag::from_bits_truncate(flags) | OFlag::O_NONBLOCK),
            )
            .map_err(io::Error::from)?;
            let mut bytes = [0_u8; 8192];
            loop {
                let count = match input.read(&mut bytes) {
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                        if stop.load(Ordering::SeqCst) {
                            return Err(io::Error::new(
                                io::ErrorKind::Interrupted,
                                "capture interrupted",
                            ));
                        }
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    other => other?,
                };
                if count == 0 {
                    break;
                }
                let previous = total.fetch_add(count as u64, Ordering::SeqCst);
                let allowed = limit.saturating_sub(previous).min(count as u64) as usize;
                output.write_all(&bytes[..allowed])?;
                if allowed != count {
                    fault.store(1, Ordering::SeqCst);
                    break;
                }
            }
            output.sync_all()
        })();
        if result.is_err() {
            fault.store(2, Ordering::SeqCst);
        }
        result
    })
}

#[cfg(unix)]
fn signal_group(pid: u32, signal: nix::sys::signal::Signal) {
    if let Ok(pid) = i32::try_from(pid) {
        if pid > 1 {
            let _ = nix::sys::signal::killpg(nix::unistd::Pid::from_raw(pid), signal);
        }
    }
}

/// Run one explicit invocation, retaining bounded stdout/stderr captures.
///
/// `output` must be a fresh private directory created by the caller. Paths in
/// `Invocation::cwd` are resolved relative to `root`. The environment contains
/// only the named variables that exist in the caller. Stdin is closed unless
/// explicitly inherited for a transport such as Deepsec's `--files-from -`.
///
/// Unix execution uses a new process group and stops that group on interruption,
/// deadline, capture failure or lingering descendants. Non-Unix execution is
/// explicitly unsupported rather than claiming unimplemented child cleanup.
pub fn run_command(
    invocation: &Invocation,
    root: &Path,
    output: &Path,
    cancelled: &AtomicBool,
    inherit_stdin: bool,
) -> io::Result<CapturedRun> {
    run_command_with_env(
        invocation,
        root,
        output,
        cancelled,
        inherit_stdin,
        &std::collections::BTreeMap::new(),
    )
}

// Internal fixed-value metadata policy only. User checks declare inherited
// names in Invocation; their values are deliberately absent from receipts.
pub(crate) fn run_command_with_env(
    invocation: &Invocation,
    root: &Path,
    output: &Path,
    cancelled: &AtomicBool,
    inherit_stdin: bool,
    overrides: &std::collections::BTreeMap<String, String>,
) -> io::Result<CapturedRun> {
    #[cfg(not(unix))]
    {
        let _ = (
            invocation,
            root,
            output,
            cancelled,
            inherit_stdin,
            overrides,
        );
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "bounded process groups require Unix",
        ))
    }
    #[cfg(unix)]
    {
        use nix::sys::signal::{Signal, killpg};
        use nix::unistd::Pid;
        use std::os::unix::process::{CommandExt, ExitStatusExt};
        invocation.validate().map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "invalid invocation contract")
        })?;
        let started_at = unix_now();
        let start = Instant::now();
        let stdout_path = output.join("stdout.log");
        let stderr_path = output.join("stderr.log");
        let stdout_file = private_file(&stdout_path)?;
        let stderr_file = private_file(&stderr_path)?;
        let mut result = CapturedRun {
            termination: Termination::FailedToStart,
            exit_code: None,
            signal: None,
            started_at_ms: started_at,
            finished_at_ms: started_at,
            elapsed_ms: 0,
            stdout: stdout_path,
            stderr: stderr_path,
            error: None,
        };
        if cancelled.load(Ordering::SeqCst) {
            result.termination = Termination::Interrupted;
            result.error = Some("cancelled before invocation".into());
            return Ok(result);
        }
        if invocation.timeout_ms == 0 || invocation.max_output_bytes == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "execution bounds must be positive",
            ));
        }
        let cwd = root.join(&invocation.cwd);
        let mut command = Command::new(&invocation.program);
        command
            .args(&invocation.args)
            .current_dir(&cwd)
            .env_clear()
            .process_group(0)
            .stdin(if inherit_stdin {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for name in &invocation.env {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        command.envs(overrides);
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(err) => {
                result.termination = if err.kind() == io::ErrorKind::NotFound {
                    Termination::Unavailable
                } else {
                    Termination::FailedToStart
                };
                result.error = Some(format!("process start failed: {}", err.kind()));
                result.finished_at_ms = unix_now();
                result.elapsed_ms = start.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
                return Ok(result);
            }
        };
        let pid = child.id();
        let total = Arc::new(AtomicU64::new(0));
        let fault = Arc::new(AtomicU8::new(0));
        let stop_capture = Arc::new(AtomicBool::new(false));
        let stdout = capture(
            child.stdout.take().expect("piped stdout"),
            stdout_file,
            total.clone(),
            fault.clone(),
            stop_capture.clone(),
            invocation.max_output_bytes,
        );
        let stderr = capture(
            child.stderr.take().expect("piped stderr"),
            stderr_file,
            total,
            fault.clone(),
            stop_capture.clone(),
            invocation.max_output_bytes,
        );
        let mut status = None;
        result.termination = loop {
            if cancelled.load(Ordering::SeqCst) {
                break Termination::Interrupted;
            }
            if start.elapsed() >= Duration::from_millis(invocation.timeout_ms) {
                break Termination::TimedOut;
            }
            if fault.load(Ordering::SeqCst) != 0 {
                break Termination::CaptureFailed;
            }
            match child.try_wait() {
                Ok(Some(exit)) => {
                    status = Some(exit);
                    break Termination::Completed;
                }
                Ok(None) => thread::sleep(Duration::from_millis(10)),
                Err(_) => {
                    result.error = Some("could not observe process exit".into());
                    break Termination::CaptureFailed;
                }
            }
        };
        if result.termination == Termination::Completed {
            // A completed parent must not strand background writers or spenders.
            let group_alive =
                i32::try_from(pid).is_ok_and(|p| p > 1 && killpg(Pid::from_raw(p), None).is_ok());
            if group_alive {
                result.termination = Termination::CaptureFailed;
                result.error = Some("process left unfinished descendants".into());
            } else {
                // A slow filesystem flush is not evidence of a live descendant.
                while !stdout.is_finished() || !stderr.is_finished() {
                    if cancelled.load(Ordering::SeqCst) {
                        result.termination = Termination::Interrupted;
                        break;
                    }
                    if start.elapsed() >= Duration::from_millis(invocation.timeout_ms) {
                        result.termination = Termination::TimedOut;
                        break;
                    }
                    thread::sleep(Duration::from_millis(5));
                }
            }
        }
        if result.termination != Termination::Completed {
            signal_group(pid, Signal::SIGINT);
            let grace = Instant::now();
            while grace.elapsed() < Duration::from_millis(250) {
                if status.is_none() {
                    status = child.try_wait().ok().flatten();
                }
                if stdout.is_finished() && stderr.is_finished() && status.is_some() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
            signal_group(pid, Signal::SIGKILL);
            stop_capture.store(true, Ordering::SeqCst);
        }
        if status.is_none() {
            status = child.wait().ok();
        }
        for capture in [stdout, stderr] {
            if !matches!(capture.join(), Ok(Ok(()))) {
                fault.store(2, Ordering::SeqCst);
            }
        }
        if fault.load(Ordering::SeqCst) != 0 {
            if result.termination == Termination::Completed {
                result.termination = Termination::CaptureFailed;
            }
            result.error = Some(
                if fault.load(Ordering::SeqCst) == 1 {
                    "output limit exceeded"
                } else {
                    "capture failed"
                }
                .into(),
            );
        }
        result.exit_code = status.and_then(|s| s.code());
        result.signal = status.and_then(|s| s.signal());
        result.finished_at_ms = unix_now();
        result.elapsed_ms = start.elapsed().as_millis().try_into().unwrap_or(u64::MAX);
        Ok(result)
    }
}
