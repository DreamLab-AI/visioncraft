//! External reviewer protocol controls using a model-free executable double.
#![cfg(all(target_os = "linux", not(target_env = "uclibc"), feature = "cli"))]
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    bin: PathBuf,
    list: PathBuf,
}
impl Fixture {
    fn new(case: &str) -> Self {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("repo");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("input.py"), "print('safe fixture')\n").unwrap();
        fs::write(root.join(".fixture-case"), case).unwrap();
        for args in [
            vec!["init", "-q"],
            vec!["add", "input.py"],
            vec![
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "-qm",
                "baseline",
            ],
        ] {
            assert!(
                Command::new("git")
                    .args(args)
                    .current_dir(&root)
                    .status()
                    .unwrap()
                    .success()
            );
        }
        let bin = temp.path().join("bin");
        fs::create_dir(&bin).unwrap();
        let fake = bin.join("deepsec");
        fs::write(&fake, include_bytes!("fixtures/deepsec.py")).unwrap();
        fs::set_permissions(fake, fs::Permissions::from_mode(0o700)).unwrap();
        let list = temp.path().join("files.txt");
        fs::write(&list, "input.py\n").unwrap();
        Self {
            _temp: temp,
            root,
            bin,
            list,
        }
    }
    fn run(&self, mode: &[&str], policy: Value) -> Output {
        let path = std::env::join_paths(std::iter::once(self.bin.clone()).chain(
            std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()),
        ))
        .unwrap();
        Command::new(env!("CARGO_BIN_EXE_visioncraft"))
            .arg("deepsec")
            .args(mode)
            .arg("--root")
            .arg(&self.root)
            .arg("--policy-json")
            .arg(policy.to_string())
            .env("PATH", path)
            .output()
            .unwrap()
    }
    fn gate(&self) -> Output {
        self.run(
            &["--files-from", self.list.to_str().unwrap()],
            json!({"max_duration":"10s"}),
        )
    }
    fn receipt(&self) -> Value {
        let directory = fs::read_dir(self.root.join(".deepsec-gate/reports"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        serde_json::from_slice(&fs::read(directory.join("receipt.json")).unwrap()).unwrap()
    }
}
#[test]
fn actual_review_and_threshold_have_positive_controls() {
    for (case, code, result) in [
        ("pass", 0, "PASS"),
        ("block", 1, "BLOCK"),
        ("low", 0, "PASS"),
    ] {
        let f = Fixture::new(case);
        let out = f.gate();
        assert_eq!(
            out.status.code(),
            Some(code),
            "{case}: {} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let receipt = f.receipt();
        assert_eq!(receipt["result"], result);
        assert_eq!(
            receipt["observed_review"]["observed_files"],
            json!(["input.py"])
        );
    }
}

#[test]
fn metadata_does_not_execute_repository_fsmonitor() {
    let f = Fixture::new("pass");
    let marker = f.root.join("unexpected-hook");
    assert!(
        Command::new("git")
            .args(["config", "core.fsmonitor", "touch unexpected-hook"])
            .current_dir(&f.root)
            .status()
            .unwrap()
            .success()
    );
    let output = f.gate();
    assert_eq!(output.status.code(), Some(0));
    assert!(
        !marker.exists(),
        "metadata inspection executed repository config"
    );
}

#[test]
fn dirty_metadata_and_working_selection_do_not_execute_clean_filters() {
    for working_selection in [false, true] {
        let f = Fixture::new("pass");
        assert!(
            Command::new("git")
                .args([
                    "config",
                    "filter.fixture.clean",
                    "sh -c 'touch FILTER_EXECUTED; cat'"
                ])
                .current_dir(&f.root)
                .status()
                .unwrap()
                .success()
        );
        fs::write(f.root.join(".gitattributes"), "input.py filter=fixture\n").unwrap();
        fs::write(f.root.join("input.py"), "print('changed fixture')\n").unwrap();
        assert!(!f.root.join("FILTER_EXECUTED").exists());
        let output = if working_selection {
            f.run(&["--diff-working"], json!({"max_duration":"10s"}))
        } else {
            f.gate()
        };
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !f.root.join("FILTER_EXECUTED").exists(),
            "read-only dirty/selection metadata invoked a repository clean filter (working_selection={working_selection})"
        );
    }
}

#[test]
fn selected_dirty_scope_does_not_inspect_unselected_filter_managed_assets() {
    let f = Fixture::new("pass");
    fs::write(f.root.join("asset.bin"), "version https://git-lfs.github.com/spec/v1\noid sha256:0000000000000000000000000000000000000000000000000000000000000000\nsize 34603008\n").unwrap();
    for args in [
        vec!["add", "asset.bin"],
        vec![
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "asset pointer",
        ],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(&f.root)
                .status()
                .unwrap()
                .success()
        );
    }
    assert!(
        Command::new("git")
            .args([
                "config",
                "filter.lfs.clean",
                "sh -c 'touch UNEXPECTED_LFS; cat'"
            ])
            .current_dir(&f.root)
            .status()
            .unwrap()
            .success()
    );
    fs::write(f.root.join(".gitattributes"), "asset.bin filter=lfs\n").unwrap();
    fs::File::create(f.root.join("asset.bin"))
        .unwrap()
        .set_len(33 * 1024 * 1024)
        .unwrap();
    let output = f.gate();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!f.root.join("UNEXPECTED_LFS").exists());
    let receipt = f.receipt();
    assert_eq!(receipt["dirty_scope"], "selected-inputs");
    assert_eq!(receipt["dirty"], false);
    assert_eq!(
        receipt["observed_review"]["observed_files"],
        json!(["input.py"])
    );
}

#[test]
fn tracked_review_workspace_is_refused_before_external_reviewer_version_probe() {
    let f = Fixture::new("pass");
    fs::create_dir(f.root.join(".deepsec-gate")).unwrap();
    let ignored = b"# tracked fixture sentinel\n*\n";
    let configuration = b"// tracked configuration sentinel\nexport default {};\n";
    fs::write(f.root.join(".deepsec-gate/.gitignore"), ignored).unwrap();
    fs::write(
        f.root.join(".deepsec-gate/deepsec.config.mjs"),
        configuration,
    )
    .unwrap();
    assert!(
        Command::new("git")
            .args([
                "add",
                "-f",
                ".deepsec-gate/.gitignore",
                ".deepsec-gate/deepsec.config.mjs"
            ])
            .current_dir(&f.root)
            .status()
            .unwrap()
            .success()
    );
    fs::write(
        f.bin.join("deepsec"),
        format!(
            "#!/bin/sh\ntouch '{}'\nexit 0\n",
            f.root.join("VERSION_EXECUTED").display()
        ),
    )
    .unwrap();
    let output = f.gate();
    assert_eq!(output.status.code(), Some(78));
    assert!(
        !f.root.join("VERSION_EXECUTED").exists(),
        "tracked state reached the external reviewer"
    );
    assert_eq!(
        fs::read(f.root.join(".deepsec-gate/.gitignore")).unwrap(),
        ignored
    );
    assert_eq!(
        fs::read(f.root.join(".deepsec-gate/deepsec.config.mjs")).unwrap(),
        configuration
    );
}

#[test]
fn review_state_refuses_nested_symlinks_before_external_tools() {
    use std::os::unix::fs::symlink;
    for nested in ["data", "data/repo/files", ".gitignore"] {
        let f = Fixture::new("pass");
        let state = f.root.join(".deepsec-gate");
        let path = state.join(nested);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let outside = f._temp.path().join("outside");
        fs::create_dir(&outside).unwrap();
        symlink(&outside, path).unwrap();
        let output = f.gate();
        assert_eq!(
            output.status.code(),
            Some(78),
            "{nested}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read_dir(outside).unwrap().count(), 0);
    }
}

#[test]
fn workspace_root_rejects_unmanaged_loader_inputs() {
    for name in [".env", "node_modules", "deepsec.config.ts"] {
        let f = Fixture::new("pass");
        let state = f.root.join(".deepsec-gate");
        fs::create_dir(&state).unwrap();
        fs::write(state.join(name), "untrusted loader input").unwrap();
        let output = f.gate();
        assert_eq!(output.status.code(), Some(78), "{name}");
    }
}

#[test]
fn historical_reports_are_not_revisited_as_active_reviewer_state() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new("pass");
    let old = f.root.join(".deepsec-gate/reports/archived");
    fs::create_dir_all(&old).unwrap();
    let outside = f._temp.path().join("old-capture");
    fs::write(&outside, "preserved evidence").unwrap();
    symlink(&outside, old.join("old-output")).unwrap();
    let output = f.gate();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(fs::read_to_string(outside).unwrap(), "preserved evidence");
}
#[test]
fn no_empty_partial_or_ambiguous_review_can_pass() {
    for case in [
        "runtime",
        "no-evidence",
        "incomplete",
        "refused",
        "partial",
        "unknown-refusal",
        "missing-skipped",
        "null-skipped",
        "scalar-skipped",
        "zero-output",
        "missing-model",
        "stale",
        "wrong-hash",
        "drift",
        "export-failure",
        "malformed",
        "duplicate-key",
    ] {
        let f = Fixture::new(case);
        let out = f.gate();
        assert_eq!(
            out.status.code(),
            Some(70),
            "{case}: {} {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(f.receipt()["result"], "ERROR", "{case}");
    }
}
#[test]
fn unsupported_version_and_timeout_preserve_non_pass_states() {
    let f = Fixture::new("unsupported");
    assert_eq!(f.gate().status.code(), Some(78));
    assert_eq!(f.receipt()["result"], "UNAVAILABLE");
    let f = Fixture::new("timeout");
    let out = f.run(
        &["--files-from", f.list.to_str().unwrap()],
        json!({"max_duration":"0.5s"}),
    );
    assert_eq!(out.status.code(), Some(124));
    assert_eq!(f.receipt()["result"], "TIMEOUT");
}
#[test]
fn scan_only_is_not_model_review_and_empty_scope_cannot_pass() {
    let f = Fixture::new("pass");
    let out = f.run(&["--scan-only"], json!({}));
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(f.receipt()["result"], "SCANNED");
    let f = Fixture::new("pass");
    fs::write(&f.list, "").unwrap();
    assert_eq!(f.gate().status.code(), Some(78));
}
#[test]
fn staged_selection_must_match_reviewed_working_bytes() {
    let f = Fixture::new("pass");
    fs::write(f.root.join("input.py"), "staged\n").unwrap();
    assert!(
        Command::new("git")
            .args(["add", "input.py"])
            .current_dir(&f.root)
            .status()
            .unwrap()
            .success()
    );
    fs::write(f.root.join("input.py"), "different\n").unwrap();
    let out = f.run(&["--diff-staged"], json!({}));
    assert_eq!(out.status.code(), Some(78));
    assert!(
        f.receipt()["error"]
            .as_str()
            .unwrap()
            .contains("staged inputs")
    );
}
#[test]
fn dry_run_is_read_only_and_carries_names_not_credentials() {
    let f = Fixture::new("pass");
    let out=f.run(&["--files-from",f.list.to_str().unwrap(),"--dry-run"],json!({"model_auth":"direct","ai_provider":"anthropic","ai_api_key_env":"FIXTURE_CREDENTIAL"}));
    assert!(out.status.success());
    assert!(!f.root.join(".deepsec-gate").exists());
    let plan: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(plan["policy"]["ai_api_key_env"], "FIXTURE_CREDENTIAL");
    let out = f.run(
        &["--files-from", f.list.to_str().unwrap(), "--dry-run"],
        json!({"enabled":false}),
    );
    assert_eq!(out.status.code(), Some(78));
    assert!(!f.root.join(".deepsec-gate").exists());
}
