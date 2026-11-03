//! Executed native CLI contracts using local temporary Git fixtures only.
#![cfg(all(unix, feature = "cli"))]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use visioncraft::{ExecutionReceipt, QualityPlan};

fn git(root: &Path, args: &[&str]) -> Output {
    let output = Command::new("git")
        .args(["-c", "core.hooksPath=/dev/null", "-C"])
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "-q"]);
    fs::write(root.path().join("source.txt"), "original\n").unwrap();
    git(root.path(), &["add", "source.txt"]);
    git(
        root.path(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "baseline",
        ],
    );
    root
}

fn cli(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_visioncraft"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}

fn success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn init(root: &Path, script: &str) {
    success(&cli(
        root,
        &[
            "plan",
            "init",
            "--out",
            "plan.json",
            "--input",
            "source.txt",
            "--configuration",
            "native fixture",
            "--",
            "/bin/sh",
            "-c",
            script,
        ],
    ));
}

fn run(root: &Path) -> Output {
    cli(root, &["run", "plan.json", "--all", "--out", "evidence"])
}

fn evaluate(root: &Path) -> Output {
    cli(
        root,
        &["evaluate", "plan.json", "--receipts", "evidence", "--json"],
    )
}

#[test]
fn plan_run_and_evaluate_use_real_output_but_do_not_invent_scope_or_tests() {
    use std::os::unix::fs::PermissionsExt;
    let root = fixture();
    init(root.path(), "printf 'real-output'; printf 'real-error' >&2");
    success(&cli(root.path(), &["plan", "check", "plan.json", "--json"]));
    success(&run(root.path()));
    let receipt = ExecutionReceipt::parse(
        &fs::read(root.path().join("evidence/check-0000/receipt.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(receipt.observed_scope, None);
    assert!(receipt.observed_tests.is_empty());
    let executable = Path::new("/bin/sh").canonicalize().unwrap();
    assert_eq!(receipt.executable, executable.to_str().unwrap());
    assert_eq!(
        receipt.executable_sha256.as_deref(),
        Some(visioncraft::sha256(&fs::read(executable).unwrap()).as_str())
    );
    assert_eq!(
        fs::read(root.path().join("evidence/check-0000/stdout.log")).unwrap(),
        b"real-output"
    );
    assert_eq!(
        fs::read(root.path().join("evidence/check-0000/stderr.log")).unwrap(),
        b"real-error"
    );
    let result = evaluate(root.path());
    success(&result);
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["outcome"], "passed");
    assert_eq!(
        fs::metadata(root.path().join("evidence"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(root.path().join("evidence/check-0000/stdout.log"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    git(
        root.path(),
        &["check-ignore", "evidence/check-0000/receipt.json"],
    );
    success(&cli(
        root.path(),
        &[
            "expectations",
            "check",
            "plan.json",
            "--receipts",
            "evidence",
            "--json",
        ],
    ));
    success(&cli(
        root.path(),
        &[
            "gate",
            "evaluate",
            "plan.json",
            "--receipts",
            "evidence",
            "--json",
        ],
    ));
}

#[test]
fn failing_exit_retains_receipts_and_blocks_the_declared_gate() {
    let root = fixture();
    init(root.path(), "printf 'assertion-reached' >&2; exit 9");
    assert_eq!(run(root.path()).status.code(), Some(1));
    let result = evaluate(root.path());
    assert_eq!(result.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["outcome"], "blocked");
    assert!(
        root.path()
            .join("evidence/check-0000/receipt.json")
            .is_file()
    );
}

#[test]
fn source_drift_before_execution_refuses_to_launch_and_after_execution_is_stale() {
    let root = fixture();
    init(root.path(), "printf reached > SHOULD_NOT_RUN");
    fs::write(root.path().join("source.txt"), "changed\n").unwrap();
    assert_eq!(run(root.path()).status.code(), Some(1));
    assert!(!root.path().join("SHOULD_NOT_RUN").exists());
    assert!(!root.path().join("evidence").exists());
    fs::write(root.path().join("source.txt"), "original\n").unwrap();
    success(&run(root.path()));
    fs::write(root.path().join("source.txt"), "changed again\n").unwrap();
    let result = evaluate(root.path());
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap()["outcome"],
        "incomplete"
    );
}

#[test]
fn deleting_selected_input_records_actual_empty_after_state() {
    let root = fixture();
    init(root.path(), "rm source.txt");
    assert_eq!(run(root.path()).status.code(), Some(1));
    let receipt = ExecutionReceipt::parse(
        &fs::read(root.path().join("evidence/check-0000/receipt.json")).unwrap(),
    )
    .unwrap();
    assert!(receipt.inputs_after.is_empty());
    assert_eq!(evaluate(root.path()).status.code(), Some(1));
}

#[test]
fn evaluation_never_executes_a_command_recorded_in_an_imported_receipt() {
    let root = fixture();
    init(root.path(), "printf valid");
    success(&run(root.path()));
    let path = root.path().join("evidence/check-0000/receipt.json");
    let mut receipt: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    receipt["invocation"]["args"] = serde_json::json!(["-c", "touch RECEIPT_EXECUTED"]);
    fs::write(&path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    assert!(!evaluate(root.path()).status.success());
    assert!(!root.path().join("RECEIPT_EXECUTED").exists());
}

#[test]
fn altered_or_symlinked_evidence_cannot_borrow_an_existing_success() {
    use std::os::unix::fs::symlink;
    let root = fixture();
    init(root.path(), "printf expected");
    success(&run(root.path()));
    let output = root.path().join("evidence/check-0000/stdout.log");
    fs::write(&output, "altered").unwrap();
    assert!(!evaluate(root.path()).status.success());
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("private"), "PRIVATE_SENTINEL").unwrap();
    fs::remove_file(&output).unwrap();
    symlink(outside.path().join("private"), &output).unwrap();
    let result = evaluate(root.path());
    assert!(!result.status.success());
    assert!(!String::from_utf8_lossy(&result.stderr).contains("PRIVATE_SENTINEL"));
    assert!(!String::from_utf8_lossy(&result.stdout).contains("PRIVATE_SENTINEL"));
}

#[test]
fn input_output_and_cwd_symlinks_are_refused_with_ordinary_root_alias_control() {
    use std::os::unix::fs::symlink;
    let root = fixture();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("file"), "outside").unwrap();
    symlink(outside.path().join("file"), root.path().join("linked.txt")).unwrap();
    assert!(
        !cli(
            root.path(),
            &[
                "plan",
                "init",
                "--out",
                "bad.json",
                "--input",
                "linked.txt",
                "--configuration",
                "fixture",
                "--",
                "/bin/true"
            ]
        )
        .status
        .success()
    );
    assert!(!root.path().join("bad.json").exists());
    init(root.path(), "printf ok");
    symlink(outside.path(), root.path().join("redirect")).unwrap();
    assert!(
        !cli(
            root.path(),
            &["run", "plan.json", "--all", "--out", "redirect/evidence"]
        )
        .status
        .success()
    );
    assert!(!outside.path().join("evidence").exists());
    let alias_root = tempfile::tempdir().unwrap();
    symlink(root.path(), alias_root.path().join("source")).unwrap();
    success(&cli(
        root.path(),
        &[
            "run",
            "plan.json",
            "--root",
            alias_root.path().join("source").to_str().unwrap(),
            "--all",
            "--out",
            "evidence",
        ],
    ));
}

#[test]
fn dirty_snapshots_are_exact_private_ignored_and_required_for_execution() {
    let root = fixture();
    fs::write(root.path().join("source.txt"), "PRIVATE_DIRTY_SOURCE\n").unwrap();
    init(root.path(), "printf ok");
    let plan = QualityPlan::parse(&fs::read(root.path().join("plan.json")).unwrap()).unwrap();
    assert!(plan.subject.dirty);
    let snapshot = plan.subject.working_tree.unwrap();
    let bytes = fs::read(root.path().join(&snapshot.path)).unwrap();
    assert_eq!(visioncraft::sha256(&bytes), snapshot.sha256);
    git(root.path(), &["check-ignore", &snapshot.path]);
    success(&run(root.path()));
    success(&evaluate(root.path()));
    fs::write(root.path().join(snapshot.path), "corrupted").unwrap();
    assert!(
        !cli(
            root.path(),
            &["run", "plan.json", "--all", "--out", "another-attempt"]
        )
        .status
        .success()
    );
    assert!(!root.path().join("another-attempt").exists());
}

#[test]
fn plan_and_evaluation_do_not_trigger_repository_clean_filters() {
    let root = fixture();
    fs::write(
        root.path().join(".gitattributes"),
        "source.txt filter=canary\n",
    )
    .unwrap();
    git(root.path(), &["add", ".gitattributes"]);
    git(
        root.path(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "filter declaration",
        ],
    );
    git(
        root.path(),
        &[
            "config",
            "filter.canary.clean",
            "touch FILTER_EXECUTED; cat",
        ],
    );
    fs::write(root.path().join("source.txt"), "changed\n").unwrap();
    git(
        root.path(),
        &["hash-object", "--path=source.txt", "source.txt"],
    ); // positive control: the configured filter is executable.
    assert!(root.path().join("FILTER_EXECUTED").exists());
    fs::remove_file(root.path().join("FILTER_EXECUTED")).unwrap();
    init(root.path(), "printf ok");
    success(&run(root.path()));
    success(&evaluate(root.path()));
    assert!(!root.path().join("FILTER_EXECUTED").exists());
}

#[test]
fn fresh_directory_and_one_selection_are_required() {
    let root = fixture();
    init(root.path(), "printf ok");
    success(&run(root.path()));
    let before = fs::read(root.path().join("evidence/check-0000/receipt.json")).unwrap();
    assert!(!run(root.path()).status.success());
    assert_eq!(
        fs::read(root.path().join("evidence/check-0000/receipt.json")).unwrap(),
        before
    );
    assert_eq!(
        cli(
            root.path(),
            &[
                "run",
                "plan.json",
                "--all",
                "--check",
                "check",
                "--out",
                "unused"
            ]
        )
        .status
        .code(),
        Some(2)
    );
    assert!(!root.path().join("unused").exists());
}

#[test]
fn signal_termination_still_preserves_a_valid_nonpassing_receipt() {
    let root = fixture();
    init(root.path(), "kill -TERM $$");
    let result = run(root.path());
    assert_eq!(result.status.code(), Some(130));
    let receipt = ExecutionReceipt::parse(
        &fs::read(root.path().join("evidence/check-0000/receipt.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(receipt.process, visioncraft::ProcessStatus::Interrupted);
    assert!(receipt.signal.is_some());
    assert!(!evaluate(root.path()).status.success());
}

#[test]
fn manual_context_and_explicit_current_subject_are_byte_checked() {
    let root = fixture();
    init(root.path(), "printf ok");
    let plan_path = root.path().join("plan.json");
    let mut plan = QualityPlan::parse(&fs::read(&plan_path).unwrap()).unwrap();
    plan.expectations.push(visioncraft::Expectation {
        id: "MANUAL-1".into(),
        claim: "fixture observation".into(),
        required: true,
        category: visioncraft::EvidenceCategory::Manual,
        checks: vec![],
        regression_critical: false,
        stabilizer: None,
        positive_control: None,
        independent_audit: false,
        limitation: Some("synthetic manual-observation fixture".into()),
    });
    fs::write(&plan_path, serde_json::to_vec(&plan).unwrap()).unwrap();
    success(&run(root.path()));
    assert_eq!(evaluate(root.path()).status.code(), Some(1));
    let receipt = ExecutionReceipt::parse(
        &fs::read(root.path().join("evidence/check-0000/receipt.json")).unwrap(),
    )
    .unwrap();
    let record = b"fixture observer recorded the scoped state";
    fs::write(root.path().join("evidence/manual.txt"), record).unwrap();
    let observation = visioncraft::ManualObservation {
        expectation_id: "MANUAL-1".into(),
        plan_sha256: plan.digest().unwrap(),
        subject: plan.subject.clone(),
        observer: "fixture observer".into(),
        observed: "scoped fixture state".into(),
        verdict: visioncraft::AuditVerdict::Supported,
        artifact: visioncraft::Artifact {
            path: "manual.txt".into(),
            sha256: visioncraft::sha256(record),
        },
        captured_at_ms: receipt.finished_at_ms,
    };
    fs::write(
        root.path().join("evidence/context.json"),
        serde_json::to_vec(&serde_json::json!({"manual_observations":[observation]})).unwrap(),
    )
    .unwrap();
    fs::write(
        root.path().join("current.json"),
        serde_json::to_vec(&plan.subject).unwrap(),
    )
    .unwrap();
    let args = [
        "evaluate",
        "plan.json",
        "--receipts",
        "evidence",
        "--context",
        "evidence/context.json",
        "--current-subject",
        "current.json",
        "--json",
    ];
    success(&cli(root.path(), &args));
    fs::write(root.path().join("evidence/manual.txt"), b"changed").unwrap();
    assert!(!cli(root.path(), &args).status.success());
}

#[test]
fn ambient_git_environment_cannot_redirect_metadata_to_a_sibling_repo() {
    let root = fixture();
    let foreign = fixture();
    fs::write(foreign.path().join("source.txt"), "foreign bytes\n").unwrap();
    git(
        foreign.path(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-am",
            "foreign",
        ],
    );
    let result = Command::new(env!("CARGO_BIN_EXE_visioncraft"))
        .current_dir(root.path())
        .env("GIT_DIR", foreign.path().join(".git"))
        .env("GIT_WORK_TREE", foreign.path())
        .args([
            "plan",
            "init",
            "--out",
            "plan.json",
            "--input",
            "source.txt",
            "--configuration",
            "fixture",
            "--",
            "/bin/true",
        ])
        .output()
        .unwrap();
    success(&result);
    let plan = QualityPlan::parse(&fs::read(root.path().join("plan.json")).unwrap()).unwrap();
    let expected = String::from_utf8(git(root.path(), &["rev-parse", "HEAD"]).stdout).unwrap();
    assert_eq!(plan.subject.revision, expected.trim());
    assert!(!plan.subject.dirty);
}

#[test]
fn committed_blob_size_is_bounded_before_requesting_blob_contents() {
    use std::os::unix::fs::PermissionsExt;
    let root = fixture();
    let tools = tempfile::tempdir().unwrap();
    let actual_git = String::from_utf8(
        Command::new("sh")
            .args(["-c", "command -v git"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let script = format!(
        "#!/bin/sh\ncase \"$*\" in\n  *'cat-file -s'*) printf '1073741825\\n';;\n  *'cat-file blob'*) touch SHOULD_NOT_READ_BLOB; exit 1;;\n  *) exec '{}' \"$@\";;\nesac\n",
        actual_git.trim()
    );
    fs::write(tools.path().join("git"), script).unwrap();
    fs::set_permissions(tools.path().join("git"), fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        tools.path().display(),
        std::env::var("PATH").unwrap()
    );
    let result = Command::new(env!("CARGO_BIN_EXE_visioncraft"))
        .current_dir(root.path())
        .env("PATH", path)
        .args([
            "plan",
            "init",
            "--out",
            "plan.json",
            "--input",
            "source.txt",
            "--configuration",
            "fixture",
            "--",
            "/bin/true",
        ])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(!root.path().join("SHOULD_NOT_READ_BLOB").exists());
    assert!(!root.path().join("plan.json").exists());
}

#[test]
fn cancellation_reaches_bounded_metadata_inspection_before_any_check_runs() {
    use std::os::unix::fs::PermissionsExt;
    let root = fixture();
    let tools = tempfile::tempdir().unwrap();
    fs::write(tools.path().join("git"), "#!/bin/sh\nsleep 30\n").unwrap();
    fs::set_permissions(tools.path().join("git"), fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        tools.path().display(),
        std::env::var("PATH").unwrap()
    );
    let mut child = Command::new(env!("CARGO_BIN_EXE_visioncraft"))
        .current_dir(root.path())
        .env("PATH", path)
        .args([
            "plan",
            "init",
            "--out",
            "plan.json",
            "--input",
            "source.txt",
            "--configuration",
            "fixture",
            "--",
            "/bin/true",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(child.id() as i32),
        nix::sys::signal::Signal::SIGTERM,
    )
    .unwrap();
    let start = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > std::time::Duration::from_secs(3) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("metadata child did not respond to cancellation");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    assert_eq!(status.code(), Some(130));
    assert!(!root.path().join("plan.json").exists());
}
