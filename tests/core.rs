//! Offline contract probes: no child processes, filesystem fixtures or models.
use visioncraft::*;

fn artifact(path: &str, bytes: &[u8]) -> Artifact {
    Artifact {
        path: path.into(),
        sha256: sha256(bytes),
    }
}

fn plan() -> QualityPlan {
    let invocation = Invocation {
        program: "cargo".into(),
        args: vec!["test".into()],
        cwd: ".".into(),
        env: vec!["PATH".into()],
        timeout_ms: 1000,
        max_output_bytes: 1024,
    };
    QualityPlan {
        version: 1,
        id: "quality".into(),
        subject: Subject {
            repository: "example".into(),
            revision: "a".repeat(40),
            configuration: "default features; unit fixture".into(),
            dirty: false,
            working_tree: None,
            inputs: vec![artifact("src/lib.rs", b"fn implementation() {}")],
        },
        checks: ["main", "control", "probe"]
            .into_iter()
            .map(|id| CheckSpec {
                id: id.into(),
                required: id == "main",
                invocation: invocation.clone(),
                evaluator: Evaluator::ExitCode {
                    success_codes: vec![0],
                },
                scope: vec!["src/lib.rs".into()],
            })
            .collect(),
        expectations: vec![],
        policy: GatePolicy {
            max_age_ms: Some(1000),
            require_observed_scope: true,
        },
    }
}

fn receipt(plan: &QualityPlan, id: &str, verified: &mut VerifiedArtifacts) -> ExecutionReceipt {
    let stdout = artifact(&format!("{id}/stdout.txt"), b"ok\n");
    let stderr = artifact(&format!("{id}/stderr.txt"), b"");
    verified.insert(&stdout, b"ok\n").unwrap();
    verified.insert(&stderr, b"").unwrap();
    ExecutionReceipt {
        version: 1,
        attempt_id: format!("attempt-{id}"),
        plan_sha256: plan.digest().unwrap(),
        check_id: id.into(),
        subject: plan.subject.clone(),
        invocation: plan
            .checks
            .iter()
            .find(|c| c.id == id)
            .unwrap()
            .invocation
            .clone(),
        executable: "/usr/bin/cargo".into(),
        executable_sha256: None,
        tool_version: Some("fixture-1".into()),
        started_at_ms: 1000,
        finished_at_ms: 1010,
        elapsed_ms: 10,
        process: ProcessStatus::Completed,
        exit_code: Some(0),
        signal: None,
        capture_complete: true,
        stdout: Some(stdout),
        stderr: Some(stderr),
        inputs_after: plan.subject.inputs.clone(),
        observed_scope: Some(vec!["src/lib.rs".into()]),
        observed_tests: vec!["unit::contract".into()],
        reported: None,
        diagnostic: None,
    }
}

fn expectation() -> Expectation {
    Expectation {
        id: "EXP-001".into(),
        claim: "the scoped contract holds".into(),
        required: true,
        category: EvidenceCategory::Executable,
        checks: vec!["main".into()],
        regression_critical: true,
        stabilizer: Some(Stabilizer {
            check_id: "main".into(),
            test_id: "unit::contract".into(),
        }),
        positive_control: Some("control".into()),
        independent_audit: false,
        limitation: None,
    }
}

fn report(
    plan: &QualityPlan,
    receipts: &[ExecutionReceipt],
    verified: &VerifiedArtifacts,
) -> GateReport {
    evaluate(
        plan,
        receipts,
        &EvaluationContext::new(1020, &plan.subject, verified),
    )
}

fn audit(plan: &QualityPlan, verified: &mut VerifiedArtifacts) -> AuditReceipt {
    let artifact = artifact("audit.txt", b"independent scoped observation");
    verified
        .insert(&artifact, b"independent scoped observation")
        .unwrap();
    AuditReceipt {
        expectation_id: "EXP-001".into(),
        plan_sha256: plan.digest().unwrap(),
        subject: plan.subject.clone(),
        producer: Actor {
            agent_id: "producer".into(),
            attempt_id: "attempt-main".into(),
            model: "gateway:claude-sonnet".into(),
        },
        reviewer: Actor {
            agent_id: "reviewer".into(),
            attempt_id: "attempt-probe".into(),
            model: "gateway:gpt-6-astra".into(),
        },
        probe_check: "probe".into(),
        verdict: AuditVerdict::Supported,
        observed: "control and counterexample probe executed".into(),
        artifact,
        captured_at_ms: 1015,
    }
}

#[test]
fn normal_receipts_pass_without_optional_runs_or_an_expectation_ledger() {
    let plan = plan();
    let mut verified = VerifiedArtifacts::default();
    let run = receipt(&plan, "main", &mut verified);
    let report = report(&plan, &[run], &verified);
    assert!(report.passed(), "{report:?}");
    assert_eq!(report.required_checks, 1);
    assert_eq!(report.passed_required_checks, 1);
    assert_eq!(report.checks.len(), 3);
    assert!(report.expectations.is_empty());
}

#[test]
fn missing_incomplete_and_failed_checks_cannot_be_required_passes() {
    let plan = plan();
    let mut verified = VerifiedArtifacts::default();
    assert_eq!(
        report(&plan, &[], &verified).outcome,
        GateOutcome::Incomplete
    );
    for status in [
        ProcessStatus::Unavailable,
        ProcessStatus::TimedOut,
        ProcessStatus::Interrupted,
        ProcessStatus::FailedToStart,
        ProcessStatus::CaptureFailed,
    ] {
        let mut run = receipt(&plan, "main", &mut verified);
        run.process = status;
        run.diagnostic = Some("fixture could not execute to completion".into());
        run.capture_complete = false;
        assert!(!report(&plan, &[run], &verified).passed(), "{status:?}");
    }
    let mut run = receipt(&plan, "main", &mut verified);
    run.exit_code = Some(1);
    assert_eq!(
        report(&plan, &[run], &verified).outcome,
        GateOutcome::Blocked
    );
}

#[test]
fn optional_failure_remains_visible_without_changing_the_denominator() {
    let plan = plan();
    let mut verified = VerifiedArtifacts::default();
    let main = receipt(&plan, "main", &mut verified);
    let mut optional = receipt(&plan, "probe", &mut verified);
    optional.exit_code = Some(1);
    let report = report(&plan, &[main, optional], &verified);
    assert!(report.passed());
    assert_eq!(report.checks[2].outcome, CheckOutcome::Blocked);
    assert_eq!(report.required_checks, 1);
}

#[test]
fn changing_policy_subject_or_argv_cannot_reuse_an_old_pass() {
    let original = plan();
    let mut verified = VerifiedArtifacts::default();
    let run = receipt(&original, "main", &mut verified);
    for case in 0..3 {
        let mut changed = original.clone();
        match case {
            0 => changed.policy.require_observed_scope = false,
            1 => changed.subject.configuration.push_str(" other"),
            _ => changed.checks[0].invocation.args.push("--release".into()),
        }
        assert_eq!(
            report(&changed, std::slice::from_ref(&run), &verified).outcome,
            GateOutcome::Error
        );
    }
    for case in 0..3 {
        let mut changed = run.clone();
        match case {
            0 => changed.plan_sha256 = "b".repeat(64),
            1 => changed.subject.revision = "b".repeat(40),
            _ => changed.invocation.args.push("--release".into()),
        }
        assert_eq!(
            report(&original, &[changed], &verified).outcome,
            GateOutcome::Error
        );
    }
}

#[test]
fn output_hashes_are_required_and_mutated_bytes_never_verify() {
    let plan = plan();
    let mut verified = VerifiedArtifacts::default();
    let run = receipt(&plan, "main", &mut verified);
    assert_eq!(
        report(
            &plan,
            std::slice::from_ref(&run),
            &VerifiedArtifacts::default()
        )
        .outcome,
        GateOutcome::Error
    );
    let mut bad = VerifiedArtifacts::default();
    assert!(
        bad.insert(run.stdout.as_ref().unwrap(), b"different")
            .is_err()
    );
    assert!(!bad.contains(run.stdout.as_ref().unwrap()));
    assert!(report(&plan, &[run], &verified).passed());
}

#[test]
fn exceeding_output_limit_cannot_be_hidden_by_capture_complete() {
    let mut plan = plan();
    plan.checks[0].invocation.max_output_bytes = 2;
    let mut verified = VerifiedArtifacts::default();
    let run = receipt(&plan, "main", &mut verified); // stdout is 3 actual bytes.
    assert_eq!(report(&plan, &[run], &verified).outcome, GateOutcome::Error);
}

#[test]
fn deleted_all_selected_inputs_remains_an_honest_mutation_receipt() {
    let plan = plan();
    let mut verified = VerifiedArtifacts::default();
    let mut run = receipt(&plan, "main", &mut verified);
    run.inputs_after.clear();
    run.validate().unwrap();
    assert_eq!(
        report(&plan, &[run], &verified).outcome,
        GateOutcome::Incomplete
    );
}

#[test]
fn duplicate_attempts_and_unknown_check_ids_are_not_silently_selected() {
    let plan = plan();
    let mut verified = VerifiedArtifacts::default();
    let run = receipt(&plan, "main", &mut verified);
    assert_eq!(
        report(&plan, &[run.clone(), run.clone()], &verified).outcome,
        GateOutcome::Error
    );
    let mut unknown = run.clone();
    unknown.check_id = "not-planned".into();
    assert_eq!(
        report(&plan, &[run, unknown], &verified).outcome,
        GateOutcome::Error
    );
}

#[test]
fn known_partial_scope_and_required_unknown_scope_do_not_pass() {
    let plan = plan();
    let mut verified = VerifiedArtifacts::default();
    for scope in [None, Some(vec![])] {
        let mut run = receipt(&plan, "main", &mut verified);
        run.observed_scope = scope;
        assert_eq!(
            report(&plan, &[run], &verified).outcome,
            GateOutcome::Incomplete
        );
    }
    let mut tolerant = plan.clone();
    tolerant.policy.require_observed_scope = false;
    let mut run = receipt(&tolerant, "main", &mut verified);
    run.observed_scope = None;
    assert!(report(&tolerant, &[run], &verified).passed());
}

#[test]
fn freshness_uses_injected_clock_and_relevant_bytes_not_unrelated_head_changes() {
    let plan = plan();
    let mut verified = VerifiedArtifacts::default();
    let run = receipt(&plan, "main", &mut verified);
    assert!(
        evaluate(
            &plan,
            std::slice::from_ref(&run),
            &EvaluationContext::new(2010, &plan.subject, &verified)
        )
        .passed()
    );
    for now in [1009, 2011] {
        assert_eq!(
            evaluate(
                &plan,
                std::slice::from_ref(&run),
                &EvaluationContext::new(now, &plan.subject, &verified)
            )
            .outcome,
            GateOutcome::Incomplete
        );
    }
    let mut current = plan.subject.clone();
    current.revision = "b".repeat(40);
    let ancestry = [plan.subject.revision.clone()];
    let mut context = EvaluationContext::new(1020, &current, &verified);
    assert_eq!(
        evaluate(&plan, std::slice::from_ref(&run), &context).outcome,
        GateOutcome::Incomplete
    );
    context.ancestor_revisions = &ancestry;
    assert!(evaluate(&plan, std::slice::from_ref(&run), &context).passed());
    current.inputs[0].sha256 = "c".repeat(64);
    let mut context = EvaluationContext::new(1020, &current, &verified);
    context.ancestor_revisions = &ancestry;
    assert_eq!(
        evaluate(&plan, &[run], &context).outcome,
        GateOutcome::Incomplete
    );
}

#[test]
fn malformed_and_unknown_json_never_echoes_source_values() {
    let json = serde_json::to_string(&plan()).unwrap();
    for invalid in [
        json.replacen("{", "{\"secret-SENTINEL\":1,", 1),
        json.replacen("\"version\":1", "\"version\":1,\"version\":1", 1),
        "{\"version\":\"private-SENTINEL\"}".into(),
    ] {
        let error = QualityPlan::parse(invalid.as_bytes())
            .unwrap_err()
            .to_string();
        assert!(!error.contains("SENTINEL"));
    }
    let mut empty = plan();
    empty.checks.clear();
    assert!(empty.validate().is_err());
    let mut optional = plan();
    for c in &mut optional.checks {
        c.required = false;
    }
    assert!(optional.validate().is_err());
    let mut duplicate = plan();
    duplicate.checks.push(duplicate.checks[0].clone());
    assert!(duplicate.validate().is_err());
}

#[test]
fn policy_references_and_resource_limits_are_strict() {
    for case in 0..7 {
        let mut plan = plan();
        match case {
            0 => plan.checks[0].invocation.timeout_ms = 0,
            1 => plan.checks[0].invocation.max_output_bytes = u64::MAX,
            2 => plan.checks[0].invocation.cwd = "../outside".into(),
            3 => plan.checks[0].scope = vec!["missing.rs".into()],
            4 => plan.subject.inputs.push(plan.subject.inputs[0].clone()),
            5 => plan.subject.dirty = true,
            _ => {
                plan.checks[0].evaluator = Evaluator::ExitCode {
                    success_codes: vec![0, 0],
                }
            }
        }
        assert!(plan.validate().is_err(), "case {case}");
    }
}

#[test]
fn stabilisation_needs_an_executed_test_and_a_successful_distinct_control() {
    let mut plan = plan();
    plan.expectations.push(expectation());
    let mut verified = VerifiedArtifacts::default();
    let mut runs = vec![
        receipt(&plan, "main", &mut verified),
        receipt(&plan, "control", &mut verified),
    ];
    assert!(report(&plan, &runs, &verified).passed());
    runs[0].observed_tests.clear();
    assert_eq!(
        report(&plan, &runs, &verified).outcome,
        GateOutcome::Incomplete
    );
    runs[0].observed_tests.push("unit::contract".into());
    runs[1].exit_code = Some(1);
    assert_eq!(
        report(&plan, &runs, &verified).outcome,
        GateOutcome::Blocked
    );
    plan.expectations[0].positive_control = Some("main".into());
    assert!(plan.validate().is_err());
}

#[test]
fn model_independence_uses_actual_families_and_binds_actor_attempts() {
    let mut plan = plan();
    let mut expectation = expectation();
    expectation.independent_audit = true;
    plan.expectations.push(expectation);
    let mut verified = VerifiedArtifacts::default();
    let runs = ["main", "control", "probe"].map(|id| receipt(&plan, id, &mut verified));
    let valid = audit(&plan, &mut verified);
    let context = EvaluationContext::new(1020, &plan.subject, &verified);
    assert_eq!(
        evaluate(&plan, &runs, &context).outcome,
        GateOutcome::Incomplete
    );
    for case in 0..5 {
        let mut audit = valid.clone();
        match case {
            0 => audit.reviewer.model = "other:claude-opus".into(),
            1 => audit.reviewer.model = "gateway:unknown".into(),
            2 => audit.reviewer.agent_id = audit.producer.agent_id.clone(),
            3 => audit.reviewer.attempt_id = "unrelated-attempt".into(),
            _ => audit.plan_sha256 = "0".repeat(64),
        }
        let audits = [audit];
        let mut context = EvaluationContext::new(1020, &plan.subject, &verified);
        context.audits = &audits;
        assert_eq!(
            evaluate(&plan, &runs, &context).outcome,
            GateOutcome::Incomplete,
            "case {case}"
        );
    }
    let audits = [valid];
    let mut context = EvaluationContext::new(1020, &plan.subject, &verified);
    context.audits = &audits;
    assert!(evaluate(&plan, &runs, &context).passed());
}

#[test]
fn a_completed_counterexample_probe_blocks_even_when_its_test_assertion_failed() {
    let mut plan = plan();
    let mut expectation = expectation();
    expectation.independent_audit = true;
    plan.expectations.push(expectation);
    let mut verified = VerifiedArtifacts::default();
    let mut runs = ["main", "control", "probe"].map(|id| receipt(&plan, id, &mut verified));
    runs[2].exit_code = Some(1);
    let mut audit = audit(&plan, &mut verified);
    audit.verdict = AuditVerdict::Contradicted;
    let audits = [audit];
    let mut context = EvaluationContext::new(1020, &plan.subject, &verified);
    context.audits = &audits;
    assert_eq!(
        evaluate(&plan, &runs, &context).outcome,
        GateOutcome::Blocked
    );
}

#[test]
fn manual_observations_require_a_bound_named_observation_not_a_boolean_claim() {
    let mut plan = plan();
    let mut e = expectation();
    e.category = EvidenceCategory::Manual;
    e.limitation = Some("device requires manual observation".into());
    plan.expectations.push(e);
    let mut verified = VerifiedArtifacts::default();
    let runs = ["main", "control"].map(|id| receipt(&plan, id, &mut verified));
    assert_eq!(
        report(&plan, &runs, &verified).outcome,
        GateOutcome::Incomplete
    );
    let artifact = artifact("human.txt", b"observed state");
    verified.insert(&artifact, b"observed state").unwrap();
    let observation = ManualObservation {
        expectation_id: "EXP-001".into(),
        plan_sha256: plan.digest().unwrap(),
        subject: plan.subject.clone(),
        observer: "operator receipt #1".into(),
        observed: "rendering observed on named device".into(),
        verdict: AuditVerdict::Supported,
        artifact,
        captured_at_ms: 1010,
    };
    let observations = [observation];
    let mut context = EvaluationContext::new(1020, &plan.subject, &verified);
    context.manual_observations = &observations;
    assert!(evaluate(&plan, &runs, &context).passed());
}

#[test]
fn reported_evaluators_need_the_exact_adapter_and_verified_raw_report() {
    let mut plan = plan();
    plan.checks[0].evaluator = Evaluator::Reported {
        adapter: "fixture-review".into(),
    };
    let mut verified = VerifiedArtifacts::default();
    let mut run = receipt(&plan, "main", &mut verified);
    assert_eq!(
        report(&plan, std::slice::from_ref(&run), &verified).outcome,
        GateOutcome::Error
    );
    let artifact = artifact("review.json", b"{\"completed\":true}");
    run.reported = Some(ReportedResult {
        adapter: "fixture-review".into(),
        outcome: CheckOutcome::Passed,
        reason: "lower-priority findings remain below configured threshold".into(),
        artifact: artifact.clone(),
    });
    assert_eq!(
        report(&plan, std::slice::from_ref(&run), &verified).outcome,
        GateOutcome::Error
    );
    verified.insert(&artifact, b"{\"completed\":true}").unwrap();
    assert!(report(&plan, &[run], &verified).passed());
}

#[test]
fn hash_vector_and_literal_command_contract_do_not_require_runtime_access() {
    assert_eq!(
        sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let mut plan = plan();
    plan.checks[0].invocation.args = vec!["$(do-not-execute); `nor-this`".into()];
    let mut verified = VerifiedArtifacts::default();
    let run = receipt(&plan, "main", &mut verified);
    let roundtrip = QualityPlan::parse(&serde_json::to_vec(&plan).unwrap()).unwrap();
    assert_eq!(plan.digest().unwrap(), roundtrip.digest().unwrap());
    assert!(report(&plan, &[run], &verified).passed());
}

#[test]
fn optional_contract_corruption_is_an_input_error_not_an_optional_failure() {
    let plan = plan();
    let mut verified = VerifiedArtifacts::default();
    let main = receipt(&plan, "main", &mut verified);
    let mut invalid = receipt(&plan, "probe", &mut verified);
    invalid.exit_code = None;
    assert_eq!(
        report(&plan, &[main, invalid], &verified).outcome,
        GateOutcome::Error
    );
}

#[test]
fn audit_cannot_predate_the_executions_it_claims_to_have_observed() {
    let mut plan = plan();
    let mut e = expectation();
    e.independent_audit = true;
    plan.expectations.push(e);
    let mut verified = VerifiedArtifacts::default();
    let runs = ["main", "control", "probe"].map(|id| receipt(&plan, id, &mut verified));
    let mut audit = audit(&plan, &mut verified);
    audit.captured_at_ms = 1001;
    let audits = [audit];
    let mut context = EvaluationContext::new(1020, &plan.subject, &verified);
    context.audits = &audits;
    assert_eq!(
        evaluate(&plan, &runs, &context).outcome,
        GateOutcome::Incomplete
    );
}

#[test]
fn a_manual_expectation_does_not_need_a_fabricated_executable_scenario() {
    let mut plan = plan();
    let mut e = expectation();
    e.category = EvidenceCategory::Manual;
    e.limitation = Some("requires physical observation".into());
    e.checks.clear();
    e.regression_critical = false;
    e.stabilizer = None;
    e.positive_control = None;
    plan.expectations.push(e);
    plan.validate().unwrap();
    let mut verified = VerifiedArtifacts::default();
    let run = receipt(&plan, "main", &mut verified);
    let artifact = artifact("manual.txt", b"physical observation");
    verified.insert(&artifact, b"physical observation").unwrap();
    let observations = [ManualObservation {
        expectation_id: "EXP-001".into(),
        plan_sha256: plan.digest().unwrap(),
        subject: plan.subject.clone(),
        observer: "operator observation #2".into(),
        observed: "device inspection".into(),
        verdict: AuditVerdict::Supported,
        artifact,
        captured_at_ms: 1010,
    }];
    let mut context = EvaluationContext::new(1020, &plan.subject, &verified);
    context.manual_observations = &observations;
    assert!(evaluate(&plan, &[run], &context).passed());
}

#[test]
fn a_supplied_contradiction_cannot_be_ignored_as_unrequested_optional_metadata() {
    let mut plan = plan();
    plan.expectations.push(expectation());
    let mut verified = VerifiedArtifacts::default();
    let runs = ["main", "control", "probe"].map(|id| receipt(&plan, id, &mut verified));
    let mut audit = audit(&plan, &mut verified);
    audit.verdict = AuditVerdict::Contradicted;
    let audits = [audit];
    let mut context = EvaluationContext::new(1020, &plan.subject, &verified);
    context.audits = &audits;
    assert!(!evaluate(&plan, &runs, &context).passed());
}

#[test]
fn valid_individual_checks_cannot_form_an_unbounded_aggregate_plan() {
    let mut p = plan();
    p.checks.truncate(2);
    for check in &mut p.checks {
        check.invocation.timeout_ms = 43_200_000;
        check.invocation.max_output_bytes = 536_870_912;
    }
    assert!(
        p.validate().is_ok(),
        "exact aggregate ceilings remain usable"
    );
    p.checks[1].invocation.timeout_ms += 1;
    assert!(
        p.validate().is_err(),
        "individually valid deadlines cannot exceed the whole-plan ceiling"
    );
    p.checks[1].invocation.timeout_ms -= 1;
    p.checks[1].invocation.max_output_bytes += 1;
    assert!(
        p.validate().is_err(),
        "individually valid captures cannot exceed the aggregate ceiling"
    );
    let mut p = plan();
    let seed = p.checks[0].clone();
    p.checks = (0..1025)
        .map(|n| {
            let mut c = seed.clone();
            c.id = format!("check-{n}");
            c
        })
        .collect();
    assert!(
        p.validate().is_err(),
        "required and optional checks share admission limits"
    );
}
