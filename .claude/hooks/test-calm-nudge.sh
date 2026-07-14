#!/usr/bin/env bash
# Regression test for calm-nudge.sh's per-file native-Edit-block gate
# (docs/superskills/plans/2026-07-13-calm-agent-experience-upgrade.md,
# Task C1). No test framework exists for this project's shell hooks, so
# this is a plain bash script: run it directly, it prints PASS or exits
# non-zero with a FAIL message naming which assertion broke.
set -euo pipefail
cd "$(dirname "$0")/../.."

session_id_test="test-$$"
state_dir=".calm/.hook-state"
state_file="$state_dir/${session_id_test}.json"
rm -f "$state_file"
cleanup() { rm -f "$state_file"; }
trap cleanup EXIT

run_hook() {
  # $1=tool_name  $2=file_path (Edit) or path (edit_context)  $3=symbol (edit_context)
  jq -nc --arg session "$session_id_test" \
    --arg tool "$1" --arg path "${2:-}" --arg symbol "${3:-}" \
    '{session_id: $session, tool_name: $tool, tool_input: {file_path: $path, path: $path, symbol: $symbol}}' \
    | bash .claude/hooks/calm-nudge.sh
}

fail() {
  echo "FAIL: $1"
  exit 1
}

# 1. edit_context on a.rs, then native Edit on a.rs -> ALLOWED (no deny)
run_hook "mcp__calm__edit_context" "crates/calm-core/src/a.rs" "SomeSymbol" >/dev/null
out=$(run_hook "Edit" "crates/calm-core/src/a.rs")
if echo "$out" | jq -e '.hookSpecificOutput.permissionDecision == "deny"' >/dev/null 2>&1; then
  fail "expected allow for a.rs after its own edit_context, got deny: $out"
fi

# 2. native Edit on a DIFFERENT file b.rs, same session, no edit_context for
#    it -> DENIED even though edit_context was called for a.rs above (this
#    is the exact regression Task C1 fixes: a session-wide unlock would
#    have allowed this).
out=$(run_hook "Edit" "crates/calm-core/src/b.rs")
if ! echo "$out" | jq -e '.hookSpecificOutput.permissionDecision == "deny"' >/dev/null 2>&1; then
  fail "expected deny for b.rs (never edit_context'd this session), got: $out"
fi
reason=$(echo "$out" | jq -r '.hookSpecificOutput.permissionDecisionReason')
if [[ "$reason" != *"b.rs"* ]]; then
  fail "deny reason must name b.rs specifically, got: $reason"
fi
if [[ "$reason" != *"other file(s), but not this one"* ]]; then
  fail "deny reason must explain that edit_context was satisfied for OTHER files but not this one (message-wording regression), got: $reason"
fi

# 3. edit_context on b.rs now unlocks it too (per-file, additive, not
#    replacing a.rs's earlier unlock).
run_hook "mcp__calm__edit_context" "crates/calm-core/src/b.rs" "OtherSymbol" >/dev/null
out=$(run_hook "Edit" "crates/calm-core/src/b.rs")
if echo "$out" | jq -e '.hookSpecificOutput.permissionDecision == "deny"' >/dev/null 2>&1; then
  fail "expected allow for b.rs after its own edit_context, got deny: $out"
fi
out=$(run_hook "Edit" "crates/calm-core/src/a.rs")
if echo "$out" | jq -e '.hookSpecificOutput.permissionDecision == "deny"' >/dev/null 2>&1; then
  fail "expected a.rs to STILL be allowed after b.rs was separately unlocked (per-file state must be additive), got deny: $out"
fi

# 4. edit_context(symbol) with NO `path` (the normal way to call it for a
#    globally-unique symbol name -- see EditContextParams' own doc comment)
#    must still unlock that file for a later native Edit, by falling back to
#    a read-only lookup against .calm/index.db mirroring resolve_symbol's own
#    "exactly one row named X" criterion. Uses a real symbol from this repo
#    (crates/calm-server/src/tools/common.rs::resolve_symbol_candidates) so
#    the DB lookup has something real to find -- this is the exact regression
#    a prior session hit editing crates/calm-cli/src/main.rs (see memory
#    calm-two-tooling-bugs-root-cause-2026-07-14).
run_hook_symbol_only() {
  jq -nc --arg session "$session_id_test" --arg tool "$1" --arg symbol "$2" \
    '{session_id: $session, tool_name: $tool, tool_input: {symbol: $symbol}}' \
    | bash .claude/hooks/calm-nudge.sh
}
if [ -f .calm/index.db ]; then
  run_hook_symbol_only "mcp__calm__edit_context" "resolve_symbol_candidates" >/dev/null
  out=$(run_hook "Edit" "crates/calm-server/src/tools/common.rs")
  if echo "$out" | jq -e '.hookSpecificOutput.permissionDecision == "deny"' >/dev/null 2>&1; then
    fail "expected allow for common.rs after edit_context(symbol-only) resolved it via the DB fallback, got deny: $out"
  fi

  # 5. A symbol name with more than one row in the DB (genuinely ambiguous,
  #    or the real tool call would itself return `ambiguous`) must NOT be
  #    guessed at -- no file gets unlocked, so a same-session native Edit on
  #    an unrelated, never-edit_context'd file still denies as usual.
  multi_name=$(sqlite3 -readonly -separator '|' .calm/index.db \
    "SELECT name FROM symbols GROUP BY name HAVING COUNT(*) > 1 LIMIT 1;" 2>/dev/null || true)
  if [ -n "$multi_name" ]; then
    run_hook_symbol_only "mcp__calm__edit_context" "$multi_name" >/dev/null
    out=$(run_hook "Edit" "crates/calm-core/src/never_edit_contexted.rs")
    if ! echo "$out" | jq -e '.hookSpecificOutput.permissionDecision == "deny"' >/dev/null 2>&1; then
      fail "expected deny: an ambiguous symbol name (>1 row) must not unlock any file, got: $out"
    fi
  fi
fi

echo "PASS"
