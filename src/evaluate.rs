//! Read-only evaluation of explicit plans, exact receipts and supplied context.

use crate::plan::same_inputs;
use crate::{
    AuditReceipt, AuditVerdict, ContractError, Evaluator, EvidenceCategory, ExecutionReceipt,
    Expectation, FamilyCatalogue, ManualObservation, ProcessStatus, QualityPlan, Subject,
    VerifiedArtifacts, artifact_shape, identifier, nonempty,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Result of a check under its recorded evaluator policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckOutcome {
    /// Completed evidence satisfies the declared check policy.
    Passed,
    /// Completed evidence fails that policy or contradicts an expectation.
    Blocked,
    /// Explicitly disabled or unavailable; never a required pass.
    Skipped,
    /// Evidence, scope, freshness or independence is insufficient.
    Inconclusive,
    /// Invalid evidence or failed execution/capture/evaluator.
    Error,
}

/// Aggregate decision, preserving incomplete evidence separately from defects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GateOutcome {
    /// Every required check and expectation passed its explicit policy.
    Passed,
    /// At least one completed required check/expectation blocks.
    Blocked,
    /// Required evidence is missing, skipped, stale or inconclusive.
    Incomplete,
    /// Invalid contract or execution/evaluator error prevents a decision.
    Error,
}

/// Recorded outcome and limitations for one planned check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckReport {
    /// Plan-local check identity.
    pub id: String,
    /// Whether this check is required by the unchanged plan.
    pub required: bool,
    /// Attempt used, if exactly one was supplied.
    pub attempt_id: Option<String>,
    /// Scoped policy outcome.
    pub outcome: CheckOutcome,
    /// Missing evidence, observed failures or explicit interpretation limits.
    pub reasons: Vec<String>,
}

/// Evidence coverage for one expectation, not a semantic proof of its prose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExpectationReport {
    /// Expectation identity from the plan.
    pub id: String,
    /// Whether this expectation is required by the unchanged plan.
    pub required: bool,
    /// Coverage/assessment outcome.
    pub outcome: CheckOutcome,
    /// Scope limits and unmet requirements.
    pub reasons: Vec<String>,
}

/// Deterministic evaluation output; no delivery stage is inferred.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GateReport {
    /// Aggregate policy result, not a security certification.
    pub outcome: GateOutcome,
    /// Unchanged declared required-check denominator.
    pub required_checks: usize,
    /// Required checks satisfying their policy in this evaluation.
    pub passed_required_checks: usize,
    /// Every planned check, including missing and optional checks.
    pub checks: Vec<CheckReport>,
    /// Every planned expectation, including unsupported ones.
    pub expectations: Vec<ExpectationReport>,
    /// Structural errors in the supplied data, without raw document contents.
    pub errors: Vec<String>,
}

impl GateReport {
    /// Whether the declared gate policy passed with complete required evidence.
    pub fn passed(&self) -> bool {
        self.outcome == GateOutcome::Passed
    }
}

/// External facts supplied explicitly; the library performs no IO or clock reads.
#[derive(Debug)]
pub struct EvaluationContext<'a> {
    /// Evaluation time in milliseconds since the Unix epoch.
    pub now_ms: u64,
    /// Current source/configuration snapshot, independently obtained by caller.
    pub current_subject: &'a Subject,
    /// Exact captured evidence bytes verified by the caller in memory.
    pub verified: &'a VerifiedArtifacts,
    /// Revisions known to be ancestors of the current revision. Empty means
    /// unknown ancestry when the current and recorded revisions differ.
    pub ancestor_revisions: &'a [String],
    /// Optional exact aliases for served model families, governed by the caller.
    pub families: FamilyCatalogue,
    /// Post-run audit records; absent required audits remain incomplete.
    pub audits: &'a [AuditReceipt],
    /// Post-run manual observations; their meaning is assessed by the observer.
    pub manual_observations: &'a [ManualObservation],
}

impl<'a> EvaluationContext<'a> {
    /// Construct a context with no inferred ancestry, audits or family aliases.
    pub fn new(now_ms: u64, current_subject: &'a Subject, verified: &'a VerifiedArtifacts) -> Self {
        Self {
            now_ms,
            current_subject,
            verified,
            ancestor_revisions: &[],
            families: FamilyCatalogue::default(),
            audits: &[],
            manual_observations: &[],
        }
    }
}

fn context_limits(plan: &QualityPlan, context: &EvaluationContext<'_>) -> Vec<String> {
    let mut reasons = Vec::new();
    let recorded = &plan.subject;
    let current = context.current_subject;
    if recorded.repository != current.repository || recorded.configuration != current.configuration
    {
        reasons.push("current repository/configuration differs from reviewed subject".into());
    }
    if !same_inputs(&recorded.inputs, &current.inputs) {
        reasons.push("selected source inputs changed since review".into());
    }
    if recorded.revision != current.revision
        && !context.ancestor_revisions.contains(&recorded.revision)
    {
        reasons.push(
            "reviewed revision ancestry is unknown or no longer on the current branch".into(),
        );
    }
    reasons
}

fn age_limit(at: u64, plan: &QualityPlan, context: &EvaluationContext<'_>) -> Option<String> {
    match context.now_ms.checked_sub(at) {
        None => Some("observation timestamp is in the future".into()),
        Some(age) if plan.policy.max_age_ms.is_some_and(|max| age > max) => {
            Some("observation exceeds configured maximum age".into())
        }
        _ => None,
    }
}

fn check_receipt(
    plan: &QualityPlan,
    digest: &str,
    check: &crate::CheckSpec,
    receipt: &ExecutionReceipt,
    context: &EvaluationContext<'_>,
) -> (CheckOutcome, Vec<String>) {
    let error = |message: &str| (CheckOutcome::Error, vec![message.to_owned()]);
    if let Err(e) = receipt.validate() {
        return error(&e.to_string());
    }
    if receipt.plan_sha256 != digest
        || receipt.subject != plan.subject
        || receipt.invocation != check.invocation
    {
        return error("receipt plan, subject or invocation does not match the selected check");
    }
    if !same_inputs(&receipt.subject.inputs, &receipt.inputs_after) {
        return (
            CheckOutcome::Inconclusive,
            vec!["selected inputs changed during execution".into()],
        );
    }
    let mut limits = context_limits(plan, context);
    if let Some(age) = age_limit(receipt.finished_at_ms, plan, context) {
        limits.push(age);
    }
    if !limits.is_empty() {
        return (CheckOutcome::Inconclusive, limits);
    }
    if receipt.process != ProcessStatus::Completed {
        let outcome = if receipt.process == ProcessStatus::Unavailable {
            CheckOutcome::Skipped
        } else {
            CheckOutcome::Error
        };
        return (
            outcome,
            vec![
                receipt
                    .diagnostic
                    .clone()
                    .unwrap_or_else(|| "execution did not complete".into()),
            ],
        );
    }
    if receipt.elapsed_ms > check.invocation.timeout_ms {
        return error("completed execution exceeded the configured deadline");
    }
    for output in receipt.stdout.iter().chain(receipt.stderr.iter()) {
        if !context.verified.contains(output) {
            return error("raw output artifact has not been byte-verified");
        }
    }
    if let Some(snapshot) = &receipt.subject.working_tree {
        if !context.verified.contains(snapshot) {
            return error("dirty working-tree snapshot has not been byte-verified");
        }
    }
    match &receipt.observed_scope {
        Some(paths) if !check.scope.iter().all(|p| paths.contains(p)) => {
            return (
                CheckOutcome::Inconclusive,
                vec!["observed scope omits selected source inputs".into()],
            );
        }
        None if plan.policy.require_observed_scope => {
            return (
                CheckOutcome::Inconclusive,
                vec!["observed source scope is unknown".into()],
            );
        }
        _ => {}
    }
    match &check.evaluator {
        Evaluator::ExitCode { success_codes } => {
            if receipt.reported.is_some() {
                return error("unexpected adapter assessment for an exit-code evaluator");
            }
            if receipt
                .exit_code
                .is_some_and(|code| success_codes.contains(&code))
            {
                (
                    CheckOutcome::Passed,
                    vec!["completed exit code satisfies the configured policy".into()],
                )
            } else {
                (
                    CheckOutcome::Blocked,
                    vec!["completed exit code does not satisfy the configured policy".into()],
                )
            }
        }
        Evaluator::Reported { adapter } => {
            let Some(result) = &receipt.reported else {
                return error("missing required adapter assessment");
            };
            if &result.adapter != adapter || !context.verified.contains(&result.artifact) {
                return error("adapter identity or verified report artifact is missing");
            }
            (result.outcome, vec![result.reason.clone()])
        }
    }
}

fn referenced_check(
    id: &str,
    checks: &[CheckReport],
    reasons: &mut Vec<String>,
    outcome: &mut CheckOutcome,
) {
    if let Some(check) = checks.iter().find(|c| c.id == id) {
        if check.outcome != CheckOutcome::Passed {
            reasons.push(format!("referenced check {id} did not pass"));
            if check.outcome == CheckOutcome::Blocked {
                *outcome = CheckOutcome::Blocked;
            } else if *outcome != CheckOutcome::Blocked {
                *outcome = CheckOutcome::Inconclusive;
            }
        }
    } else {
        reasons.push(format!("referenced check {id} is missing"));
        if *outcome != CheckOutcome::Blocked {
            *outcome = CheckOutcome::Inconclusive;
        }
    }
}

fn assessment_binding(
    plan: &QualityPlan,
    digest: &str,
    subject: &Subject,
    claimed: &str,
    artifact: &crate::Artifact,
    at: u64,
    context: &EvaluationContext<'_>,
) -> Result<(), ContractError> {
    crate::require(
        subject == &plan.subject && claimed == digest,
        "assessment plan/subject mismatch",
    )?;
    artifact_shape(artifact)?;
    crate::require(
        context.verified.contains(artifact),
        "assessment artifact has not been byte-verified",
    )?;
    crate::require(
        age_limit(at, plan, context).is_none(),
        "assessment is stale or future-dated",
    )
}

fn expectation_report(
    plan: &QualityPlan,
    digest: &str,
    expectation: &Expectation,
    checks: &[CheckReport],
    runs: &BTreeMap<&str, &ExecutionReceipt>,
    context: &EvaluationContext<'_>,
) -> ExpectationReport {
    let mut outcome = CheckOutcome::Passed;
    let mut reasons = Vec::new();
    for id in &expectation.checks {
        referenced_check(id, checks, &mut reasons, &mut outcome);
    }
    if let Some(control) = &expectation.positive_control {
        referenced_check(control, checks, &mut reasons, &mut outcome);
    }
    if let Some(stabilizer) = &expectation.stabilizer {
        referenced_check(&stabilizer.check_id, checks, &mut reasons, &mut outcome);
        if !runs
            .get(stabilizer.check_id.as_str())
            .is_some_and(|r| r.observed_tests.contains(&stabilizer.test_id))
        {
            reasons.push("stabilizer test was not reported as executed".into());
            if outcome != CheckOutcome::Blocked {
                outcome = CheckOutcome::Inconclusive;
            }
        }
    }
    if let Some(limit) = &expectation.limitation {
        reasons.push(format!("declared limitation: {limit}"));
    }
    if expectation.independent_audit {
        let audits: Vec<_> = context
            .audits
            .iter()
            .filter(|a| a.expectation_id == expectation.id)
            .collect();
        let validation = if audits.len() != 1 {
            Err(ContractError::Invalid(
                "exactly one selected audit is required".into(),
            ))
        } else {
            let a = audits[0];
            (|| {
                assessment_binding(
                    plan,
                    digest,
                    &a.subject,
                    &a.plan_sha256,
                    &a.artifact,
                    a.captured_at_ms,
                    context,
                )?;
                crate::require(
                    nonempty(&a.observed)
                        && identifier(&a.producer.agent_id)
                        && identifier(&a.reviewer.agent_id),
                    "audit must identify actors and observation",
                )?;
                crate::require(
                    a.producer.agent_id != a.reviewer.agent_id
                        && a.producer.attempt_id != a.reviewer.attempt_id,
                    "audit actors and attempts must be distinct",
                )?;
                crate::require(
                    expectation.checks.iter().any(|id| {
                        runs.get(id.as_str())
                            .is_some_and(|r| r.attempt_id == a.producer.attempt_id)
                    }),
                    "producer does not reference supporting execution",
                )?;
                crate::require(
                    !expectation.checks.contains(&a.probe_check),
                    "audit requires a distinct adversarial probe",
                )?;
                crate::require(
                    runs.get(a.probe_check.as_str())
                        .is_some_and(|r| r.attempt_id == a.reviewer.attempt_id),
                    "reviewer does not reference the probe execution",
                )?;
                crate::require(
                    checks
                        .iter()
                        .any(|c| c.id == a.probe_check && c.outcome == CheckOutcome::Passed),
                    "adversarial probe did not complete successfully",
                )?;
                sealmap_corpus::route::cross_family_with(
                    &a.producer.model,
                    &a.reviewer.model,
                    &context.families,
                )
                .map_err(|_| {
                    ContractError::Invalid(
                        "audit model-family independence is unknown or not satisfied".into(),
                    )
                })?;
                Ok(a.verdict)
            })()
        };
        match validation {
            Ok(AuditVerdict::Supported) => {
                reasons.push("recorded independent audit supports the scoped claim".into())
            }
            Ok(AuditVerdict::Contradicted) => {
                outcome = CheckOutcome::Blocked;
                reasons.push("auditor recorded a counterexample".into());
            }
            Ok(AuditVerdict::Inconclusive) => {
                if outcome != CheckOutcome::Blocked {
                    outcome = CheckOutcome::Inconclusive;
                }
                reasons.push("auditor could not conclude".into());
            }
            Err(e) => {
                if outcome != CheckOutcome::Blocked {
                    outcome = CheckOutcome::Inconclusive;
                }
                reasons.push(e.to_string());
            }
        }
    }
    if expectation.category == EvidenceCategory::Manual {
        let observations: Vec<_> = context
            .manual_observations
            .iter()
            .filter(|m| m.expectation_id == expectation.id)
            .collect();
        let validation = if observations.len() != 1 {
            Err(ContractError::Invalid(
                "exactly one selected manual observation is required".into(),
            ))
        } else {
            let m = observations[0];
            assessment_binding(
                plan,
                digest,
                &m.subject,
                &m.plan_sha256,
                &m.artifact,
                m.captured_at_ms,
                context,
            )
            .and_then(|()| {
                crate::require(
                    nonempty(&m.observer) && nonempty(&m.observed),
                    "manual observation needs an observer and observed effect",
                )
            })
            .map(|()| m.verdict)
        };
        match validation {
            Ok(AuditVerdict::Supported) => {
                reasons.push("named manual observer supports the scoped claim".into())
            }
            Ok(AuditVerdict::Contradicted) => {
                outcome = CheckOutcome::Blocked;
                reasons.push("manual observation contradicts the claim".into());
            }
            Ok(AuditVerdict::Inconclusive) => {
                if outcome != CheckOutcome::Blocked {
                    outcome = CheckOutcome::Inconclusive;
                }
                reasons.push("manual observation is inconclusive".into());
            }
            Err(e) => {
                if outcome != CheckOutcome::Blocked {
                    outcome = CheckOutcome::Inconclusive;
                }
                reasons.push(e.to_string());
            }
        }
    }
    ExpectationReport {
        id: expectation.id.clone(),
        required: expectation.required,
        outcome,
        reasons,
    }
}

/// Evaluate an unchanged plan against selected attempts and explicit external facts.
///
/// No source, artifact, timestamp or command is automatically dereferenced. Choose
/// one attempt per check: historical attempts belong in the archive, not an
/// ambiguous bag from which the evaluator silently chooses the greenest result.
/// Invalid input is reported as an error; missing required evidence never passes.
pub fn evaluate(
    plan: &QualityPlan,
    receipts: &[ExecutionReceipt],
    context: &EvaluationContext<'_>,
) -> GateReport {
    let mut report = GateReport {
        outcome: GateOutcome::Error,
        required_checks: plan.checks.iter().filter(|c| c.required).count(),
        passed_required_checks: 0,
        checks: vec![],
        expectations: vec![],
        errors: vec![],
    };
    let digest = match plan.digest() {
        Ok(digest) => digest,
        Err(e) => {
            report.errors.push(e.to_string());
            return report;
        }
    };
    if let Err(e) = context.current_subject.validate() {
        report.errors.push(e.to_string());
        return report;
    }
    let mut attempts = BTreeSet::new();
    let mut runs = BTreeMap::new();
    let ids: BTreeSet<_> = plan.checks.iter().map(|c| c.id.as_str()).collect();
    for receipt in receipts {
        if !ids.contains(receipt.check_id.as_str()) {
            report
                .errors
                .push("receipt references an unplanned check".into());
        }
        if !attempts.insert(receipt.attempt_id.as_str()) {
            report.errors.push("duplicate execution attempt id".into());
        }
        if runs.insert(receipt.check_id.as_str(), receipt).is_some() {
            report
                .errors
                .push("multiple attempts for one check; select an attempt explicitly".into());
        }
    }
    let expectation_ids: BTreeSet<_> = plan.expectations.iter().map(|e| e.id.as_str()).collect();
    for id in context
        .audits
        .iter()
        .map(|a| a.expectation_id.as_str())
        .chain(
            context
                .manual_observations
                .iter()
                .map(|m| m.expectation_id.as_str()),
        )
    {
        if !expectation_ids.contains(id) {
            report
                .errors
                .push("assessment references an unplanned expectation".into());
        }
    }
    for check in &plan.checks {
        let run = runs.get(check.id.as_str());
        let (outcome, reasons) = match run {
            Some(receipt) => check_receipt(plan, &digest, check, receipt, context),
            None => (
                CheckOutcome::Inconclusive,
                vec!["no execution receipt was supplied".into()],
            ),
        };
        if check.required && outcome == CheckOutcome::Passed {
            report.passed_required_checks += 1;
        }
        report.checks.push(CheckReport {
            id: check.id.clone(),
            required: check.required,
            attempt_id: run.map(|r| r.attempt_id.clone()),
            outcome,
            reasons,
        });
    }
    for expectation in &plan.expectations {
        report.expectations.push(expectation_report(
            plan,
            &digest,
            expectation,
            &report.checks,
            &runs,
            context,
        ));
    }
    let required: Vec<_> = report
        .checks
        .iter()
        .filter(|r| r.required)
        .map(|r| r.outcome)
        .chain(
            report
                .expectations
                .iter()
                .filter(|r| r.required)
                .map(|r| r.outcome),
        )
        .collect();
    report.outcome = if !report.errors.is_empty() || required.contains(&CheckOutcome::Error) {
        GateOutcome::Error
    } else if required.contains(&CheckOutcome::Blocked) {
        GateOutcome::Blocked
    } else if required.iter().any(|o| *o != CheckOutcome::Passed) {
        GateOutcome::Incomplete
    } else {
        GateOutcome::Passed
    };
    report
}
