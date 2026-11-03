# VisionCraft

**Engineering guidance and executable evidence gates for coding agents and people.**

VisionCraft helps you define a change, run proportionate checks, assess what the
results support, and carry that evidence through review and release. Use its
[skill](skills/visioncraft/SKILL.md), Rust library, CLI, or a combination.

It works without Agentbox, an orchestration framework or a model subscription.
The deterministic core does not launch processes or make network calls. The optional CLI
runs explicitly requested commands; optional upstream tools provide their own
specialist capabilities.

## What belongs where

| Component | Responsibility |
|---|---|
| VisionCraft skill | EDD/TDD, debugging, design decisions, review and delivery practices |
| VisionCraft Rust core | Typed plans, evidence identity, expectation coverage and deterministic gate decisions |
| VisionCraft CLI | Explicit command execution, bounded private captures and filesystem verification |
| Agentic QE | Optional upstream QE tools: generate tests, inspect coverage and assess supplied quality evidence |
| Deepsec | Optional upstream security investigations through a versioned CLI adapter |
| Sealmap | Source-derived diagrams, model-family identity and adjacent finding-disposition contracts |
| Your repository and CI | Authoritative requirements, ADRs, tests, configuration and release observations |

A passing gate means the declared checks and evidence policy were satisfied.
It is not proof of correctness, security, complete test coverage or deployment.

## Install

Rust 1.88 or later is required. The process CLI currently supports Unix.

```sh
cargo install visioncraft --locked --version 0.1.0
visioncraft --help
```

For the pure Rust library without CLI/process dependencies:

```toml
[dependencies]
visioncraft = { version = "0.1.0", default-features = false }
```

For a coding agent, install the complete `skills/visioncraft/` directory from the
same release using your harness's skill mechanism. Keep its `references/` and
`assets/` alongside `SKILL.md`; they contain the actual engineering method.
Reading a skill does not install or start any upstream service.

## A complete local example

Create a small Git repository with a real assertion. The input list explicitly
includes the files whose bytes the check claims to cover.

```sh
mkdir visioncraft-example
cd visioncraft-example
git init -q
printf '12\n' > limit.txt
git add limit.txt
git -c user.name=Example -c user.email=example@example.invalid commit -qm baseline

visioncraft plan init --out quality.json --input limit.txt \
  --configuration 'POSIX assertion of configured limit' \
  -- sh -c 'test "$(cat limit.txt)" = 12'
visioncraft plan check quality.json --json
visioncraft run quality.json --all --out evidence-1
visioncraft evaluate quality.json --receipts evidence-1 --json
```

The final command verifies the plan, exact source and capture hashes, check
invocation and recorded exit. `run` reports execution status; `evaluate` makes
the quality-gate decision. The explicit `sh -c` above asks for shell syntax;
VisionCraft itself passes argument arrays directly and adds no shell.

Change `limit.txt` after running the check and repeat `evaluate`: the old
receipt cannot satisfy the new input. Create a new plan and fresh run directory
after intentionally changing a check's scope or configuration. Existing output
paths are refused to avoid overwriting evidence.

Repository and configuration labels are caller declarations. Without
`--current-subject`, evaluation reports their freshness as
`plan-declared-unverified`; the CLI observes the selected source snapshot, but
does not discover repository identity or configuration from the environment.
To attest current labels, supply a current `Subject` JSON record:

```sh
visioncraft evaluate quality.json --receipts evidence-1 \
  --current-subject current-subject.json --json
```

The CLI checks that record's source fields against disk and treats its
repository/configuration fields as `caller-attested`. Different labels make the
old evidence stale; matching labels remain attestations. A configuration digest
in a label is not automatically verified against a file. Include the actual
configuration artifact in `--input` to check its bytes; changed selected
configuration invalidates the evidence even if the label stays unchanged.

For a real project, include relevant source, lockfiles and configuration in
`--input`; pass required environment **names** with repeated `--env` options.
Only named environment variables are inherited. Set useful `--timeout-ms` and
`--max-output-bytes` bounds rather than relying on unlimited jobs. A plan admits at most 1,024 checks, one
day of summed command deadlines and one GiB of summed capture limits.

## Expectations and review

The [EDD reference](skills/visioncraft/references/edd.md) describes executable,
partly verifiable and manual expectations. A plan can bind expectations to
checks, positive controls, stabilising regression tests and independent audit
requirements. Generic command execution cannot infer which test IDs or source
lines a test runner reached; observed coverage remains unknown unless an
observer supplies it.

```sh
visioncraft expectations check quality.json --receipts evidence-1 --json
visioncraft gate evaluate quality.json --receipts evidence-1 --json
```

Both evaluate the entire required gate. Neither hides failed checks merely
because an expectation does not refer to them. Missing, skipped, timed-out,
stale and incomplete required evidence remains non-passing. Optional failures
remain visible without silently becoming required.

For imported findings, use Sealmap's existing receipt contract:

```sh
sealmap review init findings.json --out finding-receipts.json
# Add actual reproductions and dispositions, preserving the original findings.
sealmap review check finding-receipts.json --json
sealmap review summary finding-receipts.json
```

VisionCraft consumes Sealmap's contracts rather than maintaining another finding
validator. Schema checks and byte hashes establish evidence identity; an auditor
still has to judge whether the evidence supports the claim.

## Specialist upstreams

Deepsec is an independently installed optional reviewer. The adapter supports
the exact protocol recorded in [its release contract](contracts/deepsec.json).
No command implicitly installs it, logs in, changes model routes or starts a
review merely because you read the skill.

```sh
# Inspect the resolved plan without running a reviewer.
visioncraft deepsec --diff origin/main --dry-run
# Run the explicitly requested review using already configured local credentials.
visioncraft deepsec --diff origin/main
# Candidate discovery only; SCANNED is not a completed review.
visioncraft deepsec --scan-only
```

The adapter freezes selected file names, checks their bytes before and after,
and requires fresh per-file analysis evidence. In staged mode, differing index
and working-tree bytes are refused. An empty export, refusal, partial review,
unknown completion or unsupported tool version cannot produce PASS. Full mode
reviews the scanner's pending candidates; it does not establish whole-program
coverage. Historical findings may remain in the exported threshold assessment.
Read [the integration guide](skills/visioncraft/references/deepsec.md) before
choosing source scope, model route or spend limits. The [upstream maintenance
procedure](docs/UPSTREAM-DEEPSEC.md) explains how to qualify a new release;
upstream `main`, a published package and an activated host are separate versions.

Agentic QE is consumed through its own discovered CLI/MCP surface. Its
[worked integration](skills/visioncraft/references/agentic-qe.md) and
[versioned result contract](contracts/agentic-qe.json) define that boundary. Generated
tests must be inspected and executed in the project. Preserve measured versus
estimated coverage and `insufficient-evidence` outcomes; a recommendation or a
registered agent name is not an executed check.

## Architecture records and diagrams

Use the project's existing ADR and diagram corpus. Link requirements, decisions,
code, checks and release observations. Implemented claims carry an inspected
commit and nonempty governed paths. Updating a diagram's rendering does not
verify its source claims; a clean relationship graph does not make an ADR current.

[Decision guidance](skills/visioncraft/references/methodologies.md),
[diagram diagnosis](skills/visioncraft/references/diagram-diagnosis.md) and
[traceability](skills/visioncraft/references/traceability.md) explain how to keep
these records useful without building a second documentation database.
This project dogfoods that workflow in its [verified ADRs and diagrams](docs/diagrams/README.md).

## Rust library

The core accepts typed data and supplied bytes. It does not trust an artifact
because its filename or claimed hash looks plausible:

```rust
use visioncraft::{Artifact, VerifiedArtifacts};

let bytes = b"actual check output\n";
let artifact = Artifact {
    path: "attempt/stdout.log".into(),
    sha256: visioncraft::sha256(bytes),
};
let mut verified = VerifiedArtifacts::default();
verified.insert(&artifact, bytes)?;
assert!(verified.contains(&artifact));
# Ok::<(), visioncraft::ContractError>(())
```

`QualityPlan`, `ExecutionReceipt` and `EvaluationContext` describe the policy,
observations and verified current inputs. `evaluate` returns both the aggregate
outcome and individual check/expectation results. See the generated rustdoc and
[executable contract tests](tests/core.rs) for complete typed examples.

## Place in the ecosystem: Agentbox

Agentbox is one consumer. It installs a pinned VisionCraft release, distributes
that release's canonical skill, projects host policy and credentials, and
registers VisionCraft in its local and Jev-assisted skill picker. Ruflo supplies
coordination; direct Agentic QE tools supply QE capabilities. Neither is needed
for portable library use.

Agentbox's Deepsec policy translator passes a typed configuration into the same
VisionCraft CLI used elsewhere. It does not retain a second gate implementation.
The hard cutover removes active `build-with-quality` entry points after checks
pass; [the migration guide](docs/MIGRATION.md) maps every former resource.
A committed pin is not evidence that an already running container adopted it.

The [Agentbox integration reference](skills/visioncraft/references/ecosystem-agentbox.md)
is the expanding record of that deployment's memory, routing and operational
boundaries. Other hosts can supply their own adapters without importing them.

## Limits and development

The process runner is **not a sandbox**. Children retain the caller's filesystem
and network authority; environment filtering is not secret isolation. Unix
process groups support cancellation and deadlines, but a deliberately escaping
process or a blocked kernel filesystem call needs host-level containment.
Private capture directories protect ordinary local handling, not a hostile
concurrent filesystem. Keep source-bearing evidence out of public commits.
Linux can recognise exited zombie group members through an unrestricted `/proc`
view; hidden, unreadable or uncertain state remains non-passing. Other Unix
platforms conservatively reject any surviving group, including zombies.

Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
and `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps`. The
[implementation contract](docs/EXPECTATIONS.md) records acceptance boundaries.
The [release review](docs/REVIEW-2026-11-03.md) records reproduced counterexamples
and their dispositions.
[Zeroshot lessons](docs/ZEROSHOT-LESSONS.md) document source-grounded design
patterns adopted without importing another execution engine. Structural skill
checks and forward evaluations are reported separately from
any measured improvement; this rewrite does not imply a success-rate uplift.

MIT OR Apache-2.0. Attributed BeautifulRust material retains its original MIT
notice. See [provenance](docs/PROVENANCE.md), [MIT](LICENSE-MIT) and
[Apache-2.0](LICENSE-APACHE).
