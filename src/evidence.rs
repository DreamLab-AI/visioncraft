//! Immutable run and audit records, with caller-supplied byte verification.

use crate::plan::validate_input_paths;
use crate::{
    Artifact, CheckOutcome, ContractError, Invocation, Subject, artifact_shape, hex, identifier,
    nonempty, require, sha256,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// What happened to the process, independently of an evaluator's verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessStatus {
    /// Process exited and was reaped; the exit code still needs evaluation.
    Completed,
    /// Required tool or execution capability was unavailable.
    Unavailable,
    /// Deadline elapsed and cleanup was attempted.
    TimedOut,
    /// Caller cancellation or signal interrupted execution.
    Interrupted,
    /// The child could not be started.
    FailedToStart,
    /// Raw output could not be captured completely or exceeded its limit.
    CaptureFailed,
}

/// Named adapter's interpretation, bound to its captured report bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportedResult {
    /// Adapter identity expected by the plan.
    pub adapter: String,
    /// Completed assessment under the adapter's declared policy.
    pub outcome: CheckOutcome,
    /// Explanation, including threshold or interpretation limits.
    pub reason: String,
    /// Raw adapter report; it must be byte-verified before credit is given.
    pub artifact: Artifact,
}

/// Receipt from one explicit invocation; no field is executed by the evaluator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReceipt {
    /// Schema version, currently exactly 1.
    pub version: u32,
    /// Unique execution identity, not inferred from a filename.
    pub attempt_id: String,
    /// Exact typed plan digest from [`crate::QualityPlan::digest`].
    pub plan_sha256: String,
    /// Check identifier within that plan.
    pub check_id: String,
    /// Examined source and configuration at invocation start.
    pub subject: Subject,
    /// Actual requested argv/environment names/resource bounds.
    pub invocation: Invocation,
    /// Resolved executable identity, or requested name when startup failed.
    pub executable: String,
    /// Optional hash of the resolved executable; not an attestation.
    pub executable_sha256: Option<String>,
    /// Version reported by the tool, when available.
    pub tool_version: Option<String>,
    /// Start time as milliseconds since the Unix epoch.
    pub started_at_ms: u64,
    /// Finish time as milliseconds since the Unix epoch.
    pub finished_at_ms: u64,
    /// Duration measured with a monotonic clock.
    pub elapsed_ms: u64,
    /// Process-level outcome, before policy interpretation.
    pub process: ProcessStatus,
    /// Reaped exit code when one exists.
    pub exit_code: Option<i32>,
    /// Terminating signal when one exists.
    pub signal: Option<i32>,
    /// Whether all raw output was retained within the configured bound.
    pub capture_complete: bool,
    /// Captured standard output, absent only when execution/capture failed.
    pub stdout: Option<Artifact>,
    /// Captured standard error, absent only when execution/capture failed.
    pub stderr: Option<Artifact>,
    /// Input hashes after execution, to detect mutation during the check.
    /// Empty records deletion of every selected input, never successful stability.
    pub inputs_after: Vec<Artifact>,
    /// Observed source coverage; `None` explicitly means unknown.
    pub observed_scope: Option<Vec<String>>,
    /// Test identifiers actually reported as executed, not just requested.
    pub observed_tests: Vec<String>,
    /// Optional evidence from the evaluator named in the plan.
    pub reported: Option<ReportedResult>,
    /// Diagnostic for failed/skipped execution; may also describe limits.
    pub diagnostic: Option<String>,
}

impl ExecutionReceipt {
    /// Decode and validate a receipt without executing any recorded command.
    ///
    /// # Errors
    /// Rejects malformed/unknown/duplicate fields and inconsistent process data.
    pub fn parse(bytes: &[u8]) -> Result<Self, ContractError> {
        let receipt: Self = crate::from_json(bytes)?;
        receipt.validate()?;
        Ok(receipt)
    }

    /// Check receipt consistency; matching a plan and bytes is a separate step.
    ///
    /// # Errors
    /// Refuses incomplete identities, impossible timestamps and success records
    /// lacking complete capture or an exit code.
    pub fn validate(&self) -> Result<(), ContractError> {
        require(self.version == 1, "unsupported receipt version")?;
        require(
            identifier(&self.attempt_id) && identifier(&self.check_id),
            "invalid receipt identifier",
        )?;
        require(hex(&self.plan_sha256, &[64]), "invalid plan digest")?;
        self.subject.validate()?;
        self.invocation.validate()?;
        require(
            nonempty(&self.executable),
            "executable identity is required",
        )?;
        if let Some(hash) = &self.executable_sha256 {
            require(hex(hash, &[64]), "invalid executable digest")?;
        }
        if let Some(version) = &self.tool_version {
            require(nonempty(version), "empty tool version")?;
        }
        require(
            self.finished_at_ms >= self.started_at_ms,
            "finish precedes start",
        )?;
        require(
            !(self.exit_code.is_some() && self.signal.is_some()),
            "exit code and terminating signal are mutually exclusive",
        )?;
        if self.process == ProcessStatus::Completed {
            require(
                self.exit_code.is_some() && self.signal.is_none(),
                "completed execution requires an exit code",
            )?;
            require(
                self.capture_complete && self.stdout.is_some() && self.stderr.is_some(),
                "completed execution requires complete stdout/stderr",
            )?;
        } else {
            require(
                self.diagnostic.as_deref().is_some_and(nonempty),
                "incomplete execution requires a diagnostic",
            )?;
        }
        for a in self.stdout.iter().chain(self.stderr.iter()) {
            artifact_shape(a)?;
        }
        validate_input_paths(&self.inputs_after)?;
        if let Some(paths) = &self.observed_scope {
            let unique: BTreeSet<_> = paths.iter().collect();
            require(
                unique.len() == paths.len() && paths.iter().all(|p| crate::relative_path(p)),
                "invalid or duplicate observed scope",
            )?;
        }
        let tests: BTreeSet<_> = self.observed_tests.iter().collect();
        require(
            tests.len() == self.observed_tests.len() && tests.iter().all(|t| nonempty(t)),
            "invalid or duplicate observed tests",
        )?;
        if let Some(reported) = &self.reported {
            require(
                identifier(&reported.adapter) && nonempty(&reported.reason),
                "reported assessment requires adapter and reason",
            )?;
            artifact_shape(&reported.artifact)?;
        }
        Ok(())
    }
}

/// Recorded actor identity. Labels alone do not attest a served model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    /// Agent/session identity, distinct from a role name.
    pub agent_id: String,
    /// Execution attempt whose evidence this actor produced.
    pub attempt_id: String,
    /// Actual responder in Sealmap's `provider:model` syntax.
    pub model: String,
}

/// Meaning of a recorded audit/manual observation, not a general quality score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuditVerdict {
    /// Scoped observation supports the claim, subject to stated limits.
    Supported,
    /// Observation contradicts the claim.
    Contradicted,
    /// No supported semantic conclusion.
    Inconclusive,
}

/// Post-run assessment; keeping it outside the plan preserves plan immutability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditReceipt {
    /// Expectation assessed.
    pub expectation_id: String,
    /// Plan to which the assessment applies.
    pub plan_sha256: String,
    /// Source/configuration assessed.
    pub subject: Subject,
    /// Evidence producer and its execution.
    pub producer: Actor,
    /// Independent auditor and its probe execution.
    pub reviewer: Actor,
    /// Distinct declared check run as the adversarial probe.
    pub probe_check: String,
    /// Assessment result; a required inconclusive audit is not a pass.
    pub verdict: AuditVerdict,
    /// Recorded claim limits and observation.
    pub observed: String,
    /// Raw audit output or signed/manual decision record.
    pub artifact: Artifact,
    /// Capture time in Unix milliseconds, used with an injected clock.
    pub captured_at_ms: u64,
}

/// Human/manual observation for a behaviour that cannot be executed here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManualObservation {
    /// Expectation assessed.
    pub expectation_id: String,
    /// Plan digest assessed.
    pub plan_sha256: String,
    /// Exact subject examined.
    pub subject: Subject,
    /// Named observer/decision reference; no authority is inferred from a name.
    pub observer: String,
    /// Actual state or effect observed.
    pub observed: String,
    /// Manual assessment.
    pub verdict: AuditVerdict,
    /// Raw observation record.
    pub artifact: Artifact,
    /// Capture time in Unix milliseconds.
    pub captured_at_ms: u64,
}

/// Exact byte references verified in memory, with no filesystem or network IO.
///
/// Callers are responsible for resolving paths within the appropriate evidence
/// root. A hash establishes byte identity, not truth, authority or safe storage.
#[derive(Debug, Clone, Default)]
pub struct VerifiedArtifacts {
    entries: BTreeMap<(String, String), u64>,
}

impl VerifiedArtifacts {
    /// Validate an artifact using Sealmap's contract and verify supplied bytes.
    ///
    /// # Errors
    /// Refuses invalid paths/digests or any mismatch; does not register failures.
    pub fn insert(&mut self, artifact: &Artifact, bytes: &[u8]) -> Result<(), ContractError> {
        artifact_shape(artifact)?;
        require(
            sha256(bytes) == artifact.sha256,
            "artifact bytes do not match SHA-256",
        )?;
        self.entries.insert(
            (artifact.path.clone(), artifact.sha256.clone()),
            bytes.len() as u64,
        );
        Ok(())
    }

    /// Whether this exact path and byte digest have been verified.
    pub fn contains(&self, artifact: &Artifact) -> bool {
        self.entries
            .contains_key(&(artifact.path.clone(), artifact.sha256.clone()))
    }

    /// Byte length of an exact verified artifact, not a claimed receipt length.
    pub fn byte_len(&self, artifact: &Artifact) -> Option<u64> {
        self.entries
            .get(&(artifact.path.clone(), artifact.sha256.clone()))
            .copied()
    }
}
