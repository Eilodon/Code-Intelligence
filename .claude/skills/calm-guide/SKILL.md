---
name: calm-guide
description: >-
  Use before searching, reading, or editing files in this repo (or any repo
  with a "calm" MCP server), or whenever unsure which CALM tool fits — a
  self-triggerable orientation to the CALM Stage 1-8 tool workflow
  (repo_overview → locate → source → callers → edit_context →
  edit_symbol/edit_lines → diff_impact → session_context). Also invoke
  manually as /calm-guide for a mid-session refresher, or whenever
  AGENTS.md's SessionStart auto-injection didn't happen this turn (some MCP
  clients — and non-Claude-Code sessions entirely — never get it).
metadata:
  version: "1.0"
  origin: >-
    Added 2026-07-14 (F6, docs/superskills/specs/2026-07-14-calm-agent-
    experience-round2-fixes.md). Companion to the calm_workflow MCP Prompt
    added the same day in crates/calm-server/src/tools.rs — same content,
    two trigger surfaces: this Skill for self-/manual-trigger inside Claude
    Code, the MCP Prompt for any other client (Cursor, plain VS Code MCP,
    etc.) where this Skill file is never loaded.
---

# CALM MCP workflow — quick trigger

**Read `AGENTS.md` at the project root in full now.** It is the single
source of truth for this workflow — this skill exists to make sure you
actually reach for it at the right moment, not to hold a second copy of its
content that would silently drift out of sync with it over time. If
`AGENTS.md` doesn't exist at the project root, this isn't a CALM-instrumented
repo (or you're in the wrong directory) — stop here, this skill doesn't
apply.

## If you only have room for the short version right now

1. **Orient** — `repo_overview()` always first, then `hotspots()`/
   `fitness_report()` as needed.
2. **Locate** — `locate(query)` (search + file_overview + symbol_info in one
   call) or `search(query, kind=...)`.
3. **Inspect** — `source(symbol)` for a symbol-precise read, or
   `understand(symbol)` for locate+source+callers together.
4. **Trace** — `callers(symbol)`, `callees(symbol)`, `path(from, to)`,
   `dependencies(path)`.
5. **Pre-Edit** — `edit_context(symbol)`: mandatory before touching any real
   code symbol, no exceptions. (A Markdown/text heading is the one
   exception — it never carries a blast radius to check.)
6. **Edit** — `edit_symbol(...)`/`edit_lines(...)`: CALM's own write path,
   hash-verified, risk-gated, reindexes immediately. Fall back to native
   `Edit`/`Write` only for a brand-new file or a path CALM doesn't index
   (dotdirs, `target/`, `node_modules/`, `dist/`, `build/`, etc.).
7. **Verify** — `diff_impact(staged=true)`: mandatory before any commit or
   push, no exceptions.
8. **Recover** — `session_context()` after 10+ calls without progress;
   `remember(topic, content)`/`recall(topic)` for durable cross-session
   notes.

Steps 5 and 7 are the only hard-enforced ones under Claude Code (a hook
denies the native equivalent without them) — the rest is strongly
recommended, not blocking. Every signal, edge case, and the preset/toolset
reference lives in `AGENTS.md` — go read it in full if you haven't this
session.
