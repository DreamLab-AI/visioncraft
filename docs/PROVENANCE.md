# Skill provenance and redistribution

VisionCraft's portable method was written from the behavioural inventory of
Agentbox's build-with-quality skill. It preserves the engineering responsibilities
without copying the old executable wrapper, descriptive configuration or removed
TypeScript API into a differently licensed runtime.

This migration examines Agentbox
`a9125eaf006e2679f5b81444c3c06d0fb45b7481`, under
`skills/build-with-quality/`. It contains the same 28 paths as the frozen
inventory at `4e1f4fff4e0f7827e8588c33bbf031c8420f52ad`; the later source updates
the Deepsec operating reference. Historical source remains recoverable in Git.

## File-level records

[The migration map](../skills/visioncraft/assets/legacy-paths.json) records all
28 original paths, their Git blob IDs, SHA-256 bytes, preserved responsibilities,
new resource or explicit retirement, and caller action. Its path set is checked
against the pinned Git tree.

[The provenance manifest](../skills/visioncraft/assets/provenance.json) records
every file in the portable payload, its licence, treatment and relevant source
mapping. Newly authored guidance and migration data use the project's
`MIT OR Apache-2.0` terms. `docs/MIGRATION.md` and this file are newly written
under the same project terms. A file record describes the import history; it
does not certify legal status of unrelated upstream repositories.

Agentbox's root licence and the old skill's MIT declarations are not treated as
a blanket permission to relicense its entire executable source. The portable
non-Rust prose is newly written around observable responsibilities and failure
contracts. Agentbox-specific integration implementation remains its own source
and licence. The Rust implementation is separately authored by the package
maintainers; it is not the old shell/Python wrapper copied into this payload.

## BeautifulRust material

The Rust subtree credits Morgan Linton's BeautifulRust at
`a9cdcfa94d1330c0d41c7f67764c5f7786fbb71d`. The complete inherited MIT notice,
copyright and upstream mapping remain in
[NOTICE.md](../skills/visioncraft/references/rust/NOTICE.md), followed by a clear
VisionCraft migration note.

- Five source digests and both lint assets are byte-identical to the pinned
  Agentbox import. Their source links and attributions remain intact.
- The three Rust method bodies retain the MIT language guidance. All thirteen
  steering sections remain reachable. New portable scope text replaces the
  estate-specific steering/port overlays, review intake, dead-path guidance and
  affected links. The old rule that an unavailable reproduction should be
  rejected is corrected: unavailable evidence remains inconclusive.
- The provenance manifest distinguishes retained bytes from modified bodies
  and records the pinned BeautifulRust source paths. This is an import mapping,
  not a claim that the underlying books or upstream guides were independently
  re-audited during this migration.
- Lints are opt-in assets. They do not change a consuming repository until
  deliberately installed and tested with its actual toolchain/MSRV.

## Method lineage without an invented licence grant

Legacy documentation associated feedback-loop debugging with Matt Pocock's
diagnose method, traceability with BHIL, and some knowledge/coordination ideas
with KHIVE/RuvNet. Those acknowledgements did not supply pinned grants for
copying their prose. VisionCraft's corresponding explanations, tables and
worksheets are newly written descriptions of engineering practice; no source
text from those external projects is vendored here. The source inventory retains
the historical attributions for readers investigating that lineage.

Retained Rust digests carry their own underlying-source acknowledgements.
Do not remove those notices or treat future upstream additions as covered by
this import record without checking them.

## Evidence limits

Structural checks, licence manifests and path coverage do not establish a
measured task-success improvement. Public examples are illustrative contracts
unless accompanied by executed receipts. Independent forward testing and a
held-out comparison are separate evaluations, reported with their actual
subjects and observations. No unsupported speed, token-saving, prediction or
consensus claims are carried forward from the old descriptive configuration.
