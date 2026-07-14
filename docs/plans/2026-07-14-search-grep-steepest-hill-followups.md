---
title: Search/grep "steepest hill" — implemented fixes + honest follow-up scope
date: 2026-07-14
status: 3 of 5 recommendations implemented + tested; 2 documented, not built (reasons below)
---

## Context

A user-supplied analysis argued search/grep is the hardest tool-preference to shift toward an
MCP alternative — deeper pretraining habit than Edit/Write, 10-40x higher call frequency, and
lower per-call "caution" since it's read-only. This session's own hook-state data confirmed the
symptom (`native_explore: 46` vs `calm_explore: 1` for one real session) and traced it to real,
fixable mechanics, not a vague "model drift."

## Implemented this session (`.claude/hooks/calm-nudge.sh`, `AGENTS.md`)

1. **Conditional enforcement, not blanket.** `Grep`/Bash-grep/`Read` now only nudge when CALM
   genuinely has leverage — multi-file/repo-wide scope, or a symbol-shaped query (`locate` beats
   a text match even in one file), or a file long enough that `source(symbol)` actually saves
   something. A single-file grep for a real regex/free-text phrase, or a short-file Read, is left
   silent — native has no disadvantage there, and nudging it anyway is the exact false-positive
   class that erodes trust in every other nudge (this hook's own header, lever #1, already
   established that principle for the Edit gate; this extends it to search/grep). `would_deny`
   shadow-mode tracking was moved inside the same conditional, so a future hard-gate decision
   would be evaluated against the right population, not blanket traffic.
2. **`TOOL_DISCOVERY_HINT`.** Every "prefer mcp__calm__X" message (nudges AND the two hard-deny
   paths, `edit_context`/`diff_impact`) now notes that some MCP clients defer tool schemas until
   an explicit discovery step — found live this session: `mcp__calm__search`/`locate` were never
   loaded (0 calls) despite dozens of grep-shaped needs, because the message previously assumed
   they were already callable. Deliberately client-agnostic wording (doesn't name `ToolSearch`
   specifically) so it's correct in a client that loads every tool upfront too.
3. **`AGENTS.md`** banner + Mandatory Rule #4 updated with the same reasoning, so any agent/client
   reading the doc (not just this session) gets the context.

All three are testable and tested: `.claude/hooks/test-calm-nudge.sh` grew from 5 to 13
assertions, including live checks against real repo files (silence on a 34-line file, nudge on a
long one; silence on a single-file grep, nudge on `-r`/no-path). One real bug was caught by these
tests before merge: one of two identical-looking `deny()` call sites had the hint appended, the
other didn't (different indentation made a `replace_all` silently miss one) — the tests, not
eyeballing, caught it.

## NOT implemented this session — scope decisions, not oversights

**PostToolUse just-in-time nudge.** The article's strongest untried idea: attach a grounded
nudge to the *result* of a native Grep ("this spanned N files — next time search() returns this
in one ranked call") instead of a generic upfront reminder. Not built because: (a) this project
has zero PostToolUse hooks today (`settings.json` only wires `SessionStart`/`PreToolUse`) — this
would be new infrastructure, not an extension of an existing one; (b) I don't have a verified
spec for what Claude Code's PostToolUse payload actually contains for a `Grep` tool call (file
paths / match counts in a parseable shape, vs. opaque rendered text) — shipping a hook that
silently no-ops against the real schema would be worse than not shipping it, and this session's
own operating principle throughout (re-verify against real code/behavior, don't trust assumption)
argues against guessing here. **Next step if picked up:** dump a real PostToolUse Grep event's
JSON (`jq . >> /tmp/posttooluse-dump.jsonl` in a throwaway hook) to see the actual shape before
writing logic against it.

**MCP-Bench-style labeled eval set.** The article's measurement fix: don't score "native vs CALM"
as one ratio: build a small set of real queries labeled "which tool should win here," and measure
compliance only against that set. Not built because it's a genuinely separate, ongoing artifact
(a benchmark, like `benchmarks/b10`/`b11` already in this repo), not a hook change, and doing it
credibly means hand-labeling real queries from real sessions — collecting that corpus is itself
a multi-session effort. **Concrete starting point for a future session:** the `decisions.jsonl`
audit trail `calm-nudge.sh` already writes (`.calm/.hook-state/decisions.jsonl`, capped at 5000
lines, `would_deny` field included) is exactly the raw material this eval set would be built
from — every real native Grep/Read this project's sessions ever made, with the tool's own
after-the-fact judgment (`would_deny`) attached. A future pass could sample that log, hand-verify
a subset of `would_deny` calls against what the actual right tool call would have been, and turn
disagreements into the labeled set the article describes.

## Why not "fix the model" instead

Explicitly out of scope per this session's own framing: the user asked for changes on CALM's
side that generalize across every agent/environment/user, not session-specific behavioral
correction for one Claude instance. Every change above is either a hook (runs identically
regardless of which model/agent is connected) or a doc (`AGENTS.md`, read by any agent this repo
is used with) — nothing here depends on this specific session's memory or this specific model.
