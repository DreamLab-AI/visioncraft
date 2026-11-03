---
sealmap: 2
source: src/evaluate.rs
module: "sym:cargo visioncraft . evaluate/"
language: rust
source_hash: blake3:e2b3c4721974ba40222309382bfb4dc2a311296e18b7324982cd29215db905a1
lines: 658
fragments: 4
---
# `sym:cargo visioncraft . evaluate/` · src/evaluate.rs
> Read-only evaluation of explicit plans, exact receipts and supplied context.

## structure
```mermaid
classDiagram
  direction LR
  class visioncraft__evaluate___tCheckOutcome["CheckOutcome"] {
    <<enum>>
    Passed
    Blocked
    Skipped
    Inconclusive
    Error
  }
  class visioncraft__evaluate___tCheckReport["CheckReport"] {
    <<struct>>
    +id: String
    +required: bool
    +attempt_id: Option#lt;String#gt;
    +outcome: CheckOutcome
    +reasons: Vec#lt;String#gt;
  }
  class visioncraft__evaluate___tEvaluationContext["EvaluationContext#lt;'a#gt;"] {
    <<struct>>
    +now_ms: u64
    +current_subject: &'a Subject
    +verified: &'a VerifiedArtifacts
    +ancestor_revisions: &'a [String]
    +families: FamilyCatalogue
    +audits: &'a [AuditReceipt]
    +manual_observations: &'a [ManualObservation]
    +new(now_ms: u64, current_subject: &'a Subject, verified: &'a VerifiedArtifacts) Self
  }
  class visioncraft__evaluate___tExpectationReport["ExpectationReport"] {
    <<struct>>
    +id: String
    +required: bool
    +outcome: CheckOutcome
    +reasons: Vec#lt;String#gt;
  }
  class visioncraft__evaluate___tGateOutcome["GateOutcome"] {
    <<enum>>
    Passed
    Blocked
    Incomplete
    Error
  }
  class visioncraft__evaluate___tGateReport["GateReport"] {
    <<struct>>
    +outcome: GateOutcome
    +required_checks: usize
    +passed_required_checks: usize
    +checks: Vec#lt;CheckReport#gt;
    +expectations: Vec#lt;ExpectationReport#gt;
    +errors: Vec#lt;String#gt;
    +passed(&self) bool
  }
  class visioncraft__evaluate["visioncraft::evaluate"] {
    <<module>>
    +evaluate(plan: &QualityPlan, receipts: &[ExecutionReceipt], context: &EvaluationContext#lt;'_#gt;,) GateReport
  }
  class visioncraft__evidence___tExecutionReceipt["ExecutionReceipt"] {
    <<struct in src/evidence.rs>>
  }
  class visioncraft__plan___tQualityPlan["QualityPlan"] {
    <<struct in src/plan.rs>>
  }
  class visioncraft__evidence___tAuditReceipt["AuditReceipt"] {
    <<struct in src/evidence.rs>>
  }
  class visioncraft__evidence___tManualObservation["ManualObservation"] {
    <<struct in src/evidence.rs>>
  }
  class visioncraft__evidence___tVerifiedArtifacts["VerifiedArtifacts"] {
    <<struct in src/evidence.rs>>
  }
  class visioncraft__plan___tSubject["Subject"] {
    <<struct in src/plan.rs>>
  }
  class _sealmap_corpus__route__FamilyCatalogue["sealmap_corpus::route::FamilyCatalogue"] {
    <<external>>
  }
  visioncraft__evaluate ..> visioncraft__evaluate___tEvaluationContext
  visioncraft__evaluate ..> visioncraft__evaluate___tGateReport
  visioncraft__evaluate ..> visioncraft__evidence___tExecutionReceipt
  visioncraft__evaluate ..> visioncraft__plan___tQualityPlan
  visioncraft__evaluate___tCheckReport *-- visioncraft__evaluate___tCheckOutcome : outcome
  visioncraft__evaluate___tEvaluationContext o-- visioncraft__evidence___tAuditReceipt : audits
  visioncraft__evaluate___tEvaluationContext o-- visioncraft__evidence___tManualObservation : manual_observations
  visioncraft__evaluate___tEvaluationContext o-- visioncraft__evidence___tVerifiedArtifacts : verified
  visioncraft__evaluate___tEvaluationContext o-- visioncraft__plan___tSubject : current_subject
  visioncraft__evaluate___tEvaluationContext *-- _sealmap_corpus__route__FamilyCatalogue : families
  visioncraft__evaluate___tExpectationReport *-- visioncraft__evaluate___tCheckOutcome : outcome
  visioncraft__evaluate___tGateReport o-- visioncraft__evaluate___tCheckReport : checks
  visioncraft__evaluate___tGateReport o-- visioncraft__evaluate___tExpectationReport : expectations
  visioncraft__evaluate___tGateReport *-- visioncraft__evaluate___tGateOutcome : outcome
```

## `sym:cargo visioncraft . evaluate/check_receipt().`
`fn check_receipt(plan: &QualityPlan, digest: &str, check: &crate::CheckSpec, receipt: &ExecutionReceipt, context: &EvaluationContext<'_>,) -> (CheckOutcome, Vec<String>)` · L160-L273
```mermaid
sequenceDiagram
  participant visioncraft__evaluate as evaluate mod
  participant visioncraft__evidence___tExecutionReceipt as ExecutionReceipt
  participant visioncraft__plan as plan mod
  participant visioncraft__evidence___tVerifiedArtifacts as VerifiedArtifacts
  visioncraft__evaluate->>visioncraft__evidence___tExecutionReceipt: validate()
  opt let Err(e) = receipt.validate()
    Note over visioncraft__evaluate: return error(&e.to_string())
  end
  opt receipt.plan_sha256 != digest || receipt.subject != …
    Note over visioncraft__evaluate: return error(#quot;receipt plan, subject or invocation does …
  end
  visioncraft__evaluate->>visioncraft__plan: same_inputs(&receipt.subject.inputs, &receipt.inputs_af…
  opt !same_inputs(&receipt.subject.inputs, &receipt.input…
    Note over visioncraft__evaluate: return (CheckOutcome::Inconclusive, vec![#quot;selected inpu…
  end
  visioncraft__evaluate->>visioncraft__evaluate: context_limits(plan, context)
  visioncraft__evaluate->>visioncraft__evaluate: age_limit(receipt.finished_at_ms, plan, context)
  opt !limits.is_empty()
    Note over visioncraft__evaluate: return (CheckOutcome::Inconclusive, limits)
  end
  opt receipt.process != ProcessStatus::Completed
    Note over visioncraft__evaluate: return (outcome, vec![receipt.diagnostic.clone().unwrap…
  end
  opt receipt.elapsed_ms> check.invocation.timeout_ms
    Note over visioncraft__evaluate: return error(#quot;completed execution exceeded the configur…
  end
  loop for output in receipt.stdout.iter().chain(receipt.stder…
    visioncraft__evaluate->>visioncraft__evidence___tVerifiedArtifacts: ~byte_len(output)
    alt let Some(bytes) = context.verified.byte_len(output)
      opt let-else
        Note over visioncraft__evaluate: return error(#quot;output length overflow#quot;)
      end
    else
      Note over visioncraft__evaluate: return error(#quot;raw output artifact has not been byte-ver…
    end
  end
  opt output_bytes> check.invocation.max_output_bytes
    Note over visioncraft__evaluate: return error(#quot;verified raw output exceeds configured ca…
  end
  opt let Some(snapshot) = &receipt.subject.working_tree
    opt !context.verified.contains(snapshot)
      Note over visioncraft__evaluate: return error(#quot;dirty working-tree snapshot has not been …
    end
  end
  alt Some(paths) if !check.scope.iter().all(| p | paths.cont…
    Note over visioncraft__evaluate: return (CheckOutcome::Inconclusive, vec![#quot;observed scop…
  else None if plan.policy.require_observed_scope
    Note over visioncraft__evaluate: return (CheckOutcome::Inconclusive, vec![#quot;observed sour…
  end
  alt Evaluator::ExitCode { success_codes }
    opt receipt.reported.is_some()
      Note over visioncraft__evaluate: return error(#quot;unexpected adapter assessment for an exit…
    end
  else Evaluator::Reported { adapter }
    opt let-else
      Note over visioncraft__evaluate: return error(#quot;missing required adapter assessment#quot;)
    end
    opt &result.adapter != adapter ||!context.verified.conta…
      Note over visioncraft__evaluate: return error(#quot;adapter identity or verified report artif…
    end
  end
```

## `sym:cargo visioncraft . evaluate/assessment_binding().`
`fn assessment_binding(plan: &QualityPlan, digest: &str, subject: &Subject, claimed: &str, artifact: &crate::Artifact, at: u64, context: &EvaluationContext<'_>,) -> Result<(), ContractError>` · L298-L320
```mermaid
sequenceDiagram
  participant visioncraft__evaluate as evaluate mod
  participant visioncraft as visioncraft mod
  visioncraft__evaluate->>visioncraft: crate::require(_, #quot;assessment p…)?
  visioncraft__evaluate->>visioncraft: artifact_shape(artifact)?
  visioncraft__evaluate->>visioncraft: crate::require(contains(), #quot;assessment a…)?
  visioncraft__evaluate->>visioncraft__evaluate: age_limit(at, plan, context)
  visioncraft__evaluate->>visioncraft: crate::require(is_none(), #quot;assessment i…)
```

## `sym:cargo visioncraft . evaluate/expectation_report().`
`fn expectation_report(plan: &QualityPlan, digest: &str, expectation: &Expectation, checks: &[CheckReport], runs: &BTreeMap<&str, &ExecutionReceipt>, context: &EvaluationContext<'_>,) -> ExpectationRe…` · L322-L524
```mermaid
sequenceDiagram
  participant visioncraft__evaluate as evaluate mod
  participant visioncraft as visioncraft mod
  participant _sealmap_corpus as sealmap_corpus ext
  loop for id in &expectation.checks
    visioncraft__evaluate->>visioncraft__evaluate: referenced_check(id, checks, &reasons, &outcome)
  end
  opt let Some(control) = &expectation.positive_control
    visioncraft__evaluate->>visioncraft__evaluate: referenced_check(control, checks, &reasons, &outcome)
  end
  opt let Some(stabilizer) = &expectation.stabilizer
    visioncraft__evaluate->>visioncraft__evaluate: referenced_check(&stabilizer.check_id, checks, &reasons…
  end
  opt expectation.independent_audit || context.audits.iter…
    opt not audits.len() != 1
      opt closure
        visioncraft__evaluate->>visioncraft__evaluate: assessment_binding(plan, digest, &a.subject, &a.plan_sh…?
        visioncraft__evaluate->>visioncraft: nonempty(&a.observed)
        visioncraft__evaluate->>visioncraft: identifier(&a.producer.agent_id)
        visioncraft__evaluate->>visioncraft: identifier(&a.reviewer.agent_id)
        visioncraft__evaluate->>visioncraft: crate::require(_, #quot;audit must i…)?
        visioncraft__evaluate->>visioncraft: crate::require(_, #quot;audit actors…)?
        visioncraft__evaluate->>visioncraft: crate::require(any(), #quot;producer doe…)?
        visioncraft__evaluate->>visioncraft: crate::require(_, #quot;audit requir…)?
        visioncraft__evaluate->>visioncraft: crate::require(is_some_and(), #quot;reviewer doe…)?
        visioncraft__evaluate->>visioncraft: crate::require(any(), #quot;adversarial …)?
        opt expectation.independent_audit
          visioncraft__evaluate->>_sealmap_corpus: route::cross_family_with(&a.producer.model, &a.reviewer…
        end
      end
    end
  end
  opt expectation.category == EvidenceCategory::Manual || …
    opt not observations.len() != 1
      visioncraft__evaluate->>visioncraft__evaluate: assessment_binding(plan, digest, &m.subject, &m.plan_sh…
      opt via and_then
        visioncraft__evaluate->>visioncraft: nonempty(&m.observer)
        visioncraft__evaluate->>visioncraft: nonempty(&m.observed)
        visioncraft__evaluate->>visioncraft: crate::require(_, #quot;manual obser…)
      end
    end
  end
```
