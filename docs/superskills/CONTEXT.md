# CONTEXT.md — CALM project context (Super Skills)

First created 2026-07-14 by `audit-distill`, distilling
`docs/superskills/specs/2026-07-14-calm-agent-experience-round2-fixes.md`.
Minimal on purpose — seeded only with what this distillation pass actually
earned, not backfilled with unrelated project history. Add to this file as
future audits/distillations produce real findings that belong here.

## Architectural Decisions

<!-- from audit: calm-agent-experience-round2-fixes F1 -->
- **A per-identity dedup/cache key must also account for a distinguishing
  event-type when the same identity can legitimately recur across
  semantically different events.** Concrete instance: `session-start-
  agents-md.sh`'s first draft deduped SessionStart injection on `session_id`
  alone; Claude Code reuses the same `session_id` across `resume` (safe to
  dedup) AND `clear`/`compact` (NOT safe — context was just wiped, the
  agent needs the full guide back). caught live by `audit-design`'s
  pre-mortem before it shipped, not after an incident. Generalizes beyond
  CALM: any cache/idempotency key scheme should be checked for "does this
  key's identity space collapse two events that need different handling"
  before shipping, not just "is this key unique enough to prevent
  duplicate work."
