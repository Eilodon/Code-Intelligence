#!/usr/bin/env bash
# RUN THIS FIRST, BEFORE TRUSTING calm-guard.sh's deny logic.
#
# Antigravity's hooks.json contract is only publicly documented at the
# surface level (event names PreToolUse/PostToolUse, matcher+command shape,
# and that stdin carries "toolCall.args", "workspacePaths", "transcriptPath"
# — see .agents/hooks/calm-guard.sh's header for sources). The exact field
# name for the tool's *identity* (assumed toolCall.name below) and how an
# MCP-server tool like calm's "edit_context" is spelled in that field
# (mcp__calm__edit_context? calm.edit_context? calm_edit_context? something
# else?) are NOT confirmed anywhere in public docs as of 2026-07-12 — every
# source found was a blog/SDK-README paraphrase, never a full schema dump.
#
# This probe hook does nothing but log the raw stdin JSON for every tool
# call, so you can point it at yourself, trigger a few real tool calls
# (including at least one CALM MCP tool, e.g. mcp__calm__edit_context) in a
# live Antigravity session, then read .agents/hooks/probe.log and confirm/
# correct the field names calm-guard.sh assumes — before wiring the deny
# hook into .agents/hooks.json for real. Always "allow"s — never blocks
# anything, safe to leave attached during this discovery phase.
set -uo pipefail

log_dir="$(dirname "${BASH_SOURCE[0]}")"
mkdir -p "$log_dir" 2>/dev/null || true
probe_log="${log_dir}/probe.log"

# Cheap unbounded-growth guard — this is a discovery scaffold meant to run
# for a short calibration session, not forever; truncate instead of
# rotating (nothing here is precious past the current calibration pass).
if [ -f "$probe_log" ] && [ "$(wc -c <"$probe_log" 2>/dev/null || echo 0)" -gt 5000000 ]; then
  tail -c 1000000 "$probe_log" >"${probe_log}.tmp" 2>/dev/null && mv "${probe_log}.tmp" "$probe_log" 2>/dev/null || true
fi

input=$(cat)

{
  printf '=== %s ===\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf '%s\n' "$input" | (command -v jq >/dev/null 2>&1 && jq '.' || cat)
  printf '\n'
} >>"$probe_log" 2>&1

printf '{"decision": "allow"}\n'
