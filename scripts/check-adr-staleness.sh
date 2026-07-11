#!/usr/bin/env bash
# Flags docs/adr/*.md files whose top-level Status line still reads as
# not-yet-implemented (Proposed/Deferred/Future/draft/chưa implement) while
# the same file also has an "## Update" section — the exact pattern that let
# ADR-0002, ADR-0004, and ADR-0005 all drift stale on 2026-07-11 (each had
# real "Update YYYY-MM-DD" sections documenting real shipped work, but the
# Status line at the top of the file was never revisited to match).
#
# This is deliberately a nudge, not a semantic diff — it can't tell you
# *what* changed, only that a file with this shape exists and is worth a
# human/agent re-read. See docs/superskills/plans/2026-07-11-market-position-
# and-roadmap.md §5.2 for why this is a repo-local script rather than a
# change to the global `adr-commit` skill (whose own ADR format/workflow
# doesn't match this repo's docs/adr/ convention).
#
# Usage:
#   scripts/check-adr-staleness.sh            # advisory — always exits 0
#   scripts/check-adr-staleness.sh --strict    # exits 1 if any ADR is flagged (for optional CI wiring)
set -euo pipefail
cd "$(dirname "$0")/.."

strict=0
if [ "${1:-}" = "--strict" ]; then
    strict=1
fi

# Case-insensitive; matches both the English and Vietnamese phrasing this
# repo's ADRs actually use for a not-yet-implemented status.
stale_pattern='Proposed|Deferred|chưa implement|chưa lên lịch|draft.*chờ review|: *Future\b'

flagged=0
shopt -s nullglob
for adr in docs/adr/*.md; do
    status_line=$(grep -m1 -E '^\s*-\s*\*\*Status\*\*:' "$adr" || true)
    [ -z "$status_line" ] && continue

    has_update_section=0
    grep -qE '^##[[:space:]]+Update' "$adr" && has_update_section=1

    if [ "$has_update_section" -eq 1 ] && echo "$status_line" | grep -qiE "$stale_pattern"; then
        echo "STALE?  $adr"
        echo "        $status_line" | cut -c1-160
        echo "        (has an \"## Update\" section — re-check whether Status above still matches it)"
        echo
        flagged=$((flagged + 1))
    fi
done

if [ "$flagged" -eq 0 ]; then
    echo "check-adr-staleness: no ADRs flagged ($(ls docs/adr/*.md 2>/dev/null | wc -l | tr -d ' ') checked)."
    exit 0
fi

echo "check-adr-staleness: $flagged ADR(s) flagged for manual review."
if [ "$strict" -eq 1 ]; then
    exit 1
fi
exit 0
