# Frozen skill comparison utility

This external Python utility checks a bounded set of engineering evidence
choices. It is not part of VisionCraft's runtime and does not install tools,
edit a skill, call the SkillOpt optimiser, or claim general coding quality.
It follows SkillOpt's batch-result and `predictions/<id>/conversation.json`
trajectory convention, so a separate training integration can consume those
artefacts. The held-out comparison itself uses the explicit runner below.

Use Python 3.10+ and an already configured Claude CLI. The target is fixed to
`claude-haiku-4-5-20251001`; no fallback model is selected. Requests have no
available tools or MCP servers, disabled settings sources, a fresh empty working
directory, and no CLI session persistence. Managed global instructions may still
apply to both arms. This is tool-level isolation, not an operating-system sandbox.

## Private dataset and public inputs

Provide JSON with a shared `rubric` object and `items`. Each item contains a safe
`id`, `split` (`train`, `val` or `test`), `prompt`, and four `accepted` arrays:
`gate`, `next_action`, `deployment`, `independence`. This fixed comparison expects
four cases per split. Optional `answer_rationale` belongs only to the private
curator record. The loader passes only each item's ID and prompt, the four
shared rubric definitions, and enum vocabulary to the target. Reference arrays
and rationales never enter its prompt.

Freeze the dataset and both skill revisions before the first call. The runner
extracts six predeclared equivalent method resources from each Git revision:
entrypoint, EDD, debugging, methodologies, review evidence and quality gates.
It retains exact file hashes, content sizes and combined context hashes. This
is a fixed-context method comparison, not a test of autonomous skill discovery.

## Run phases in order

Run the synthetic controls first:

```sh
python3 -m unittest discover -s tools/skill-eval -p test_harness.py
```

Substitute actual paths and immutable revisions below. Run `pilot`, then
`validation`, then `test` as separate invocations; inspect only aggregate scores
until the held-out comparison has finished. A zero or perfect pilot signals
saturation, so this run cannot justify a tuning-uplift claim.

```sh
python3 tools/skill-eval/run_eval.py \
  --dataset /private/evaluation/dataset.json \
  --baseline-repo /sources/agentbox --baseline-ref BASELINE_COMMIT \
  --candidate-repo /sources/visioncraft --candidate-ref CANDIDATE_COMMIT \
  --out /private/evaluation/frozen-comparison --phase pilot
```

Pilot execution uses baseline/train once; validation uses candidate/val once. The test
phase runs baseline/test and candidate/test three times each, with four cases
per invocation. Maximum concurrency is two and each invocation has a 180-second
deadline. A serial phase reservation ledger bounds this programme to ten calls,
including unsuccessful attempts. Eight calls are scheduled; no retries or
implicit follow-up calls are made. Run phases from one process at a time.
An unavailable pilot or validation stops progression to test.

Each call retains its exact request, raw CLI output, stderr, observed model,
usage fields when supplied, elapsed time and per-case conversations. Those
files contain private scenarios and must not be committed or published. Local
score rows follow SkillOpt's `id`, `hard`, `soft`, `task_description`,
`fail_reason`, `n_turns` convention. A batched CLI call is one execution shared
by four cases, not four independently executed conversations.

## Scoring and limits

A dimension scores one when the predicted label belongs to its accepted array.
Hard score is one only when all four dimensions match; soft score is their
fraction. Malformed, duplicate, missing or unknown answers are invalid outputs.
Model errors, unavailable/unverified models and timeouts are separately labelled
and excluded from completed-performance aggregates. They are not evidence that
one skill solved no tasks.

`aggregate.json` contains no prompts or reference answers. Replication measures
stochastic variation on the same four held-out cases; it does not create twelve
independent scenarios. No skill may be edited from those answers and then be
reported as evaluated by the same held-out test. A new candidate needs a fresh,
independently curated test set. Document context-length differences, scope,
curator/model identities, unavailable calls and the exact source revisions.
