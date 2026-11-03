# README example verification

[README.md](../README.md#a-complete-local-example)'s local assertion example
was executed on 2026-11-03 in a fresh owned Git repository, using a copied local
debug binary reporting `visioncraft 0.1.0`. This verifies the local command path;
it is not evidence of package publication, consumer adoption or deployment.

Inspected README SHA-256:
`c7c6114d798b70766da974ed31d79d67585ba681108a60d58598792e085c40d8`.
Executed binary SHA-256:
`ba494a4047713f94b6db4fff5d6fff8ad874c748463c30ce7e9ef11e4a9d8196`.

After creating and committing `limit.txt` containing `12`, these commands ran:

```sh
visioncraft plan init --out quality.json --input limit.txt \
  --configuration 'POSIX assertion of configured limit' \
  -- sh -c 'test "$(cat limit.txt)" = 12'
visioncraft plan check quality.json --json
visioncraft run quality.json --all --out evidence-1
visioncraft evaluate quality.json --receipts evidence-1 --json
visioncraft expectations check quality.json --receipts evidence-1 --json
visioncraft gate evaluate quality.json --receipts evidence-1 --json
```

All six commands returned exit 0. Evaluation reported `passed`, with one required
check and one passed check. The fixture defines no separate expectations; those
empty expectations are not additional evidence of behaviour.

Changing the selected input without rerunning its check exercised stale evidence:

```sh
printf '13\n' > limit.txt
visioncraft evaluate quality.json --receipts evidence-1 --json
```

Evaluation returned exit 1 with aggregate `incomplete`, the check `inconclusive`,
and reason `selected source inputs changed since review`. The old successful
receipt did not satisfy the modified input. This is a source-identity control,
not a claim that the changed assertion was executed and failed.

Retained evidence includes exact argv, stdout/stderr, exit statuses, hashes,
the isolated fixture and the copied binary. No Cargo build, model call, Deepsec
review, AQE task or library documentation example ran in this verification.

A separate local-link check inspected 50 Markdown files and 105 local file
links in the current repository, with zero missing targets. External URL
availability and heading-anchor correctness were outside that check.
