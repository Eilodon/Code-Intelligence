# compound-wiki.md — audit-distill cross-reference index

Append-only log of every `audit-distill` pass run against this project.
Check here first before re-distilling an audit (idempotency).

---
date: 2026-07-14
source: audit-distill
audit: calm-agent-experience-round2-fixes (docs/superskills/specs/2026-07-14-calm-agent-experience-round2-fixes.md)
vheatm-version: audit-design FAST (no numeric VHEATM version tag in the source audit — pre-mortem + L1-L7 quick scan format, not a scored VHEATM run)
skills-updated: [audit-design]
pattern-debts-created: [DEBT-010-hook-state-toctou-race]
mat-entries: 0
---

## Distillation: calm-agent-experience-round2-fixes audit

### Gotchas Added
- `audit-design` (~/.claude/skills/audit-design/SKILL.md): dedup/idempotency
  key schemes need an event-type check, not just identity uniqueness — the
  concrete F1 SessionStart `resume` vs `clear`/`compact` bug this pass
  caught before it shipped. ✅

### PATTERN-DEBT Created
- `DEBT-010-hook-state-toctou-race` (docs/pattern-debt-registry.yaml):
  unlocked read-modify-write on `.calm/.hook-state/<session_id>.json` in
  `calm-nudge.sh`'s `save_state`/`bump` — flagged `open`/`low` urgency
  (real but likely rare in practice; recommends measuring before fixing).
  ✅

### M.AT Entries
- None. The source audit (audit-design FAST mode) used HIGH/MEDIUM/LOW
  severity labels, not a numeric QBR score — nothing to calibrate against
  a predicted-vs-actual outcome. Skipped per Step 5's own instruction
  ("for every finding with a QBR score AND a known outcome") rather than
  fabricating a score that was never computed.

### CONTEXT.md Updates
- Architectural Decision added (docs/superskills/CONTEXT.md, first created
  by this pass): per-identity dedup keys must account for event-type
  distinctions the identity space can collapse. ✅

### Scope note

This distillation covers the 6 implemented fixes (F1/F2/F4/F5/F6 shipped +
tested this session, F3 deliberately analysis-only) and the audit-design
risk assessment that reviewed them. It does NOT resolve the two items that
audit intentionally left open for a *future* session: (1) whether to harden
Read/Grep from advisory to a real deny gate (F3 produced the evidence —
16/27 real would_deny events are clean code-file misses — but the decision
itself was deliberately deferred, not distilled as "done"); (2) the
DEBT-010 TOCTOU race above (flagged, not fixed). Both remain real backlog,
tracked in their respective source documents, not resolved by this
distillation pass.

---
