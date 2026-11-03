---
id: ADR-0001
title: Portable quality contracts with explicit execution adapters
status: accepted
date: 2026-11-03
verified_commit: 2c5a625ce245be2a5ec2f0b30633d708b50afb9e
verified_paths:
  - src/plan.rs
  - src/evidence.rs
  - src/evaluate.rs
  - src/cli.rs
  - src/process.rs
  - src/git_metadata.rs
  - Cargo.toml
---

# Portable quality contracts with explicit execution adapters

## Context

The original engineering skill mixed useful methods, host-specific orchestration,
an executable security wrapper and stale descriptions of retired implementations.
Extraction needs a useful reusable library and one canonical skill, while retaining
the host's authority over credentials, tools and deployment.

## Decision

The deterministic core accepts plans, receipts, verified artifact bytes and an
injected evaluation context. It performs no filesystem or network operations.
Plans bind code/configuration and literal invocations; aggregate admission bounds
are checked before execution. Required missing, incomplete or contradicted evidence
cannot become a pass by omission. Observed coverage is distinct from intended scope.

The native CLI observes source snapshots directly. Repository and configuration
identifiers are caller declarations, explicitly labelled as unverified when no
current-subject attestation is supplied. Include configuration files in the
selected inputs when their exact bytes must be checked automatically.

Optional Unix process/CLI adapters execute explicit commands with private captures,
deadlines and process-group cancellation. Read-only evaluation never executes an
imported receipt's command. Fixed Git metadata probes use the same bounded runner,
remove ambient Git redirection and avoid repository clean filters. This is not an
operating-system sandbox.

Sealmap owns model-family and finding-disposition contracts. VisionCraft reuses
those types and validates supplied execution evidence rather than introducing a
parallel finding store. Project ADRs and diagrams remain source-controlled records;
optional search/index tools are consumers.

## Alternatives and consequences

A prose-only extraction would preserve the original gap between requested checks
and executable evidence. A new agent orchestration runtime would duplicate host
execution and custody services. The selected boundary supports a pure library,
portable foreground CLI and independently maintained host adapters.

Generic command receipts cannot prove test coverage or claim that a model inspected
all relevant code. Independent audits and manual observations remain explicit,
scoped evidence. Publication, adoption and live deployment require separate proof.

## Verification

`tests/core.rs` checks policy/input identity, aggregate limits, missing/partial
observations, counterexamples, controls and model-family independence.
`tests/native_cli.rs` checks actual execution, source drift, private snapshots,
read-only evaluation and confined metadata inspection. `tests/process.rs` exercises
literal arguments, environment selection, timeout/cancellation and capture failure.
The inspected implementation revision above predates this documentation commit;
refresh only after rechecking changes to its governed paths.
