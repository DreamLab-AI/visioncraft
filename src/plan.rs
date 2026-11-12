//! Typed plans describe explicitly selected checks; parsing never runs them.

use crate::{
    Artifact, ContractError, artifact_shape, from_json, hex, identifier, nonempty, relative_path,
    require, sha256,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Maximum number of declared checks, including optional checks, in one plan.
pub const MAX_PLAN_CHECKS: usize = 1024;
/// Maximum number of declared behavioural expectations in one plan.
pub const MAX_PLAN_EXPECTATIONS: usize = 4096;
/// Maximum number of selected subject input files in one plan.
pub const MAX_PLAN_INPUTS: usize = 16_384;
/// Maximum sum of declared command deadlines: one day, excluding bounded cleanup.
pub const MAX_PLAN_DURATION_MS: u64 = 86_400_000;
/// Maximum sum of declared stdout/stderr capture bounds: one GiB per plan.
pub const MAX_PLAN_OUTPUT_BYTES: u64 = 1_073_741_824;

/// Immutable code and configuration identity examined by a plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    /// Repository/component identity, not automatically dereferenced.
    pub repository: String,
    /// Full lower-case Git object identifier.
    pub revision: String,
    /// Features, target, fixture and other relevant execution configuration.
    pub configuration: String,
    /// Whether the reviewed checkout contains uncommitted changes.
    pub dirty: bool,
    /// Exact diff/snapshot when dirty; absent for a clean checkout.
    pub working_tree: Option<Artifact>,
    /// Selected source, lockfile and configuration bytes, relative to source root.
    pub inputs: Vec<Artifact>,
}

impl Subject {
    /// Check identity, dirty-snapshot and unique input-path requirements.
    ///
    /// # Errors
    /// Returns an error for incomplete or malformed identities.
    pub fn validate(&self) -> Result<(), ContractError> {
        require(nonempty(&self.repository), "subject.repository is required")?;
        require(
            hex(&self.revision, &[40, 64]),
            "subject.revision must be a full Git id",
        )?;
        require(
            nonempty(&self.configuration),
            "subject.configuration is required",
        )?;
        require(
            self.dirty == self.working_tree.is_some(),
            "dirty subject must have exactly one working-tree snapshot",
        )?;
        if let Some(a) = &self.working_tree {
            artifact_shape(a)?;
        }
        validate_inputs(&self.inputs)
    }
}

pub(crate) fn validate_inputs(inputs: &[Artifact]) -> Result<(), ContractError> {
    require(!inputs.is_empty(), "subject inputs cannot be empty")?;
    require(
        inputs.len() <= MAX_PLAN_INPUTS,
        "subject input count exceeds admission ceiling",
    )?;
    validate_input_paths(inputs)
}

pub(crate) fn validate_input_paths(inputs: &[Artifact]) -> Result<(), ContractError> {
    let mut paths = BTreeSet::new();
    for a in inputs {
        artifact_shape(a)?;
        require(paths.insert(&a.path), "duplicate input path")?;
    }
    Ok(())
}

pub(crate) fn same_inputs(a: &[Artifact], b: &[Artifact]) -> bool {
    let values = |items: &[Artifact]| {
        items
            .iter()
            .map(|x| (x.path.clone(), x.sha256.clone()))
            .collect::<BTreeSet<_>>()
    };
    a.len() == b.len() && values(a) == values(b)
}

/// An argv invocation; credentials are referred to by environment names only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Invocation {
    /// Executable name/path. No shell expansion is requested by this type.
    pub program: String,
    /// Literal arguments; callers must not put secrets in ordinary argv.
    pub args: Vec<String>,
    /// Working directory relative to the explicitly selected workspace, or `.`.
    pub cwd: String,
    /// Explicit environment-variable names to pass, never their values.
    pub env: Vec<String>,
    /// Whole invocation deadline in milliseconds, between 1 and 86,400,000.
    pub timeout_ms: u64,
    /// Combined raw output limit in bytes, between 1 and 1 GiB.
    pub max_output_bytes: u64,
}

impl Invocation {
    /// Validate the bounded argv contract without resolving executables or env.
    ///
    /// # Errors
    /// Refuses invalid paths, names, NUL bytes and absent/excessive limits.
    pub fn validate(&self) -> Result<(), ContractError> {
        require(nonempty(&self.program), "invocation.program is required")?;
        require(
            self.args.iter().all(|x| !x.contains('\0')),
            "argv cannot contain NUL",
        )?;
        require(
            self.cwd == "." || relative_path(&self.cwd),
            "cwd must be a confined relative path",
        )?;
        let mut names = BTreeSet::new();
        for name in &self.env {
            require(
                name.as_bytes()
                    .first()
                    .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
                    && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'),
                "invalid environment name",
            )?;
            require(names.insert(name), "duplicate environment name")?;
        }
        require(
            (1..=86_400_000).contains(&self.timeout_ms),
            "timeout_ms outside 1..=86400000",
        )?;
        require(
            (1..=1_073_741_824).contains(&self.max_output_bytes),
            "max_output_bytes outside 1..=1073741824",
        )
    }
}

/// How a completed invocation is interpreted under an explicit policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Evaluator {
    /// A completed, fully captured process passes for these exit codes.
    ExitCode {
        /// Nonempty, unique portable process exit codes in 0..=255.
        success_codes: Vec<i32>,
    },
    /// A named trusted adapter supplies a separate, byte-bound assessment.
    Reported {
        /// Exact adapter identity expected in the receipt.
        adapter: String,
    },
}

/// A check selected by the caller, with its declared source coverage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckSpec {
    /// Unique plan-local identifier.
    pub id: String,
    /// Whether this check must pass for the aggregate gate to pass.
    pub required: bool,
    /// Explicit argv and resource limits.
    pub invocation: Invocation,
    /// Interpretation of completed output.
    pub evaluator: Evaluator,
    /// Nonempty subset of subject input paths claimed by this check.
    pub scope: Vec<String>,
}

/// Type of observation needed to support an expectation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceCategory {
    /// Scoped behaviour observed by executable checks.
    Executable,
    /// A plan/dry-run or other partial observation, with an explicit limitation.
    PartiallyVerifiable,
    /// A named observer records a manual observation, never inferred execution.
    Manual,
}

/// An automated regression reference supported by a successful check receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stabilizer {
    /// Check that actually executes the regression test.
    pub check_id: String,
    /// Exact test identifier; the runner must report this test as executed.
    pub test_id: String,
}

/// Requirements for one behavioural expectation, separate from later evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expectation {
    /// Unique plan-local identifier.
    pub id: String,
    /// Plain statement judged by a reviewer, not semantically parsed by this crate.
    pub claim: String,
    /// Whether unmet evidence prevents the aggregate gate passing.
    pub required: bool,
    /// Required observation category.
    pub category: EvidenceCategory,
    /// Check IDs that support the expectation. May be empty only for a manual
    /// expectation without a required model audit.
    pub checks: Vec<String>,
    /// Requires a linked, executed regression test.
    pub regression_critical: bool,
    /// Actual regression test reference, required when regression-critical.
    pub stabilizer: Option<Stabilizer>,
    /// A distinct successful check that guards against reject-everything fixtures.
    pub positive_control: Option<String>,
    /// Requires a distinct agent/model-family auditor and a separate probe.
    pub independent_audit: bool,
    /// Scope caveat, mandatory for partial or manual evidence.
    pub limitation: Option<String>,
}

/// Policies explicitly chosen for this plan, rather than universal thresholds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatePolicy {
    /// Maximum observation age in milliseconds; absent means no age limit.
    pub max_age_ms: Option<u64>,
    /// Require an observed source scope that covers each check's declared scope.
    pub require_observed_scope: bool,
}

/// Version-one plan; all output evidence is stored separately from this input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualityPlan {
    /// Schema version, currently exactly 1.
    pub version: u32,
    /// Human-selected identifier for this bounded quality plan.
    pub id: String,
    /// Exact source/configuration scope.
    pub subject: Subject,
    /// Nonempty checks, with at least one required check.
    pub checks: Vec<CheckSpec>,
    /// Optional expectations; small routine changes need no invented ledger.
    pub expectations: Vec<Expectation>,
    /// Explicit freshness and observed-coverage policy.
    pub policy: GatePolicy,
}

impl QualityPlan {
    /// Decode and validate a plan. Never executes its invocations.
    ///
    /// # Errors
    /// Refuses invalid JSON, unknown/duplicate fields and contract violations.
    ///
    /// # Examples
    ///
    /// This records a check; it does not run `cargo` or inspect the claimed source.
    /// Source hashes here illustrate the shape and must be replaced by actual
    /// selected bytes before producing execution receipts.
    ///
    /// ```
    /// use visioncraft::QualityPlan;
    /// let plan = QualityPlan::parse(br#"{
    ///   "version": 1, "id": "unit-contract",
    ///   "subject": {
    ///     "repository": "example", "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ///     "configuration": "default features", "dirty": false, "working_tree": null,
    ///     "inputs": [{"path": "src/lib.rs",
    ///       "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}]
    ///   },
    ///   "checks": [{
    ///     "id": "unit", "required": true,
    ///     "invocation": {"program": "cargo", "args": ["test"], "cwd": ".",
    ///       "env": ["PATH"], "timeout_ms": 60000, "max_output_bytes": 1048576},
    ///     "evaluator": {"kind": "exit-code", "success_codes": [0]},
    ///     "scope": ["src/lib.rs"]
    ///   }],
    ///   "expectations": [],
    ///   "policy": {"max_age_ms": 86400000, "require_observed_scope": false}
    /// }"#)?;
    /// assert_eq!(plan.checks[0].id, "unit");
    /// assert_eq!(plan.digest()?.len(), 64);
    /// # Ok::<(), visioncraft::ContractError>(())
    /// ```
    pub fn parse(bytes: &[u8]) -> Result<Self, ContractError> {
        let plan: Self = from_json(bytes)?;
        plan.validate()?;
        Ok(plan)
    }

    /// Validate identifiers, references and bounded execution policies.
    ///
    /// # Errors
    /// Returns the first contract violation, without reading external resources.
    pub fn validate(&self) -> Result<(), ContractError> {
        require(self.version == 1, "unsupported plan version")?;
        require(
            self.checks.len() <= MAX_PLAN_CHECKS,
            "plan check count exceeds admission ceiling",
        )?;
        require(
            self.expectations.len() <= MAX_PLAN_EXPECTATIONS,
            "plan expectation count exceeds admission ceiling",
        )?;
        require(identifier(&self.id), "invalid plan id")?;
        self.subject.validate()?;
        require(
            !self.checks.is_empty() && self.checks.iter().any(|c| c.required),
            "a gate requires at least one required check",
        )?;
        require(
            self.policy.max_age_ms != Some(0),
            "max_age_ms must be positive when present",
        )?;
        let inputs: BTreeSet<_> = self.subject.inputs.iter().map(|x| &x.path).collect();
        let mut ids = BTreeSet::new();
        let mut duration = 0_u64;
        let mut output_bytes = 0_u64;
        for c in &self.checks {
            require(
                identifier(&c.id) && ids.insert(&c.id),
                "invalid or duplicate check id",
            )?;
            c.invocation.validate()?;
            duration = duration
                .checked_add(c.invocation.timeout_ms)
                .ok_or_else(|| ContractError::Invalid("aggregate deadline overflow".into()))?;
            output_bytes = output_bytes
                .checked_add(c.invocation.max_output_bytes)
                .ok_or_else(|| ContractError::Invalid("aggregate capture overflow".into()))?;
            require(
                duration <= MAX_PLAN_DURATION_MS,
                "aggregate declared deadlines exceed one day",
            )?;
            require(
                output_bytes <= MAX_PLAN_OUTPUT_BYTES,
                "aggregate declared captures exceed one GiB",
            )?;
            require(!c.scope.is_empty(), "check scope cannot be empty")?;
            let mut scope = BTreeSet::new();
            for p in &c.scope {
                require(
                    inputs.contains(p) && scope.insert(p),
                    "scope must uniquely reference subject inputs",
                )?;
            }
            match &c.evaluator {
                Evaluator::ExitCode { success_codes } => {
                    let codes: BTreeSet<_> = success_codes.iter().collect();
                    require(
                        !codes.is_empty()
                            && codes.len() == success_codes.len()
                            && success_codes.iter().all(|c| (0..=255).contains(c)),
                        "success_codes must be unique portable exit codes",
                    )?;
                }
                Evaluator::Reported { adapter } => require(
                    identifier(adapter),
                    "reported evaluator requires an adapter id",
                )?,
            }
        }
        let mut expectations = BTreeSet::new();
        for e in &self.expectations {
            require(
                identifier(&e.id) && expectations.insert(&e.id),
                "invalid or duplicate expectation id",
            )?;
            require(nonempty(&e.claim), "expectation claim is required")?;
            let checks: BTreeSet<_> = e.checks.iter().collect();
            require(
                (!checks.is_empty()
                    || (e.category == EvidenceCategory::Manual && !e.independent_audit))
                    && checks.len() == e.checks.len()
                    && checks.iter().all(|c| ids.contains(c)),
                "expectation must uniquely reference declared checks",
            )?;
            require(
                !e.regression_critical || e.stabilizer.is_some(),
                "regression-critical expectation requires a stabilizer",
            )?;
            if let Some(s) = &e.stabilizer {
                require(
                    ids.contains(&s.check_id) && nonempty(&s.test_id),
                    "invalid stabilizer reference",
                )?;
            }
            if let Some(c) = &e.positive_control {
                require(
                    ids.contains(c) && !checks.contains(c),
                    "positive control must be a distinct declared check",
                )?;
            }
            if e.category != EvidenceCategory::Executable {
                require(
                    e.limitation.as_deref().is_some_and(nonempty),
                    "partial/manual evidence requires a limitation",
                )?;
            }
        }
        Ok(())
    }

    /// SHA-256 of the validated, deterministically serialised typed plan.
    ///
    /// Whitespace in input JSON is irrelevant; every typed field and vector order
    /// contributes. This is a content binding, not a signature or authority grant.
    ///
    /// # Errors
    /// Refuses invalid plans or serialization failures.
    pub fn digest(&self) -> Result<String, ContractError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|_| ContractError::Invalid("plan serialization failed".into()))?;
        Ok(sha256(&bytes))
    }
}
