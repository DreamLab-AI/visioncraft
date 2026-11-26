| Date | Deep | Finding | Issue | PR | Evaluated? | Verdict | Effect | Witness | Prior-night fates | Reviewer | Review-minutes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-11-06 | plan-contract | VETOED: Invocation::validate now refuses NUL in program; only args were… | NONE | VETOED | yes | REJECT |  | 41284be1e6dd |  |  |  |
| 2026-11-07 | evidence-and-gates | VETOED: evaluate() now flags dirty/working-tree drift vs current subject as… | NONE | VETOED | yes | REJECT |  | e0bdec53870f |  |  |  |
| 2026-11-08 | process-execution | VETOED: plan aggregate deadline/capture ceilings witnessed: doubled refused… | NONE | VETOED | yes | REJECT |  | d3bab5e6ba70 |  |  |  |
| 2026-11-09 | cli-and-snapshots | Environment fault, hypothesis untested: connected node annexe unhealthy: SSH com | NONE | NONE | blocked | BLOCKED-ENV |  | BLOCKED |  |  |  |
| 2026-11-10 | deepsec-adapter | VETOED: deepsec receipts must equal contracts/deepsec.json identity; LOW not… | NONE | VETOED | yes | REJECT |  | 9c1b389df567 |  |  |  |
| 2026-11-11 | skill-eval-harness | VETOED: 5 added harness controls pass: surface, fenced JSON, row shape… | NONE | VETOED | yes | BLOCKED-ENV |  | c39bdded00ff |  |  |  |
| 2026-11-12 | plan-contract | MAX_PLAN_INPUTS=16384 and MAX_PLAN_EXPECTATIONS=4096 now bound plan validate() | NONE | https://github.com/DreamLab-AI/visioncraft/pull/4 | yes | ACCEPT |  | 3061c50121ab |  |  |  |
| 2026-11-13 | evidence-and-gates | validate() rejects failed-to-start/unavailable receipts claiming exit/signal | NONE | https://github.com/DreamLab-AI/visioncraft/pull/5 | yes | ACCEPT |  | 77fdaad607b5 |  |  |  |
| 2026-11-14 | process-execution | stderr-only overflow is CaptureFailed, retains <=1KiB, error names the limit | NONE | https://github.com/DreamLab-AI/visioncraft/pull/6 | yes | ACCEPT |  | 10d03540be5a |  |  |  |
| 2026-11-15 | cli-and-snapshots | validate() refuses capture-failed receipts that claim capture_complete | NONE | https://github.com/DreamLab-AI/visioncraft/pull/7 | yes | ACCEPT |  | 2950cf6b70ca |  |  |  |
| 2026-11-16 | deepsec-adapter | policy auth guards refuse local/direct/custom misconfig with 78 before review | NONE | https://github.com/DreamLab-AI/visioncraft/pull/8 | yes | ACCEPT |  | 91c7e84834e9 |  |  |  |
| 2026-11-17 | skill-eval-harness | fenced-JSON parsing, loader rejections, completed-only aggregate now controlled | NONE | https://github.com/DreamLab-AI/visioncraft/pull/9 | yes | ACCEPT |  | 065827b35e29 |  |  |  |
| 2026-11-18 | plan-contract | VETOED: validate() now refuses invocation.program containing NUL, matching its… | NONE | VETOED | yes | REJECT |  | 1063cb215e24 |  |  |  |
| 2026-11-19 | evidence-and-gates | ACCEPT upheld: 4 required evaluator(s) passed on candidate tree aaecb1eacb2a | NONE | https://github.com/DreamLab-AI/visioncraft/pull/10 | yes | ACCEPT |  | 71741f77c8f4 |  |  |  |
| 2026-11-20 | process-execution | stderr.log symlink in a fresh output dir is refused; victim bytes unchanged | NONE | https://github.com/DreamLab-AI/visioncraft/pull/11 | yes | ACCEPT |  | fbac07c884e0 |  |  |  |
| 2026-11-21 | cli-and-snapshots | git metadata parsers refuse unresolved, gitlink and hash-count-drift records | NONE | https://github.com/DreamLab-AI/visioncraft/pull/12 | yes | ACCEPT |  | e4ce6b3f172a |  |  |  |
| 2026-11-22 | deepsec-adapter | severity cut is ordered-inclusive, not equality; unknown fail_on exits 78 | NONE | https://github.com/DreamLab-AI/visioncraft/pull/13 | yes | ACCEPT |  | 56095dcda877 |  |  |  |
| 2026-11-23 | skill-eval-harness | loader rejects rubric missing a dimension; was a post-reservation KeyError | NONE | https://github.com/DreamLab-AI/visioncraft/pull/14 | yes | ACCEPT |  | edd71aa02c7d |  |  |  |
| 2026-11-24 | plan-contract | VETOED: invocation argv and env-name counts gain ceilings: 16384 args, 1024 env… | NONE | VETOED | yes | REJECT |  | 3c6740f6812e |  |  |  |
| 2026-11-25 | evidence-and-gates | observed scope with paths beyond pinned subject inputs now grades Inconclusive | NONE | https://github.com/DreamLab-AI/visioncraft/pull/15 | yes | ACCEPT |  | b0d6b86ddfc8 |  |  |  |
