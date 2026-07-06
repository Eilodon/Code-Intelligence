# Incentive Misalignment Probe — [G.INC] (v10.0)

> **Research Basis**: Microsoft Research (replicated independently) found organizational
> structure metrics were the most accurate predictor of post-launch defects — outperforming
> code complexity, churn, and dependency analysis combined. Conway's Law (MIT/Harvard
> replication): systems mirror their organization's communication structure — and so do
> their failure modes. Biczók et al. (2025): "vendors have every incentive to rush
> low-quality software and no incentive to enhance quality control."
>
> **Implication**: A confirmed bug that no one has incentive to fix will survive any
> audit. VHEATM 9.0 could find it — and it would still ship. [G.INC] closes this gap.

Run AFTER each hypothesis is **CONFIRMED** in [E], BEFORE issuing ADR in [A].

---

## The Core Question

**"Does the organizational structure create incentive to NOT fix this bug?"**

This is distinct from all prior VHEATM questions:
- Prior questions: "Is this a bug? Where is it? How bad is it?"
- [G.INC] question: "Even if we know it's a bug — will it actually get fixed?"

---

## The 3-Question Probe

For each CONFIRMED hypothesis, answer all 3:

### INC-1: Ownership

```
"Who owns fixing this bug?"

□ Named team or role: [team/person]
□ Joint ownership (multiple teams): [list]
□ Unclear / disputed: BOUNDARY FLAG (see below)
□ No one — gap between teams: BOUNDARY FLAG

If the answer is "team X should fix this but team Y introduced it":
→ BOUNDARY FLAG automatically
→ Add to [G.ORG] Org Blast Radius probe
```

**What ownership questions reveal:**
- Orphaned code — no team claims it → bug will never be prioritized
- Historical ownership that lapsed → original team gone, no heir
- Shared ownership with no tie-breaker → both teams defer to each other

---

### INC-2: Incentive

```
"Does fixing this bug hurt the fixing team's metrics, velocity, or roadmap?"

Ask explicitly:
  □ Does fixing it require the team to acknowledge past mistakes? (face cost)
  □ Does fixing it consume sprint velocity competing with roadmap features?
  □ Does fixing it require effort from the team that benefits ANOTHER team's users?
  □ Does fixing it require the team to break backward compatibility?
  □ Is there a deadline pressure that creates incentive to defer?

If YES to any: log INCENTIVE RISK — the bug may survive fix prioritization.

INCENTIVE RISK does NOT downgrade a MANDATORY finding.
It ELEVATES the ADR's Observable Success Criteria to require stakeholder sign-off,
not just technical fix.
```

**Escalation rule:**
MANDATORY + INCENTIVE RISK + BOUNDARY FLAG → ADR requires named owner sign-off
before marking resolved. "Fixed in code" is insufficient. The team responsible for
the receiving side of a boundary-crossing fix must also confirm.

---

### INC-3: Organizational Velocity

```
"How long will this bug realistically take to get fixed — including org friction?"

Technical fix time (CLI estimate): [N]
Estimated org friction multiplier:
  □ Single team, clear ownership, no boundary: × 1.0 (no friction)
  □ Single team, clear ownership, scope creep risk: × 1.5
  □ Cross-team, cooperative relationship: × 2.0
  □ Cross-team, competing priorities: × 3.0
  □ Cross-team, disputed ownership: × 4.0
  □ Compliance-gated (must wait for legal/compliance review): × 5.0

Realistic resolution time = CLI × org friction multiplier
Compare vs SLA deadline or audit cycle length.

If realistic resolution time > next cycle → escalate to debt register with
explicit "org-friction-blocked" tag, not just "technical debt."
```

---

## BOUNDARY FLAG Protocol

When INC-1 identifies a boundary crossing (ownership crosses team lines):

```
BOUNDARY: YES
Crossing type:
  □ TECHNICAL — fix requires changes in Team A's code by Team B (access issue)
  □ HANDOFF   — Team A fixes, Team B must integrate/deploy (dependency issue)
  □ APPROVAL  — Fix is ready but requires Team B sign-off (governance issue)
  □ DISPUTED  — Neither team agrees they own this (gap issue)

Required in ADR:
  Owner:    [Primary team responsible]
  Boundary: YES / [crossing type]
  Handoff plan: [Explicit: who does what, in what order]
```

Without a handoff plan, a boundary-crossing ADR is not considered complete.
"Team X will fix it" when fix also requires Team Y action = incomplete ADR.

---

## Mode Adaptation

### FAST Mode

Run INC-1 only: "Who owns this? Does it cross a boundary?"
Document: "INC-FAST: ownership check only. INC-2 and INC-3 deferred."
If BOUNDARY FLAG found in FAST mode → escalate to Standard for full probe.

### Standard Mode

Run all 3 questions. INCENTIVE RISK and BOUNDARY FLAG trigger full documentation.
Org friction multiplier should inform next-cycle trigger criteria.

### Full Mode

Run all 3 questions + stakeholder view per owner:
```
For each team in INC-1 ownership list:
  □ What is their incentive to fix this?
  □ What is their incentive to defer?
  □ What is the minimum change in their context to make fixing attractive?
     (e.g., "If this bug causes their own metrics to degrade, they fix it")
```

---

## Output Template

```yaml
incentive_misalignment_probe:
  hypothesis_id: H-[ID]
  inc_1_ownership:
    primary_owner: "[team/role or 'unclear']"
    joint_owners: ["[team]", ...]
    boundary_flag: true | false
    crossing_type: TECHNICAL | HANDOFF | APPROVAL | DISPUTED | N/A
  inc_2_incentive:
    risks_detected: ["face cost", "velocity impact", "roadmap competition", ...]
    incentive_risk_flag: true | false
    escalation_required: true | false  # MANDATORY + INCENTIVE RISK + BOUNDARY = escalate
  inc_3_velocity:
    technical_cli: [N]
    org_friction_multiplier: [1.0 | 1.5 | 2.0 | 3.0 | 4.0 | 5.0]
    realistic_resolution_time: [N × multiplier]
    org_friction_blocked: true | false
  adr_requirements:
    owner_field: "[team]"
    boundary_field: "YES/[type] or NO"
    handoff_plan: "[explicit plan or N/A]"
    stakeholder_signoff_required: true | false
```

---

## Enterprise-Specific Patterns

The following organizational patterns are ENTERPRISE-mode specific findings.
Each warrants a REQUIRED-level ADR when confirmed:

**EP-01: Orphaned component**
Bug lives in code that was transferred between teams and current owners have limited context.
Fix requires finding someone with tribal knowledge.
INC-3 multiplier: × 4.0 minimum.

**EP-02: Cross-team mandatory but no mandate**
Fix requires Team B to change, but no governance mechanism exists to require them.
Classic in platform/consumer team relationships.
Requires escalation beyond ADR — needs product/engineering leadership alignment.

**EP-03: Compliance-gated fix**
Fix is technically trivial but requires legal review, compliance sign-off, or audit documentation.
INC-3 multiplier: × 5.0.
Document the regulatory gate explicitly.

**EP-04: Incentive inversion**
The team that caused the bug is rewarded for shipping fast (velocity metric), not for fixing bugs (quality metric).
The cost of the bug is borne by a downstream team.
This is a structural incentive problem, not a technical one.
ADR must include recommendation for incentive alignment, not just code fix.

---

## When [G.INC] Finds Nothing

"No incentive misalignment found" is a valid and important output.
Document explicitly:
```
INC-1: Owner clearly identified: [team]
INC-2: No incentive risks — team benefits directly from fixing this bug
INC-3: Org friction: × 1.0 — no boundary crossing, single-team fix
Result: No misalignment detected. ADR can proceed normally.
```

Documenting absence is as important as documenting presence.
Absence of misalignment increases confidence the fix will land.

---

## Anti-Patterns

🚫 **"Ownership is obvious, skipping probe"** — The probe surfaces hidden dynamics.
Run it even when ownership seems clear. "Obvious" ownership is often assumed, not verified.

🚫 **"INCENTIVE RISK means I should downgrade to OPTIONAL"** — No.
Incentive risk is an org-layer problem, not a technical one. The bug is still MANDATORY
if QBR says so. Incentive risk changes WHO must approve the fix, not WHAT priority it gets.

🚫 **"Boundary crossing is a deployment concern, not audit concern"** — Boundary crossings
are where enterprise bugs go to die. If the audit doesn't flag it, it won't be in the ADR.
If it's not in the ADR, the handoff won't be planned.

🚫 **"INC-3 friction multiplier is speculation"** — It's an estimate, not a guarantee.
But it's more honest than a CLI estimate that pretends org friction doesn't exist.
Document the uncertainty: "est. × 3.0 based on cross-team relationship history."

---

*Reference 14 — VHEATM 10.0 | Research basis: Microsoft org structure study (replicated),
Conway's Law MIT/Harvard replication, Biczók et al. 2025 incentive misalignment*
