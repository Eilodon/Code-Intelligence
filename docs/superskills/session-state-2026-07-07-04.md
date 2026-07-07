# Session Handoff — 2026-07-07 22:15 (superseded — see update below)

> **UPDATE (2026-07-07, later same day):** This handoff was written at the P0.3→P0.4
> boundary. Work continued past that point in (a) subsequent session(s): P0.4, P0.5,
> and all of Phase 1 (P1.1-P1.5, P1.5 half-scoped) are now DONE AND COMMITTED. The
> "Open Work"/"Open Decisions"/"Next Session Opening" sections below are STALE —
> jump to "## Update 2 — Current Status (2026-07-07, post Phase 1)" near the bottom
> of this file for what's actually left. Original content preserved above that point
> for history; don't re-read it as current state.
>
> **UPDATE 3 (2026-07-07, still later):** `benchmarks/resolution/` (the multi-language
> baseline harness) is now built and run for real — see "## Update 3" at the very
> bottom of this file. It found and this session fixed a real crash bug in the C/C++
> indexer. Jump straight to Update 3 if you only care about current state.

## Task Summary
Execute `docs/superskills/plans/2026-07-07-eight-lang-formal-tier.md` (the 8-language
formal-tier SCIP plan). This session implemented and committed P0.1, P0.2, and P0.3 —
the three foundation fixes that had to land before any new-language SCIP provider
(Phase 2) would be worth building. Session paused at the P0.3→P0.4 boundary per user
direction (P0.4 is a pure refactor with no payoff until a second concrete provider
exists to validate the abstraction against).

## Current Status (ORIGINAL — see Update 2 at bottom for current reality)
STATUS: (superseded) — at write time: P0.1-P0.3 done and committed; P0.4 onward not started

## Completed Steps
- ✅ P0.1 — wired `calm_core::scip::run_overlay` into the one-shot `calm index` CLI
  path (`crates/calm-cli/src/main.rs`), mirroring `calm-server`'s background-indexer
  call shape exactly. Commit `20f4265`. Evidence: new ignored integration test
  (`crates/calm-cli/tests/scip_overlay_cli.rs`) passes with real rust-analyzer (5
  edges upgraded); manual subprocess run with `rust.scip.enabled:false` confirms
  identical-to-before behavior when the overlay doesn't run.
- ✅ P0.2 — `parse_index`/`parse_scip_file` (`crates/calm-core/src/scip/parse.rs`) now
  take `rebase_prefix: &Path`, join+normalize occurrence paths, and handle an
  indexer-emitted absolute `relative_path` by stripping SCIP's own
  `Metadata.project_root` first (percent-decoded `file://` URI). Unknown project_root
  → path stays absolute (never silently degrades to a relative-looking string that
  could collide with a real file). Both production call sites pass an empty prefix
  (Rust always runs at repo root) — zero behavior change, confirmed by the existing
  real-rust-analyzer test still passing. Commit `40e6b40`.
- ✅ P0.3 (the plan's own "highest-leverage" item) — `call_edges.formal_source`
  migration (`'scip'|'stack_graphs'|NULL`); SCIP is now allowed to override a
  `stack_graphs`-sourced `formal` edge (never a prior `'scip'` verdict) via
  `mark_ruled_out_siblings`'s `is_formal` computation; gated-insert
  (`scip::ingest::insert_missing_edges`) creates a new `formal`/`'scip'` edge for a
  call site tree-sitter extracted (`call_sites` row exists) that
  `rebuild_graph`'s `MAX_CALLEE_CANDIDATES` cap dropped entirely. `IngestStats` gains
  `inserted`/`match_rate`, surfaced via `indexing_status`'s `scip_overlay` field
  through a new `.calm/scip-stats.json` sidecar. Config: `rust.scip.insert_missing:
  Option<bool>` (default auto-on). `types/mcp_types.ts`'s `EdgeConfidence` fixed to
  all 6 real variants. Commit `e0471f9`. **Verified on real data**: `calm index` with
  real rust-analyzer on the `rust_workspace` fixture → 5 upgraded, 1 ruled out, **3
  newly inserted** edges (the exact cap-dropped scenario this exists for),
  match_rate=0.28 (believable, not a suspicious 1.0).
- ✅ Bonus fix found while wiring P0.3's stats: all 3 `run_overlay` production call
  sites (`lib.rs`, `watcher.rs`, `main.rs`) previously only refreshed `caller_count`
  on `upgraded>0 || ruled_out>0` — missing `inserted>0`, which would have left
  newly-inserted edges' target `caller_count` stale immediately. Fixed in all three.
- ✅ Full workspace test suite green after every commit (494 passed at last check),
  clippy clean (`-D warnings`), fmt clean.
- ✅ Updated the plan doc itself (`docs/superskills/plans/2026-07-07-eight-lang-formal-tier.md`)
  to mark P0.1-P0.3 as done with commit refs and actual-outcome summaries (including
  2 deliberate deviations from the original design — see "Open Decisions" below) so a
  future session doesn't redo this work or get confused about what's still open.

## Open Work
(Ordered — dependencies in plan §9. P0.1-P0.3 done; resume at P0.4.)
- [ ] P0.4 — generalize the Rust-specific runner into a data-driven `ScipProvider`
  table (new file `crates/calm-core/src/scip/provider.rs`; refactor
  `scip/mod.rs::run_overlay`, `runner.rs`, `config.rs`). Pure refactor, no behavior
  change per its own DoD. **See Open Decisions below before starting this** — the
  user was asked whether to do this now vs. defer.
- [ ] P0.5 — `multi_lang_workspace` fixture + nightly CI job — depends on: P0.4 in
  the plan's stated order, but is mostly independent busywork (fixture files + CI
  yaml) that could plausibly be done in parallel or first if that's ever useful.
- [ ] Phase 1 (parallel after P0, per plan §9): P1.1 JS stack-graphs key · P1.2 PHP
  (call_node_types FIRST) · P1.3 Tier-1.5 same-dir preference · P1.4 C/C++ · P1.5 C#
  namespace table
- [ ] Phase 2 providers (after P0.4): go, java, csharp, python, php + ops surface
- [ ] Phase 3: scip-clang · scip-typescript · **SQL module (P3.3) — independent of
  P0.4/P0.5, can start any time now** (only needs the `edge_kind` column from P0.3's
  migration, which already landed)
- [ ] Benchmark harness `benchmarks/resolution/` — build after P0.5, measure baseline
  before Phase 2

## Open Decisions
- ❓ **P0.4 timing** — user was asked (this session) whether to: (a) do P0.4+P0.5 now
  per the plan's original sequential order, (b) skip P0.4's abstraction and build one
  concrete Phase 2 provider (e.g. Go) directly against the current Rust-specific
  shape first, generalizing only once there are 2 real cases, or (c) stop here
  entirely for this session. User's answer was to update documentation for handoff
  (implying (c) for this session) — **next session should re-ask or use judgment
  based on what the user wants to tackle next**, since no explicit choice among
  (a)/(b) was made, only that this session should end cleanly.
- ❓ Gated-insert default (`insert_missing` auto-on) — shipped as auto-on per the
  plan's original lean (gates are strict: fresh cache key + unique def symbol + real
  `call_sites` row + dedup). Real-data run found 3 inserts with no apparent false
  positives, but this has only been observed on one small fixture — worth watching
  match_rate/inserted counts on a larger real repo before fully trusting the default
  at scale.
- ❓ P1.3 V2 (confidence upgrade to Resolved via package_symbols) — unchanged from
  original plan: do only if V1 measurably insufficient on benchmark repos.

## Active Context
SPEC: (none — plan doc serves as spec)
PLAN: `docs/superskills/plans/2026-07-07-eight-lang-formal-tier.md` — now
self-annotated with ✅/⬜ status per task; read the top banner and §10 first.
BRANCH: main (uncommitted-elsewhere work note: this session found and separately
committed unrelated pre-existing work at session start — see commit `1dd4ba2`,
already landed before P0.1; not part of this plan, mentioned only so it isn't
mistaken for plan work)
CONSTITUTION_LAWS_ACTIVE: repo AGENTS.md mandatory rules (repo_overview first;
edit_context before edits; diff_impact before commit — both hook-enforced)

## Evidence Produced This Session
(Verified 2026-07-07 on working tree — supersedes the P0.1-P0.3 evidence anchors in
the plan doc's own §1, which now describe fixed-not-broken behavior)
- `crates/calm-cli/src/main.rs` — `Commands::Index` now calls `run_overlay` — T1
- `crates/calm-core/src/scip/parse.rs` — `parse_index`/`parse_scip_file` take
  `rebase_prefix`; absolute-path + `project_root` stripping; 6 new unit tests all
  passing — T1
- `crates/calm-core/src/scip/ingest.rs` — `formal_source` override logic,
  `insert_missing_edges`, `IngestStats.{inserted,match_rate}` — 5 new unit tests
  passing, all 5 pre-existing tests still passing — T1
- `crates/calm-core/src/db/schema.rs:255-268` (approx) — `formal_source` migration — T1
- Real rust-analyzer end-to-end run on `rust_workspace` fixture (copied to a tempdir,
  run via the built `calm` binary, not just `cargo test`): 5 upgraded, 1 ruled_out, 3
  inserted, match_rate=0.28, `.calm/scip-stats.json` sidecar written correctly — T1
  (this session, reproducible via the commands in commit `e0471f9`'s message)
- Full workspace test suite: 494 passed, 0 failed, 2 ignored (both real-rust-analyzer
  integration tests, both separately verified passing with `--ignored`) — T1
- `cargo clippy --workspace --all-targets --features scip-overlay -- -D warnings`:
  clean — T1
- `cargo fmt --all -- --check`: clean — T1

## Blockers
- 🚫 None. P0.4 (or a Phase 2 provider, or P3.3 SQL) can all start immediately —
  see Open Decisions for which the user should pick.

## Next Session Opening
"Read `docs/superskills/plans/2026-07-07-eight-lang-formal-tier.md`'s top banner and
§10 (both updated this session) plus this handoff file. P0.1-P0.3 are done and
committed (`20f4265`, `40e6b40`, `e0471f9`) — do not redo them. Ask the user which
of the three P0.4 options in this file's 'Open Decisions' they want (or whether
they'd rather start P3.3 SQL, which is fully independent), then proceed."

## Skills in Use
- session-handoff: this document
- (none else actively invoked this session beyond the CALM MCP tool workflow itself)

---

## Update 2 — Current Status (2026-07-07, post Phase 1)

Verified directly against `git log` (6 latest commits) and against the plan doc's own
self-annotated §3/§4/§10, which are authoritative and already up to date — this
section just closes the gap in *this* handoff file, which had gone stale.

STATUS: Phase 0 (P0.1-P0.5) and Phase 1 (P1.1-P1.5) are **fully done and committed**.
Only Phase 2, Phase 3, the benchmark harness, one CI-verification loose end, and
P1.5's deferred half remain.

### Commits since this file was originally written
- `bae5161` — P0.4-P0.5: Rust-specific SCIP runner generalized into a data-driven
  `ScipProvider` table (`crates/calm-core/src/scip/provider.rs`); `multi_lang_workspace`
  8-language fixture + `.github/workflows/scip-nightly.yml` nightly CI job.
- `fdf0aaf` — P1.3: Tier-1.5 same-directory candidate preference for go/java/c/cpp.
- `7ba5fb5` — P1.4: C/C++ heuristics (`#include` resolution + type_map), plus a real
  pre-existing bug fix (`split_receiver_callee` never recognized `->`).
- `d7178b9` — P1.5 (partial): C# type_map + constructor inference. The "using →
  namespace-to-files table" half is deliberately deferred (needs a new repo-wide
  pre-pass, not a small heuristic — see plan doc §4/P1.5 for why).
- `7b7dec7` — P1.2: PHP heuristics (call extraction, imports, PSR-4, type_map) — also
  fixed a receiver-extraction bug affecting Java (`method_invocation`'s object/name
  split was never read).
- `b89771c` — P1.1: JavaScript stack-graphs formal tier — completes Phase 1 in full.

Full workspace at last commit: 526 tests passing, clippy `-D warnings` clean, fmt clean.

### What's actually remaining (see plan doc §5/§6/§7/§9/§10 for full detail)
1. **Phase 2 — SCIP providers** (P2.1-P2.6): go, java, csharp, python, php + ops
   surface (`calm scip run`, `--scip-file` CI ingest, refresh policy config,
   `indexing_status`/`fitness_report` per-language surfacing). Plan doc recommends
   **Go first** (`scip-go` is simplest, no build-tool network resolve like
   Java/C#) — slots into the `ScipProvider` table from P0.4 as one entry, no
   `mod.rs`/`runner.rs` changes needed.
2. **Phase 3** — P3.1 scip-clang (C/C++, Linux x86_64/macOS arm64 only), P3.2
   scip-typescript (JS/TS, retires the archived stack-graphs path), **P3.3 SQL**
   (datafusion-sqlparser-rs — fully independent, can start anytime, only needs the
   `edge_kind` column already added in P0.3's migration).
3. **Benchmark harness** (`benchmarks/resolution/`) — not built yet; plan recommends
   building it now (P0.5 fixture is ready) to get a "before" baseline ahead of Phase 2.
4. **CI loose end**: `.github/workflows/scip-nightly.yml` has only been verified via
   local-equivalent run (`cargo test --workspace -- --ignored`), never observed green
   on actual GitHub Actions (no push/`workflow_dispatch` triggered yet this project).
5. **P1.5's deferred half** — C#'s `using` → namespace-to-files table. Needs a new
   pre-pass (mirrors `CrateMap::build` for Rust) threaded into `extract_file_data`,
   since `import_map` is built per-file in parallel before any repo-wide namespace
   view exists, and `import_edges.to_path` is single-valued (can't represent "one
   namespace spans N files"). Real architectural change, not S/M-sized.

No technical blocker sits in front of any of these — next session's job is to pick
one (plan doc §10 leans toward Go provider or SQL as the best next moves), not to
resolve a dependency.

### Docs already current — don't re-derive
- `docs/superskills/plans/2026-07-07-eight-lang-formal-tier.md` — self-annotated
  ✅/⬜ per task, commit refs, and a §10 "next session" priority list. This is the
  living source of truth; this handoff file is secondary.
- Memory `lang-roadmap-review-findings.md` (auto-memory) — has matching update notes
  for both the Phase 0 and Phase 1 completions.

---

## Update 3 — Resolution benchmark harness built and run (2026-07-07, later still)

Picked "benchmark first" as the next move (over jumping straight into a Phase 2
provider or P3.3 SQL) — reasoning: this sandbox has network access (confirmed) but
only JDK+Gradle among the Phase 2 toolchains (no Go/.NET/PHP/composer), and disk was
tight (9.8GB free at the time), so the benchmark harness was both the highest-leverage
move (plan §7 itself gates Phase 2's DoD threshold on a first baseline) and the
lowest-friction one to actually execute here.

**Built:** `benchmarks/resolution/` (`run_benchmark.py` + `README.md`) — clones one
small pinned real OSS repo per language (go=gin, java=spring-petclinic,
csharp=eShopOnWeb, c=redis, cpp=fmt, js=express, php=monica, sql=sakila mirror) into
`corpus/<lang>/` (gitignored), runs `calm index` with embeddings disabled via a
per-corpus `.calm/config.json`, and reads `call_edges`/`symbols` tiers directly from
`.calm/index.db`. No SCIP-overlay feature needed — no Phase 2 provider exists yet for
any of these 8 languages, so there's nothing to overlay.

**Real bug found and fixed** (not hypothetical — hit indexing redis's real `server.h`,
~4700 lines): `calm index` hard-crashed with `UNIQUE constraint failed:
symbols.qualified_name`, aborting the entire run with zero output. Root cause
(confirmed via binary-search bisection down to the single offending file/line, plus a
temporary instrumented debug build): C's symbol extractor treats every *mention* of a
forward-declared `struct` type as a parameter type (e.g. `struct redisObject *key` in
a function-pointer typedef) as its own symbol occurrence, not just at the struct's
real definition. `server.h` has dozens of `moduleType*Func` typedefs all taking
`struct redisObject *` parameters, and at least one typedef takes *two* same-typed
parameters on the same line (`moduleTypeCopyFunc(struct redisObject *fromkey, struct
redisObject *tokey, ...)`). `extract_file_data`'s intra-file dedup (`pipeline.rs`) only
ever tried one `#{line_start}` suffix on a name collision — enough for 2 duplicates,
not for a 3rd occurrence sharing the exact same (name, line). The second suffix
attempt collided right back into itself, and the resulting INSERT error wasn't
handled, aborting the whole `calm index` run.

**Fix** (`crates/calm-core/src/indexer/pipeline.rs::extract_file_data`): the dedup
loop now appends an incrementing suffix (`#{line}`, `#{line}#2`, `#{line}#3`, ...)
until genuinely unique, instead of trying exactly once. Regression test
`test_c_same_line_triple_name_collision_does_not_crash_indexing` (same file) is a
minimal 5-line synthetic repro of the exact redis pattern — confirmed to reproduce the
crash with the fix reverted, and to pass (6 distinct symbol rows, no crash) with it
applied. Full workspace: 527 tests passing (526 + this new one), clippy `-D warnings`
clean, fmt clean, release binary rebuilt after the fix.

**Deliberately NOT fixed** (out of scope for this session, noted in both the plan doc
§7 and the benchmark's own README): the deeper root cause — treating a struct *type
reference* as if it were a symbol *definition* — is still there. It's noise, not a
crash, so it was left as a known, documented limitation rather than pulled into this
session's scope. This means C/C++'s `symbols_total`/`ambiguous` counts in the baseline
numbers below are inflated by an unquantified amount.

**Baseline results** (`benchmarks/resolution/results.json`, full detail + per-language
interpretation in `benchmarks/resolution/README.md`):

| lang | edges | resolved% | ambiguous% |
|---|---:|---:|---:|
| go | 7,672 | 15.0% | 54.3% |
| java | 254 | 16.9% | 28.7% |
| csharp | 318 | 40.9% | 20.1% |
| c | 40,573 | 37.1% | 11.5% |
| cpp | 51,399 | 4.8% | **92.5%** |
| js | 36 | 30.6% | 66.7% |
| php | 9,334 | 36.4% | 34.9% |
| sql | 0 | — | — |

**Most important finding**: `ambiguous` (the `MAX_CALLEE_CANDIDATES=20` fan-out cap
described in plan §1.2), not `textual`, is the dominant unresolved tier almost
everywhere — this is now measured, not theoretical, and is exactly the gap Phase 2
(SCIP's exact (file,line) matching) exists to close. `formal_pct=0.0%` everywhere is
expected (no Phase 2 provider exists yet for any of these 8 languages) — not a bug.
SQL=0 edges is a real, honest 0 — `.sql` isn't in `language_for_extension` yet (P3.3
not started), not a measurement error.

Docs updated to match: plan doc's top banner, §7 (full writeup), §9, §10 all now mark
the benchmark harness ✅ done. `benchmarks/README.md` cross-links to it as a separate
track from the B1-B11 series (different axis — language-support breadth, not
calm-vs-naive value prop). Memory `lang-roadmap-review-findings.md` has a matching
update paragraph.

**Next session**: pick one of Phase 2 (Go provider — or Java, since this sandbox
specifically already has JDK+Gradle and no Go/.NET/PHP toolchain installed, which
flips the plan's generic "Go first" advice for *this* environment specifically), P3.3
SQL (fully independent, lowest friction), or re-running this benchmark once a Phase 2
provider lands to measure the real delta.
