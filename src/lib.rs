//! Explicit quality contracts, without implicit execution or claims of correctness.
//!
//! The core operates on supplied data and bytes. It reads no files, queries no
//! clocks, launches no processes and makes no network calls. A passing gate means
//! the recorded policy was satisfied, not that a program is secure or deployed.
//! Finding dispositions and model-family identity use Sealmap's existing contracts.
//!
//! ```
//! use visioncraft::{Artifact, VerifiedArtifacts};
//! let bytes = b"recorded output\n";
//! let artifact = Artifact {
//!     path: "attempt/stdout.txt".into(),
//!     sha256: visioncraft::sha256(bytes),
//! };
//! let mut verified = VerifiedArtifacts::default();
//! verified.insert(&artifact, bytes)?;
//! assert!(verified.contains(&artifact));
//! # Ok::<(), visioncraft::ContractError>(())
//! ```

pub mod evaluate;
pub mod evidence;
pub mod plan;

pub use evaluate::{
    CheckOutcome, CheckReport, EvaluationContext, ExpectationReport, GateOutcome, GateReport,
    evaluate,
};
pub use evidence::{
    Actor, AuditReceipt, AuditVerdict, ExecutionReceipt, ManualObservation, ProcessStatus,
    ReportedResult, VerifiedArtifacts,
};
pub use plan::{
    CheckSpec, Evaluator, EvidenceCategory, Expectation, GatePolicy, Invocation, QualityPlan,
    Stabilizer, Subject,
};
pub use sealmap_corpus::review::Artifact;
pub use sealmap_corpus::route::FamilyCatalogue;

use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};

/// Invalid input or evidence, without embedding raw document contents in errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContractError {
    /// JSON could not be deserialised under the strict typed schema.
    #[error("invalid JSON contract at line {line}, column {column}")]
    Json {
        /// One-based input line, or zero if unavailable.
        line: usize,
        /// Input column reported by the JSON decoder.
        column: usize,
    },
    /// A typed contract violates an invariant.
    #[error("invalid contract: {0}")]
    Invalid(String),
}

/// Hash exact supplied bytes with the standard SHA-256 implementation.
pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Decode a typed JSON contract without including raw input in diagnostics.
///
/// Structs in this crate reject unknown and duplicate fields. This function does
/// not replace their semantic validation methods.
///
/// # Errors
/// Returns a location-only error when decoding fails.
pub fn from_json<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ContractError> {
    serde_json::from_slice(bytes).map_err(|e| ContractError::Json {
        line: e.line(),
        column: e.column(),
    })
}

pub(crate) fn require(condition: bool, message: impl Into<String>) -> Result<(), ContractError> {
    if condition {
        Ok(())
    } else {
        Err(ContractError::Invalid(message.into()))
    }
}

pub(crate) fn nonempty(value: &str) -> bool {
    !value.trim().is_empty() && !value.contains('\0')
}

pub(crate) fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

pub(crate) fn hex(value: &str, lengths: &[usize]) -> bool {
    lengths.contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(crate) fn artifact_shape(artifact: &Artifact) -> Result<(), ContractError> {
    sealmap_corpus::review::validate_artifact(artifact).map_err(|_| {
        ContractError::Invalid("artifact requires a confined relative path and SHA-256".into())
    })
}

pub(crate) fn relative_path(path: &str) -> bool {
    artifact_shape(&Artifact {
        path: path.into(),
        sha256: "0".repeat(64),
    })
    .is_ok()
}

/// Bounded process execution with private evidence capture.
#[cfg(feature = "process")]
pub mod process;

/// Versioned adapter for an independently installed Deepsec reviewer.
#[cfg(feature = "cli")]
pub mod deepsec;

/// Explicit command-line and filesystem adapter for the quality core.
#[cfg(feature = "cli")]
pub mod cli;

#[cfg(feature = "cli")]
mod git_metadata;
