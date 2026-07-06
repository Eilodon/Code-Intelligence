# Session Handoff — 2026-07-07 03:00

## Task Summary
Deep-review + finalize the roadmap that brings CALM's remaining 8 languages (Go, Java, C#, C, C++, JavaScript, PHP, SQL — plus Python uplift and Kotlin bonus) to Formal-tier call-graph accuracy via the proven SCIP-overlay architecture. Review is DONE; the revised, execution-ready plan is written. Next session's job is to EXECUTE it.

## Current Status
STATUS: IN_PROGRESS (plan finalized, implementation not started)

## Completed Steps
- ✅ Audited the user's original 8-language plan against the working tree — every codebase claim verified with file:line anchors (see plan §1); found 3 plan-breaking issues (upgrade-only ingest ceiling, scip-php existence, stack-graphs archived).
- ✅ Web-verified the July-2026 SCIP tool ecosystem (versions, prereqs, platform gaps) — see plan §2.
- ✅ Wrote the full revised execution plan: `docs/superskills/plans/2026-07-07-eight-lang-formal-tier.md` (phases P0.1→P3.3, per-task files/steps/tests/DoD).
- ✅ Persisted key findings to agent memory: `~/.claude/projects/-home-ybao-B-1-CALM/memory/lang-roadmap-review-findings.md`.

## Open Work
(Ordered — dependencies in plan §9. Do NOT reorder P0.)
- [ ] P0.1 wire overlay into one-shot `calm index` (crates/calm-cli/src/main.rs; mirror lib.rs:195)
- [ ] P0.2 path rebase in scip/parse.rs for subroot indexer runs — depends on: P0.1 merged (touches same module)
- [ ] P0.3 provenance column + gated-insert mode + match_rate in scip/ingest.rs (+ `edge_kind` column for SQL in same migration; fix stale types/mcp_types.ts EdgeConfidence)
- [ ] P0.4 generalize runner → ScipProvider table (scip/provider.rs) — prerequisite for all of Phase 2
- [ ] P0.5 commit fixtures tests/fixtures/multi_lang_workspace/ + nightly CI job
- [ ] Phase 1 (parallel after P0): P1.1 JS stack-graphs key · P1.2 PHP (call_node_types FIRST, then imports/PSR-4/type_map) · P1.3 Tier-1.5 same-dir preference go/java/c/cpp · P1.4 C/C++ includes+type_map · P1.5 C# namespace table
- [ ] Phase 2 providers (after P0.4): go, java, csharp, python, php + ops surface (`calm scip run`, `calm index --scip-file`)
- [ ] Phase 3: scip-clang · scip-typescript · SQL module (SQL can start any time after P0.3)
- [ ] Benchmark harness benchmarks/resolution/ — build right after P0.5, measure baseline BEFORE Phase 2

## Open Decisions
- ❓ Gated-insert default: `insert_missing` auto-on vs opt-in — Lean: auto-on (gates are strict: fresh cache key + unique def symbol + known enclosing), but re-evaluate after measuring false-insert rate on the Rust benchmark repo.
- ❓ P1.3 V2 (confidence upgrade to Resolved via package_symbols) — do only if V1 (same-dir candidate preference) measurably insufficient on benchmark repos.
- ❓ java/csharp providers on unknown repos: plain auto vs explicit opt-in (they execute the repo's build system) — Lean: auto, with documented security note + per-lang off-switch; revisit if users object.
- ❓ Benchmark formal_pct thresholds — set after first baseline run, not before.

## Active Context
SPEC: (none — plan doc serves as spec)
PLAN: docs/superskills/plans/2026-07-07-eight-lang-formal-tier.md  ← READ THIS FIRST, it is self-contained
BRANCH: main (implementation work should branch per phase, e.g. feat/scip-provider-table)
CONSTITUTION_LAWS_ACTIVE: repo AGENTS.md mandatory rules (repo_overview first; edit_context before edits; diff_impact before commit — both hook-enforced)

## Evidence Produced This Session
(Verified 2026-07-07 on working tree — do not re-derive unless these files changed)
- crates/calm-core/src/scip/ingest.rs:34 — ingest is upgrade-only, (file,line) match, never inserts (test `never_downgrades_or_inserts` at :236) — T1
- crates/calm-core/src/indexer/pipeline.rs:20,642-649 — MAX_CALLEE_CANDIDATES=20; >20 candidates + no same-file match → zero edges — T1
- crates/calm-core/src/scip/parse.rs:29 — relative_path used verbatim (no rebase) — T1
- crates/calm-server/src/watcher.rs:188 + crates/calm-server/src/lib.rs:195 — only production run_overlay call sites — T1
- crates/calm-core/src/resolver/formal.rs — stack-graphs registered for python/typescript(+tsx)/java only; formal upgrade is name-set based (pipeline.rs:374-379) — T1
- crates/calm-core/src/indexer/lang_constants.rs — PHP call_node_types lacks member/scoped/nullsafe/new call kinds — T1
- Tool ecosystem status (scip-java v0.13.1 07/2026, scip-go v0.2.7, scip-dotnet v0.2.14, scip-typescript v0.4.0, scip-php exists, stack-graphs archived 2025-09-09, SCIP community governance 2026-03) — T2 (web, cited in plan §2)

## Blockers
- 🚫 None. Implementation can start immediately at P0.1.

## Next Session Opening
"Start by: reading docs/superskills/plans/2026-07-07-eight-lang-formal-tier.md end-to-end, then run repo_overview(), branch off main, and implement P0.1 (wire scip::run_overlay into the one-shot `calm index` CLI path, mirroring crates/calm-server/src/lib.rs:195). Context loaded from this file."

## Skills in Use
- session-handoff: this document
- tdd-verified / verification-before-completion: each P-task in the plan ships with named tests + DoD — write tests first, show fresh output before claiming done
- pattern-globalize: when fixing per-language constants (e.g. PHP call kinds), check the sibling languages for the same gap before committing
