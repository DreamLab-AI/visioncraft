# Maintaining the optional Deepsec adapter

VisionCraft adapts the external [Deepsec CLI](https://github.com/vercel-labs/deepsec).
It does not fork Deepsec's reviewer agents, implement their reasoning, or require
Deepsec for its core planning and evidence functions. A host installs the tool,
chooses credentials and model routes, authorises source access and spend, and
decides when to activate an update. VisionCraft checks a supported protocol,
bounds an explicit invocation, retains evidence and applies a declared threshold.

## Current protocol pin

[contracts/deepsec.json](../contracts/deepsec.json) is the machine-readable release
identity and adapter protocol declaration. The inspected npm release is
`deepsec@2.3.10`, with npm `gitHead`
`ce646748b6b2d7b088107ac29944c4129c49f11d`. Its
[immutable package manifest](https://github.com/vercel-labs/deepsec/blob/ce646748b6b2d7b088107ac29944c4129c49f11d/packages/deepsec/package.json)
declares that version and Apache-2.0 licensing. The registry tarball's SHA-512
integrity was independently recomputed and matched the registry metadata.

These read-only commands retrieve the release facts without installing it or
running lifecycle scripts:

```sh
npm view deepsec@2.3.10 version dist gitHead repository --json
npm pack deepsec@2.3.10 --ignore-scripts --json --pack-destination ./evidence
```

Create the evidence directory first. Inspect the downloaded package's `SKILL.md`
and `dist/docs/{reviewing-changes,configuration,models,data-layout}.md`; they
describe the installed release, whereas upstream `main` can describe newer code.
At the 2026-11-03 check, upstream `main` was
`4fa67220aff67ac93dfc840206d3d32bfa4411ec`, a different commit from npm 2.3.10.
A moving tag or a current README does not update this protocol pin.

Receipts must retain the observed CLI version, expected npm git commit, expected
npm integrity and VisionCraft protocol-fixture version. The expected commit and
integrity identify the approved release; they do not prove which bytes are in
an installed binary. Host installation/lock evidence establishes that additional
fact. Unsupported versions are unavailable (exit 78), never an implicit pass.

## Completion and scope contract

VisionCraft freezes a nonempty selected file set and its hashes. Direct processing
uses that explicit set with Deepsec's `--files-from` and `--no-ignore`, avoiding
a second independently computed diff. A staged selection whose working bytes
differ is refused; use an exact index checkout. Full mode scans first, then
processes the selected pending candidates. No pending candidates means no new
review, not a clean full review. Scan-only output cannot satisfy model review.

Completion requires one fresh persisted processing run for the expected project
and root, phase `done`, a completion timestamp, and a processed-file count equal
to the selected review set. Every selected file must have an `analyzed` record
with the expected path, project and content hash, plus exactly one history entry
for that run. That entry must name the model and agent, report positive output
tokens, state `refusal.refused: false`, and contain no skipped scope. Missing,
zero-work, refused, partial or contradictory records prevent a pass.

Retain the run and file records, raw export, normalised findings and all bounded
process output. Exit 1 alone does not distinguish findings from runtime failure;
the pinned completion path and valid export remain necessary. A comment alone
does not prove that every intended file was reviewed. Aggregate exports can
contain historical findings: retain them and distinguish the current attempt.
These protocol checks verify recorded investigations of selected bytes. They
cannot prove semantic correctness, dependency coverage or runtime safety.

## Update checklist

1. Fetch exact npm metadata and integrity, identify its immutable upstream
   source, and inspect its bundled help/docs. Do not substitute upstream `main`.
   Verify licences and changes to transitive reviewer/tool dependencies.
2. Diff source-selection flags, project/root resolution, model/auth routes,
   configuration precedence, cache/resume rules, run/file records, export
   schema, severity vocabulary and exit behaviour. Record deliberate changes
   to the adapter protocol version.
3. Run compatibility fixtures for a clean completed review and a completed
   finding, alongside missing inputs, empty selection, stale state, malformed
   export, unknown severity, source mutation, wrong project/hash, zero output,
   refusal/partial scope, provider failure, timeout and cancellation. Include
   output bounds, managed-path checks and cleanup of child processes.
4. Perform one bounded real smoke review under explicit host authority using
   non-sensitive fixture source and approved credentials. Record the actual
   model, current run/file records, export, cost when available and limitations.
   Local fixture success does not establish a real provider's completion path.
5. Update the contract, fixture version, adapter and docs together. Align the
   exact host package pin, lock/integrity and consumer tests; do not loosen the
   version check just to accept a newer executable.
6. Release the tested change, update consumer pins, rebuild through the host's
   normal delivery process, then observe the installed version and a bounded
   activation check. Preserve source, publication, adoption and deployment
   evidence separately.

An optional release monitor should only open a reviewable update PR containing
the proposed pin, source comparison and compatibility work. This is a proposed
maintenance workflow, not an installed monitor. It must not auto-activate the
release, change default models, initialise accounts or start paid reviews.

## Host responsibilities

The invoking user and host-projected policy are trusted configuration. Treat
`--policy-json`, executable lookup through `PATH`, credential environment names
and provider URLs with the same authority as starting the reviewer directly.
Never derive them from an untrusted repository's contents. HTTP remains available
for explicitly configured local providers; the host owns transport and endpoint
approval. A name prefix would not establish that trust or authorise a recipient.

Metadata probes share the native CLI's restricted Git environment and disable
repository filesystem-monitor hooks. Review checkpoints reject symlinks, hard
links and special files throughout the bounded state tree before invoking
Deepsec, and rewrite their ignore file privately. Existing ordinary checkpoint
records are not proof of a fresh review: current run identity and exact source
hashes remain required. These checks do not isolate concurrent writers with the
same Unix identity or the reviewer's own tools.

VisionCraft never implicitly installs Deepsec, runs `init` or login, changes
global defaults, or chooses a new credential provider. It does not create a
sandbox: the invoked reviewer and its children have the authority granted by
the host. Source-bearing output stays in private evidence locations according
to that host's policy. See the portable
[operating method](../skills/visioncraft/references/deepsec.md) and
[Agentbox example](../skills/visioncraft/references/ecosystem-agentbox.md).
