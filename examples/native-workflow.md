# Capture, run and evaluate a bounded check

Run from a committed Git worktree. Choose the source, locks and configuration
that determine the behaviour you intend to check; the CLI does not infer a
complete dependency closure from the command name.

```sh
visioncraft plan init --out quality-plan.json \
  --input Cargo.toml --input Cargo.lock \
  --configuration 'library, default features disabled' \
  --timeout-ms 600000 --max-output-bytes 8388608 \
  -- cargo test --locked --no-default-features

visioncraft plan check quality-plan.json --json
visioncraft run quality-plan.json --all --out .visioncraft/run-001
visioncraft evaluate quality-plan.json --receipts .visioncraft/run-001 --json
```

The program and arguments after `--` are literal argv. Invoke another installed
quality tool the same way, using its current documented command and selected
source inputs. VisionCraft does not add a shell, infer model runtime support or
interpret an upstream result as a stronger claim than its declared evaluator.

`plan init` writes one required exit-code check. Its defaults are a 60-second
deadline, 8 MiB combined output, `PATH` as the only inherited environment name
and a 30-day maximum evidence age. Choose bounds and environment names explicitly
for the real check. Program lookup uses that PATH when selected, otherwise
`/usr/bin:/bin`; resolved executable identity and its hash, when readable, are
recorded separately from the requested program. Commands retain the invoking
user's authority: this is not a sandbox.

An edited selected input produces a private adjacent
`.quality-plan.json.evidence/selected-inputs.json` snapshot containing the exact
selected bytes. That directory and run directories are gitignored before raw
capture. Keep the snapshot beside the plan until execution; the runner copies it
into the evidence root. Clean plans need no dirty snapshot. Reusing output
directories is refused. Source, evidence and managed output paths reject nested
symlinks after resolving the explicitly chosen source/evidence root.

`run` records selected invocations. Its exit code does not claim the whole plan
passed: `evaluate` applies every required check and expectation. All required
receipts must be supplied, with one selected attempt per check. Preserve older
attempts separately and name multiple `--receipts` directories when combining
different selected checks. `gate evaluate` and `expectations check` use the same
whole-plan evaluation; neither hides another required check's failure.

The native runner records actual stdout, stderr, exit/signal, before/after source
hashes and elapsed time. It leaves `observed_scope` unknown and `observed_tests`
empty. Selecting files or putting a test name in argv does not prove what the
tool inspected or executed. Plans requiring observed coverage or a stabiliser
therefore need a real tool-specific observation/adapter. `reported` evaluators
must use their named adapter; generic `run` refuses to invent that assessment.

An optional `--context context.json` supplies the core's `AuditReceipt` and
`ManualObservation` records plus explicit model-family aliases:

```json
{
  "audits": [],
  "manual_observations": [],
  "families": [{"route": "local:reviewer-alias", "family": "anthropic"}]
}
```

Artifact references in this context resolve relative to its parent directory.
Use the public Rust type documentation for the complete audit/manual fields.
Empty arrays do not satisfy an expectation that requires those observations.
Sealmap checks family declarations; a new provider label does not make a model
independent. `--current-subject` can supply a subject record, but the CLI checks
it against actual selected source bytes and the current Git revision before use.

Evaluation reads evidence and fixed Git metadata operations; it never runs
commands found in plans or receipts. It compares unfiltered committed blobs and
selected current bytes rather than invoking repository clean filters. A later
unrelated commit can retain fresh evidence when ancestry and selected bytes
match; changed or missing selected inputs cannot borrow the old pass. The
optional `--as-of-ms` clock is for explicit reproducible evaluations, not a claim
that old evidence was captured again.

Each Git metadata operation uses the same bounded process runner with a
10-second deadline and private temporary captures. Committed blob size is
checked before requesting its contents. Ambient `GIT_DIR`/`GIT_WORK_TREE` are
excluded, lazy fetching is disabled, and cancellation stops inspection too.

For review findings and independent delivery stages, retain the immutable raw
export and use `sealmap review init/check/summary`. VisionCraft does not create a
second disposition store or infer deployment from a test or publication result.
