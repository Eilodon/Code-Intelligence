#!/usr/bin/env bash
# TEMPORARY discovery script (2026-07-14) — NOT part of the permanent hook
# design. Purpose: capture the REAL SessionStart stdin payload before writing
# any session_id/source-aware dedup logic against it (F1, see
# docs/superskills/specs/2026-07-14-calm-agent-experience-round2-fixes.md's
# audit-design Risk Assessment — "Before implementing, capture one real
# SessionStart hook JSON payload"). Same pattern as
# posttooluse-discovery-dump.sh: no parsing, no decision output, no side
# effects — just an unconditional append-to-disk. Delete this file (and its
# settings.json entry) once the real schema is confirmed and F1's permanent
# session-source-aware logic is built into session-start-agents-md.sh.
mkdir -p .calm/.hook-state 2>/dev/null || true
cat >> .calm/.hook-state/sessionstart-dump.jsonl
exit 0
