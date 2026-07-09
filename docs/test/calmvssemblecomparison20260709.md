# CALM vs Semble — 1:1 Tool Comparison

**Two runs on record, same day — read the re-run first.** The original run (kept below for the record) is superseded by the 2026-07-09 re-run: it fixes stale ground-truth line numbers (drifted from same-day commits), verifies determinism for *both* tools over N=3 independent process launches (the original was N=1), and uncovers + controls for a real corpus-hygiene bug. Cite the re-run's numbers, not the original's.

## 2026-07-09 re-run — commit `71b1239`, release build, worktree-contamination controlled for

**Why re-run:** two determinism fixes landed same-day (`rrf_merge_n` and `search_symbol`, both `HashMap`→`BTreeMap`), plus an unrelated clippy fix (`chunk_at`'s `ChunkMatch` type alias) that lightly reshuffled `embedding.rs`'s chunk boundaries. All 15 ground-truth line ranges were re-verified against current source (`file_overview`) before re-running — several had drifted a few lines and would have silently mis-scored otherwise.

**Determinism, both tools, N=3 independent process launches each:** CALM — 10/10 search cases byte-identical result order across all 3 runs, 0 mismatches (confirms both same-day fixes hold under real query load, not just the unit tests). Semble — also 0 mismatches across 3 runs; its accuracy gap below was never a run-to-run nondeterminism problem.

**Methodology bug found and controlled for — Semble indexes stale git worktrees.** This repo has a legitimate, git-registered worktree at `.claude/worktrees/upbeat-kalam-7081e9/` (gitignored via `.git/info/exclude`, left over from an earlier, finished `using-git-worktrees` session — clean, no uncommitted changes, not touched by this benchmark). CALM's indexer correctly skips it: 0 of CALM's results reference that path (verified programmatically over the full raw result set). Semble's does not — **45% of all result slots returned across the 15 queries (34/75) were duplicate content from that worktree**, not the real corpus. Rather than relocate someone else's live worktree to get a "clean" run (attempted — blocked by the harness's own shared-resource guard, correctly), Semble's numbers below are reported two ways: **raw** (as literally returned) and **dedup** (worktree-duplicate entries filtered out of Semble's result lists before rescoring — the fairer number, computed from the same raw JSON, not re-queried). CALM needed no adjustment (0% contaminated).

### Search axis (10 cases)

| Metric | CALM | Semble (raw) | Semble (dedup) |
|---|---|---|---|
| File-hit@5 | 10/10 | 9/10 | 9/10 |
| Symbol-hit@5 | 6/10 | 3/10 | 3/10 |
| Top-1 exact | 2/10 | 2/10 | 2/10 |
| MRR (symbol) | 0.345 | 0.225 | 0.233 |

Contamination barely moves this axis — Semble's search-ranking gap vs CALM (~2x symbol-hit@5, ~1.5x MRR) looks like a genuine ranking difference, not a corpus-hygiene artifact.

### Similar/related axis (5 cases)

| Metric | CALM | Semble (raw) | Semble (dedup) |
|---|---|---|---|
| File-hit@5 | 4/5 | 4/5 | 4/5 |
| Symbol-hit@5 | 4/5 | 3/5 | 3/5 |
| Top-1 exact | 3/5 | 0/5 | 1/5 |
| MRR (symbol) | 0.700 | 0.157 | 0.400 |

Here contamination mattered a lot: deduped MRR (0.400) is 2.5x the raw figure (0.157) — R1/R3/R5 all had their real answer pushed down a rank or more by duplicate worktree entries competing for top-5 slots. The honest gap is CALM ~1.75x Semble on this axis, not the ~4.5x the raw number would suggest.

### Token efficiency — unchanged conclusion

CALM is still ~2.5–2.7x leaner: 305 vs 835 est. tokens/response on search, 330 vs 672 on similar — same default-snippet-configuration effect noted in the original run (Semble's `max_snippet_lines=10` embeds source in every result; CALM's bare metadata doesn't).

### Net verdict vs. the original run's headline numbers

CALM still leads Semble on ranking accuracy on both axes in a fresh, same-commit, contamination-controlled, determinism-verified comparison. But the margin is meaningfully smaller than the original run's 7x/1.4x split — **~2x on search, ~1.75x on similar (dedup)** is the number to cite going forward, not the original's. Some of the shift is ordinary single-run variance (the original run's own Limitations section already flagged N=1 as not statistically powered); some is the corpus-hygiene bug above. Neither run is "wrong" — they're different snapshots under different rigor, and this one controls for more.

**Action item surfaced, not fixed here:** `benchmarks/b10_real_competitor_ab/` and `benchmarks/b11_extended_competitor_ab/` (this repo's official, git-tracked competitor benchmarks) may have the same worktree-contamination exposure if that worktree already existed when they last ran (it was created 2026-07-06) — worth checking whether CodeGraph/Semble/grepai/Serena's indexers respect `.git/info/exclude` before trusting those published numbers as-is.

**Reproduce:** not reproducible from this directory anymore, by design. `calm_vs_semble_bench.py`, `verify_fixes.py`, and `calm_vs_semble_raw_results.json` were deleted after this write-up was finalized (2026-07-09) — the goal (compare CALM against the Semble MCP server, decide whether CALM had caught up) was met, and per that decision the Semble MCP server integration itself (the `semble` entry in this repo's `.mcp.json`) was removed too. This file and `calmrootcauseandfixes20260709.md` are kept intentionally, as the record of what was measured and how — see the methodology notes throughout this section in lieu of a re-runnable script. `benchmarks/b10_real_competitor_ab/` and `benchmarks/b11_extended_competitor_ab/` (untouched by this cleanup) still exercise Semble independently via `uvx`, if a live re-comparison is ever needed again.

---

## Original run (2026-07-09T03:39–03:40 UTC) — historical, see re-run above for current numbers

**Run started (UTC):** 2026-07-09T03:39:35.65Z
**Run finished (UTC):** 2026-07-09T03:39:43.34Z
**Wall-clock duration of the whole run:** ~7.7s
**Repo under test:** `/home/user/CALM` @ commit `cda9b29a158c8e3284fcdd53eb3e0fe8e7259fbe`
**Report generated:** 2026-07-09T03:40:27Z UTC

This is a one-off comparison script written specifically for this request — it does not reuse any prior benchmark. Both MCP servers were spawned as fresh subprocesses and driven directly over raw JSON-RPC (stdio), so timing is measured identically for both and neither run went through an already-warmed, already-connected session.

- Script: `calm_vs_semble_bench.py` (this directory)
- CALM binary: `target/debug/calm` (rebuilt from the commit above, `--features embeddings,tier0-5,scip-overlay`)
- Semble: `uvx --from semble[mcp] semble` (fresh process, own in-memory index)
- **Not yet known at the time:** the worktree-contamination issue documented in the re-run above was present in this run too (the worktree was created 2026-07-06, before this run) but undetected — so these numbers carry the same, uncorrected bias.

## What was compared

Two axes, matching the tool-mapping analysis from earlier in this session:

1. **Search** — CALM `search(kind="semantic")` vs Semble `search()`. Free-text query → ranked code locations.
2. **Similar/Related** — CALM `search(kind="similar", path, line)` vs Semble `find_related(file_path, line)`. Location anchor → ranked code locations elsewhere that look like it.

10 search cases + 5 similar-code cases = 15 queries per tool, 30 calls total. Ground truth (the "correct" symbol for each query) was fixed *before* running either tool, verified by hand against the actual source via CALM's own `symbol_info`/`source` tools.

## Scoring method

For every result list returned by either tool, each entry was normalized to `{path, start_line, end_line}`. A case scores:

- **file_rank**: 1-indexed position of the first result whose file matches the ground-truth file (or `—` if absent from the top 5).
- **symbol_rank**: 1-indexed position of the first result whose line range *overlaps* the ground-truth symbol's line range (strictly stronger than file_rank — matching the right file but the wrong function does not count here).

**Token usage** is an estimate (`chars / 4`), not a real tokenizer count — `tiktoken` wasn't available in this environment and neither tool speaks Claude's own tokenizer, so treat it as a rough, consistently-applied proxy for "how much an agent would have to read," not an exact figure.

## Aggregate results

### Search axis (10 cases, free-text query)

| Metric | CALM `kind="semantic"` | Semble `search()` |
|---|---|---|
| File-hit@5 rate | 10/10 (100%) | 10/10 (100%) |
| **Symbol-hit@5 rate (strict)** | **7/10 (70%)** | **1/10 (10%)** |
| Top-1 exact (symbol) | 5/10 | 1/10 |
| Mean Reciprocal Rank (symbol) | 0.545 | 0.100 |
| Latency — mean | 28.6 ms | 54.7 ms |
| Latency — median | 17.2 ms | 3.9 ms |
| Latency — min / max | 15.2 / 135.2 ms | 3.8 / 507.6 ms |
| Response size — mean tokens (est.) | 306 | 819 |

### Similar/Related axis (5 cases, location anchor)

| Metric | CALM `kind="similar"` | Semble `find_related()` |
|---|---|---|
| File-hit@5 rate | 4/5 (80%) | 4/5 (80%) |
| **Symbol-hit@5 rate (strict)** | **4/5 (80%)** | **3/5 (60%)** |
| Top-1 exact (symbol) | 3/5 | 2/5 |
| Mean Reciprocal Rank (symbol) | 0.700 | 0.500 |
| Latency — mean | 9.9 ms | 6.7 ms |
| Latency — median | 9.9 ms | 4.8 ms |
| Latency — min / max | 9.5 / 10.3 ms | 4.3 / 14.8 ms |
| Response size — mean tokens (est.) | 330 | 710 |

### One-time "cold" costs (excluded from the per-query numbers above)

| | CALM | Semble |
|---|---|---|
| MCP `initialize` handshake | 9 ms | 814 ms |
| Embedder/index ready for the *first* real semantic answer | **5.62 s** (background model load; blocks `kind=semantic` only — `kind=similar`, `grep`, etc. work immediately) | **~508 ms** (included inside case S1's latency — full-repo index+embed on first call to this repo) |

Both are one-time-per-process costs in practice (CALM's model load is per server lifetime; Semble's index is currently in-memory per process, so it recurs if the process restarts, same as CALM's would if restarted). CALM's is markedly larger here — worth double-checking against a larger repo, since this run only has 126 files.

## Per-case breakdown

### Search cases (natural-language query)

| Case | Query (truncated) | Ground truth | CALM file/sym rank | CALM ms | CALM tok | Semble file/sym rank | Semble ms | Semble tok |
|---|---|---|---|---|---|---|---|---|
| S1 | split a file into overlapping sliding-window chunks... | `chunker.rs::chunk_file` | 3/— | 135.2 | 298 | 3/— | 507.6 | 744 |
| S2 | brute force nearest neighbour cosine distance search... | `embedding.rs::top_k_by_cosine` | 1/**1** | 17.4 | 291 | 1/— | 8.0 | 680 |
| S3 | detect vendored model file is an unresolved git lfs pointer... | `embedding.rs::is_lfs_pointer_stub` | 1/— | 15.9 | 321 | 1/— | 4.3 | 919 |
| S4 | reciprocal rank fusion merging ranked result lists | `search.rs::rrf_merge_n` | 1/**1** | 15.7 | 325 | 1/— | 3.8 | 618 |
| S5 | walk filesystem doing raw regex grep honoring gitignore | `search.rs::search_grep` | 3/5 | 15.2 | 312 | 1/— | 3.8 | 880 |
| S6 | open dedicated read-only sqlite connection, single writer | `tools/common.rs::make_read_conn` | 1/**1** | 17.2 | 304 | 1/— | 3.8 | 936 |
| S7 | check sqlite table exists before running a migration | `db/schema.rs::migrate_add_project_memory_fts` | 1/**1** | 17.2 | 321 | 1/**1** | 3.9 | 849 |
| S8 | resolve which indexed chunk contains a given line | `embedding.rs::chunk_at` (brand-new symbol) | 4/**4** | 18.1 | 301 | 3/— | 3.9 | 785 |
| S9 | hash a symbol's source range for optimistic concurrency | `edit.rs::range_checksum` | 3/— | 18.6 | 284 | 2/— | 4.0 | 951 |
| S10 | vector similarity anchored at a location, not text | `search.rs::search_similar` (brand-new symbol) | 1/**1** | 15.4 | 305 | 1/— | 3.8 | 826 |

### Similar/related cases (location anchor)

| Case | Anchor | Ground truth | CALM file/sym rank | CALM ms | CALM tok | Semble file/sym rank | Semble ms | Semble tok |
|---|---|---|---|---|---|---|---|---|
| R1 | `embedding.rs:60` (inside `create_embedding_table`) | `embedding.rs::create_chunk_embedding_table` | 1/**1** | 9.9 | 334 | 1/**1** | 14.8 | 786 |
| R2 | `embedding.rs:289` (inside `store_embedding`) | `embedding.rs::store_chunk_embedding` | 1/**2** | 9.8 | 327 | 1/— | 5.0 | 714 |
| R3 | `embedding.rs:343` (inside `knn`) | `embedding.rs::knn_chunks` | 1/**1** | 9.5 | 324 | 1/**1** | 4.3 | 685 |
| R4 | `db/schema.rs:369` (sqlite_master existence check) | `embedding.rs::chunk_at` | —/— | 10.3 | 339 | —/— | 4.8 | 703 |
| R5 | `search.rs:145` (inside `search_symbol`) | `search.rs::search_text` | 1/**1** | 10.0 | 327 | 2/**2** | 4.4 | 660 |

("file/sym rank" = rank at which the ground-truth *file* first appears / rank at which the ground-truth *symbol's line range* first appears. `—` = not in top 5.)

## Findings

1. **Accuracy — CALM ranked functions more precisely on this test set.** Both tools found the *right file* almost every time (file-hit@5: 100% search / 80% similar for both). The gap is at the *symbol* level: CALM's chunk-level embeddings landed on the exact function 70% of the time on free-text queries vs Semble's 10%; on location-anchored similarity, 80% vs 60%. Semble very consistently found the right *file* (it correctly favors `embedding.rs` for embedding-related queries every time) but its top candidate inside that file was often an adjacent function, a doc comment block, or a test — not the specific symbol asked about. This is a real, reproducible pattern in this run, not a one-off case (see S2–S10, where Semble's file_rank is almost always 1 but symbol_rank is almost always `—`).
2. **CALM found its own brand-new code (S8, S10) correctly** — `chunk_at` and `search_similar`, both added earlier in this session — confirming the index picked up the new symbols and embedded them correctly. Semble found the right *file* for both but not the exact function.
3. **Neither tool solved R4** (recognizing my new `chunk_at`'s "does this table exist" check as similar to the pre-existing `migrate_add_project_memory_fts` check in `schema.rs`, despite both using the literal same `SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=...` idiom). Both tools' embeddings evidently weight the surrounding function's *purpose* more heavily than a shared low-level SQL idiom — a genuine shared blind spot, not something either tool "wins" on.
4. **Token cost — CALM is markedly leaner by default**, about 2.5–2.7× fewer tokens per response (306 vs 819 tokens on search; 330 vs 710 on similar). This is mostly a default-configuration effect, not an architecture one: Semble's default `max_snippet_lines=10` embeds a source snippet in every result; CALM's `search`/`similar` return bare metadata (path/line/score) with no snippet unless the caller asks for source separately. An agent burns more context per Semble call as configured here, but also gets the code inline without a follow-up read.
5. **Latency — mixed, and cold-start-dependent.** Once warm, Semble's median latency (3.9–4.8 ms) beats CALM's (9.9–17.2 ms) — CALM is a longer round trip per call in this run. But CALM's process reaches "ready" for grep/similar/structural tools instantly (9 ms) while the semantic embedder loads in the background (5.6 s, not blocking other tools); Semble's cold path bundles indexing into the first call itself, visible as the 507 ms outlier on S1. Neither number should be read as "the" latency — they depend heavily on process lifecycle assumptions (long-lived server vs spawned-per-need).
6. **Caveat on Semble's cold-start being cheap:** 508 ms to index+embed 126 files is fast — plausibly this run benefited from `uv`'s package cache and OS file cache already being warm from an earlier exploratory call in this same session. Worth re-measuring on a cold environment before treating that number as representative.

## Limitations of this run (read before citing these numbers elsewhere)

- **N=15 queries, single run, no repeated trials** — this is directional, not statistically powered. Latency numbers especially can have noisy tails (see the two outliers: CALM S1 at 135 ms, Semble S1 at 508 ms).
- **Ground truth was authored by the same session that also just wrote the new CALM feature being tested** (`kind="similar"`) — bias risk acknowledged. It was fixed by reading actual source/line ranges before running either tool, not adjusted afterward, but an independent reviewer picking the test set would be a stronger check.
- **Token counts are a `chars/4` heuristic**, not a real tokenizer. Directionally reliable for comparing two JSON-ish text blobs, not exact.
- **Semble's index is in-process/in-memory** in this test (per its own tool description) — a persistent Semble server would amortize its cold-start the same way CALM's long-lived server does; this run cannot distinguish "inherently slower cold start" from "this particular process model."
- Only one language/codebase (this Rust repo) — no signal here on CALM's or Semble's behavior on Python/JS/other stacks.

## Reproduce

```
python3 calm_vs_semble_bench.py
```
from this directory. Requires `target/debug/calm` built with `--features embeddings,tier0-5,scip-overlay` and `uvx` on PATH. Writes `calm_vs_semble_raw_results.json` next to itself.
