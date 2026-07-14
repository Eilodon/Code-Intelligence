#!/usr/bin/env bash
# TEMPORARY discovery script (2026-07-14) — NOT part of the permanent hook
# design. Purpose: capture the REAL PostToolUse stdin payload for a Grep call
# before writing any parser logic against it, per
# docs/plans/2026-07-14-search-grep-steepest-hill-followups.md's own stated
# principle ("dump a real event's JSON to see the actual shape before writing
# logic against it" — don't build against a guessed/doc-derived schema alone).
# No parsing, no decision output, no side effects on the tool call itself —
# just an unconditional append-to-disk. Delete this file (and its
# settings.json entry) once the real schema is confirmed and the permanent
# hook is built.
mkdir -p .calm/.hook-state 2>/dev/null || true
cat >> .calm/.hook-state/posttooluse-grep-dump.jsonl
exit 0
