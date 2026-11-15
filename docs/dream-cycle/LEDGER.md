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
