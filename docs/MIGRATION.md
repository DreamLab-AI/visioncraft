# Migration from build-with-quality

This authorised cutover replaces the active build-with-quality system with one
canonical VisionCraft payload and executable. Once the new gates pass, migrate
all active callers and rebuild the consumer. Do not retain an old-name skill,
redirect, duplicate method tree or parallel wrapper implementation. Historical
documents keep their original claims and revisions with dated amendments where
needed.

## Preserved method and changed boundaries

VisionCraft retains EDD's seven-step loop and evidence categories,
feedback-loop debugging and static-oracle mode, diagram diagnosis, DDD/ADR/TDD,
BHIL-style traceability, all review-lesson families, five project examples and
the full Rust guidance/digests/lints. Responsibilities are logical roles unless
an actual host execution is recorded. Required unavailable evidence does not
become a pass through a weaker fallback.

No Agentbox, model provider, memory service, fleet or paid review is required by
the portable entrypoint. Agentbox's policies and actual capabilities are an
optional documented integration. The removed npm classes and the old loaderless
YAML are retired interfaces, not hidden VisionCraft functionality. No empirical
uplift is inferred from this migration.

## Complete 28-file map

Source: Agentbox `a9125eaf006e2679f5b81444c3c06d0fb45b7481`, relative to
`skills/build-with-quality/`. Resource targets below are relative to the new
`skills/visioncraft/` root. The [machine-readable roster](../skills/visioncraft/assets/legacy-paths.json)
includes exact original blob IDs and SHA-256 values. A map row is an instruction
to migrate a caller, not an installed backward-compatibility alias.

| Original path | New target or retirement | Preserved responsibility / action |
|---|---|---|
| `SKILL.md` | `SKILL.md` | Rewrite every active caller/registry/picker to visioncraft and remove the legacy entrypoint at hard cutover; no retained redirect. |
| `README.md` | `references/migration.md` | Historical removed npm API is documented as retired; actual CLI/library documentation belongs to the package README. |
| `BUILD-WITH-QUALITY-PROMPT.md` | `assets/activation.md` | Rewritten portable task brief preserves phases and explicit profile choices. |
| `DEBUGGING-PROTOCOL.md` | `references/debugging.md` | Feedback loop, blocked setup, static oracle and design interrogation; diagram diagnosis is separately linked. |
| `EDD-PROTOCOL.md` | `references/edd.md` | Seven steps, evidence categories, independence, freshness, workshops and stabilisation retained without host model defaults. |
| `USAGE-EXAMPLES.md` | `references/worked-examples.md` | All five project examples retained as explicit example contracts, not claimed execution. |
| `config/skill.yaml` | **retired** | No discovered runtime loader; retire this descriptive config instead of advertising inert policy. See references/architecture.md. |
| `scripts/deepsec-gate.sh` | **execution-migration** | Rewrite all callers to the installed VisionCraft Deepsec command and remove the old script; verify argument/exit/receipt contracts in adapter tests. |
| `scripts/deepsec-gate.test.mjs` | **execution-migration** | Migrate behavioural cases into the Rust/host conformance suites; remove the old wrapper test implementation. Source is not copied into this portable skill. |
| `references/agents.md` | `references/roles.md` | All 34 historical labels remain logical roles; coordination and unavailable ML labels do not become workers. |
| `references/architecture.md` | `references/architecture.md` | Portable boundaries and optional host capabilities replace unsupported learning, consensus and benchmark claims. |
| `references/bhil-traceability.md` | `references/traceability.md` | Requirement/decision/task/release lineage plus prompt/evaluation/guardrail concepts, independently worded. |
| `references/deepsec-security-gate.md` | `references/deepsec.md` | Protocol-level completion, exports, failure and scope guidance; host setup is in ecosystem-agentbox.md. |
| `references/methodologies.md` | `references/methodologies.md` | DDD/ADR/TDD and non-vacuous source verification retained; no assumed ADR plugin runtime. |
| `references/migration.md` | `references/migration.md` | Canonical successor and explicit retired surfaces. |
| `references/quality-gates-and-workflow.md` | `references/quality-gates.md` | Six check families and lifecycle retained with project-specific applicability and honest outcomes. |
| `references/review-evidence.md` | `references/review-evidence.md` | All audit lesson families rewritten around cause, consumed version, authority, effects, gate completion and delivery. |
| `references/rust/NOTICE.md` | `references/rust/NOTICE.md` | MIT BeautifulRust resource retained; estate overlays replaced where noted in provenance. |
| `references/rust/steering.md` | `references/rust/steering.md` | MIT BeautifulRust resource retained; estate overlays replaced where noted in provenance. |
| `references/rust/review.md` | `references/rust/review.md` | MIT BeautifulRust resource retained; estate overlays replaced where noted in provenance. |
| `references/rust/port.md` | `references/rust/port.md` | MIT BeautifulRust resource retained; estate overlays replaced where noted in provenance. |
| `references/rust/lints/clippy.toml` | `references/rust/lints/clippy.toml` | MIT BeautifulRust resource retained; estate overlays replaced where noted in provenance. |
| `references/rust/lints/lints.toml` | `references/rust/lints/lints.toml` | MIT BeautifulRust resource retained; estate overlays replaced where noted in provenance. |
| `references/rust/sources/effective-rust.md` | `references/rust/sources/effective-rust.md` | MIT BeautifulRust resource retained; estate overlays replaced where noted in provenance. |
| `references/rust/sources/google-rust-style-guide.md` | `references/rust/sources/google-rust-style-guide.md` | MIT BeautifulRust resource retained; estate overlays replaced where noted in provenance. |
| `references/rust/sources/microsoft-rust-guidelines.md` | `references/rust/sources/microsoft-rust-guidelines.md` | MIT BeautifulRust resource retained; estate overlays replaced where noted in provenance. |
| `references/rust/sources/rust-api-guidelines.md` | `references/rust/sources/rust-api-guidelines.md` | MIT BeautifulRust resource retained; estate overlays replaced where noted in provenance. |
| `references/rust/sources/rust-design-patterns.md` | `references/rust/sources/rust-design-patterns.md` | MIT BeautifulRust resource retained; estate overlays replaced where noted in provenance. |

## Consumer cutover contract

1. Pin and install the complete released `skills/visioncraft/` tree. Check its
   bytes against the immutable source, including Rust notices and referenced
   assets; an entrypoint without its references is not the payload.
2. Rewrite both harness registries, discovery/routing sources, the Jev picker,
   generated indexes, stack projection, preflight, agent instructions, neighbour
   skills and active documentation. Regenerate associated fixtures from the
   reviewed source change; changing a golden label is not measured routing uplift.
3. Rewrite every direct uppercase protocol/reference path through the map above.
   Replace old Deepsec script invocations with the installed VisionCraft command
   and test flags, environment precedence, exits, completion/export semantics
   and receipt paths. Migrate meaningful tests; remove the old implementation.
4. Remove the old active skill and registration. Check the installed harness
   reconciliation behaviour explicitly: deleting a manifest row must not leave
   a stale discoverable personal skill directory. Keep historical audit evidence
   pinned to the old version rather than mass-renaming its contents.
5. Check a generic no-Agentbox consumer as well as the host adapter. Verify the
   released dependency/lockfile and source hash after removing local development
   patches. Then rebuild and observe installed versions and a harmless end-to-end
   check before claiming deployed activation.

Do not create a second memory, finding or budget ledger during this cutover.
Sealmap remains the finding/disposition and model-family contract owner. Source
changes, publication, consumer adoption and runtime activation retain separate
receipts. The [Agentbox use case](../skills/visioncraft/references/ecosystem-agentbox.md)
provides the integration-specific boundary; [PROVENANCE.md](PROVENANCE.md)
records licensing and source treatment.
