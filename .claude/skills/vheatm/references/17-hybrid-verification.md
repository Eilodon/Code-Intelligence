# Hybrid Verification — [E.HV] (v11.0)

> **Research Basis:**
> [Du et al. 2026 (Tencent)](https://consensus.app/papers/details/55adca2d59fc54fa81e92099ec30fcea/):
> first industrial-scale study of LLM-based false alarm reduction at Tencent.
> 433 alarms (328 FP, 105 TP). Hybrid LLM+static techniques eliminate **94-98% of false
> positives** with high recall. Cost: 2.1-109.5 seconds and $0.0011-$0.12 per alarm.
> "Hugely potential of LLMs for reducing false alarms in industrial settings."
>
> [Gnieciak et al. 2025 (3 citations)](https://consensus.app/papers/details/898b91f3d86f577b964417ea163f6ed7/):
> LLMs achieve F1=0.75-0.79 (higher recall), traditional static tools F1=0.26-0.55
> (higher precision). "Recommend hybrid pipeline: LLMs early for broad context-aware
> triage; deterministic scanners for high-assurance verification."
>
> [Nunes et al. 2025](https://consensus.app/papers/details/2bad6790f4ff5613a2319c4adee13e56/):
> SA+DA combined confirms/refutes **76.7%** of SA findings — "decreasing tremendously
> the usual need for manual work."
>
> [LLM4PFA Du et al. 2025 (1 citation)](https://consensus.app/papers/details/4955f199da1d5493b971b62895354b60/):
> LLM-enhanced path feasibility analysis filters **72-96%** of false positives,
> outperforming baselines by 41.1-105.7%. Misses only 3/45 true positives.
>
> **Core insight:** VHEATM [E] traditionally relies on a single verification pass
> (STATIC or LIVE). Research consistently shows neither pure approach is sufficient.
> [E.HV] formalizes a 3-step pipeline: confidence-rated static → dynamic confirmation
> for uncertain findings → optional LLM triage for large candidate lists.

---

## When [E.HV] Applies

```
MANDATORY (run for all MANDATORY findings, HG-HV gate):
  Any finding where QBR ≥ 17 and you are issuing a MANDATORY ADR.

RECOMMENDED:
  Any finding where you have LOW/MEDIUM static confidence.
  Any finding in a large codebase where manual review is the bottleneck.

NOT APPLICABLE:
  FAST mode (use abbreviated confidence rating only).
  Findings that have already been dynamically confirmed by production incident.
  DESIGN mode (no runnable code).
```

---

## Step 1 — Static Confidence Rating

Before issuing any MANDATORY ADR, rate your static confidence:

```
After [E] static analysis / code reasoning, assign confidence:

HIGH confidence — direct evidence in code:
  □ Bug anchor is a specific file:line showing the anti-pattern unambiguously
  □ You have read the actual code (not reasoned about it)
  □ The bug can be triggered by a simple, deterministic input
  □ No significant path complexity between trigger and failure
  → Can proceed to ADR. Document: "Static confidence: HIGH. Dynamic confirmation deferred."

MEDIUM confidence — indirect or uncertain evidence:
  □ Bug requires reasoning across multiple files
  □ Trigger path has conditional branches that may not always execute
  □ The bug was identified by pattern-matching, not direct read
  □ You have not read all relevant files in this session
  → Proceed to Step 2 (dynamic confirmation). Do NOT issue MANDATORY ADR yet.

LOW confidence — theoretical or inferred:
  □ Bug is plausible based on architecture reasoning
  □ No direct evidence in code — inferred from documentation or prior experience
  □ Bug would require unusual runtime state to trigger
  → MUST complete Step 2 before issuing any ADR ≥ REQUIRED priority.
  → If dynamic confirmation is not possible (DESIGN mode, no test env): downgrade to
    REQUIRED and document: "Confidence: LOW. Dynamic confirmation not possible in this mode."
```

---

## Step 2 — Dynamic Confirmation

For MEDIUM/LOW confidence MANDATORY findings:

```
Option A — Test-based confirmation (preferred):
  □ Write a minimal test that directly triggers the suspected bug.
  □ The test should: (1) fail before any fix, (2) pass after the fix.
  □ If the test cannot be written → document WHY (environment, state requirements).
  → If test FAILS as expected: promote to HIGH confidence. Issue ADR.
  → If test PASSES (bug not triggered): downgrade finding to FALSE_POSITIVE.
    Document: "[H-ID] FP via [E.HV] Step 2: test [test name] did not trigger."
    Add to AI FP Catalog (if AI-assisted audit) or SNF removal.

Option B — Live system observation (if LIVE/ENTERPRISE mode):
  □ Identify a metrics signal or log pattern that would appear if the bug fires.
  □ Observe the running system for the expected signal.
  → Signal present: promote to HIGH confidence.
  → Signal absent: MEDIUM downgrade (not conclusive either way without more time).
  → Document observation window and result.

Option C — Symbolic/Formal reasoning with constraints (when test not possible):
  □ Trace the full data flow from trigger to failure, enumerating every branch.
  □ Confirm no branch prevents the failure on the identified trigger input.
  □ Cite ProveNFix-style typestate: "resource state OPENED → not CLOSED → leak confirmed"
  → This upgrades MEDIUM to HIGH if path is complete and unambiguous.
  → Does not upgrade LOW (too many unknowns to close symbolically).
```

---

## Step 3 — LLM Triage (Optional, for large candidate lists)

When an audit cycle produces > 10 candidate findings awaiting verification:

```
Step 3 is OPTIONAL and applies to Standard/Full modes with large hypothesis lists.
It does NOT replace Step 1/2 for MANDATORY findings — it filters REQUIRED/OPTIONAL candidates.

Protocol:
  □ For each REQUIRED/OPTIONAL candidate, ask (structured prompt):
    "Given this code: [snippet]. Is this a genuine bug or a false positive?
     Rate: LIKELY_BUG / LIKELY_FP / UNCERTAIN. State your reasoning."
  □ Route:
    LIKELY_BUG → proceed to full [E] verification
    LIKELY_FP → deprioritize; document in AI FP Catalog if AI-assisted
    UNCERTAIN → manual review required; do not skip

  Automation Bias Guard applies: do NOT accept LIKELY_FP without checking the reasoning.
  If reasoning cites "this pattern is safe" → verify that claim against actual code.

Evidence basis: Du 2026 (94-98% FP reduction), LLM4PFA 2025 (72-96% FP filter,
misses only 3/45 true positives). Cost-effective: orders of magnitude cheaper than
manual review.
```

---

## [E.MS] Mutation Score Check (companion protocol)

After confirming a MANDATORY finding and issuing its ADR:

```
Mutation Score Check — optional but RECOMMENDED for MANDATORY:

  □ Does a test suite exist for the affected module? (from [P] Test Existence Check)
  □ If yes: is the mutation score known?
    - mutation_score ≥ 80%: test suite can detect this bug class → ADR sufficient
    - mutation_score 60-79%: marginal coverage → recommend adding targeted mutation test
    - mutation_score < 60%: test suite will NOT reliably detect regression
      → ADR requires a "Test Gap" companion recommendation
    - mutation_score UNKNOWN: document "mutation score not measured"
      → Add note: "consider running mutmut (Python) / cargo-mutants (Rust) / PIT (Java)"

  ADR field (optional): test_suite_mutation_score: [%] | UNKNOWN
  Companion recommendation if < 60%:
    "Test Gap: module [X] has mutation score [N]%. Fix may regress.
     Recommended: add property-based or boundary tests targeting [bug class]."

Evidence basis:
  Just et al. 2014 (629 citations, FSE): mutant detection correlates significantly
  with real fault detection, independently of code coverage.
  Petrovic et al. 2021 (Google, 54 citations): developers with mutation feedback
  write measurably higher-quality tests.
  Baker et al. 2013 (89 citations, IEEE Trans. SE): mutation testing finds issues
  in safety-critical software even AFTER 100% structural coverage.
```

---

## [E.HV] Output in ADR

```yaml
hybrid_verification:
  static_confidence: HIGH | MEDIUM | LOW
  static_evidence_anchor: "[file:line — must be a READ, not reasoned]"
  dynamic_confirmation:
    required: true | false
    method: test | live-observation | symbolic | deferred
    result: CONFIRMED | FALSE_POSITIVE | INCONCLUSIVE | DEFERRED
    evidence: "[test name / log signal / symbolic trace]"
  mutation_score:
    score: [%] | UNKNOWN
    test_gap_flag: true | false
  final_confidence: HIGH | MEDIUM | LOW
  adr_proceed: true | false
  downgrade_reason: "[if false: why ADR priority was reduced]"
```

---

## [E.HV] in FAST Mode (abbreviated)

```
FAST [E.HV]:
  For each finding, before issuing ADR: rate confidence HIGH/MEDIUM/LOW.
  If LOW: downgrade ADR to REQUIRED. Document "Low confidence — needs dynamic confirmation."
  If MEDIUM on MANDATORY: flag "HV deferred — needs confirmation before implementation."
  Skip Step 2 and 3 (time constraint).
```

---

## Verify-Before-Claim (companion rule — from ABG Guard 3)

The most common source of LOW confidence in AI-assisted audits:
Evidence Anchors issued from reasoning, not actual file reads.

```
Before issuing ANY Evidence Anchor (file:line):
  □ "Did I read this file in the current session?" YES → proceed.
  □ "Am I citing this from memory, prior context, or reasoning?" → READ THE FILE FIRST.

"Confident reasoning without reading" is an Automation Bias Guard violation
regardless of how certain the reasoning feels.
```

This rule lives in ABG Guard 3 (`references/04-automation-bias-guard.md`).
[E.HV] enforces it at the MANDATORY finding level as a Hard Gate check.

---

## Anti-Patterns

🚫 **"I'm confident it's a bug, skipping Step 2"** — Confidence is not evidence.
HIGH static confidence requires: actual file read + unambiguous code path.
Reasoning-based confidence is MEDIUM at best.

🚫 **"Step 2 test passed so there's no bug"** — One passing test doesn't rule out
all trigger conditions. Step 2 confirms or refutes a SPECIFIC trigger path,
not the entire bug hypothesis.

🚫 **"Mutation score is a research metric, not practical"** — Petrovic 2021 (Google):
deployed at scale, developers write better tests with mutation feedback.
"Not practical" means "we haven't set it up." Worth the setup cost for critical modules.

🚫 **"LLM triage in Step 3 is double-counting (I'm already using LLM)"** — Step 3
uses a DIFFERENT prompt with structured output and explicit FP/TP framing.
It's not the same as the original analysis pass.

---

*Reference 17 — VHEATM 11.0 | Du 2026 Tencent; Gnieciak 2025; Nunes 2025; LLM4PFA 2025;
Just 2014 mutation testing; Petrovic 2021 Google mutation deployment*
