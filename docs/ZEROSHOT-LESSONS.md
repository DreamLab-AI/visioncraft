# Lessons from Zeroshot

Reviewed 3 November 2026 at
[`df16ca0cb5ceb5e2503a770f53b45578f4d8b731`](https://github.com/the-open-engine/zeroshot/tree/df16ca0cb5ceb5e2503a770f53b45578f4d8b731).
This is a source and contract review, not an executed qualification of Zeroshot.
No dependency or engine source was imported.

## Admission before execution

Zeroshot's production verifier checks structural validity and resource bounds
before looking up worker implementations. It derives aggregate bounds across
sequences, branches, maps and loops rather than trusting each worker's local
limit. The verifier distinguishes invalid inputs from internal faults.
[Source](https://github.com/the-open-engine/zeroshot/blob/df16ca0cb5ceb5e2503a770f53b45578f4d8b731/crates/openengine-cluster-server/src/graph_verifier.rs).

Applied to VisionCraft's smaller contract: a plan admits at most 1,024 checks;
its declared deadlines sum to at most one day and capture limits to at most one
GiB, including optional checks. Checked arithmetic rejects overflow. The
regression combines individually valid checks whose aggregate exceeds the
ceiling, beside an exact-boundary passing control. This bounds one declared
pass, not arbitrary repeated invocations or blocked kernel I/O. VisionCraft has
no retry graph or implicit model worker fleet.

The canonical [quality-gate method](../skills/visioncraft/references/quality-gates.md)
also requires aggregate admission before workers start and leaves retry/spend
limits with the host.

## Review sessions and authority

Zeroshot's software-change workflow retains the worker session for repairs and
starts reviewers afresh. Authored graph structure controls which reviews must
succeed and where rejection returns the work.
[Workflow](https://github.com/the-open-engine/zeroshot/blob/df16ca0cb5ceb5e2503a770f53b45578f4d8b731/README.md#save-and-reuse-a-profile).

Useful host practice: scope each reviewer to the current candidate and evidence,
use a fresh reviewer context for a new repair pass, and keep the worker's own
account as a claim. Session separation does not establish model-family
independence. VisionCraft already checks the actual actor/family evidence when
that independence is required. A model response cannot reuse a receipt after
changing its bound plan or silently turn a missing required check into an
optional one.

## Delivery is an observation of an external effect

Zeroshot's inspected change separates a graph's output from delivery confirmed
by its runtime history, and retains a reference to that confirmation for replay.
The commit describes matching the delivery target and final writer and keeping
an unsuccessful graph unsuccessful despite a delivery receipt.
[Change](https://github.com/the-open-engine/zeroshot/commit/df16ca0cb5ceb5e2503a770f53b45578f4d8b731).

This supports the existing VisionCraft/Sealmap separation between source, tests,
merge, publication, consumer adoption and live deployment. Keep each receipt
bound to the actual revision and target. A natural-language completion message,
passing quality gate or published crate must not substitute for a live runtime
observation. Use Sealmap's existing delivery axes instead of another status store.

## Recovery and observation

Zeroshot's recovery contract gives a successor a new run identity, restores a
specific source/input checkpoint and resolves credentials again. Parallel writer
groups share a group checkpoint. Its observer can detach without terminating
the controller; stopping the run is a separate operation.
[Contract](https://github.com/the-open-engine/zeroshot/blob/df16ca0cb5ceb5e2503a770f53b45578f4d8b731/docs/guides/observe-and-control.md).

VisionCraft keeps every execution in a fresh directory and rejects stale or
mutated source/evidence. Host orchestration owns checkpoint recovery, including
writer quiescence and credential refresh. The foreground VisionCraft CLI owns
its child process group and cancels it on interruption; it is not a detached
controller. Host adapters must state which lifetime they control.

The [review-evidence method](../skills/visioncraft/references/review-evidence.md#resume-from-an-explicit-checkpoint)
now specifies predecessor attempt/checkpoint links, fresh captures, credential
resolution, writer quiescence and checking an unknown external effect before
retry. This is host workflow guidance; it does not claim the crate automatically
records recovery lineage or runs a recovery service.

## Optional future consumption

Zeroshot publishes separate protocol, server, client and testkit crates in its
workspace; its graph verifier lives in the server crate with runtime dependencies.
[Manifest](https://github.com/the-open-engine/zeroshot/blob/df16ca0cb5ceb5e2503a770f53b45578f4d8b731/crates/openengine-cluster-server/Cargo.toml).

A future orchestrator integration should consume a qualified upstream release
through an explicit host boundary. First compare it with the host's existing
Ruflo/session services, define one execution owner, qualify failure and recovery
behaviour, and pin the protocol. Its presence or an installed harness binary
alone does not establish an enabled adapter. VisionCraft's portable gate remains
usable without that runtime.
