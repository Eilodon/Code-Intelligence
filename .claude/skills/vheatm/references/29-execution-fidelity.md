# Execution Fidelity — [M.EF] (v13.0)

> **Gap closed**: VHEATM v12.0 introduced the Ritual Suppression Rule ("every section must
> produce decision/finding/downgrade/debt/next action/skip reason") but provides no
> mechanical enforcement. A compliant-looking output ("Phase X: PASS") satisfies the
> letter without the spirit. Additionally: no "incomplete cycle" signal exists in the
> handoff, no finding genealogy tracking, and no cycle delta-diff between N and N+1.
> This reference closes all four gaps.

Run at [M] before issuing final handoff. HG-EF gate requires [M.EF] to pass.

---

## Part 1 — Ritual Suppression Checker

### The Problem

v12.0 ref 21 Part 7 defines:
> "Every phase section must produce exactly one of: decision, finding, downgrade, debt,
> next action, skip reason."

But in practice, sections can produce superficially compliant text:
- "Phase V: Architecture mapped. No issues found." → is this a skip reason? A decision?
- "Bias probe: None detected." → is this a decision? Or is it ceremony?
- "L4: No concurrency issues identified." → finding or skip reason?

The rule is correct but under-enforced. [M.EF] adds the enforcement layer.

### Ritual Suppression Checker — Self-Validation Protocol

At [M] closure, before finalizing output, validate EVERY phase section:

```
For each phase section in the output:
  □ Does it contain at least one of these OUTPUT MARKERS?
    DECISION:     "Based on [evidence], [conclusion reached]"
    FINDING:      H-[ID] with anchor, priority, description
    DOWNGRADE:    "H-[ID] downgraded from [X] to [Y] because [reason]"
    DEBT:         "Debt-[ID]: [description], age [N], owner [team]"
    NEXT_ACTION:  "Required before next cycle: [specific condition]"
    SKIP_REASON:  "Skipped [phase/step] because [reason]. Implication: [what was not covered]."
    ADR:          ADR-[N] with priority, evidence anchor, decision

  □ If the section has ONLY:
    - Prose explanation without a marker → CEREMONY: flag for removal/rewrite
    - Bare "PASS" or "None found" without marker → CEREMONY: must add SKIP_REASON at minimum
    - Assertions without evidence → CEREMONY if no evidence anchor

Remediation: for each ceremony-flagged section, add the appropriate marker or remove the section.
```

### Output Density Metric

```
Output density = (sections with ≥1 output marker) / (total phase sections)

Target: ≥ 85% density
  90-100%: HIGH fidelity output
  80-89%:  ACCEPTABLE
  70-79%:  LOW fidelity — review flagged sections
  <70%:    CEREMONY RISK — output likely ceremonial, re-run phases
```

---

## Part 2 — Incomplete Cycle Signal

### The Problem

When a cycle ends prematurely (context budget exhausted, session timeout, user stopping),
the handoff can look like a completed cycle. The next instance gets a "completed" handoff
and doesn't know which phases were cut short.

### cycle_status Field

Add to AI-ingestion YAML handoff (ref 18):

```yaml
handoff:
  cycle: [N]
  cycle_status: COMPLETE | PARTIAL | HALTED | BUDGET_HALT
  # COMPLETE: all Hard Gates passed, all phases complete
  # PARTIAL:  some phases skipped (documented), no critical gaps
  # HALTED:   stopped mid-phase due to external reason (user request, error)
  # BUDGET_HALT: context budget exhausted, remaining phases incomplete

  # If cycle_status ≠ COMPLETE:
  interrupted_at_phase: "[P | V | G | E | A | T | M | KB]"
  phases_not_completed: ["[phase name]", ...]
  critical_hypotheses_unverified: ["H-[ID]", ...]  # MANDATORY hypotheses not yet in [E]
  minimum_resume_action: "[what next instance MUST do before continuing]"
```

### Incomplete Cycle Handling for Next Instance

When `cycle_status ≠ COMPLETE`:

```
[P] next cycle activation:
  □ Read interrupted_at_phase
  □ Read critical_hypotheses_unverified
  □ Set Priority Mode: MANDATORY + REQUIRED only (no OPTIONAL/RECOMMENDED) until
    all critical_hypotheses_unverified are resolved
  □ Do NOT run full [V] unless SDS > 30% — resume from interrupted_at_phase
  □ Log: "Resuming partial cycle N. Minimum actions: [list from minimum_resume_action]"

Output header note:
  "PARTIAL CYCLE RESUME: continuing from [phase] in cycle [N].
   [M] hypotheses still unverified: [list]. Running expedited path."
```

---

## Part 3 — Finding Genealogy

### The Problem

Findings in VHEATM output currently have no explicit "ancestry" — what research
informed them, what hypothesis generated them, what prior cycles touched them.
Without genealogy, trend analysis requires manual reconstruction.

### Finding Genealogy Schema

Add to each ADR's metadata:

```yaml
finding_genealogy:
  adr_id: ADR-[N]
  # Research that informed this finding
  research_basis:
    - type: "empirical" | "case_study" | "best_practice" | "first_principles"
      source: "[paper name / framework name]"
      evidence_tier: T1 | T2 | T3 | T4
      year: [year verified as current]
  # Hypothesis lineage
  generated_by: H-[ID]
  layer: "[L4.1 | L6.3 | etc.]"
  specialist_lens: "[STRIDE | FMEA | none | etc.]"
  # Cycle history
  first_cycle: [cycle number]
  previous_occurrences:
    - cycle: [N]
      status: OPEN | CONFIRMED | FALSE_POSITIVE | RESOLVED
      adr_id: ADR-[previous]
  # Bug Class association
  bug_class_id: "BC-[NNN] | new"
  # Quality
  static_confidence: HIGH | MEDIUM | LOW
  dynamic_confirmed: true | false | deferred
```

### Why Genealogy Matters

1. **Trend analysis**: recurring findings (same bug class across cycles) automatically surface
2. **Evidence aging**: `year` field + Evidence Currency Protocol (ref 30) → flag stale research
3. **Specialist routing memory**: if finding consistently comes from STRIDE → router learns
4. **False positive tracking**: genealogy connects ADR → FP entry in AI FP Catalog
5. **ADR resolution proof**: `status: RESOLVED` in prior occurrence → Fix Anchor must exist

---

## Part 4 — Cycle Delta-Diff

### The Problem

Between cycle N and N+1, there is no systematic protocol for "what changed?" relative
to findings. New findings may be genuinely NEW bugs or regressions. Resolved findings
should be tracked. Without delta-diff, cognitive load of comparing two full cycles is high.

### Delta-Diff Protocol

At start of [G] in cycle N+1, produce a delta-diff against cycle N:

```
Finding status categories:
  NEW:          H-ID not present in cycle N handoff
  REGRESSED:    H-ID was RESOLVED in cycle N but appears again in cycle N+1
  UNCHANGED:    H-ID still OPEN from cycle N, no new evidence
  RESOLVED:     H-ID was OPEN in cycle N, now RESOLVED (Fix Anchor present in [T.FV])
  SUPERSEDED:   H-ID from cycle N is replaced by broader finding in cycle N+1
  DOWNGRADED:   H-ID priority was MANDATORY in cycle N, now REQUIRED in cycle N+1
  🆕 NEVER_REACHED: H-ID was in a phase that was not executed in cycle N due to
                BUDGET_HALT or HALTED (cycle_status ≠ COMPLETE and phase not reached).
                Treatment: handle as NEW — go through full [G.H] generation.
                Do NOT assume UNCHANGED (no evidence either way — absence of finding ≠ safe).

  NEVER_REACHED rule: "A hypothesis that was never checked has unknown status.
    Treat with the same rigor as a brand-new hypothesis."

Format in output:
  delta_summary:
    new_findings:       [count] — [H-IDs]
    regressed:          [count] — [H-IDs and regression reason]
    resolved:           [count] — [H-IDs and Fix Anchor]
    unchanged_open:     [count] — [H-IDs, reason still open]
    superseded:         [count] — [old H-ID → new H-ID]
    downgraded:         [count] — [H-IDs]
    🆕 never_reached:   [count] — [H-IDs from prior BUDGET_HALT phases; re-enter as NEW]
```

### REGRESSION Escalation Rule

If any finding shows status = REGRESSED (was resolved, now back):
- Reopen at MANDATORY priority regardless of original priority
- Trigger [G.SCR] (self-audit protocol) for the fix that was supposed to resolve it
- Add to AI FP Catalog if the prior resolution was AI-generated
- Root cause: "Fix Verification ([T.FV]) may have been incomplete"

---

## [M.EF] Execution Checklist (HG-EF gate)

```
□ Ritual Suppression Check:
    □ Every phase section reviewed for output markers
    □ Ceremony-flagged sections revised or removed
    □ Output density ≥ 85%

□ cycle_status set accurately:
    □ COMPLETE if all Hard Gates passed
    □ PARTIAL/HALTED/BUDGET_HALT with appropriate fields if not

□ Finding Genealogy:
    □ Every new ADR this cycle has finding_genealogy populated
    □ research_basis.year checked against Evidence Currency Protocol (→ ref 30 Part 1)

□ Delta-Diff produced (if prior cycle handoff exists):
    □ All prior H-IDs classified: RESOLVED | UNCHANGED | SUPERSEDED | DOWNGRADED
    □ Any REGRESSED → escalated to MANDATORY + [G.SCR] flagged
```

---

## Anti-Patterns

🚫 **"Output density check is bureaucratic overhead"** — A 65% density output
means 35% of your phase content produced nothing actionable. That's waste,
not rigor. Fix the sections or remove them.

🚫 **"COMPLETE is always better than PARTIAL"** — A PARTIAL cycle with all MANDATORY
hypotheses verified is better than a COMPLETE cycle that produced ceremonial OPTIONAL
findings. Honest cycle_status > optimistic cycle_status.

🚫 **"Finding genealogy takes too long"** — Populate incrementally at ADR creation time.
Each ADR already requires evidence anchor, priority, decision. Genealogy adds 3 fields
(research_basis, first_cycle, bug_class_id) — ~30 seconds per ADR.

🚫 **"Delta-diff is obvious from reading both outputs"** — Manual comparison across
two full audit outputs misses REGRESSED findings. Explicit delta-diff mechanically
surfaces what manual comparison skips.

---

*Reference 29 — VHEATM 13.0 | Execution Fidelity
Gap source: VHEATM v12.0 meta-audit — Ritual Suppression rule without checker,
no incomplete cycle signal, no finding genealogy, no delta-diff between cycles.*
