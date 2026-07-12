#!/usr/bin/env bash
# Antigravity port of .claude/hooks/calm-nudge.sh's two HARD-enforced gates
# (Claude Code's PreToolUse "deny" mechanism) — nothing else from that
# script is ported here (the softer Read/Grep nudges are Claude-Code-only
# for now; see AGENTS.md Stages 5 and 7 for what these two gates encode):
#
#   - Stage 5: a native edit tool must not touch a code file until
#     mcp__calm__edit_context has been called at least once this session.
#   - Stage 7: a real `git commit`/`git push` must not run until
#     mcp__calm__diff_impact has been called since the last write.
#
# DRAFT / UNVERIFIED (2026-07-12) — read before trusting the deny path:
# Antigravity's hooks.json contract (PreToolUse/PostToolUse, matcher+command
# shape, decision:allow|deny|ask stdout) is confirmed from public
# blog/SDK-README sources (danicat.dev "Mastering Hooks in Coding Agents";
# google-antigravity/antigravity-sdk-python hooks README). What is NOT
# confirmed anywhere public as of this writing:
#   1. The exact stdin field for tool *identity* — assumed `.toolCall.name`
#      below (sibling of the documented `.toolCall.args`), with fallbacks.
#   2. How an MCP-server tool (e.g. calm's `edit_context`) is spelled in
#      that field — Claude Code uses `mcp__calm__edit_context`; Antigravity's
#      exact format is unknown, so tool_name matching below is done by
#      SUBSTRING, not exact match, specifically to survive whichever
#      naming convention it turns out to be (mcp__calm__edit_context,
#      calm.edit_context, calm_edit_context, ...).
#   3. The exact arg key for a file path on write_to_file/replace_file_
#      content/multi_replace_file_content — several candidates are tried.
# Run calm-guard-probe.sh in a real Antigravity session first (see its
# header) and tighten the lookups below against real payloads before
# relying on this for anything beyond local dogfooding. Until then, every
# `deny` here is a best-effort guess dressed up as a hard gate — the
# `note` field in decisions.jsonl records exactly which fallback path each
# lookup took, specifically so a stale/wrong assumption is visible in the
# log instead of silently mismatching forever.
set -uo pipefail

hook_dir="$(dirname "${BASH_SOURCE[0]}")"
state_dir="${hook_dir}/.state"
mkdir -p "$state_dir" 2>/dev/null || true

input=$(cat)
jqr() { jq -r "$1" 2>/dev/null <<<"$input"; }

# --- tool identity: try the documented sibling of toolCall.args first,
# then a few plausible alternates other agent hook systems use.
tool_name=""
name_field_used=""
for path in '.toolCall.name' '.tool_name' '.toolName' '.name' '.tool.name'; do
  v=$(jqr "$path")
  if [ -n "$v" ] && [ "$v" != "null" ]; then
    tool_name="$v"
    name_field_used="$path"
    break
  fi
done

# --- session key: transcriptPath is the one documented session-scoped
# field (Antigravity's equivalent of Claude Code's session_id). Hash it so
# it's filesystem-safe; fall back to a fixed bucket (still correct within a
# single continuous session, just not perfectly session-isolated) if the
# field is ever absent.
transcript_path=$(jqr '.transcriptPath')
if [ -n "$transcript_path" ] && [ "$transcript_path" != "null" ]; then
  session_key=$(printf '%s' "$transcript_path" | cksum | awk '{print $1}')
else
  session_key="unknown"
fi
state_file="${state_dir}/${session_key}.json"
find "$state_dir" -maxdepth 1 -name '*.json' -mtime +1 -delete 2>/dev/null || true

state='{}'
[ -f "$state_file" ] && state=$(cat "$state_file" 2>/dev/null || echo '{}')
edit_context_called=$(jq -r '.edit_context_called // false' <<<"$state" 2>/dev/null || echo false)
needs_diff_impact=$(jq -r '.needs_diff_impact // false' <<<"$state" 2>/dev/null || echo false)

decision_log="${state_dir}/decisions.jsonl"
decision_log_cap=5000
decision="allow"
detail=""
log_decision() {
  jq -nc --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --arg tool "$tool_name" \
    --arg decision "$decision" --arg detail "$detail" --arg name_field "$name_field_used" \
    '{ts:$ts, tool_name:$tool, decision:$decision, detail:$detail, name_field_used:$name_field}' \
    >>"$decision_log" 2>/dev/null || true
  if [ -f "$decision_log" ]; then
    lines=$(wc -l <"$decision_log" 2>/dev/null || echo 0)
    if [ "${lines:-0}" -gt "$decision_log_cap" ]; then
      tail -n "$decision_log_cap" "$decision_log" >"$decision_log.tmp" 2>/dev/null \
        && mv "$decision_log.tmp" "$decision_log" 2>/dev/null || true
    fi
  fi
}
trap log_decision EXIT

save_state() {
  jq -n --argjson ec "$1" --argjson nd "$2" '{edit_context_called:$ec, needs_diff_impact:$nd}' \
    >"$state_file" 2>/dev/null || true
}

deny() {
  decision="deny"; detail="$1"
  jq -n --arg reason "$1" '{decision:"deny", reason:$reason}'
  exit 0
}

is_code_file() {
  case "$1" in
    *.py | *.ts | *.tsx | *.js | *.jsx | *.mjs | *.cjs | *.java | *.rs | *.go | \
    *.c | *.h | *.cpp | *.cc | *.cxx | *.hpp | *.hxx | *.cs | *.rb | *.php | \
    *.kt | *.kts | *.swift | *.sh | *.bash)
      return 0 ;;
    *) return 1 ;;
  esac
}

# Same segment-splitting approach as calm-nudge.sh's is_real_git_commit_or_push
# — ported near-verbatim since it's pure bash, not Claude-Code-specific.
is_real_git_commit_or_push() {
  local cmd="$1" segment first tok i extra
  extra=$(grep -oE '\$\([^()]*\)|`[^`]*`' <<<"$cmd")
  extra=$(sed -E 's/^\$\(//; s/\)$//; s/^`//; s/`$//' <<<"$extra")
  while IFS= read -r segment; do
    [ -z "$segment" ] && continue
    segment="${segment#"${segment%%[![:space:]]*}"}"
    while [[ "$segment" =~ ^[A-Za-z_][A-Za-z0-9_]*=[^[:space:]]*[[:space:]]+(.*)$ ]]; do
      segment="${BASH_REMATCH[1]}"
    done
    segment="${segment#sudo }"; segment="${segment#command }"; segment="${segment#exec }"
    read -r -a tokens <<<"$segment"
    first="${tokens[0]:-}"
    [[ "$first" == "git" || "$first" == */git ]] || continue
    i=1
    while [ "$i" -lt "${#tokens[@]}" ]; do
      tok="${tokens[$i]}"
      case "$tok" in
        -C | --git-dir | --work-tree | -c) i=$((i + 2)) ;;
        -*) i=$((i + 1)) ;;
        commit | push) return 0 ;;
        *) break ;;
      esac
    done
  done < <(printf '%s\n%s\n' "$cmd" "$extra" | sed -E 's/&&|\|\||[;|]/\n/g')
  return 1
}

# --- MCP write/reset tracking, by SUBSTRING (see header point 2) ---
case "$tool_name" in
  *edit_context*)
    detail="state:edit_context_called=true"
    save_state true "$needs_diff_impact"
    printf '{"decision": "allow"}\n'
    exit 0
    ;;
  *diff_impact*)
    detail="state:needs_diff_impact=false"
    save_state "$edit_context_called" false
    printf '{"decision": "allow"}\n'
    exit 0
    ;;
  *edit_lines* | *edit_symbol*)
    detail="state:needs_diff_impact=true(mcp-write)"
    save_state "$edit_context_called" true
    printf '{"decision": "allow"}\n'
    exit 0
    ;;
esac

# --- native edit tools (Stage 5 gate) ---
case "$tool_name" in
  write_to_file | replace_file_content | multi_replace_file_content)
    file_path=""
    for path in '.toolCall.args.TargetFile' '.toolCall.args.file_path' \
                '.toolCall.args.path' '.toolCall.args.Path' '.toolCall.args.target_file'; do
      v=$(jqr "$path")
      if [ -n "$v" ] && [ "$v" != "null" ]; then file_path="$v"; break; fi
    done
    detail="state:needs_diff_impact=true(native-write)"
    save_state "$edit_context_called" true
    if [ -n "$file_path" ] && is_code_file "$file_path" && [ "$edit_context_called" != "true" ]; then
      deny "MANDATORY per AGENTS.md Stage 5 (ported) — call the CALM edit_context MCP tool for the symbol in $file_path before this edit, then retry. If this hook fired on the wrong field, check .agents/hooks/probe.log — tool identity was read from '${name_field_used:-<none matched>}'."
    fi
    printf '{"decision": "allow"}\n'
    ;;
  run_command)
    cmd=""
    for path in '.toolCall.args.CommandLine' '.toolCall.args.command' '.toolCall.args.cmd'; do
      v=$(jqr "$path")
      if [ -n "$v" ] && [ "$v" != "null" ]; then cmd="$v"; break; fi
    done
    if [ -n "$cmd" ] && is_real_git_commit_or_push "$cmd" && [ "$needs_diff_impact" = "true" ]; then
      deny "MANDATORY per AGENTS.md Stage 7 (ported) — call the CALM diff_impact MCP tool before this commit/push, never skip. Files changed since the last diff_impact check."
    fi
    printf '{"decision": "allow"}\n'
    ;;
  *)
    printf '{"decision": "allow"}\n'
    ;;
esac
