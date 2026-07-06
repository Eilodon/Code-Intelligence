<!-- VHEATM 10.0: unchanged from 9.0 -->
# Automation Bias Guard — [E]

Required when: Audit is run by or with significant assistance from an AI agent.
Recommended even for human-only audits.

Academic basis: Automation bias — over-reliance on automated recommendations — is a critical challenge in AI-assisted decisions (Romeo et al., 2025 — 19 citations). Explanation mechanisms can reinforce misplaced trust rather than reduce it. Cognitive forcing interventions that require independent verification are more effective than passive awareness (Bertrand et al., 2022 — 128 citations).

---

## Guard 1: Self-Challenge Protocol

For each HIGH QBR hypothesis (QBR ≥ 17):

```
"What would a skeptical senior engineer say about this finding?"

Skeptic argument: "[Counter-argument — the strongest possible objection]"
My response: "[Why I maintain or downgrade this finding, with evidence]"
Verdict: MAINTAIN / DOWNGRADE to REQUIRED / REMOVE

If I cannot construct a meaningful skeptic argument → that's a signal
the finding may be under-examined. Probe deeper before maintaining MANDATORY.
```

---

## Guard 2: Training Data Recency Check

"Are my findings influenced by patterns from training data that don't reflect this specific codebase/context?"

Check each potential staleness:

```
□ Framework version assumptions:
  "Am I flagging this as outdated when the codebase intentionally uses this version?"

□ Language idiom assumptions:
  "Am I calling this a bug when it's idiomatic for this language/version?"

□ Domain assumptions:
  "Am I applying patterns from Domain A to Domain B incorrectly?"

□ Temporal assumptions:
  "Am I assuming a best practice from 2020 applies to a 2025 codebase?"
```

For each YES: explicitly note in finding — "Staleness risk — verify against current standards before treating as MANDATORY."

---

## Guard 3: Confidence Calibration + Verify-Before-Claim

"Do my QBR scores reflect actual evidence or confident-sounding reasoning?"

For each MANDATORY ADR:

```
Evidence actually reviewed: [specific files / docs / data / quotes]
Evidence I'm inferring from: [reasoning chain — be explicit]
Is confidence justified by evidence? YES / PARTIAL / NO

If PARTIAL: note caveat in ADR
If NO: downgrade from MANDATORY to REQUIRED until more evidence found
```

**Key test:** "Could I point a human reviewer to exactly where they should look to independently verify this finding?" If NO → confidence is not justified.

**🆕 Verify-Before-Claim gate (v11.0 — AI-native addition):**

Before issuing ANY Evidence Anchor (file:line citation):

```
□ "Did I READ this file in the current session?"
  YES → proceed to cite it.
  NO  → READ THE FILE NOW, then cite.

"Confident reasoning about a file I haven't read" = Automation Bias violation.
This applies even when:
  - The pattern is "obvious" from architecture
  - The file path was mentioned in the handoff
  - A similar pattern exists in other files I have read
  - I have seen this codebase many times before

Unread file → MEDIUM confidence by default, regardless of reasoning confidence.
MEDIUM confidence → [E.HV] Step 2 required before issuing MANDATORY ADR.
```

AI-specific false positive type this prevents: FP-AI-01 (Hallucinated Path),
FP-AI-02 (Misread Variable), FP-AI-05 (Stale Context). All three are rooted
in "confident reasoning without reading."

---

## Guard 4: Blind Spot Declaration

"What types of issues am I systematically less likely to find given my reasoning patterns?"

Declare explicitly for this audit:

```
My declared blind spots:
  □ Race conditions in concurrent code (hard to reason about statically)
  □ Business logic errors (require domain knowledge I may not have)
  □ Performance degradation under specific load patterns
  □ Security vulnerabilities requiring attacker mental model
  □ [Other — context-specific]

Mitigation:
  "For these blind spots, I explicitly recommend human review of:
   [specific sections / modules / scenarios]
   rather than relying on my analysis alone."
```

---

## Guard 4 — FAST Mode

In FAST mode, run Guard 4 only (Blind Spot Declaration) — 3 minutes maximum.
Document: "ABG Guards 1-3 deferred — FAST mode."

---

## ABG Output

```
Automation Bias Guard verdict: PASSED / ADJUSTED

Adjustments made:
  - [Finding ID] downgraded from MANDATORY to REQUIRED — confidence not justified by evidence
  - [Finding ID] staleness risk noted — verify against current standards
  - [Blind spots declared for human review]

(or: "PASSED — no adjustments needed after guard review")
```
