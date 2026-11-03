# VisionCraft implementation contract

The operator authorised extraction and integration on 3 November 2026 after
reviewing the complete build-with-quality surface. This implements the design
and inventory retained in Agentbox docs/developer/visioncraft-handoff/.

1. Preserve the complete engineering method as portable skill resources: EDD,
   debugging and static-oracle mode, diagram diagnosis, DDD/ADR/TDD/BHIL, review
   evidence, Rust guidance and worked examples. Every old resource is mapped.
2. Provide a useful deterministic Rust library and CLI for explicit quality
   plans, bounded executions, receipts, gate decisions and expectation coverage.
   No invented agent runtime, model training, consensus or measured improvements.
3. Required missing/failed/skipped/unknown evidence never becomes PASS. Preserve
   actual errors and successful controls; schema, scope and bytes are explicit.
4. Commands run only through explicit run/security operations, as argv with a
   deadline/output bounds, complete child cleanup and private evidence. Imported
   receipts, inspection and reports never execute their contents.
5. Consume Sealmap 0.4 for disposition and family contracts; delegate existing
   receipt byte checks to Sealmap. Preserve separate source/test/merge/publish/
   adoption/deployment facts. A runner is not a sandbox.
6. Replace the actual Deepsec compatibility consumer and migrate Agentbox
   registries, routes, resources and CI. Keep the legacy name/script paths as
   thin adapters to one canonical payload. Generic use needs no Agentbox.
7. Publish complete docs/licences/rustdoc/examples and execute offline regression
   controls, package checks, independent review and registry-consumer checks.
   Skill-builder compliance is distinct from measured skill-tuning results.
8. Source publication and a pinned image input are distinct from live activation.

Use source-controlled tests and exact evidence for the 22 cases in the design
report. Keep current/private model inputs out of public files. A first release
is complete only when the claimed surfaces work, consumers use them and every
remaining environment boundary is stated.
