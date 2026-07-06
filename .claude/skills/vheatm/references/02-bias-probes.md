# Auditor Bias Probe — [G.B] (v10.0)

Run AFTER hypothesis list is formed in [G.H], BEFORE [G.D] Debate Gate.

**v10.0 addition**: Bias Check 6 (Organizational Capture) addresses the failure mode
where the auditor is reviewing code owned by their own team. Subtler than self-audit
(Check 5) — the auditor didn't write this specific code, but has professional incentives
to find it "clean." Accounting audit literature documents this in formal audit contexts;
software engineering adaptation is overdue.

Academic basis (all checks):
- Mohanani et al. (2017, 123 citations): 37 cognitive biases in software engineering
- Saposnik et al. (2016, 780 citations): automation and anchoring bias in decisions
- Bertrand et al. (2022, 128 citations): XAI and cognitive bias interaction
- Berthet (2020, 117 citations): cognitive bias in professional decision-making
- Fasolo et al. (2024, 18 citations): bias mitigation in organizational decisions
- 🆕 Camilli et al. (2025): cognitive biases in audit judgment — professional incentive
  conflicts suppress skepticism in auditors reviewing work of their own organization

---

## Bias Check 1: Anchoring Bias (unchanged)

"Is my first hypothesis influencing how I evaluate all subsequent ones?"

```
First hypothesis found: H-[ID] "[description]"
Did subsequent hypotheses get higher/lower QBR BECAUSE of this first finding?
→ YES / NO
If YES: re-evaluate QBR of top 3 hypotheses independently.
```

---

## Bias Check 2: Confirmation Bias (unchanged)

"Am I searching for evidence that CONFIRMS a pre-existing suspicion rather than evidence that DISPROVES it?"

```
Pre-existing suspicion entering this cycle:
  "[What I expected to find before starting]"

Counter-evidence question:
  "What would I need to see to conclude this suspicion is WRONG?"
  → [Specific signal, code pattern, or data point]

Did I actually search for that counter-evidence?
→ YES (document what I found) / NO → Do it now before proceeding
```

---

## Bias Check 3: Availability Bias (unchanged)

"Am I overweighting issues because they're recent or dramatic?"

```
Recent incident influencing this audit: "[incident]"
Causing overweight? YES → apply availability discount (-2 QBR for H-[ID])
```

---

## Bias Check 4: Automation Bias (unchanged)

"Am I accepting AI-generated findings too readily?"

→ See `references/04-automation-bias-guard.md` for full protocol.

---

## Bias Check 5: Self-Audit Confirmation Bias (unchanged from 9.0)

**Triggered when**: SELF_AUDIT flag = YES.

"Am I auditing my OWN prior work? My brain wants to confirm it's correct."

→ Full [G.SCR] Same-Cycle Re-Audit Protocol below.

---

## 🆕 Bias Check 6: Organizational Capture Bias (v10.0)

**Triggered when**: ORG-CONTEXT = auditor's own team (declared at [P]).

"Am I auditing code owned by my OWN team? If yes, professional incentives
create implicit pressure to find it clean — regardless of its actual quality."

### The Organizational Capture Dynamic

Distinct from self-audit (Check 5) in a critical way:
- Self-audit (Check 5): Auditor wrote this specific code. Direct personal stake.
- Org Capture (Check 6): Auditor's team wrote it. Indirect professional stake.

The capture mechanism:
- Finding serious bugs in your team's code → perceived as team failure
- Reporting team bugs publicly → creates friction with teammates
- Clean audit of team code → reinforces team reputation
- Team velocity/roadmap depends on audit passing cleanly

This creates implicit (not conscious) pressure to apply lower scrutiny.
Accounting audit literature calls this "familiarity threat" and mandates rotation.
Software engineering has no equivalent — VHEATM 10.0 introduces it.

### Bias Check 6 Protocol

```
Step 1: Acknowledge the dynamic explicitly.

"This code is owned by [my team].
Professional capture risk: [HIGH if team lead is stakeholder; MEDIUM if peer
review; LOW if I am isolated from team's incentives in this audit]."

Step 2: Apply +15% QBR skepticism to all findings in own-team code.

Effective_QBR = Computed_QBR × 1.15

(Note: if SELF_AUDIT is also YES, total skepticism = × 1.20 × 1.15 = × 1.38
 — do not stack multiplicatively, cap at × 1.40 to avoid over-correction)

Step 3: Capture-Specific Counter-Questions

For each hypothesis about own-team code that you are DECLINING to escalate:
  □ "If a stranger's team wrote this code, would I rate it the same?"
  □ "Would I give this same benefit of the doubt to Team Y if they submitted it?"
  □ If answer is NO to either → re-evaluate.

Step 4: Declare captured-scope findings explicitly.

In the output, mark findings about own-team code:
  org_capture_scope: true
  skepticism_applied: +15%

This makes the capture scope visible to any reviewer.
```

### When Capture Risk is HIGH

If the auditor is also:
- A team lead of the code-owning team
- Evaluated on this team's quality metrics
- Responsible for the code's prior design decisions

→ Apply +20% instead of +15% (same as Self-Audit skepticism).
→ Recommend a [M]-phase Stranger Review specifically from outside the team.
→ Note in handoff: "Org Capture HIGH — external reviewer recommended before implementation."

---

## [G.SCR] Same-Cycle Re-Audit Protocol (unchanged from 9.0)

→ Full protocol below (5 steps):
1. Identify prior outputs
2. Apply +20% QBR skepticism
3. Document-Code Cross-Check for every claimed prior fix
4. Anti-Pattern Sweep on prior work
5. Adversarial Mirror

---

## Bias Probe Output (updated for 6 checks)

```
Biases detected this cycle:
  Check 1 (Anchoring): [result + adjustment if any]
  Check 2 (Confirmation): [counter-evidence searched + result]
  Check 3 (Availability): [incident noted + discount if any]
  Check 4 (Automation): [ABG reference]
  Check 5 (Self-Audit): [SCR protocol completed? YES/NO — triggered? YES/NO]
  🆕 Check 6 (Org Capture): [triggered? YES/NO — risk level? HIGH/MEDIUM/LOW
                              — +15/20% skepticism applied to: [scope]]

Confidence adjustments applied:
  "+20% skepticism on all H-IDs — self-audit"
  "🆕 +15% skepticism on [scope] — org capture"
  "(or: No adjustment needed)"

Log to [KB] Auditor Bias History:
  | [Cycle] | [Bias types] | [Adjustments] | [Finding impact] | [Self-Audit?] | [🆕 Org Capture?] |
```

---

## FAST Mode — Abbreviated Version

Run only:
- Bias Check 1 (Anchoring) — always
- Bias Check 5 (Self-Audit) — if SELF_AUDIT = YES
- 🆕 Bias Check 6 (Org Capture) — if ORG-CONTEXT = own team

Document: "Bias probe abbreviated — FAST mode. Checks 2-4 deferred."

---

*Reference 02 — VHEATM 10.0 | v9.0 + Bias Check 6 Org Capture*
