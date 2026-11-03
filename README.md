# VisionCraft

**Engineering guidance and executable evidence gates for coding agents and people.**

VisionCraft helps you define a change, run proportionate checks, assess what the
results support, and carry that evidence through review and release. Use its
[skill](skills/visioncraft/SKILL.md), Rust library, CLI, or a combination.

Developed within DreamLab-AI's [VisionFlow estate](https://github.com/DreamLab-AI/VisionFlow),
VisionCraft packages the estate's engineering and review practices for reuse in
any project. It is published on [crates.io](https://crates.io/crates/visioncraft),
with [Rust API documentation](https://docs.rs/visioncraft).

## Why VisionCraft

Every finished change should come with a clear account of what it was meant to
do, what was checked, what the results show, and where the change has actually
been delivered. VisionCraft makes that a repeatable engineering discipline. It
combines a skill that coding agents can follow with a Rust library and command-line
tools for recording and assessing evidence. It grew from the Build with Quality
skill used across DreamLab-AI's projects.

When coding agents work on your code, you still need to understand what they have
done. A maintainer needs enough context to review the change, another developer
needs to know which behaviour was tested, and the person deploying it needs to
know which version users will receive. VisionCraft gives those conversations a
shared basis: the intended behaviour, the relevant inputs, the checks that ran
and their actual results. That record can accompany the work through review
and release.

Start by agreeing the behaviour and reproducing the problem.
Consider an application where an empty search query should return an empty
result, but currently produces an error. A useful repair establishes that
expectation, reproduces the fault, changes the code and runs a test that reaches
the original failure. A normal-query control checks that ordinary searches
still work. The reviewer can then inspect both the repair and the evidence
supporting it. This is a small, focused piece of work.

A shared event-format change needs more depth. Each consumer has expectations
about the data it receives. Older events may still be stored, and services may
upgrade at different times. The same method expands to cover consumer tests,
compatibility, migration and the decisions behind the new format. Merging the
code, publishing a version and observing its deployment remain separate steps.
Teams can scale the work to the change and its consequences.

VisionCraft's tools support that discipline with explicit checks and captured command
output associated with the source inputs being tested. The gate refuses to call
required evidence a pass when it is missing, failed, stale or incomplete. A
reviewer can see what supports the result and what still needs attention. These
records help people make a judgement; they do not prove that the software is
correct or secure, or that every important behaviour has been tested.

Adoption can start with the skill in a coding agent or with the library and CLI
in an existing repository and CI process. Keep your issue tracker, tests and
architecture decisions. There is no required coding-agent framework, Agentbox
installation or model subscription for the deterministic core. When configured,
Agentic QE testing tools and Deepsec security review can contribute specialist
evidence to the same engineering process.

VisionCraft belongs to the VisionFlow estate, where architecture and audit
records connect the projects. Sealmap supplies shared diagram and review
contracts, while Agentbox consumes the published VisionCraft release and makes
its method available to agents. The reusable skill, library and CLI are also
available to independent maintainers and teams outside that estate.

## Execution boundary

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

Rust 1.88 or later is required. Native execution currently supports Linux with glibc or musl;
the deterministic library remains independent of the operating system.

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

## Place in the VisionFlow estate

VisionCraft grew from the Build with Quality skill used across the VisionFlow
estate. The reusable method, Rust library and CLI now live in this repository;
hosts consume a pinned release and supply their own execution environment.

| Project | Role and connection to VisionCraft |
|---|---|
| [VisionFlow](https://github.com/DreamLab-AI/VisionFlow) | Cross-repository architecture diagrams, audit records and estate documentation; VisionCraft guides how changes are checked and evidenced. |
| [Sealmap](https://github.com/DreamLab-AI/sealmap) | Shared source-derived diagrams and review contracts consumed by VisionCraft. |
| [Agentbox](https://github.com/DreamLab-AI/agentbox) | A host integration that installs VisionCraft, supplies policy and routes agents to its canonical skill. |
| VisionCraft | Portable engineering guidance, executable quality plans and evidence gates for these projects and independent consumers. |

Each project keeps its own requirements, ADRs, tests and release records.
VisionCraft connects the work to that evidence; host adapters provide credentials,
model access and deployment controls where needed.

### Agentbox integration

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
Structural skill checks and forward evaluations are reported separately from
any measured improvement; this rewrite does not imply a success-rate uplift.

MIT OR Apache-2.0. Attributed BeautifulRust material retains its original MIT
notice. See [provenance](docs/PROVENANCE.md), [MIT](LICENSE-MIT) and
[Apache-2.0](LICENSE-APACHE).
