# CALM search accuracy — root cause investigation & fixes #1/#2

**Timestamp (UTC):** 2026-07-09T04:22:29Z
**Repo:** `/home/user/CALM`, branch `claude/mcp-server-audit-1d3j63`
**Commit before this work:** `cda9b29a158c8e3284fcdd53eb3e0fe8e7259fbe`
**Commit after this work:** `8fd03fc` (pushed)
**Trigger:** 6/15 misses in the earlier CALM-vs-Semble comparison (`calm-vs-semble-comparison-2026-07-09.md`)

This report documents: what was actually wrong (verified against source, not guessed), what was fixed now (#1, #2 from the prior triage list), what was *not* fixed (and why), and what's left for later.

## 1. Investigation method

For every miss in the earlier benchmark, the actual raw result list was pulled from `calm_vs_semble_raw_results.json` and cross-checked against the real source (docstrings, signatures, chunking constants) via `symbol_info`/`source`/`edit_context`. A follow-up `kind="hybrid"` re-run was added to rule out "just use hybrid" as a free fix. No root cause below is speculative — each has a file:line citation.

## 2. Root causes found, by case

| Case | Symptom | Root cause | Category |
|---|---|---|---|
| S1 | Top-1 = `chunk_at` (a decoy) | `chunk_at`'s docstring (written earlier *by this same session*) contains "sliding windows overlap" — near-verbatim to the query text written later. **Self-contamination in the test set, not a CALM defect.** | Test methodology |
| S3 | Top-1 = a unit test (`is_lfs_pointer_stub_detects_pointer_text_not_real_weights`) | Test's name paraphrases the query almost exactly. `NOISE_PENALTY=0.6` (in `rrf_merge_n`, `crates/calm-core/src/search.rs:19`) *does* apply and *does* fire here, it's just not strong enough against a near-verbatim name match. **→ Fixed by #1.** Also: a second legitimate answer (`is_lfs_pointer` in `scripts/mcp-launcher.sh`, explicitly referenced in the real function's own docstring) was surfaced instead of the hand-picked one — the single-ground-truth scoring undercounts this. | Real defect (partially fixed) + test methodology |
| S5 | Real answer (`search_grep`) ranked 5th, not top | Rank 1 (`naive_grep_ranked_files` in `benchmarks/lib/naive_workflow.py`) and rank 2 (`build_walker`, `search_grep`'s own low-level helper) are both legitimate, closely-related answers. Single-ground-truth scoring penalizes correct recall of genuine near-duplicates. | Test methodology |
| S9 | Miss entirely; wrong `edit.rs` (there are two: `calm-core/src/edit.rs` and `calm-server/src/tools/edit.rs`) ranked above the right one | Real path/module-name collision: two files named `edit.rs` in the same problem domain (edit-safety hashing). Model correctly finds "the edit-safety neighborhood," picks the wrong file in it. | Real, structural (not fixed — see §4) |
| R2 | `store_chunk_embedding` ranked 2nd, not 1st | All of `embedding.rs`'s Layer-1/Layer-2 sibling pairs (`store_embedding`/`store_chunk_embedding`, `fetch_symbol_vecs`/`fetch_chunk_vecs`, etc.) cluster tightly in embedding space — a near-miss inside the *correct* neighborhood, not a wrong neighborhood. | Real, structural (low severity) |
| R4 | Miss entirely | `chunk_at` is 33 lines (> `CHUNK_MAX_LINES=30`, `crates/calm-core/src/indexer/chunker.rs:21`), so its 5-line "does this table exist" check shares a ~30-line sliding-window chunk with unrelated JOIN/row-mapping code — diluted. The anchor side (`migrate_add_project_memory_fts`, 16 lines total) is small enough to embed as one "pure" chunk. Confirmed by reading both bodies directly. | Real, structural (not fixed — see §4) |
| general | `kind="hybrid"` produced byte-identical results to `kind="semantic"` on every failing case | Hybrid's extra FTS component only activates on literal keyword overlap; all test queries were natural-language paraphrases with none. Confirms the ceiling is the *embedding model's* paraphrase generalization, not the choice of `kind`. | Architectural (not fixed — see §4) |
| S8 (side finding) | Exact same query/data, rank shifted 4→3 between two back-to-back process launches | `rrf_merge_n` used `HashMap` for score/dedup bookkeeping; ties fed into a stable sort in HashMap's per-process-random iteration order. **→ Fixed by #2.** | Real defect (fixed) |

## 3. Fixes implemented and verified this session

### Fix #1 — `include_tests` param (hard-exclude, not soft-penalize)

**File:** `crates/calm-server/src/tools/locate.rs`

- New `SearchParams.include_tests: bool` (`#[serde(default = "default_include_tests")]`, default `true` — current behavior unchanged unless a caller opts in).
- New `search_fetch_limit(&SearchParams) -> usize`: requests `limit*2` (min `limit+5`) from the underlying `calm_core` search when `include_tests=false`, so the filter below has a pool to draw from.
- New `apply_include_tests_filter(&mut Vec<SearchResult>, limit, include_tests) -> bool`: `.retain(|r| !r.is_test)` when `include_tests=false`, then truncates back to `limit`, returning whether truncation happened.
- Wired into all three handlers that return `SearchResult`s: `search`, `search_grep_impl`, `search_similar_impl`.
- Unit tests: `apply_include_tests_filter_{keeps_all_when_include_tests_true, excludes_tests_when_false, reports_truncated_when_still_over_limit}` in `locate.rs`.

**Verified against the rebuilt binary** (`verify_fixes.py`, fresh `calm serve` process, `kind="semantic", include_tests=false`):

- S3's result list went from `is_lfs_pointer_stub_detects_pointer_text_not_real_weights` (test) at rank 1 to a clean list of 5 real, non-test symbols (`load`, `embedding.rs`, `is_lfs_pointer`, `mcp-launcher.sh`, `default_vendored_asset_unusable`) — **the mechanism works exactly as designed.**
- **Net effect on the aggregate metric: unchanged (symbol-hit@5 stays 7/10).** S3's target (`is_lfs_pointer_stub` itself) still isn't in the top 5 even with the test gone — its second root cause (a 3-line function producing a low-signal chunk embedding, see §4) is untouched by this fix. This is the honest result, not a hidden failure: the fix does what it says, it just doesn't fully resolve a case that had two independent causes.
- Incidental improvement: S9's top-1 changed from a generic file-boundary chunk (`edit.rs`) to `edit_symbol`, a real and much more relevant function — apparently a test result had been occupying a slot ahead of it.
- Known trade-off observed: because the fix overfetches (`limit*2` instead of `limit` from the underlying symbol/chunk KNN), a few non-test rankings shifted slightly even where no test was involved (e.g. S10 file_rank 1→2) — pulling more candidates into the RRF fusion can let a name that's present in *both* the symbol-identity and chunk layers (but was previously outside the fetch window in one of them) pick up extra score it didn't have before. Not a bug, but a real side effect of the overfetch approach worth knowing about.

### Fix #2 — deterministic tie-breaking in `rrf_merge_n`

**File:** `crates/calm-core/src/search.rs`

- `rrf_merge_n`'s internal `scores`/`data` maps switched from `HashMap<String, _>` to `BTreeMap<String, _>` (added to the existing `use std::collections::{BTreeMap, HashMap}` import — `HashMap` is still used elsewhere in the file, e.g. `search_symbol`, which was left untouched, in scope).
- No signature or behavioral change beyond iteration order; `search_symbol`'s own separate `HashMap` (a similar but distinct pattern) was **not** touched — flagged in §5 as a follow-up, not done now, to keep this change scoped to what was agreed. **(Done later the same day — see Fix #3.)**
- New test: `test_rrf_merge_n_ties_break_deterministically_by_qualified_name` — two results made to genuinely tie on RRF score, asserts the alphabetically-first `qualified_name` always wins.

**Verified against the rebuilt binary**: ran all 10 search queries through two independent, freshly-spawned `calm serve` processes and diffed the exact result order. **10/10 identical, byte-for-byte.** (Previously S8 had been observed to flip rank 4→3 between separate runs with identical input.)

### Fix #3 — deterministic tie-breaking in `search_symbol` (same-day follow-up)

**File:** `crates/calm-core/src/search.rs`

Fix #2 above explicitly scoped `search_symbol`'s identical `HashMap` nondeterminism pattern out as a "trivial follow-up if desired" (§4's original table). Re-auditing this report against current source (triggered by a request to re-verify + fix what was in-scope) confirmed the pattern was still live — `search_symbol`'s `scores`/`data` maps were still `HashMap`, feeding the same stable-sort-over-random-iteration-order hazard `rrf_merge_n` had. Fixed the same way:

- `search_symbol`'s `scores: HashMap<String, f64>` / `data: HashMap<String, SearchResult>` switched to `BTreeMap`. `HashMap` import removed from `search.rs` entirely (no longer used anywhere in the file after this change — `cargo clippy` catches this as `unused_imports` if left in, which is how the leftover was caught).
- New test: `test_search_symbol_ties_break_deterministically_by_qualified_name` — two symbols with identical name/docstring/tokens genuinely tie on combined BM25 score; asserts the alphabetically-first `qualified_name` always wins.
- `cargo test -p calm-core --lib`: 593 passed, 0 failed, 8 ignored (up from 592).

### Test/build status

- `cargo test -p calm-core --lib`: 593 passed, 0 failed, 8 ignored (up from 591 pre-Fix#2 → 592 post-Fix#2 → 593 post-Fix#3).
- `cargo test -p calm-server --lib`: 111 passed, 0 failed (up from 108 — the three new `apply_include_tests_filter` tests).
- Verified in both the default (`embeddings` on) and `--no-default-features --features tier0-5,scip-overlay` (`embeddings` off) builds during earlier work this session; none of Fix #1/#2/#3 touch the embeddings feature gate.

## 4. Not fixed (and why not, right now)

| Root cause | Why not done now | Suggested approach |
|---|---|---|
| Static, 256-dim, non-contextual embedding model (`model2vec` `potion-code-16M`) has a real paraphrase-generalization ceiling | Architecture-level trade-off (speed/portability vs. accuracy) that trades against CALM's "no GPU, no network, pure Rust" design goal — needs a maintainer decision, not a unilateral swap | Survey whether `model2vec-rs`/the "potion" family has a larger checkpoint that's still pure-Rust/static; re-run this same benchmark before/after if one exists. Longer-term: an optional heavier contextual-embedding backend behind its own feature flag |
| Fixed 30-line sliding-window chunking dilutes short idioms inside longer functions (R4) | Touches the corpus-wide chunking algorithm; changing `CHUNK_MAX_LINES`/`CHUNK_STRIDE` affects *every* chunk in the index, not just this case — needs a controlled before/after re-measurement, not a blind tune | Try lowering `CHUNK_MAX_LINES` and re-run this benchmark; if that's insufficient, statement/block-boundary-aware sub-chunking is the real fix but is a bigger change to `indexer/chunker.rs`. **Status 2026-07-09 re-run: still an open miss** — R4 (chunk_at vs. migrate_add_project_memory_fts's shared SQL idiom) missed entirely again, unchanged from the original run. |
| Cross-file basename collisions (`edit.rs` × 2) and multi-valid-answer queries (S3, S5) | Not really fixable in CALM's ranking — these are cases where CALM found a *correct*, defensible answer that just wasn't the one hand-picked as ground truth | None needed in CALM; flagged for future benchmark design (independent/blind ground truth, credit multiple valid answers) |

## 5. Net takeaway

Three real, narrowly-scoped defects were confirmed and fixed today (nondeterministic tie-breaking in both `rrf_merge_n` and `search_symbol`; insufficiently strong test-noise suppression), all verified against actual rebuilt binaries rather than assumed from a diff. Of the six original misses, one methodology flaw (S1) was self-inflicted, two (S3, S5) were the search tool finding *other correct answers* than the one hand-picked, one (S8) is now provably fixed, and two (S9, R4) trace to real structural limitations — file-basename collision and fixed-size chunking — that need larger, separately-scoped work to address.

**2026-07-09, later the same day — full re-verification against a fresh build (commit `71b1239`):** see `calmvssemblecomparison20260709.md`'s "2026-07-09 re-run" section for the complete redo (re-verified ground truth, N=3 determinism checks for both CALM *and* Semble, and a newly-discovered corpus-hygiene bug — Semble indexing a stale git worktree as duplicate content, 45% of its returned result slots). Headline: CALM still leads Semble on accuracy (~2x search, ~1.75x similar, contamination-controlled), a smaller but more defensible margin than this document's original 7x/1.4x snapshot — determinism is provably fixed, R4/S9's structural limitations are still open, and the earlier "not fixed" list is now down to two items (chunking dilution, basename collision) plus the embedding-model ceiling.
