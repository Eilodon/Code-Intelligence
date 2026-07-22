---
title: Session-start orientation gate, pending-diff_impact persistence, and widened edit-context gate
date: 2026-07-22
SPEC_APPROVED: true
SPEC_ESCALATION: false
---

## Problem

`CalmServer::get_info().with_instructions(...)` (`crates/calm-server/src/tools.rs`)
already pushes a "call `calm_workflow` first" pointer to every MCP client on
the `initialize` handshake — confirmed live, not assumed: a Claude Code
session working in this repo received that exact string as an "MCP Server
Instructions" system block, and then proceeded to spend an entire
investigation phase calling native `Grep`/`Read` instead of any `mcp__calm__*`
tool anyway. The push reached the model correctly; nothing about receiving it
compelled acting on it. Advisory instructions — `get_info().with_instructions`,
tool descriptions, AGENTS.md, the `calm_workflow` Prompt — have zero technical
consequence for an agent that ignores them.

Two separate, real gates already exist and *are* enforced, but each has a gap:

- `edit_lines`/`edit_symbol`'s write-time risk gate (`tools/edit.rs`,
  `EDIT_CONTEXT_REQUIRED`/`CONFIRM_REQUIRED`/`REASON_NOT_GROUNDED`) is
  protocol-level — the tool's own JSON-RPC error response, universal across
  every MCP client, not a Claude-Code hook. But it only fires for a symbol
  that's a hub, high-caller, or uncertain-zero-caller — a plain low-risk edit
  passes through with zero gate on every client, including Claude Code.
- `session_context.pending_diff_impact` already tracks files written this
  session with no `diff_impact` run on them since (`written_files_snapshot`,
  `tools/common.rs`) — but it's only surfaced when the client proactively
  calls `session_context`, which nothing forces either.

The one genuinely universal lever nothing had used yet: `CalmServer::call_tool`
(`tools.rs`, the `rmcp::ServerHandler::call_tool` implementation) is the single
dispatch point every `tools/call` request from *any* MCP client — Claude Code,
Cursor, Windsurf, Codex CLI, or a hand-rolled client — passes through before
reaching any individual tool handler. Nothing at this chokepoint was doing
anything besides tracing-span setup before this change.

## Design

### 1. Session-start orientation gate (`Config.orientation`)

New `OrientationConfig`/`OrientationMode` in `calm-core/src/config.rs`, own
top-level `Config.orientation` field (matches this file's existing per-domain
struct convention — `RustConfig`, `HotspotsConfig`, `EditConfig`, etc. — not a
generic catch-all "gates" bucket).

```
mode: "off" | "inject" (default) | "block"
remind_pending_diff_impact: bool (default true)
```

Implemented in `call_tool` as pre-call (mode) and post-call
(`remind_pending_diff_impact`) middleware around the existing
`self.tool_router.call(...)` delegation:

- **`inject`** — the first non-orientation-adjacent tool call
  (`repo_overview`/`indexing_status`/`session_context` are the adjacent set —
  calling one of them already *is* the orientation) still runs normally and
  returns its real result; the server merges a compact orientation summary
  (`_calm_orientation`) into that same response as an extra content block.
  Never fails the call, never adds a round trip, cannot be "missed" by an
  agent that doesn't read `instructions` — it's embedded in the very response
  the agent already reads to get its answer.
- **`block`** — refuses the same first call outright with
  `{"error":{"code":"ORIENTATION_REQUIRED",...}}` until `repo_overview` has
  actually been called this session.
- **`off`** — pre-2026-07-22 behavior, push only.

**Pre-mortem finding that shaped `block`'s exact semantics**: `repo_overview`
lives only in the `orient` toolset (`crates/calm-server/src/tools/orient.rs`,
`orient_tool_router()`). `resolve_preset`/`toolset_tools`
(`tools/common.rs`) confirm a real, legitimately-configurable preset —
`--preset "security"` alone, or any composed spec subtracting `orient`
(`"full,-orient"`) — produces a `tool_router` with **no** orientation-adjacent
tool registered at all. A literal `block` gated by tool name, with no
awareness of what's actually in the active router, would refuse every single
tool call for the rest of that connection's life with no escape hatch —
verified as a real (not hypothetical) deadlock via
`orientation_escape_hatch_missing_for_security_only_preset` and
`effective_orientation_mode_downgrades_block_to_inject_without_escape_hatch`
(`tools.rs` tests), and live end-to-end against a running `calm serve`
process (`--preset security` + `mode: "block"` + first call `scan_text` →
succeeds with `_calm_orientation` injected, not `ORIENTATION_REQUIRED`).
`effective_orientation_mode()` (`tools/common.rs`) is the fix:
`orientation_escape_hatch_available()` checks the live `tool_router` for any
adjacent tool, and downgrades `Block` to `Inject` when none exists — `block`
can never deadlock a session, by construction, not by operator discipline.

Config precedent checked before deciding the struct shape (not guessed):
`EditConfig.elicit_hub_confirm` (existing, `crates/calm-core/src/config.rs`)
is the closest sibling — same "off by default, protocol-level gate toggle"
shape, its own small domain struct rather than folded into something generic.
`orientation` follows the same pattern instead of a shared `session_gate.*`
block that would exist nowhere else in this file's convention.

### 2. Pending-`diff_impact` reminder persistence

Same `call_tool` middleware, post-call: while `written_files_snapshot()` is
non-empty and the tool just called wasn't `diff_impact`, every response (not
only a `session_context` call) carries a `_calm_pending_diff_impact` reminder
block, gated on `orientation.remind_pending_diff_impact` (default `true`).

Explicitly **not** a hard gate, and structurally can't become one on any
client including Claude Code: an MCP server has no protocol-level visibility
into a client's own native Bash/Edit tool calls (`git commit` in particular).
Claude Code's own `diff_impact`-before-commit enforcement
(`.claude/hooks/calm-nudge.sh`, this repo's internal dogfooding tool; or the
generic `calm init --hooks=enforce` scaffold for other projects) exists
precisely because it's a client-side hook watching Bash — a capability this
change does not, and cannot, replicate at the protocol layer. This piece only
makes the already-real `pending_diff_impact` signal harder to miss, uniformly,
on every client.

### 3. Widened edit-context gate (`Config.edit.always_require_edit_context`)

New bool on the existing `EditConfig`, default `false`. Threaded into
`edit_lines_impl_gated`'s existing gate condition
(`tools/edit.rs`) as an additional disjunct alongside
`hub_hit`/`risk == "high"`/`uncertain_zero_caller.is_some()` — when set, every
`edit_lines`/`edit_symbol` call requires `edit_context` this session
regardless of computed risk, using the exact same
`EDIT_CONTEXT_REQUIRED`/`CONFIRM_REQUIRED`/`REASON_NOT_GROUNDED` protocol-level
responses the hub/high-risk path already returns — universal across clients
for the same reason that existing gate already is.

## Implementation-review findings applied

- **`for_connection` field lifecycle** (`tools/common.rs`): the new
  `oriented: Arc<AtomicBool>` field on `CalmServer` is explicitly reset to a
  fresh `Arc` inside `for_connection()`'s literal field list, the same
  treatment `session_log` already gets — NOT inherited via `..self.clone()`
  the way `phase`/`coverage`/etc. correctly are. Getting this wrong would
  silently share one gate flag across every connection on a shared daemon
  (`calm connect`): the first client to ever connect would flip it to `true`
  and every later connection would see the gate as already satisfied.
  Regression-tested directly:
  `for_connection_gives_oriented_flag_fresh_state_per_connection`.
- **`tools/list` is unaffected** — `list_tools` and `call_tool`
  (`tools.rs`) are separate `ServerHandler` methods; introspection never
  passes through this middleware.
- **Concurrency** — `rmcp` dispatches tool calls concurrently per connection
  (see `CalmServer::edit_lock`'s own doc comment); `oriented` is a plain
  `AtomicBool` read/store, so the worst case under a true race is one
  redundant orientation injection, never an incorrect gate decision.

## What this does not attempt

No change here makes `diff_impact`-before-commit or edit-context-before-
native-Edit a hard gate on any client other than Claude Code (via its
existing hook layer) — that specific bypass (an agent using the client's own
native file/shell tools instead of CALM's) is not closeable at the MCP
protocol layer, on any client, full stop. This spec's 3 pieces are the
complete set of what *is* closeable there.
