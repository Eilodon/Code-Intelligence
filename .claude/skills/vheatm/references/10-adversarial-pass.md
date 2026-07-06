# Adversarial Pass — [M.AP] (v10.0)

> **v9.0 basis**: Tikai Round 4 meta-audit found 7 endpoints missing rate limits after
> 2 rigorous prior rounds. Single auditor adversarial pass — even when well-executed —
> has structural blind spots by construction.
>
> **v10.0 addition**: Full mode gets multi-perspective adversarial pass. Each lens is
> assigned a stakeholder perspective. Research basis: red teaming literature shows
> multi-perspective passes have 3× lower miss rate than single-perspective.
> ENTERPRISE mode adds Lens 5 (ORG perspective) for cross-team blind spots.

---

## The Core Posture Shift (unchanged from 9.0)

**Default closing posture**: "I've found everything I can."
**Adversarial posture**: "There are N more bugs I haven't found. What are they?"

This is not pessimism — it's calibration. Tikai data shows that even after 2 rigorous
audit rounds, 18% miss rate remained. Adversarial Pass acknowledges this and forces
one more search before closure.

---

## The Adversarial Hypothesis (unchanged)

```
"There are N additional bugs in this audit I have NOT found.
These bugs share the blindspots I declared in ABG Guard 4.
I will spend [time-box] searching for them."
```

**Setting N** (calibration table, unchanged):
| Mode | Cycle # | N |
|------|---------|---|
| FAST | Any | 1 |
| Standard | 1st | 5 |
| Standard | 2nd+ | 3 |
| Full | 1st | 7 |
| Full | 2nd+ | 5 |
| Self-Audit | Always | ≥5 |

N is empirically calibrated from Tikai miss-rate data. Don't reduce it because "I think I'm thorough."

---

## Protocol — Standard Mode (4 Lenses, unchanged from 9.0)

Same as 9.0: Pattern, Self, Cross-Cutting, Compound lenses.

The cross-cutting lens now includes L7.11:
- Rate limits, idempotency, timeouts, observability, authz, resource lifecycle
- 🆕 "Any regulatory obligation that I haven't explicitly scanned? YES/NO"

---

## 🆕 Protocol — Full Mode (Independent Generation Protocol, 4+1 Perspectives) — v13.0

> **v13.0 update:** CONTEXT_DENIED replaced by **Independent Generation Protocol** (IGP).
> Transformer models cannot truly "forget" prior context — CONTEXT_DENIED was aspirational,
> not mechanically enforceable. IGP uses YAML-only cross-lens visibility (bounded contamination)
> instead of impossible full context isolation.
> → Full IGP specification: `references/28-auditor-defense.md` Part 2.
>
> **v11.0 basis (retained):** Full mode personas use Structured Lens Frame. Research basis:
> SPP (Wang 2023, 238+100 citations) — fine-grained context **constraints** produce genuine
> perspective shifts in LLMs, not role labels alone.
>
> **v13.0 lens format** (replaces CONTEXT_DENIED with IGP CONSTRAINT):

For each lens, declare BEFORE running:
```
LENS: [name]
PERSPECTIVE: [stakeholder role]
CONTEXT_GRANTED: [what this lens has access to]
CONSTRAINT: "Generate findings ONLY from your perspective context above.
             Do not reference or build on findings from prior lenses.
             Output: YAML only, no prose."
QUESTION_FRAME: [single forcing question]
N: [bugs to find, from calibration table]
```

After each lens: load ONLY the YAML summaries of prior lenses (≤500 tokens per lens summary).
After all lenses: reconcile (→ ref 28 Part 2 Step 3).

In Full mode, each of the 4 standard lenses is run from a distinct stakeholder
perspective. This is not just "more lenses" — it's different threat models.

### Lens 1 — Pattern Lens (SRE Perspective)

**Role**: You are an SRE who gets paged at 3am when this system breaks.

```
SRE adversarial questions:
  □ What failure mode would I see in production metrics BEFORE the bug manifests fully?
  □ What cascading failure does this create when it fires under load?
  □ What runbook would I need that doesn't exist yet?
  □ What resource exhaustion does this set up over time?
  □ Is there a retry loop that becomes infinite under certain conditions?
  □ What monitoring alert is missing for this failure class?

→ Focus: operational failure patterns, cascading, resource exhaustion, missing observability
```

### Lens 2 — Self Lens (Security Perspective)

**Role**: You are a security engineer doing a threat model of this code.

```
Security adversarial questions:
  □ What new attack surface does this code open?
  □ What trust boundary did this code change — did we validate both sides?
  □ What data does this code now have access to that it didn't before?
  □ What happens if an attacker provides malformed input specifically designed
    to trigger the edge cases this code doesn't handle?
  □ What third-party dependency does this introduce — is it audited?
  □ What authentication or authorization assumption does this code make
    that an attacker could violate?

→ Focus: attack surface, trust boundaries, data access, third-party risk
```

### Lens 3 — Cross-Cutting Lens (Compliance Perspective)

**Role**: You are a compliance officer reviewing this for an upcoming SOC2 audit.

```
Compliance adversarial questions:
  □ What user data does this touch that isn't in the privacy policy?
  □ What audit trail would I need to produce for a regulator that isn't being created?
  □ What consent or lawful basis am I assuming exists that might not?
  □ If a user requests data deletion, does this code's data get erased?
  □ What's the data retention for data this code creates? Is it enforced?
  □ If this code fails, what notification obligation is triggered?
     (GDPR 72h breach notification? PCI-DSS incident response?)

→ Focus: regulatory obligations, audit trails, user rights, breach exposure
```

### Lens 4 — Compound Lens (Product Perspective)

**Role**: You are a product manager demoing this to a customer and it fails.

```
Product adversarial questions:
  □ What user action produces a wrong result that I'd have to explain to a customer?
  □ What edge case exists that a real user (not developer) would hit in week 2?
  □ What does "success" look like from the user's point of view — and does the
    code actually deliver that, or a close approximation?
  □ What data shown to users could be incorrect or stale in a way that
    affects their business decision?
  □ What feature is documented in the UI but not fully implemented in the backend?
  □ What compound feature is "mostly done" but missing one component?

→ Focus: UX failures, data correctness for business decisions, compound feature gaps
```

### 🆕 Lens 5 — ORG Lens (ENTERPRISE mode only, cross-team perspective)

**Role**: You are an engineer from Team B who receives the output of this system.

```
ORG adversarial questions:
  □ What assumption does Team A's code make about Team B's system that isn't
    in any SLA or documented contract?
  □ What change in Team A's code would silently break Team B's integration?
  □ What does "Team A fixed it" mean when the fix requires Team B to also change
    something — and Team B doesn't know?
  □ What ownership gap exists — something both teams think the other owns?
  □ What organizational constraint makes it impossible to fix the right way in
    the next sprint? (team capacity, budget, roadmap commitment)

→ Focus: cross-team assumptions, integration failures, ownership gaps, org constraints
```

---

## Time Budget

```
Total [M.AP] budget:
  FAST:     5 min — 1 lens
  Standard: 15 min — 4 lenses (3.75 min/lens)
  Full:     30 min — 4 lenses (7.5 min/lens) or 5 lenses for ENTERPRISE (6 min/lens)

Strict rule: budget cap per lens — even if finding nothing, don't extend.
The discipline is in the posture, not finding bugs at any cost.
```

---

## Acceptance Criteria (updated)

```
□ All lenses run (or explicitly skipped with documented reason)
□ For Full mode: each lens run from declared perspective (not generic pass)
□ At least 1 candidate per lens (or "none found" explicitly documented)
□ Each candidate routed: NEW_HYPOTHESIS | DEFER_DEBT | DISMISS
□ DISMISS requires reason — silent dismissal is a protocol violation
□ Output written even if all findings are "no new bugs found"
```

---

## Output Template (updated for multi-perspective)

```yaml
adversarial_pass:
  mode: Standard | Full | FAST
  multi_perspective: true | false  # true in Full mode
  assumed_remaining_bugs_N: [N]
  time_budget_minutes: [N]
  time_actually_spent: [N]
  lenses:
    pattern:
      perspective: "SRE (Full mode) | standard (Standard mode)"
      time_spent: [minutes]
      adversarial_questions_used: ["[Q1]", "[Q2]", ...]
      candidates_found: [count]
      candidate_findings: [...]
    self:
      perspective: "Security (Full mode) | standard (Standard mode)"
      time_spent: [minutes]
      candidates_found: [count]
      candidate_findings: [...]
    cross_cutting:
      perspective: "Compliance (Full mode) | standard (Standard mode)"
      time_spent: [minutes]
      l7_11_compliance_checked: true | false
      candidates_found: [count]
      candidate_findings: [...]
    compound:
      perspective: "Product (Full mode) | standard (Standard mode)"
      time_spent: [minutes]
      candidates_found: [count]
      candidate_findings: [...]
    org:  # ENTERPRISE mode only
      perspective: "Team B / cross-team"
      time_spent: [minutes]
      candidates_found: [count]
      candidate_findings: [...]
  routing_summary:
    new_hypotheses: [count]
    deferred_debt: [count]
    dismissed: [count]
  closure_verdict: CLEAN | DEFERRED_DEBT_LOGGED | REOPENED_CYCLE
```

---

## When [M.AP] REOPENS the Cycle (unchanged)

MANDATORY threshold finding in [M.AP] → cycle goes back through [G.PG], [E], [A], [T], [T.FV].

---

## FAST Mode (updated)

```
[M.AP] FAST: 5 min, 1 lens.
Pick the highest-yield lens for this audit's profile:
  Code-heavy → Pattern lens (SRE perspective: "what breaks at 3am?")
  Self-audit → Self lens (Security: "what attack surface did I create?")
  Feature work → Compound (Product: "what user action breaks week 2?")
  API/infra → Cross-Cutting (Compliance: "what audit trail is missing?")
  🆕 ENTERPRISE → ORG lens ("what does Team B assume about my code?")
```

---

*Reference 10 — VHEATM 10.0 | Multi-perspective AP from red teaming literature;
SRE/Security/Compliance/Product lens personas from enterprise threat modeling practice*
