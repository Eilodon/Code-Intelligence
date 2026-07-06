<!-- VHEATM 10.0: unchanged from 9.0 -->
# Signal-to-Noise Filter — [A]

Run BEFORE issuing any ADR. Apply to every hypothesis with QBR ≥ 9 (REQUIRED+).

Purpose: Distinguish real signals from noise before they become ADRs. Research shows overconfidence bias leads to overidentification — finding "issues" that aren't actually issues worth acting on (Saposnik et al., 2016).

---

## Question 1: Worst Case Without Fix

"If this is not fixed in the next 6 months, what is the realistic worst case?"

```
Worst case: [Specific — not "could cause issues" but "X happens to Y users resulting in Z"]
Probability of worst case: HIGH / MEDIUM / LOW / NEGLIGIBLE

If NEGLIGIBLE → downgrade to OPTIONAL or REMOVE from ADR list
```

---

## Question 2: Fix Cost vs Leave Cost

"Is the cost of fixing less than the cost of leaving it unfixed?"

```
Fix cost (CLI): [N units]
Not-fix cost estimate: [QBR impact × probability]

If fix cost > 3× not-fix cost AND probability = LOW:
  → Defer to backlog — document as known risk, do NOT issue ADR
```

---

## Question 3: Detect-and-Fix-Later?

"If this issue materializes after launch, can it be detected quickly and fixed without major harm?"

```
Detectable by monitoring/alerting? YES / NO
Time to detect if it occurs: [hours / days / weeks / unknown]
Time to fix once detected: [hours / days / weeks]

If YES + fix time ≤ 1 day:
  → Consider downgrading MANDATORY → RECOMMENDED
  → Note: "Monitorable — acceptable risk to defer"
```

---

## Question 4: Already Known and Accepted?

"Has this issue already been identified and deliberately accepted as a known risk?"

```
In existing backlog? YES / NO
Explicitly accepted risk (documented somewhere)? YES / NO

If YES to either:
  → Note in handoff — do NOT issue ADR (avoids double-counting)
  → Log: "H-[ID] already in backlog as [reference] — not duplicated here"
```

---

## SNF Output Table

```
| H-ID | Original Priority | Q1 Worst Case | Q2 Cost Ratio | Q3 Monitorable | Q4 Known | SNF Verdict | Reason |
|---|---|---|---|---|---|---|---|
| H-01 | MANDATORY | Data loss, HIGH | Fix << Leave | NO | NO | MAINTAIN | Critical, high prob, not monitorable |
| H-03 | REQUIRED | Minor UX friction, LOW | Fix 5× Leave | YES <1d | NO | DOWNGRADE → OPTIONAL | Low prob, easily fixed post-launch |
| H-07 | MANDATORY | Auth bypass, MEDIUM | Fix << Leave | NO | NO | MAINTAIN | Security, not monitorable |
| H-09 | REQUIRED | Same as backlog item | — | — | YES | REMOVE | Already tracked, avoid duplication |

ADRs issued after SNF: [N] (reduced from [M] pre-SNF)
SNF removed/downgraded: [M-N] hypotheses
```

---

## SNF Rules

- SNF is not a rubber stamp — every REMOVE or DOWNGRADE must have a documented reason
- SNF does not apply to MANDATORY findings with security implications — always escalate security
- In FAST mode: run Q1 and Q3 only — document "Q2 and Q4 skipped — FAST mode"
- If unsure between MAINTAIN and DOWNGRADE: default to MAINTAIN (conservative bias is correct for audit)
