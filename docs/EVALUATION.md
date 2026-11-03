# VisionCraft evaluation evidence

VisionCraft's packaging and source-retention checks are separate from evidence
that its guidance helps an agent make a useful decision. This report describes
one bounded, frozen comparison. It does not establish general coding quality,
a security guarantee, a production-readiness percentage or measured tuning
uplift.

## Structural checks

Candidate packaging retains a complete map of the 28 legacy skill paths. Its portable
payload contains 34 files, with file-level provenance, 73 resolved local Markdown
links, all thirteen Rust steering sections, seven byte-identical MIT Rust assets
and the complete inherited notice. Skill frontmatter validation passed. The
three AQE consumed-result schemas compiled under a strict JSON Schema validator;
input field names were checked against the pinned published MCP implementation.
Inherited lint assets loaded in an isolated dependency-free Clippy smoke
check on Rust 1.99.1. That check does not establish support for an older MSRV.

## Frozen decision comparison

An independently invoked Claude Sonnet 5.5 curator produced twelve synthetic
engineering handoff scenarios, split before execution into four training, four
validation and four held-out test cases. The scenarios ask for four bounded
choices: whether the stated evidence supports completion, the next action,
deployment observation status and review independence. The author/evaluator did
not inspect the held-out prompts or reference answers, and no candidate wording
was changed from their results.

Target execution used the exact observed model `claude-haiku-4-5-20251001`, through
Claude CLI 2.1.309. Each invocation received four scenarios, shared rubric enum
definitions and one frozen guidance context. Reference answers and curator
rationales stayed in the private scorer. Tools and MCP were empty, settings
sources and session persistence were disabled, and each invocation used a fresh
empty working directory. Managed global instructions may still apply to both
arms; this is not an operating-system isolation experiment.

| Context | Source revision | Bytes | Words |
|---|---|---:|---:|
| Canonical Agentbox baseline | `a9125eaf006e2679f5b81444c3c06d0fb45b7481` | 68,048 | 9,283 |
| Frozen VisionCraft candidate | `899e76600119a5159869551b374600d4cfb4a83e` | 37,149 | 5,119 |

Each context contains six predeclared equivalent resources: entrypoint, EDD,
debugging, methodologies, review evidence and quality gates. No task-specific
trimming or retrieval was performed. Per-file hashes and exact context bytes
were retained. These different context lengths are a property of this comparison,
not a demonstrated token-efficiency improvement. Optional resources, runtime
adapters, actual coding, autonomous resource selection and deployed host tools
were not exercised by these classification tasks.

Dataset SHA-256:
`2da88232856635de992c68417061acca664359508ba6a27df53a965809181340`.
Baseline context SHA-256:
`b54f6cc1a4b502a5165d0a2606062ecca4fccdf14dc3fc3ea10b548df17792e3`.
Candidate context SHA-256:
`043c48d4de9ebbc2beec7eb66ec8ad7b1baebba8806da027fc86aa82a77263ad`.

## Scoring and result

Each of four dimensions scores one when its label belongs to the curator's
accepted array. Hard accuracy requires all four labels; soft accuracy is the
fraction of matching dimensions. Invalid output, model unavailability and
execution errors remain distinct from valid wrong answers. A completed tool
call alone earns no task credit.

Baseline training pilot scored 4/4 hard and 16/16 dimension decisions. This
is a saturated pilot, so no training loop or prose optimisation was attempted.
Remaining evaluation is a forward comparison of the already frozen rewrite.

| Evaluation | Unique cases | Repetitions | Hard result | Soft result |
|---|---:|---:|---:|---:|
| Baseline training pilot | 4 | 1 | 4/4 | 16/16 |
| Candidate validation | 4 | 1 | 4/4 | 16/16 |
| Baseline held-out test | 4 | 3 | 12/12 case evaluations | 48/48 dimension decisions |
| Candidate held-out test | 4 | 3 | 12/12 case evaluations | 48/48 dimension decisions |

All eight target calls completed using the requested model, without fallback,
timeout or invalid output. Each held-out repetition scored 100% hard and soft
for both contexts. No regression was observed on these four held-out scenarios.
This task is fully saturated and provides no evidence of an improvement over
the baseline. No training edits or further target calls were made.

An [aggregate record](evaluation/2026-11-03-aggregate.json),
[context manifest](evaluation/2026-11-03-contexts.json) and
[execution-utility hashes](evaluation/2026-11-03-harness.json) are public. Raw
case material remains private. A later inline host-lint exception in the optional
AQE reference is outside these frozen contexts and is not an evaluated method
change.

Three repetitions revisit the same four held-out cases. They measure variation
on those cases, not twelve independent examples. A small synthetic enum task can
miss important ambiguities and operational failures. A tie or high score here
cannot show that the skill improves real debugging, reviews or release work.
Future improvement claims need a harder independently curated training/validation
programme, relevant executed engineering tasks, and a new untouched test set.

## Reproduction and retained evidence

[The external evaluation utility](../tools/skill-eval/README.md) documents the
exact execution/scoring interface. It follows SkillOpt's batch-result and
per-item conversation convention but does not invoke an optimiser or add a
production dependency. Five synthetic fixture tests check answer isolation,
exact parsing, multi-answer acceptance, soft scoring and dataset validation.

Private evidence retains curator output, the frozen dataset, contexts and hashes,
all raw target outputs, per-case conversations, usage/model metadata and scores.
Those scenario prompts, answers and conversations are deliberately not published
with this report. `aggregate.json` contains only aggregate results; the source
revisions and dataset hash allow an authorised custodian to reproduce or inspect
the comparison. Reusing a held-out set after editing from its answers would be a
new training exercise, not another independent test.
