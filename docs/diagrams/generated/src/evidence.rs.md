---
sealmap: 2
source: src/evidence.rs
module: "sym:cargo visioncraft . evidence/"
language: rust
source_hash: blake3:edfe990a885f2bcad685855fe0c87e11878174d676e4414f4f9448e818ca520a
lines: 292
fragments: 2
---
# `sym:cargo visioncraft . evidence/` · src/evidence.rs
> Immutable run and audit records, with caller-supplied byte verification.

## structure
```mermaid
classDiagram
  direction LR
  class visioncraft__evidence___tActor["Actor"] {
    <<struct>>
    +agent_id: String
    +attempt_id: String
    +model: String
  }
  class visioncraft__evidence___tAuditReceipt["AuditReceipt"] {
    <<struct>>
    +expectation_id: String
    +plan_sha256: String
    +subject: Subject
    +producer: Actor
    +reviewer: Actor
    +probe_check: String
    +verdict: AuditVerdict
    +observed: String
    +artifact: Artifact
    +captured_at_ms: u64
  }
  class visioncraft__evidence___tAuditVerdict["AuditVerdict"] {
    <<enum>>
    Supported
    Contradicted
    Inconclusive
  }
  class visioncraft__evidence___tExecutionReceipt["ExecutionReceipt"] {
    <<struct>>
    +version: u32
    +attempt_id: String
    +plan_sha256: String
    +check_id: String
    +subject: Subject
    +invocation: Invocation
    +executable: String
    +executable_sha256: Option#lt;String#gt;
    +tool_version: Option#lt;String#gt;
    +started_at_ms: u64
    +finished_at_ms: u64
    +elapsed_ms: u64
    +process: ProcessStatus
    +exit_code: Option#lt;i32#gt;
    +signal: Option#lt;i32#gt;
    +capture_complete: bool
    +stdout: Option#lt;Artifact#gt;
    +stderr: Option#lt;Artifact#gt;
    +inputs_after: Vec#lt;Artifact#gt;
    +observed_scope: Option#lt;Vec#lt;String#gt;#gt;
    +observed_tests: Vec#lt;String#gt;
    +reported: Option#lt;ReportedResult#gt;
    +diagnostic: Option#lt;String#gt;
    +parse(bytes: &[u8]) Result#lt;Self, ContractError#gt;
    +validate(&self) Result#lt;#40;#41;, ContractError#gt;
  }
  class visioncraft__evidence___tManualObservation["ManualObservation"] {
    <<struct>>
    +expectation_id: String
    +plan_sha256: String
    +subject: Subject
    +observer: String
    +observed: String
    +verdict: AuditVerdict
    +artifact: Artifact
    +captured_at_ms: u64
  }
  class visioncraft__evidence___tProcessStatus["ProcessStatus"] {
    <<enum>>
    Completed
    Unavailable
    TimedOut
    Interrupted
    FailedToStart
    CaptureFailed
  }
  class visioncraft__evidence___tReportedResult["ReportedResult"] {
    <<struct>>
    +adapter: String
    +outcome: CheckOutcome
    +reason: String
    +artifact: Artifact
  }
  class visioncraft__evidence___tVerifiedArtifacts["VerifiedArtifacts"] {
    <<struct>>
    -entries: BTreeMap#lt;#40;String, String#41;, u64#gt;
    +byte_len(&self, artifact: &Artifact) Option#lt;u64#gt;
    +contains(&self, artifact: &Artifact) bool
    +insert(&mut self, artifact: &Artifact, bytes: &[u8]) Result#lt;#40;#41;, ContractError#gt;
  }
  class visioncraft__plan___tSubject["Subject"] {
    <<struct in src/plan.rs>>
  }
  class _sealmap_corpus__review__Artifact["sealmap_corpus::review::Artifact"] {
    <<external>>
  }
  class visioncraft___tContractError["ContractError"] {
    <<enum in src/lib.rs>>
  }
  class visioncraft__plan___tInvocation["Invocation"] {
    <<struct in src/plan.rs>>
  }
  class visioncraft__evaluate___tCheckOutcome["CheckOutcome"] {
    <<enum in src/evaluate.rs>>
  }
  visioncraft__evidence___tAuditReceipt *-- visioncraft__evidence___tActor : producer, reviewer
  visioncraft__evidence___tAuditReceipt *-- visioncraft__evidence___tAuditVerdict : verdict
  visioncraft__evidence___tAuditReceipt *-- visioncraft__plan___tSubject : subject
  visioncraft__evidence___tAuditReceipt *-- _sealmap_corpus__review__Artifact : artifact
  visioncraft__evidence___tExecutionReceipt ..> visioncraft___tContractError
  visioncraft__evidence___tExecutionReceipt *-- visioncraft__evidence___tProcessStatus : process
  visioncraft__evidence___tExecutionReceipt o-- visioncraft__evidence___tReportedResult : reported
  visioncraft__evidence___tExecutionReceipt *-- visioncraft__plan___tInvocation : invocation
  visioncraft__evidence___tExecutionReceipt *-- visioncraft__plan___tSubject : subject
  visioncraft__evidence___tExecutionReceipt o-- _sealmap_corpus__review__Artifact : stdout, stderr, inputs_after
  visioncraft__evidence___tManualObservation *-- visioncraft__evidence___tAuditVerdict : verdict
  visioncraft__evidence___tManualObservation *-- visioncraft__plan___tSubject : subject
  visioncraft__evidence___tManualObservation *-- _sealmap_corpus__review__Artifact : artifact
  visioncraft__evidence___tReportedResult *-- visioncraft__evaluate___tCheckOutcome : outcome
  visioncraft__evidence___tReportedResult *-- _sealmap_corpus__review__Artifact : artifact
  visioncraft__evidence___tVerifiedArtifacts ..> visioncraft___tContractError
  visioncraft__evidence___tVerifiedArtifacts ..> _sealmap_corpus__review__Artifact
```

## `sym:cargo visioncraft . evidence/ExecutionReceipt#validate().`
`pub fn validate(&self) -> Result<(), ContractError>` · L107-L178
> Check receipt consistency; matching a plan and bytes is a separate step.
```mermaid
sequenceDiagram
  participant visioncraft__evidence___tExecutionReceipt as ExecutionReceipt
  participant visioncraft as visioncraft mod
  participant visioncraft__plan___tSubject as Subject
  participant visioncraft__plan___tInvocation as Invocation
  participant visioncraft__plan as plan mod
  visioncraft__evidence___tExecutionReceipt->>visioncraft: require(_, #quot;unsupported …)?
  visioncraft__evidence___tExecutionReceipt->>visioncraft: identifier(&self.attempt_id)
  visioncraft__evidence___tExecutionReceipt->>visioncraft: identifier(&self.check_id)
  visioncraft__evidence___tExecutionReceipt->>visioncraft: require(_, #quot;invalid rece…)?
  visioncraft__evidence___tExecutionReceipt->>visioncraft: hex(&self.plan_sha256, &_)
  visioncraft__evidence___tExecutionReceipt->>visioncraft: require(hex(), #quot;invalid plan…)?
  visioncraft__evidence___tExecutionReceipt->>visioncraft__plan___tSubject: validate()?
  visioncraft__evidence___tExecutionReceipt->>visioncraft__plan___tInvocation: validate()?
  visioncraft__evidence___tExecutionReceipt->>visioncraft: nonempty(&self.executable)
  visioncraft__evidence___tExecutionReceipt->>visioncraft: require(nonempty(), #quot;executable i…)?
  opt let Some(hash) = &self.executable_sha256
    visioncraft__evidence___tExecutionReceipt->>visioncraft: hex(hash, &_)
    visioncraft__evidence___tExecutionReceipt->>visioncraft: require(hex(), #quot;invalid exec…)?
  end
  opt let Some(version) = &self.tool_version
    visioncraft__evidence___tExecutionReceipt->>visioncraft: nonempty(version)
    visioncraft__evidence___tExecutionReceipt->>visioncraft: require(nonempty(), #quot;empty tool v…)?
  end
  visioncraft__evidence___tExecutionReceipt->>visioncraft: require(_, #quot;finish prece…)?
  visioncraft__evidence___tExecutionReceipt->>visioncraft: require(_, #quot;exit code an…)?
  alt self.process == ProcessStatus::Completed
    visioncraft__evidence___tExecutionReceipt->>visioncraft: require(_, #quot;completed ex…)?
    visioncraft__evidence___tExecutionReceipt->>visioncraft: require(_, #quot;completed ex…)?
  else
    visioncraft__evidence___tExecutionReceipt->>visioncraft: require(is_some_and(), #quot;incomplete e…)?
  end
  loop for a in self.stdout.iter().chain(self.stderr.iter())
    visioncraft__evidence___tExecutionReceipt->>visioncraft: artifact_shape(a)?
  end
  visioncraft__evidence___tExecutionReceipt->>visioncraft__plan: validate_input_paths(&self.inputs_after)?
  opt let Some(paths) = &self.observed_scope
    loop each via all
      visioncraft__evidence___tExecutionReceipt->>visioncraft: crate::relative_path(p)
    end
    visioncraft__evidence___tExecutionReceipt->>visioncraft: require(_, #quot;invalid or d…)?
  end
  loop each via all
    visioncraft__evidence___tExecutionReceipt->>visioncraft: nonempty(t)
  end
  visioncraft__evidence___tExecutionReceipt->>visioncraft: require(_, #quot;invalid or d…)?
  opt let Some(reported) = &self.reported
    visioncraft__evidence___tExecutionReceipt->>visioncraft: identifier(&reported.adapter)
    visioncraft__evidence___tExecutionReceipt->>visioncraft: nonempty(&reported.reason)
    visioncraft__evidence___tExecutionReceipt->>visioncraft: require(_, #quot;reported ass…)?
    visioncraft__evidence___tExecutionReceipt->>visioncraft: artifact_shape(&reported.artifact)?
  end
```
