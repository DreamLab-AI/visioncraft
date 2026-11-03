---
sealmap: 2
source: src/plan.rs
module: "sym:cargo visioncraft . plan/"
language: rust
source_hash: blake3:a0eaf6b6e34139a3b02ddc3d521d69dc4c064884b96dfe85585b1a1e7da2107d
lines: 413
fragments: 4
---
# `sym:cargo visioncraft . plan/` · src/plan.rs
> Typed plans describe explicitly selected checks; parsing never runs them.

## structure
```mermaid
classDiagram
  direction LR
  class visioncraft__plan___tCheckSpec["CheckSpec"] {
    <<struct>>
    +id: String
    +required: bool
    +invocation: Invocation
    +evaluator: Evaluator
    +scope: Vec#lt;String#gt;
  }
  class visioncraft__plan___tEvaluator["Evaluator"] {
    <<enum>>
    ExitCode#123; success_codes: Vec#lt;i32#gt;, #125;
    Reported#123; adapter: String, #125;
  }
  class visioncraft__plan___tEvidenceCategory["EvidenceCategory"] {
    <<enum>>
    Executable
    PartiallyVerifiable
    Manual
  }
  class visioncraft__plan___tExpectation["Expectation"] {
    <<struct>>
    +id: String
    +claim: String
    +required: bool
    +category: EvidenceCategory
    +checks: Vec#lt;String#gt;
    +regression_critical: bool
    +stabilizer: Option#lt;Stabilizer#gt;
    +positive_control: Option#lt;String#gt;
    +independent_audit: bool
    +limitation: Option#lt;String#gt;
  }
  class visioncraft__plan___tGatePolicy["GatePolicy"] {
    <<struct>>
    +max_age_ms: Option#lt;u64#gt;
    +require_observed_scope: bool
  }
  class visioncraft__plan___tInvocation["Invocation"] {
    <<struct>>
    +program: String
    +args: Vec#lt;String#gt;
    +cwd: String
    +env: Vec#lt;String#gt;
    +timeout_ms: u64
    +max_output_bytes: u64
    +validate(&self) Result#lt;#40;#41;, ContractError#gt;
  }
  class visioncraft__plan___tQualityPlan["QualityPlan"] {
    <<struct>>
    +version: u32
    +id: String
    +subject: Subject
    +checks: Vec#lt;CheckSpec#gt;
    +expectations: Vec#lt;Expectation#gt;
    +policy: GatePolicy
    +digest(&self) Result#lt;String, ContractError#gt;
    +parse(bytes: &[u8]) Result#lt;Self, ContractError#gt;
    +validate(&self) Result#lt;#40;#41;, ContractError#gt;
  }
  class visioncraft__plan___tStabilizer["Stabilizer"] {
    <<struct>>
    +check_id: String
    +test_id: String
  }
  class visioncraft__plan___tSubject["Subject"] {
    <<struct>>
    +repository: String
    +revision: String
    +configuration: String
    +dirty: bool
    +working_tree: Option#lt;Artifact#gt;
    +inputs: Vec#lt;Artifact#gt;
    +validate(&self) Result#lt;#40;#41;, ContractError#gt;
  }
  class visioncraft__plan["visioncraft::plan"] {
    <<module>>
    +const MAX_PLAN_CHECKS: usize
    +const MAX_PLAN_DURATION_MS: u64
    +const MAX_PLAN_OUTPUT_BYTES: u64
    ~same_inputs(crate) bool
    ~validate_input_paths(crate) Result#lt;#40;#41;, ContractError#gt;
    ~validate_inputs(crate) Result#lt;#40;#41;, ContractError#gt;
  }
  class visioncraft___tContractError["ContractError"] {
    <<enum in src/lib.rs>>
  }
  class _sealmap_corpus__review__Artifact["sealmap_corpus::review::Artifact"] {
    <<external>>
  }
  visioncraft__plan ..> visioncraft___tContractError
  visioncraft__plan ..> _sealmap_corpus__review__Artifact
  visioncraft__plan___tCheckSpec *-- visioncraft__plan___tEvaluator : evaluator
  visioncraft__plan___tCheckSpec *-- visioncraft__plan___tInvocation : invocation
  visioncraft__plan___tExpectation *-- visioncraft__plan___tEvidenceCategory : category
  visioncraft__plan___tExpectation o-- visioncraft__plan___tStabilizer : stabilizer
  visioncraft__plan___tInvocation ..> visioncraft___tContractError
  visioncraft__plan___tQualityPlan ..> visioncraft___tContractError
  visioncraft__plan___tQualityPlan o-- visioncraft__plan___tCheckSpec : checks
  visioncraft__plan___tQualityPlan o-- visioncraft__plan___tExpectation : expectations
  visioncraft__plan___tQualityPlan *-- visioncraft__plan___tGatePolicy : policy
  visioncraft__plan___tQualityPlan *-- visioncraft__plan___tSubject : subject
  visioncraft__plan___tSubject ..> visioncraft___tContractError
  visioncraft__plan___tSubject o-- _sealmap_corpus__review__Artifact : working_tree, inputs
```

## `sym:cargo visioncraft . plan/Subject#validate().`
`pub fn validate(&self) -> Result<(), ContractError>` · L36-L58
> Check identity, dirty-snapshot and unique input-path requirements.
```mermaid
sequenceDiagram
  participant visioncraft__plan___tSubject as Subject
  participant visioncraft as visioncraft mod
  participant visioncraft__plan as plan mod
  visioncraft__plan___tSubject->>visioncraft: nonempty(&self.repository)
  visioncraft__plan___tSubject->>visioncraft: require(nonempty(), #quot;subject.repo…)?
  visioncraft__plan___tSubject->>visioncraft: hex(&self.revision, &_)
  visioncraft__plan___tSubject->>visioncraft: require(hex(), #quot;subject.revi…)?
  visioncraft__plan___tSubject->>visioncraft: nonempty(&self.configuration)
  visioncraft__plan___tSubject->>visioncraft: require(nonempty(), #quot;subject.conf…)?
  visioncraft__plan___tSubject->>visioncraft: require(_, #quot;dirty subjec…)?
  opt let Some(a) = &self.working_tree
    visioncraft__plan___tSubject->>visioncraft: artifact_shape(a)?
  end
  visioncraft__plan___tSubject->>visioncraft__plan: validate_inputs(&self.inputs)
```

## `sym:cargo visioncraft . plan/Invocation#validate().`
`pub fn validate(&self) -> Result<(), ContractError>` · L104-L137
> Validate the bounded argv contract without resolving executables or env.
```mermaid
sequenceDiagram
  participant visioncraft__plan___tInvocation as Invocation
  participant visioncraft as visioncraft mod
  visioncraft__plan___tInvocation->>visioncraft: nonempty(&self.program)
  visioncraft__plan___tInvocation->>visioncraft: require(nonempty(), #quot;invocation.p…)?
  visioncraft__plan___tInvocation->>visioncraft: require(all(), #quot;argv cannot …)?
  visioncraft__plan___tInvocation->>visioncraft: relative_path(&self.cwd)
  visioncraft__plan___tInvocation->>visioncraft: require(_, #quot;cwd must be …)?
  loop for name in &self.env
    visioncraft__plan___tInvocation->>visioncraft: require(_, #quot;invalid envi…)?
    visioncraft__plan___tInvocation->>visioncraft: require(insert(), #quot;duplicate en…)?
  end
  visioncraft__plan___tInvocation->>visioncraft: require(contains(), #quot;timeout_ms o…)?
  visioncraft__plan___tInvocation->>visioncraft: require(contains(), #quot;max_output_b…)
```

## `sym:cargo visioncraft . plan/QualityPlan#validate().`
`pub fn validate(&self) -> Result<(), ContractError>` · L291-L398
> Validate identifiers, references and bounded execution policies.
```mermaid
sequenceDiagram
  participant visioncraft__plan___tQualityPlan as QualityPlan
  participant visioncraft as visioncraft mod
  participant visioncraft__plan___tSubject as Subject
  visioncraft__plan___tQualityPlan->>visioncraft: require(_, #quot;unsupported …)?
  visioncraft__plan___tQualityPlan->>visioncraft: require(_, #quot;plan check c…)?
  visioncraft__plan___tQualityPlan->>visioncraft: identifier(&self.id)
  visioncraft__plan___tQualityPlan->>visioncraft: require(identifier(), #quot;invalid plan…)?
  visioncraft__plan___tQualityPlan->>visioncraft__plan___tSubject: validate()?
  visioncraft__plan___tQualityPlan->>visioncraft: require(_, #quot;a gate requi…)?
  visioncraft__plan___tQualityPlan->>visioncraft: require(_, #quot;max_age_ms m…)?
  loop for c in &self.checks
    visioncraft__plan___tQualityPlan->>visioncraft: identifier(&c.id)
    visioncraft__plan___tQualityPlan->>visioncraft: require(_, #quot;invalid or d…)?
    visioncraft__plan___tQualityPlan->>visioncraft: require(_, #quot;aggregate de…)?
    visioncraft__plan___tQualityPlan->>visioncraft: require(_, #quot;aggregate de…)?
    visioncraft__plan___tQualityPlan->>visioncraft: require(_, #quot;check scope …)?
    loop for p in &c.scope
      visioncraft__plan___tQualityPlan->>visioncraft: require(_, #quot;scope must u…)?
    end
    alt Evaluator::ExitCode { success_codes }
      visioncraft__plan___tQualityPlan->>visioncraft: require(_, #quot;success_code…)?
    else Evaluator::Reported { adapter }
      visioncraft__plan___tQualityPlan->>visioncraft: identifier(adapter)
      visioncraft__plan___tQualityPlan->>visioncraft: require(identifier(), #quot;reported eva…)?
    end
  end
  loop for e in &self.expectations
    visioncraft__plan___tQualityPlan->>visioncraft: identifier(&e.id)
    visioncraft__plan___tQualityPlan->>visioncraft: require(_, #quot;invalid or d…)?
    visioncraft__plan___tQualityPlan->>visioncraft: nonempty(&e.claim)
    visioncraft__plan___tQualityPlan->>visioncraft: require(nonempty(), #quot;expectation …)?
    visioncraft__plan___tQualityPlan->>visioncraft: require(_, #quot;expectation …)?
    visioncraft__plan___tQualityPlan->>visioncraft: require(_, #quot;regression-c…)?
    opt let Some(s) = &e.stabilizer
      visioncraft__plan___tQualityPlan->>visioncraft: nonempty(&s.test_id)
      visioncraft__plan___tQualityPlan->>visioncraft: require(_, #quot;invalid stab…)?
    end
    opt let Some(c) = &e.positive_control
      visioncraft__plan___tQualityPlan->>visioncraft: require(_, #quot;positive con…)?
    end
    opt e.category != EvidenceCategory::Executable
      visioncraft__plan___tQualityPlan->>visioncraft: require(is_some_and(), #quot;partial/manu…)?
    end
  end
```
