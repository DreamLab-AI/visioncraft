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
    if let Ok(pid) = i32::try_from(pid)
        && pid > 1
    {
        let _ = nix::sys::signal::killpg(nix::unistd::Pid::from_raw(pid), signal);
    }
}

#[cfg(unix)]
fn group_probe(result: Result<(), nix::errno::Errno>) -> io::Result<bool> {
    match result {
        Ok(()) => Ok(true),
        Err(nix::errno::Errno::ESRCH) => Ok(false),
        Err(error) => Err(io::Error::from(error)),
    }
}

#[cfg(unix)]
fn group_running(pid: u32, deadline: Instant) -> io::Result<bool> {
    use nix::sys::signal::killpg;
    use nix::unistd::Pid;
    let group = i32::try_from(pid)
        .ok()
        .filter(|pid| *pid > 1)
        .ok_or_else(|| io::Error::other("invalid process group"))?;
    if !group_probe(killpg(Pid::from_raw(group), None))? {
        return Ok(false);
    }
    #[cfg(target_os = "linux")]
    {
        // Signal 0 proves existence, not execution: zombies still occupy groups.
        linux_proc::group_running(group, deadline)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = deadline;
        // Without a supported state observer, an existing group remains unsafe.
        Ok(true)
    }
}

#[cfg(target_os = "linux")]
mod linux_proc {
    use super::*;
    use nix::unistd::{Pid, getpgid};
    use std::collections::BTreeSet;

    fn unknown() -> io::Error {
        io::Error::other("process group state could not be established")
    }

    fn bounded_read(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > limit {
            return Err(unknown());
        }
        Ok(bytes)
    }

    // comm may contain spaces, parentheses, newlines or non-UTF-8 bytes. Only
    // parse the numeric prefix and fields after the final closing parenthesis.
    fn state(bytes: &[u8], expected_pid: u32, expected_group: Option<i32>) -> io::Result<u8> {
        let space = bytes.iter().position(|c| *c == b' ').ok_or_else(unknown)?;
        let close = bytes
            .windows(2)
            .rposition(|s| s == b") ")
            .ok_or_else(unknown)?;
        let pid = std::str::from_utf8(&bytes[..space])
            .ok()
            .and_then(|s| s.parse::<u32>().ok());
        if pid != Some(expected_pid) || bytes.get(space + 1) != Some(&b'(') || close <= space + 1 {
            return Err(unknown());
        }
        let text = std::str::from_utf8(&bytes[close + 2..]).map_err(|_| unknown())?;
        let mut fields = text.split_ascii_whitespace();
        let status = fields
            .next()
            .filter(|s| s.len() == 1)
            .ok_or_else(unknown)?
            .as_bytes()[0];
        fields
            .next()
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(unknown)?;
        let group = fields
            .next()
            .and_then(|s| s.parse::<i32>().ok())
            .ok_or_else(unknown)?;
        if expected_group.is_some_and(|expected| group != expected) {
            return Err(unknown());
        }
        Ok(status)
    }

    fn unrestricted(mounts: &str) -> bool {
        let mut found = false;
        for line in mounts.lines() {
            let fields: Vec<_> = line.split_ascii_whitespace().collect();
            if fields.get(1) == Some(&"/proc") {
                if fields.get(2) != Some(&"proc") {
                    return false;
                }
                let Some(options) = fields.get(3) else {
                    return false;
                };
                if options
                    .split(',')
                    .any(|o| o.starts_with("hidepid=") && o != "hidepid=0" && o != "hidepid=off")
                {
                    return false;
                }
                found = true;
            }
        }
        found
    }

    fn ids(path: &Path, deadline: Instant) -> io::Result<Vec<u32>> {
        let mut ids = Vec::new();
        for entry in fs::read_dir(path)? {
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "process observation deadline",
                ));
            }
            let name = entry?.file_name();
            if let Some(id) = name.to_str().and_then(|s| s.parse::<u32>().ok()) {
                ids.push(id);
                if ids.len() > 65_536 {
                    return Err(unknown());
                }
            }
        }
        Ok(ids)
    }

    fn dead(state: u8) -> bool {
        matches!(state, b'Z' | b'X' | b'x')
    }

    fn no_longer_member(pid: u32, group: i32) -> io::Result<bool> {
        match getpgid(Some(Pid::from_raw(
            i32::try_from(pid).map_err(|_| unknown())?,
        ))) {
            Ok(actual) => Ok(actual.as_raw() != group),
            Err(nix::errno::Errno::ESRCH) => Ok(true),
            Err(error) => Err(io::Error::from(error)),
        }
    }

    fn tasks_dead(process: &Path, group: i32, deadline: Instant) -> io::Result<bool> {
        let tasks = ids(&process.join("task"), deadline)?;
        if tasks.is_empty() {
            return Err(unknown());
        }
        for tid in tasks {
            let record = match bounded_read(&process.join(format!("task/{tid}/stat")), 8192) {
                Ok(record) => record,
                Err(error)
                    if error.kind() == io::ErrorKind::NotFound && no_longer_member(tid, group)? =>
                {
                    continue;
                }
                Err(error) => return Err(error),
            };
            if !dead(state(&record, tid, Some(group))?) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn dead_members(group: i32, deadline: Instant) -> io::Result<Option<BTreeSet<u32>>> {
        let mut members = BTreeSet::new();
        for pid in ids(Path::new("/proc"), deadline)? {
            if no_longer_member(pid, group)? {
                continue;
            }
            let process = PathBuf::from(format!("/proc/{pid}"));
            let record = match bounded_read(&process.join("stat"), 8192) {
                Ok(record) => record,
                Err(error)
                    if error.kind() == io::ErrorKind::NotFound && no_longer_member(pid, group)? =>
                {
                    continue;
                }
                Err(error) => return Err(error),
            };
            if !dead(state(&record, pid, Some(group))?) {
                return Ok(None);
            }
            // A zombie thread-group leader can still have executing threads.
            // Missing/unreadable task information cannot prove they have exited.
            match tasks_dead(&process, group, deadline) {
                Ok(true) => (),
                Ok(false) => return Ok(None),
                Err(error)
                    if error.kind() == io::ErrorKind::NotFound && no_longer_member(pid, group)? =>
                {
                    continue;
                }
                Err(error) => return Err(error),
            }
            members.insert(pid);
        }
        Ok(Some(members))
    }

    pub(super) fn group_running(group: i32, deadline: Instant) -> io::Result<bool> {
        let mounts = bounded_read(Path::new("/proc/mounts"), 1_048_576)?;
        if !unrestricted(std::str::from_utf8(&mounts).map_err(|_| unknown())?) {
            return Err(unknown());
        }
        // A proc mount for another PID namespace cannot establish membership.
        state(
            &bounded_read(Path::new("/proc/self/stat"), 8192)?,
            std::process::id(),
            None,
        )?;
        let mut previous = None;
        for _ in 0..3 {
            let Some(members) = dead_members(group, deadline)? else {
                return Ok(true);
            };
            if members.is_empty() {
                return if super::group_probe(nix::sys::signal::killpg(Pid::from_raw(group), None))?
                {
                    Err(unknown())
                } else {
                    Ok(false)
                };
            }
            // Retry only changes to this group. Unrelated process churn must not
            // make a completed command fail on a busy host.
            if previous.as_ref() == Some(&members) {
                return Ok(false);
            }
            previous = Some(members);
        }
        Err(unknown())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn stat_handles_unusual_names_and_never_calls_live_states_dead() {
            for status in b"RS DTtWKPIZXx".iter().filter(|c| **c != b' ') {
                let mut record = b"123 (odd ) \xff name) ".to_vec();
                record.push(*status);
                record.extend_from_slice(b" 1 456 0 0");
                let observed = state(&record, 123, Some(456)).unwrap();
                assert_eq!(dead(observed), matches!(status, b'Z' | b'X' | b'x'));
            }
            assert!(state(b"123 (test) Z 1 456", 124, Some(456)).is_err());
            assert!(state(b"123 (test) Z 1 456", 123, Some(457)).is_err());
            assert!(state(b"123 (test) Z", 123, None).is_err());
        }

        #[test]
        fn hidden_or_missing_proc_views_are_not_complete_observations() {
            assert!(unrestricted("proc /proc proc rw,hidepid=0 0 0\n"));
            assert!(unrestricted("proc /proc proc rw,hidepid=off 0 0\n"));
            assert!(!unrestricted("proc /proc proc rw,hidepid=2 0 0\n"));
            assert!(!unrestricted("proc /proc proc rw,hidepid=ptraceable 0 0\n"));
            assert!(!unrestricted("tmpfs /proc tmpfs rw 0 0\n"));
            assert!(!unrestricted(""));
        }

        #[test]
        fn zombie_leader_does_not_hide_a_live_thread_or_unreadable_state() {
            let root = tempfile::tempdir().unwrap();
            for (tid, status) in [(123, "Z"), (124, "S")] {
                let path = root.path().join(format!("task/{tid}"));
                fs::create_dir_all(&path).unwrap();
                fs::write(path.join("stat"), format!("{tid} (fixture) {status} 1 456")).unwrap();
            }
            let deadline = Instant::now() + Duration::from_secs(1);
            assert!(!tasks_dead(root.path(), 456, deadline).unwrap());
            fs::write(root.path().join("task/124/stat"), "124 (fixture) Z 1 456").unwrap();
            assert!(tasks_dead(root.path(), 456, deadline).unwrap());
            fs::write(root.path().join("task/124/stat"), "unreadable record").unwrap();
            assert!(tasks_dead(root.path(), 456, deadline).is_err());
        }

        #[test]
        fn permission_and_other_probe_errors_do_not_mean_absent() {
            assert!(!super::super::group_probe(Err(nix::errno::Errno::ESRCH)).unwrap());
            assert!(super::super::group_probe(Err(nix::errno::Errno::EPERM)).is_err());
            assert!(super::super::group_probe(Err(nix::errno::Errno::EINVAL)).is_err());
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
/// Linux excludes zombie/dead group members only after bounded, unrestricted
/// proc observations, including their threads. Unknown or unreadable membership
/// remains a failure. Other Unix systems conservatively reject any surviving
/// process group, including unreaped zombies. No subreaper state is changed.
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
        use nix::sys::signal::Signal;
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
            match group_running(pid, start + Duration::from_millis(invocation.timeout_ms)) {
                Ok(true) => {
                    result.termination = Termination::CaptureFailed;
                    result.error = Some("process left unfinished descendants".into());
                }
                Err(error) => {
                    result.termination = if error.kind() == io::ErrorKind::TimedOut {
                        Termination::TimedOut
                    } else {
                        Termination::CaptureFailed
                    };
                    result.error = Some("could not establish process group completion".into());
                }
                Ok(false) => (),
            }
            if result.termination == Termination::Completed {
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
