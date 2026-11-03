//! Executable process-boundary controls.
#![cfg(all(unix, feature = "process"))]

use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use visioncraft::Invocation;
use visioncraft::process::{Termination, private_directory, run_command};

fn invocation(script: &str) -> Invocation {
    Invocation {
        program: "sh".into(),
        args: vec!["-c".into(), script.into()],
        cwd: ".".into(),
        env: vec!["PATH".into()],
        timeout_ms: 2_000,
        max_output_bytes: 8192,
    }
}

#[test]
fn argv_remains_literal_and_only_named_environment_is_inherited() {
    let root = tempfile::tempdir().unwrap();
    let out = root.path().join("attempt");
    private_directory(&out).unwrap();
    let mut spec = invocation("printf '%s\\n' \"$1\"; printf '%s' \"${HOME-unset}\" >&2");
    let literal = "$(touch SHOULD_NOT_EXIST); `echo secret`";
    spec.args.extend(["fixture".into(), literal.into()]);
    let result = run_command(&spec, root.path(), &out, &AtomicBool::new(false), false).unwrap();
    assert_eq!(result.termination, Termination::Completed, "{result:?}");
    assert_eq!(result.exit_code, Some(0));
    assert_eq!(
        fs::read_to_string(result.stdout).unwrap(),
        format!("{literal}\n")
    );
    assert_eq!(fs::read_to_string(result.stderr).unwrap(), "unset");
    assert!(!root.path().join("SHOULD_NOT_EXIST").exists());
}

#[test]
fn exit_code_is_observed_without_being_relabelled_as_process_failure() {
    let root = tempfile::tempdir().unwrap();
    let out = root.path().join("attempt");
    private_directory(&out).unwrap();
    let result = run_command(
        &invocation("printf 'assertion reached' >&2; exit 9"),
        root.path(),
        &out,
        &AtomicBool::new(false),
        false,
    )
    .unwrap();
    assert_eq!(result.termination, Termination::Completed, "{result:?}");
    assert_eq!(result.exit_code, Some(9));
    assert_eq!(
        fs::read_to_string(result.stderr).unwrap(),
        "assertion reached"
    );
}

#[test]
fn timeout_stops_a_descendant_and_preserves_partial_output() {
    let root = tempfile::tempdir().unwrap();
    let out = root.path().join("attempt");
    private_directory(&out).unwrap();
    let mut spec = invocation("sleep 30 & printf '%s' \"$!\"; wait");
    spec.timeout_ms = 100;
    let start = Instant::now();
    let result = run_command(&spec, root.path(), &out, &AtomicBool::new(false), false).unwrap();
    assert_eq!(result.termination, Termination::TimedOut);
    assert!(start.elapsed() < Duration::from_secs(3));
    let pid = fs::read_to_string(result.stdout).unwrap();
    #[cfg(target_os = "linux")]
    if let Ok(stat) = fs::read_to_string(format!("/proc/{pid}/stat")) {
        // An orphan may await the container's reaper; it must no longer execute.
        let state = stat
            .rsplit_once(") ")
            .unwrap()
            .1
            .split_whitespace()
            .next()
            .unwrap();
        assert_eq!(state, "Z", "descendant still running: {stat}");
    }
}

#[test]
fn output_limit_and_lingering_background_processes_cannot_pass() {
    let root = tempfile::tempdir().unwrap();
    let out = root.path().join("overflow");
    private_directory(&out).unwrap();
    let mut spec = invocation("yes output");
    spec.max_output_bytes = 1024;
    let result = run_command(&spec, root.path(), &out, &AtomicBool::new(false), false).unwrap();
    assert_eq!(result.termination, Termination::CaptureFailed);
    assert!(
        fs::metadata(result.stdout).unwrap().len() + fs::metadata(result.stderr).unwrap().len()
            <= 1024
    );
    let out = root.path().join("background");
    private_directory(&out).unwrap();
    let result = run_command(
        &invocation("sleep 30 & exit 0"),
        root.path(),
        &out,
        &AtomicBool::new(false),
        false,
    )
    .unwrap();
    assert_eq!(result.termination, Termination::CaptureFailed);
}

#[cfg(target_os = "linux")]
fn completed_leader_with_group_member(exited: bool) -> visioncraft::process::CapturedRun {
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command, Stdio};

    struct Reap(Child);
    impl Drop for Reap {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    let root = tempfile::tempdir().unwrap();
    let out = root.path().join("attempt");
    private_directory(&out).unwrap();
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            run_command(
                &invocation(
                    "printf '%s' \"$$\" > group; while [ ! -e release ]; do sleep 0.01; done",
                ),
                root.path(),
                &out,
                &AtomicBool::new(false),
                false,
            )
            .unwrap()
        });
        let deadline = Instant::now() + Duration::from_secs(1);
        let group = loop {
            if let Ok(raw) = fs::read_to_string(root.path().join("group"))
                && let Ok(pid) = raw.parse::<i32>()
            {
                break pid;
            }
            assert!(Instant::now() < deadline, "fixture leader did not start");
            std::thread::sleep(Duration::from_millis(5));
        };
        // The test owns this member, so its zombie is reproducible even with a
        // reaping init, and cleanup can wait it without changing subreaper state.
        let member = Reap(
            Command::new("sh")
                .args(["-c", if exited { "exit 0" } else { "exec sleep 30" }])
                .process_group(group)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        );
        if exited {
            loop {
                let stat = fs::read_to_string(format!("/proc/{}/stat", member.0.id())).unwrap();
                if stat.rsplit_once(") ").unwrap().1.starts_with("Z ") {
                    break;
                }
                assert!(Instant::now() < deadline, "fixture member did not exit");
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        fs::write(root.path().join("release"), "ready").unwrap();
        let result = worker.join().unwrap();
        drop(member);
        result
    })
}

#[cfg(target_os = "linux")]
#[test]
fn exited_unreaped_group_member_does_not_fail_completed_capture() {
    let result = completed_leader_with_group_member(true);
    assert_eq!(result.termination, Termination::Completed, "{result:?}");
    assert_eq!(result.exit_code, Some(0));
}

#[cfg(target_os = "linux")]
#[test]
fn live_group_member_with_closed_streams_cannot_pass() {
    let result = completed_leader_with_group_member(false);
    assert_eq!(result.termination, Termination::CaptureFailed, "{result:?}");
}

#[test]
fn cancellation_and_missing_program_are_explicit() {
    let root = tempfile::tempdir().unwrap();
    let out = root.path().join("missing");
    private_directory(&out).unwrap();
    let mut spec = invocation("exit 0");
    spec.program = "visioncraft-no-such-program-20261103".into();
    let result = run_command(&spec, root.path(), &out, &AtomicBool::new(false), false).unwrap();
    assert_eq!(result.termination, Termination::Unavailable);
    let out = root.path().join("cancelled");
    private_directory(&out).unwrap();
    let cancelled = AtomicBool::new(false);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(60));
            cancelled.store(true, Ordering::SeqCst);
        });
        let result = run_command(
            &invocation("sleep 30"),
            root.path(),
            &out,
            &cancelled,
            false,
        )
        .unwrap();
        assert_eq!(result.termination, Termination::Interrupted);
    });
}

#[test]
fn private_output_is_fresh_and_does_not_follow_symlinks() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = tempfile::tempdir().unwrap();
    let out = root.path().join("attempt");
    private_directory(&out).unwrap();
    assert_eq!(
        fs::metadata(&out).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert!(private_directory(&out).is_err());
    let other = tempfile::tempdir().unwrap();
    symlink(other.path(), root.path().join("foreign")).unwrap();
    assert!(private_directory(&root.path().join("foreign/new")).is_err());
    assert!(!other.path().join("new").exists());
    let victim = other.path().join("victim");
    fs::write(&victim, "unchanged").unwrap();
    symlink(&victim, out.join("stdout.log")).unwrap();
    assert!(
        run_command(
            &invocation("printf altered"),
            root.path(),
            &out,
            &AtomicBool::new(false),
            false
        )
        .is_err()
    );
    assert_eq!(fs::read_to_string(victim).unwrap(), "unchanged");
}
