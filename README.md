# VisionCraft

**Engineering guidance and executable evidence gates for coding agents and people.**

VisionCraft helps you define a change, test it and give reviewers a clear record
of what the results support. Use its [coding-agent skill](skills/visioncraft/SKILL.md),
Rust library or CLI with your existing repository, CI and issue tracker.

For an empty-search bug, that might mean reproducing the error, fixing it and
checking both empty and ordinary queries. A shared event-format change needs
consumer, compatibility and migration checks too. The method scales with the
consequences, while keeping a merged fix distinct from a deployed one.

| Component | What it provides |
|---|---|
| Skill | Expectation-driven development, TDD, debugging, architecture decisions and review practices |
| Rust core | Typed plans, evidence verification and deterministic gate decisions; no process or network calls |
| CLI | Explicit command execution, bounded captures and source/evidence freshness checks |

Required missing, failed, stale or incomplete evidence cannot earn a pass.
Passing means the declared checks satisfied their policy; it does not establish
complete coverage, correctness, security or deployment. No Agentbox installation,
agent framework or model subscription is required for the core.

[Crates.io](https://crates.io/crates/visioncraft) ·
[Rust API](https://docs.rs/visioncraft) ·
[VisionFlow estate](#place-in-the-visionflow-estate)

## Install

Rust 1.88 or later is required. Native execution currently supports Linux with glibc or musl;
the deterministic library remains independent of the operating system.

```sh
cargo install visioncraft --locked --version 0.3.0
visioncraft --help
```

For the pure Rust library without CLI/process dependencies:

```toml
[dependencies]
visioncraft = { version = "0.3.0", default-features = false }
```

For a coding agent, install the complete `skills/visioncraft/` directory from the
same release using your harness's skill mechanism. Keep its `references/` and
`assets/` alongside `SKILL.md`; they contain the actual engineering method.

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

`run` captures execution; `evaluate` verifies the plan, source and capture hashes,
invocation and exit status. Commands use argument arrays without an implicit
shell; this example explicitly requests `sh -c`.

Change `limit.txt` and repeat `evaluate`: the old receipt no longer satisfies the
input. After intentionally changing scope or configuration, create a new plan
and run directory. Existing output paths are refused.

Include relevant source, lockfiles and configuration in `--input`. Only environment
variables named through `--env` are inherited. Set `--timeout-ms` and
`--max-output-bytes`; aggregate limits are 1,024 checks, one day of declared
deadlines and one GiB of capture per plan.

Repository/configuration labels are caller declarations, reported as
`plan-declared-unverified`. To attest current labels, supply a `Subject` record:

```sh
visioncraft evaluate quality.json --receipts evidence-1 \
  --current-subject current-subject.json --json
```

Its source fields are checked against disk; matching repository/configuration
labels remain `caller-attested`. A digest written in a label does not verify a
file: include the actual configuration in `--input` to bind its bytes.

## Expectations and review

The [EDD reference](skills/visioncraft/references/edd.md) explains how to bind
expectations to checks, positive controls, regression tests and independent
review requirements.

```sh
visioncraft expectations check quality.json --receipts evidence-1 --json
visioncraft gate evaluate quality.json --receipts evidence-1 --json
```

Both assess the entire required gate, including checks not referenced by an
expectation. Optional failures stay visible. Actual test or line coverage remains
unknown unless an observer supplies it.

For imported findings, VisionCraft uses Sealmap's receipt contract:

```sh
sealmap review init findings.json --out finding-receipts.json
# Add actual reproductions and dispositions, preserving the original findings.
sealmap review check finding-receipts.json --json
sealmap review summary finding-receipts.json
```

These checks establish structure and byte identity; an auditor judges whether
the observations support the findings.

## Specialist upstreams

**Deepsec** is an optional, independently installed security reviewer with a
[versioned adapter contract](contracts/deepsec.json).

```sh
# Inspect the resolved plan without running a reviewer.
visioncraft deepsec --diff origin/main --dry-run
# Run the explicitly requested review using already configured local credentials.
visioncraft deepsec --diff origin/main
# Candidate discovery only; SCANNED is not a completed review.
visioncraft deepsec --scan-only
```

The adapter freezes file selection, checks source bytes before and after, and
requires fresh analysis evidence. Staged mode refuses differing index and
working-tree bytes. Empty exports, refusals, partial reviews and unsupported
versions cannot pass. Full mode covers the scanner's pending candidates, not
the whole program; historical findings may remain in threshold assessment.
See the [integration guide](skills/visioncraft/references/deepsec.md) for scope,
routes and spend controls, and [maintenance procedure](docs/UPSTREAM-DEEPSEC.md)
for qualifying upgrades.

**Agentic QE** supplies optional test and coverage tools through its own CLI/MCP
surface. Follow its [worked integration](skills/visioncraft/references/agentic-qe.md)
and [result contract](contracts/agentic-qe.json). Inspect and execute generated
tests in the project; keep measured coverage distinct from estimates and retain
`insufficient-evidence` results.

Reading the skill does not install, authenticate or activate either service.

## Architecture records and diagrams

Keep decisions and diagrams in the project's existing corpus, linked to
requirements, inspected commits, governed source paths and release observations.
Rendering a diagram does not verify its claims.

See [decision guidance](skills/visioncraft/references/methodologies.md),
[diagram diagnosis](skills/visioncraft/references/diagram-diagnosis.md),
[traceability](skills/visioncraft/references/traceability.md) and this project's
[verified ADRs and diagrams](docs/diagrams/README.md).

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

## Place in the VisionFlow estate

VisionCraft succeeds the Build with Quality skill developed across DreamLab-AI's
estate. The [migration guide](docs/MIGRATION.md) accounts for every former resource.
It remains independently usable.

| Project | Connection |
|---|---|
| [VisionFlow](https://github.com/DreamLab-AI/VisionFlow) | Cross-project architecture, audit records and documentation |
| [Sealmap](https://github.com/DreamLab-AI/sealmap) | Shared diagram, review and model-family contracts consumed by VisionCraft |
| [Agentbox](https://github.com/DreamLab-AI/agentbox) | Installs a pinned release and canonical skill, supplies host policy and credentials, and routes agents through Jev |

Agentbox's Deepsec adapter projects policy into the same VisionCraft executable;
it retains no parallel gate implementation. Ruflo handles coordination and
Agentic QE provides direct testing tools. A source pin does not establish live
activation. The [Agentbox integration reference](skills/visioncraft/references/ecosystem-agentbox.md)
tracks memory, routing and deployment responsibilities.

## Limits and development

The process runner is **not a sandbox**. Children retain the caller's filesystem
and network authority; environment filtering is not secret isolation. Linux
process groups support cancellation and deadlines, but a deliberately escaping
process or a blocked kernel filesystem call needs host-level containment.
Private capture directories protect ordinary local handling, not a hostile
concurrent filesystem. Keep source-bearing evidence out of public commits.
Linux can recognise exited zombie group members through an unrestricted `/proc`
view; hidden, unreadable or uncertain state remains non-passing. Native execution
on other platforms returns unsupported before spawning a child. Process-wide
child reapers must not compete with the runner's ownership of its children.

Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
and `RUSTDOCFLAGS='-D warnings' cargo doc --no-deps`. The
[implementation contract](docs/EXPECTATIONS.md) records acceptance boundaries.
The [release review](docs/REVIEW-2026-11-03.md) records reproduced counterexamples
and their dispositions.
See [evaluation evidence](docs/EVALUATION.md) for the bounded comparison; no
measured success-rate improvement is claimed.

MIT OR Apache-2.0. Attributed BeautifulRust material retains its original MIT
notice. See [provenance](docs/PROVENANCE.md), [MIT](LICENSE-MIT) and
[Apache-2.0](LICENSE-APACHE).
