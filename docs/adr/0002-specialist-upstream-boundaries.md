---
id: ADR-0002
title: Consume specialist reviewers through versioned upstream contracts
status: accepted
date: 2026-11-03
depends_on: [ADR-0001]
verified_commit: 6f233886824ef3896cbd8c18fd35fc3b9fbc2bb6
verified_paths:
  - src/deepsec.rs
  - src/deepsec_evidence.rs
  - contracts/deepsec.json
  - contracts/agentic-qe.json
  - skills/visioncraft/references/agentic-qe.md
---

# Consume specialist reviewers through versioned upstream contracts

## Decision

Deepsec and Agentic QE remain separate upstream implementations. The host installs
qualified versions and supplies credentials, source-sharing policy and spend
controls. VisionCraft consumes their observations with explicit provenance; neither
is a mandatory dependency of its deterministic library.

Deepsec uses one native VisionCraft CLI adapter. It freezes source selection,
retains exact bytes' hashes, checks a fresh completed processing run and per-file
analysis records, rejects refusal/partial/unknown output, and applies the configured
severity threshold to the raw export. The receipt distinguishes observed CLI
version from expected package integrity and source revision. Installation integrity
belongs to host evidence. Full mode covers pending scanner candidates, not every
possible programme behaviour.

AQE uses its own discovered MCP/CLI interfaces. Preserve measured, estimated and
insufficient-evidence outcomes. Execute generated tests with the project's real
runner. The versioned AQE result subsets describe observations and do not themselves
grant a successful quality gate. No second QE executor is introduced.

## Host integration

Agentbox supplies the concrete case: pinned release binary and canonical skill,
direct AQE MCP registration, a thin Deepsec policy translator, shared memory and
Jev-assisted discovery. It removes the old active build-with-quality implementation
when the replacement is qualified. A source pin is not runtime activation evidence;
the operator performs the subsequent image rebuild.

AQE's ephemeral memory mode does not disable every embedding attempt. Its upstream
default embedding model differs from Agentbox's bge model. The integration does not
pretend that setting an endpoint alone establishes compatible semantic learning.

## Maintenance and evidence

[Deepsec maintenance](../UPSTREAM-DEEPSEC.md) names source/registry checks, protocol
fixtures, bounded smoke tests and coordinated host activation. The
[AQE guide](../../skills/visioncraft/references/agentic-qe.md) documents its direct
workflow and capability boundaries. Exact package/source metadata lives in the
contracts linked above, rather than competing defaults in each host.

`tests/deepsec.rs` contains positive and negative protocol controls. Host fixtures
exercise policy precedence and disabled/unavailable paths through the actual
translator. A scan-only smoke test is recorded as SCANNED, never a completed model
review. Independent model review and package adoption have separate receipts.
