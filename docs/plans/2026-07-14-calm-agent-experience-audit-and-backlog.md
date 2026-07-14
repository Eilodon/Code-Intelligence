# CALM agent-experience audit + backlog — 2026-07-14

## Why this doc exists

A user session pushed on a meta-question that started from 2 concrete tooling bugs
(hit editing `crates/calm-cli/src/main.rs` in the 2026-07-13 23:34 one-command-
distribution session, commit `8370985`) and expanded into: why does an agent keep
defaulting to native tools over CALM even when CALM covers the task strictly
better, and what actually fixes that (not "try to remember harder"). This is the
distilled result — a live, evidenced self-audit (not introspection alone), cross-
checked against an independent agent's (SWE 1.6 / Windsurf) unprompted reflection
on the same question, filtered hard for fit against CALM's actual architecture and
actual consumer (an LLM agent with structured tool-calling, not a human driver).

Sequel to `docs/superskills/specs/2026-07-13-calm-agent-experience-upgrade.md` —
that spec built the hook + line-numbering foundation this audit stress-tested a
day later, live, in a real session.

Everything below is ranked **evidence first**: shipped-and-tested > confirmed-real-
but-unfixed > design-principle > explicitly-rejected. Reject entries are kept, not
deleted, so a future session doesn't re-propose them without re-deriving why.

---

## Part A — Shipped this session (uncommitted)

Files touched: `.claude/hooks/calm-nudge.sh`, `.claude/hooks/test-calm-nudge.sh`,
`crates/calm-core/src/edit.rs`. `diff_impact` confirmed `aggregate_risk: low` on
all of it. Nothing committed — left for review.

1. **`edit_context` hook false-deny when `path` is omitted.** `EditContextParams.path`
   is optional (only needed to disambiguate a same-named symbol), so the common,
   correct call for a unique symbol name never passes it — but the PreToolUse hook
   can only see the *request*, never which file the server resolved, so it never
   unlocked that file for the next native `Edit`. Fixed by mirroring
   `resolve_symbol_candidates`'s own query (`SELECT path FROM symbols WHERE name = ?`)
   read-only against `.calm/index.db`: exactly 1 row → record it (matches the
   server's own Found-vs-Ambiguous criterion exactly); 0 or >1 rows → stay silent,
   correctly, since the real call didn't resolve either. No Rust change, no
   redeploy. Verified: reverted just the hook and re-ran the test suite to confirm
   it fails pre-fix, passes post-fix.
2. **`Write` branch never checked `is_definitely_unindexed_path`** (unlike `Edit`,
   which got this guard in the 2026-07-13 redesign) — false-nudged on every write
   outside the repo entirely (live-hit: this session's own memory-file writes).
   Fixed with the same guard `Edit` already has.
3. **Read/Grep nudge precision, via real ground truth instead of guessing.** Found
   `file_index` (`.calm/index.db`, `PRIMARY KEY(path)`, 207 rows = `repo_overview
   .total_files`) — exact, instant proof of "does CALM track this exact file,"
   replacing `is_code_file`'s extension-allowlist guess. This fixed a live false
   **negative**, not just precision: `.md` isn't in that allowlist, so a native
   `Read` on `AGENTS.md` itself (293 lines, 13 indexed heading symbols) was never
   nudged all session — found because I did exactly that, minutes after reflecting
   on this exact failure mode, with the rule and the capability both already in
   context. `is_indexed_file()` now takes priority for `Read`/`Grep`; falls back to
   the old path-shape heuristics only when `file_index` can't confirm either way,
   so existing coverage never shrinks.
4. **Shadow-mode `would_deny` logging.** Decision-log JSONL now records, per
   `Read`/`Grep` call, whether `is_indexed_file` confirms this is exactly the case
   a *future* hard gate (mirroring `edit_context`/`diff_impact`) would legitimately
   block — logged, never enforced. Deliberate: measure the real false-positive
   rate across real sessions before ever flipping Read/Grep from advisory to deny
   — Read/Grep fire far more often than Edit, so a bad hard-gate here is far more
   costly than anything else touched today.
5. Two regression tests added to `crates/calm-core/src/edit.rs`:
   `test_replacing_one_function_with_multiple_functions_parses_cleanly` (proves
   `apply_hunks`/`validate_syntax` correctly handle a 1-symbol→N-symbols replace —
   the originally-reported "PARSE_ERROR false positive" could NOT be reproduced as
   a real defect here) and
   `test_insertion_hunk_before_sandwiches_between_leading_doc_comment_and_symbol`
   (proves a real, separate, lower-severity gap — see B1).

Deliberately NOT done this session (explicit user choice, correctly deferred):
Bash `grep`/`find` never got the `file_index` treatment (command-string parsing to
extract a single target path is a harder, separate problem); nothing was flipped
to a real hard `deny` yet.

---

## Part B — Confirmed real gaps, not yet fixed, ranked by cost/value

**B1. Doc-comment-sandwich warning** (low cost, ready to build, no schema change).
`edit_symbol(position="before")` anchors on the symbol's raw `line_start`
(`walk_symbols`, `crates/calm-core/src/indexer/parser.rs:587`), which never
includes a leading `///`/`//!` doc comment (a separate tree-sitter sibling node).
Inserting "before" a documented symbol lands the new code *between* the comment
and the symbol — the comment silently ends up describing the wrong thing. Always
syntactically valid (can't cause `PARSE_ERROR`), so low severity, but real and
reproduced (test in Part A). Cheapest fix found: `insertion_hunk_for` already does
a fresh parse to get `line_start`/`line_end` (`best_live_range`) — a cheap check
("does the line immediately above `line_start` look like a doc-comment for this
language, and is `position == before`?") can attach a warning note to the response,
the same way `edit_lines_impl` already attaches an `ambiguity_note`/"position
warning" for hash-ambiguous ranges. No `ParsedSymbol`/DB schema change needed for
this cheaper version — see B4 for the more invasive real fix.

**B2. `NUDGE_CAP` measures the wrong thing.** It counts *how many times shown*
as a proxy for *lesson learned*, then goes fully silent past the cap — but a
silent nudge that never actually changed behavior is worse than a repeated one,
and the current design can't tell the difference. Part A's `would_deny` +
existing `native_explore`/`calm_explore` counters are the raw material: after a
nudge fires, check whether native usage for that *specific category* trends down
over the next N opportunities, not just whether the raw show-count hit 2. If it
doesn't trend down, that's evidence the soft nudge isn't working — the correct
response is escalation (toward a hard gate, per B — Part A item 4's shadow-mode
data), not silence.

**B3. Audit other heuristic-guess spots in the hook for the same `file_index`
upgrade.** `is_definitely_unindexed_path` (path-shape guessing) is still used
directly for `Edit`'s gate and for the Bash `grep`/`find`/git-target-resolution
paths — Part A's fix only touched `Read`/`Grep`. Each remaining use is a candidate
for the same "guess → verified DB lookup" upgrade; not evaluated one-by-one this
session, flagged for a follow-up pass.

**B4. Doc-comment sandwich, the real fix (not just a warning).** Needs a new
`doc_start_line`-style field: computed at index time in `walk_symbols` (a sibling
walk like `collect_doc_comment_lines` already does, returning a line number
instead of text) → new `symbols` table column (schema migration) → `CandidateRow`
→ `insertion_hunk_for` uses it instead of raw `line_start` for `position="before"`.
Meaningfully bigger than B1 for a cosmetic-severity bug (misplaced doc comment,
never data loss) — do B1 first; only invest here if the warning proves
insufficient in practice.

**B5. Session-overlap warning** (small, narrow version of "multi-agent
awareness" — see Part D for why the broader version was rejected). Existing
infra: `session_context().other_active_sessions` (presence only, today) plus the
cross-process edit lock (already shipped, see `calm-multiclient-concurrency-fixes`
/ `calm-cross-process-edit-race` memory). The real, narrow gap: no warning when
another active session has *recently touched overlapping files/symbols* — a small
enrichment of the existing `other_active_sessions` data, not a new subsystem.

---

## Part C — Design principles distilled (apply to future CALM UX decisions generally, not just these bugs)

1. **Voice beats map, always, measurably.** Interception exactly at the tool-call
   decision point (a hook, or information embedded in a tool's own response)
   reliably changes behavior; static documentation read once does not — regardless
   of clarity or how recently it was read. Proven 3 times live in one conversation
   here, including a case where the rule, the capability, AND a fresh read of the
   relevant memory were all simultaneously in context and the drift still
   happened. The two mechanisms that hit 100% compliance in this whole project
   (`edit_context`, `diff_impact`) are both hard PreToolUse gates, not nudges.
2. **Ground truth beats heuristics, and precision is what unlocks stronger
   enforcement.** Wherever a DB table already has the real answer (`file_index`
   for "is this indexed"), use it instead of guessing from path shape/patterns —
   this is *why* Read/Grep can now be safely considered for a future hard gate
   when they couldn't be before (imprecision was the actual historical blocker,
   not a philosophical objection to gating reads).
3. **False positives cost more than an equal number of missed true positives**
   (trust is loss-averse/asymmetric) — a single confirmed false-deny does more
   damage to an agent's trust in a gate than many correct silent passes build up.
   Any new hard gate should get a shadow-mode measurement period before
   enforcement as *standard practice*, not a case-by-case argument each time.
4. **Read/Grep and Edit are not the same enforcement problem.** Read/Grep on an
   indexed file has no legitimate native-preferred case found anywhere in this
   audit (`source`/`file_overview` are strictly at least as good, verified
   byte-identical to native `Read`) — safe to eventually gate unconditionally.
   Edit genuinely has exceptions (B1/B4's own doc-comment-sandwich gap is a real
   example of `edit_symbol` falling short of what a hand-computed `edit_lines`
   could do) — its existing hard gate correctly has an unlock condition
   (`edit_context` called this session), never blanket denial. Don't generalize
   one policy across both.
5. **Duplicate critical instructions into tool RESPONSES, not just static
   injected docs.** `repo_overview`'s `workflow_guide` field already does this
   well (survives any SessionStart-hook truncation, since it arrives in-band on
   the first real tool call) — the same pattern deserves consideration for other
   critical, easily-truncated guidance, rather than relying on an agent having
   separately, proactively read a long file start-to-finish.
6. **Match the mechanism to the actual consumer and to CALM's actual
   architecture before adopting a generic "AI assistant" feature.** CALM is a
   deterministic Rust/SQLite graph-and-parser engine consumed by an LLM agent
   that is already fluent in structured tool calls — not an ML platform, and not
   serving a human who needs natural language/voice/visual aids. Several
   cross-checked ideas failed this test (Part D) purely because they imported a
   generic "AI assistant" wishlist without checking either fact.

---

## Part D — Explicitly rejected, with reasoning (from cross-checking an independent agent's reflection)

Kept here so a future session doesn't re-propose these without re-deriving why:

- **ML/behavioral pattern-learning personalization** ("learns fen edits hub
  symbols on Mondays," predictive suggestion engines). Conflicts directly with
  CALM's own safety-by-construction stance: `remember`/`recall` is deliberately
  manual, explicit, and auditable specifically so behavior doesn't depend on
  opaque accumulated state nobody can verify. An auto-learning layer reintroduces
  exactly what that design avoided.
- **A natural-language intent layer in front of CALM's tools.** Category error:
  the agent (the consumer) is *already* the natural-language-understanding layer;
  CALM's structured JSON schemas exist precisely so the agent can consume them
  directly with no translation loss. Adding NLP between the agent and the tool
  schema is redundant indirection, not a simplification.
- **Literal voice I/O** (speech-to-text/text-to-speech). Takes the GPS metaphor
  literally instead of structurally — "voice" throughout this whole audit means
  *information arriving at the decision point*, not audio. An agent has no ears.
- **Interactive visual graph UI for the agent.** Right idea, wrong audience — a
  human supervisor reviewing CALM's findings might want this; the agent consumes
  text/JSON, and a visual graph is, if anything, more "map" than "voice" for that
  consumer.
- **Cross-repo dependency/vulnerability scanning.** Already served by a separate,
  purpose-built tool available in this same environment (a semgrep supply-chain
  plugin) — CALM re-implementing it would be scope creep and duplication, not a
  gap.
- **Fake-precision confidence numbers** (e.g. "85% confidence safe to edit").
  Would be a *regression* from the current honest categorical `low`/`medium`/
  `high` + a `reasons[]` array — manufactured precision without a real
  statistical basis is worse than an honest, coarser signal.

## Part E — Already exists; corrected the record

Raised as gaps by the cross-checked reflection, but already shipped — listed so
the record is accurate for whoever reads this next:

- **Incremental indexing** — shipped, ADR-0007, default on since 2026-07-13.
- **Multi-level caching** — `edges_etag`/`if_none_match` conditional fetch on
  `edit_context`, `source.etag`, plus DB-backed embedding/chunk vector caches
  already exist.
- **"Code review assistant" / PR risk summary with reviewer suggestions** — this
  is `diff_impact` (`aggregate_risk`, `affected_symbols`, `suggested_reviewers`),
  not a missing feature.
- **Multi-agent session awareness** — `session_context().other_active_sessions`
  plus the cross-process edit lock already exist (see B5 for the real, narrower
  remaining gap: overlap warning, not "coordination" from scratch).

---

## Suggested next steps (not started, awaiting an explicit go-ahead each)

## Status update — 2026-07-14, same session

Ran a lightweight `audit-design` pass over Part B (pre-mortem + L1-L7 scan, not the
full spec/frontmatter ceremony — no formal spec doc existed for this backlog).
Result: B1 and B5 cleared the Complexity Gate outright (no schema, no persistent
state, no auth) and were implemented directly; B2 came back PASS WITH FLAGS,
scoped down to avoid the real risk found (auto-escalating to enforcement would
have skipped Part A's own agreed shadow-mode-before-enforcement step); B4 came
back PASS WITH FLAGS but recommended deferred (real risk found: Python's docstring
convention — a string literal INSIDE the function body — doesn't fit the same
"doc_start_line precedes the symbol" model Rust/JS doc comments do, so a single
generic field would need real per-language design work first, not just a schema
migration).

**Implemented and tested this session** (uncommitted — `.claude/hooks/calm-nudge.sh`,
`crates/calm-server/src/tools/edit.rs`, `crates/calm-server/src/tools/recover.rs`,
`crates/calm-server/src/tools.rs`):

- **B1** — `insertion_hunk_for` now attaches a warning to the response `note`
  when `position="before"` would sandwich new content between a leading doc
  comment and its target symbol (reuses the already-extracted `docstring`, no
  schema change). 2 new tests (`edit_symbol_position_before_warns_when_symbol_
  has_leading_doc_comment`, `..._omits_warning_when_no_leading_doc_comment`).
- **B3** — audited every `is_definitely_unindexed_path` call site: `Edit`'s hard
  gate and `Write`'s advisory nudge both now check `is_indexed_file` (the
  `file_index` ground truth from Part A) FIRST, closing a real safety gap — a
  path-shape false positive (e.g. a source file living under a literally-named
  `target/` directory) could previously skip the mandatory `edit_context`
  requirement entirely. Live-verified via direct hook invocations, no regression.
- **B5** — `session_context()` gained `overlapping_files: Vec<String>` — purely
  derived from `explored_files` (untruncated) and `other_active_sessions[].
  last_touched_file`, zero new state/locks. Informational only, like
  `possibly_stuck`. New test: `session_context_reports_overlapping_files_with_
  other_active_sessions`.
- **B2** — `nudge()` no longer freezes its own counter at `NUDGE_CAP` and goes
  silent forever; it keeps counting and resurfaces every `TALLY_EVERY`-th
  occurrence with the real count ("occurrence #6 of the same situation..."),
  mirroring what `nudge_or_tally` already did for native-explore. Verified via
  8 consecutive calls (2 full messages, 3 silent, 1 resurfaced at #6) and
  confirmed via the decision log that every decision stayed advisory
  (`nudge`/`allow_nudge_capped`/`nudge_repeat_tally`) — never `deny`.

All: `cargo test -p calm-server --lib` 208/208, `cargo test -p calm-core --lib`
701/701 (+11 ignored), `.claude/hooks/test-calm-nudge.sh` PASS.

**Still open, deliberately not started**: B4 (needs per-language design first,
see above).

1. B1 (doc-comment-sandwich warning) — cheapest, no schema change, ready to
   implement whenever wanted.
2. Let Part A's shadow-mode (`would_deny`) run across a few real sessions before
   any decision on flipping Read/Grep to a real hard gate.
3. B3's heuristic audit (other `is_definitely_unindexed_path` call sites) — scope
   not yet sized, worth a dedicated look before committing to it.
4. B5 (session-overlap warning) — small, additive, low risk whenever picked up.
