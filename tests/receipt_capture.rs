//! Receipt-schema boundaries: capture-failed receipts versus capture claims.

use visioncraft::ExecutionReceipt;

fn receipt(process: &str, capture_complete: bool) -> Vec<u8> {
    let json = format!(
        r#"{{
  "version": 1,
  "attempt_id": "receipt-attempt",
  "plan_sha256": "{plan}",
  "check_id": "receipt-check",
  "subject": {{
    "repository": "example",
    "revision": "{revision}",
    "configuration": "debug",
    "dirty": false,
    "working_tree": null,
    "inputs": [
      {{
        "path": "src/lib.rs",
        "sha256": "{input}"
      }}
    ]
  }},
  "invocation": {{
    "program": "cargo",
    "args": ["--version"],
    "cwd": ".",
    "env": [],
    "timeout_ms": 60000,
    "max_output_bytes": 1048576
  }},
  "executable": "cargo",
  "executable_sha256": null,
  "tool_version": null,
  "started_at_ms": 1000,
  "finished_at_ms": 2000,
  "elapsed_ms": 1000,
  "process": "{process}",
  "exit_code": null,
  "signal": null,
  "capture_complete": {capture_complete},
  "stdout": null,
  "stderr": null,
  "inputs_after": [],
  "observed_scope": null,
  "observed_tests": [],
  "reported": null,
  "diagnostic": "capture stopped at the configured byte limit"
}}"#,
        plan = "a".repeat(64),
        revision = "a".repeat(40),
        input = "b".repeat(64),
    );
    json.into_bytes()
}

#[test]
fn capture_failed_receipt_without_a_complete_capture_claim_parses() {
    let bytes = receipt("capture-failed", false);
    assert!(ExecutionReceipt::parse(&bytes).is_ok());
}

#[test]
fn capture_failed_receipt_claiming_complete_capture_is_refused() {
    let bytes = receipt("capture-failed", true);
    assert!(ExecutionReceipt::parse(&bytes).is_err());
}

#[test]
fn interrupted_receipt_with_complete_capture_still_parses() {
    let bytes = receipt("interrupted", true);
    assert!(ExecutionReceipt::parse(&bytes).is_ok());
}
