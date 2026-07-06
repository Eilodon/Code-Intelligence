# Phase Guide — Full VHEATM 15.0 Methodology

> **What's new vs 14.0 (Track S/T/U/V/W — v15.0):**
> [P] gains `AUDIT_TARGET_TIER` (1/2/3) — gate sets and protocol requirements scale with target
> maturity. [E] gains [E.IJ] Independent Judge (separate LLM context; mandatory for Tier 3 and
> SELF_AUDIT; reconciliation + divergence metrics). [M] gains [M.BA] Auditor Behavior Analysis
> (layer distribution, confirmation bias, thread abandonment, pattern staleness). Output gains
> Heuristic Acknowledgment note and Attestation Artifact with expiration. Hard Gates: 21 (+HG-IJ).
> Evidence: Petri (Anthropic/Meridian Labs), OSPS Baseline (OpenSSF), Scorecard V5 (OpenSSF).
>
> **What's new vs 13.1 (Track N/O/P/Q/R — v14.0):**
> [G.AB] expands to 3 modes: REACTIVE (unchanged), PROACTIVE (pre-emptive abduction for
> top MANDATORY hypotheses), VOCABULARY EXPANSION (layer proposals from OOV-AB clusters).
> [M] gains [M.EP] Per-Cycle Eigenstate Probe (12-question rotating pool, 3 per cycle).
> [M.AT] threshold calibration upgraded from fixed 65% to 3-phase evidence-anchored protocol.
> [KB] gains [KB.AD] Unified Accuracy Dashboard and [KB.CMA] Comparative Methodology Audit.
> ref 30 gains Parts 5-7. Hard Gates: 20 (unchanged). Reference files: 30 (unchanged).
>
> **What's new vs 13.0 (Track K/L/M — v13.1):**
> [G] gains [G.AB] Abductive Phase (symptom-driven backward inference — POST-INCIDENT or
> anomaly-triggered). [M] gains [M.AT] Mandatory Accuracy Tracker (production validation
> of MANDATORY findings; feeds accuracy_rate to KB). [KB.FST] gains INVARIANT-06 (Eigenstate
> Detection: protocol claim-vs-behavior consistency check). Hard Gates: 20 (unchanged —
> [G.AB] folds into HG-G conditional; [M.AT] folds into HG-M). Bug Class Catalog gains
> production_validated + false_positive_retroactive fields. ref 30 gains Part 4 (Empirical
> Accuracy Tracking + Revision Triggers). No new reference files added.
>
> **What's new vs 12.0 (Track G/H/I/J):**
> [P] gains `FRAMEWORK_VERSION`, `ORG_SIZE`, `EXECUTOR_DEPTH` declarations + [P.FD] Framework Diff check.
> [G] gains [G.AD] Auditor Defense scan (AI-S5; ref 28) and [G.H] L4 now uses canonical
> L4.1-L4.6 definitions (ref 27). [M] gains [M.EF] Execution Fidelity check (ref 29).
> [KB] gains [KB.EC] Evidence Currency check (ref 30). Hard Gates: 20 (was 18) — HG-AD + HG-EF added.
> FMEA RPN→QBR Detectability mapping corrected (see SKILL.md). CONTEXT_DENIED replaced by
> Independent Generation Protocol (ref 28 Part 2). Bug Class Catalog gains PERMANENT tier.
> Reference files: 30 (was 26). New refs: 27-l4-sublayers, 28-auditor-defense,
> 29-execution-fidelity, 30-framework-lifecycle.
>
> **What's new vs 11.0 (carried forward from v12):** [V] gains [V.AS] Architecture Smell scan.
> [G] gains [G.T] Temporal Scan Mode and L4 expands to L4.1-L4.6 sub-layers. [E] gains
> [E.HV] Hybrid Verification and [E.MS] Mutation Score. [P] gains Language, AI_INTEGRATED,
> Context_Budget declarations and Test Existence Check. [M.AP] Full mode gains Structured Lens
> Frame. [KB] gains AI False Positive Catalog. Hard Gates added: HG-AS, HG-HV.

## Table of Contents
1. [P] Pre-conditions (+ [P.FD], **🆕 AUDIT_TARGET_TIER**)
2. [V] Vision (+ [V.AS] Architecture Smell)
3. [G] Generation ([G.INC], [G.ORG], [G.CF], [G.SCR], [G.PG], [G.T], L4.1-L4.6, L6.7-L6.9, L7+L7.11, [G.AD], [G.AB] 3-mode)
4. [E] Experiment ([E.HV], [E.MS], **🆕 [E.IJ] Independent Judge**)
5. [A] Architecture Synthesis (9-part ADR)
6. [T] Transformation ([T.FV])
7. [M] Closure ([M.AP], [M.EF], [M.AT], [M.EP], **🆕 [M.BA] Behavior Analysis**, **🆕 Attestation**)
8. [KB] Knowledge Base (AI FP, [KB.EC], [KB.FST], [KB.AD], [KB.CMA])
9. QBR Formula
10. ADR 9-Part Template

---

## [P] Pre-conditions

**Purpose:** Establish audit context, parse prior state, set cost budget.

```
Checklist:
  □ Threat model — what could go wrong with this audit itself?
  □ ≥3 assumptions declared explicitly
  □ Handoff PARSED (not just reviewed) — extract H-IDs, debt items, open questions
  □ CLI budget calculated for this cycle (LCC if LEGACY; Enterprise formula if ENTERPRISE)
  □ Carryover debts reviewed from [KB]
  □ Context Mode declared (→ references/00-context-modes.md)
  □ Stakeholder declared
  □ Audit Scope Intent declared:
      BUG HUNT | GAP MAP | PRE-LAUNCH | POST-INCIDENT | FULL SPECTRUM
  □ SELF-AUDIT flag declared (YES if reviewing own prior cycle work)
  □ 🆕 ORG-CONTEXT declared: team that owns this code (if known)
  □ 🆕 LANGUAGE declared: rust | typescript-react | python | other | N/A
  □ 🆕 AI_INTEGRATED declared: YES | NO
  □ 🆕 CONTEXT_BUDGET declared: [tokens available | "unknown"]
  □ 🆕🆕 FRAMEWORK_VERSION declared: [prior cycle's VHEATM version | "first cycle"]
  □ 🆕🆕 ORG_SIZE declared: [<10 | 10-100 | 100-1000 | enterprise | N/A]
  □ 🆕🆕 EXECUTOR_DEPTH declared: [frameworks executor is competent in | "standard"]
  □ 🆕🆕🆕 AUDIT_TARGET_TIER declared: [1 | 2 | 3]
      Tier 1 (MVP/Prototype): reduced gate set; [G.AB] REACTIVE only; skip [M.EP], [KB.CMA]
      Tier 2 (Production): all 21 gates; [G.AB] REACTIVE+PROACTIVE; [M.EP] required
      Tier 3 (Critical Infrastructure): all 21 gates + [E.IJ] mandatory for MANDATORY findings;
        all 3 [G.AB] modes; [KB.CMA] required every 5 cycles
  □ Bug Class Catalog from [KB] loaded — top HOT classes ready for replay
  □ 🆕 AI FP Catalog from [KB] loaded — top FP types for this codebase (if AI-assisted)
  □ 🆕🆕 [P.FD] Framework Diff check: if FRAMEWORK_VERSION ≠ current → read SKILL.md changelog,
      generate H-FD-N hypotheses for new gates/phases (→ ref 30 Part 3)
  □ 🆕 Test Existence Check:
      - Test suite exists? YES / NO / PARTIAL
      - Coverage: [line%] | [branch%] | UNKNOWN
      - Mutation score (if known): [%] | UNKNOWN
      - Test type distribution: unit / integration / e2e / property-based
      → If NO tests: all RECOMMENDED ADRs → REQUIRED; all REQUIRED → MANDATORY
```

🆕 **ENTERPRISE activation check at [P]:**
If CONTEXT_MODE = ENTERPRISE:
- [G.INC] activates → declare org ownership map intent (will build at [V])
- [G.ORG] activates → note SLA documentation available?
- L7.11 activates → note applicable regulatory frameworks
- LCC activates for any legacy components

🆕 **Org Capture check at [P]:**
If ORG-CONTEXT = auditor's own team → Bias Check 6 activates at [G.B].

🆕🆕 **v13 activation checks at [P]:**
- `FRAMEWORK_VERSION` ≠ current → [P.FD] Framework Diff check (→ ref 30 Part 3)
- `ORG_SIZE` declared → [M.AM] size-aware calibration (→ ref 30 Part 2)
- `EXECUTOR_DEPTH` declared → Specialist Router depth-check (→ ref 28 Part 3)
- `AI_INTEGRATED = YES` → AI-S1 to **AI-S5** active (→ ref 11; ref 28 Part 1 for AI-S5)

🆕 **v11.0 activation checks at [P]:**
- LANGUAGE declared → load `references/19-language-profiles.md` for hypothesis pre-seeding
- AI_INTEGRATED = YES → L6.7-L6.9 activates at [G.H]; AI FP Catalog loaded at [KB]
- CONTEXT_BUDGET constrained → compression routing (→ references/18-ai-native-addenda.md Part 1)
- Test Existence Check: coverage = 0% → all priorities escalated one tier

---

## [V] Vision

**Purpose:** Map the system, establish what exists, update hypothesis lifecycle.

**Full mode:** Complete C4 diagram (Context → Container → Component → Code)
**Standard/FAST:** Delta from last cycle or high-level architecture summary

🆕 **ENTERPRISE mode: Org Ownership Map (required alongside C4)**
```
Org Ownership Map:
  Component/Service X → Team: [name] → On-call: [person]
  Component/Service Y → Team: [name] → On-call: [person]
  Shared: [components with joint ownership] → Escalation: [process]
  SLA commitments: [what's committed to whom]
  Regulatory scope: [frameworks that apply to which components]
```

This map is the foundation for [G.ORG] probes — build it here rather than
reconstructing it in the middle of an audit.

**Hypothesis Lifecycle Review** (every cycle):
- OPEN → still unverified
- STALE → conditions changed
- SUPERSEDED → newer finding replaces this
- RE-VERIFY → confirmed but codebase changed significantly

Exit: HG-V — C4/delta documented + hypothesis lifecycle updated

**🆕 [V.UT] ATAM Utility Tree Mini-pass (Standard+Full, runs before [V.AS]):**
Using ISO/IEC 25010 vocabulary, rank top 3-5 Quality Attributes for this system.
Define one concrete scenario per top QA (stimulus → response → measure → current state).
Identify 1-2 sensitivity points and 1 tradeoff point per scenario.
QA priority ranking feeds [V.AS] smell weighting (e.g., Reliability P1 → AS-01 × 1.5).
→ Full protocol: `references/23-atam-utility-tree.md`
Exit: HG-UT — QA priorities declared; [V.AS] weighting set
🆕 (ENTERPRISE) + Org Ownership Map documented
🆕 [V.AS] Architecture Smell scan complete → HG-AS gate

**🆕 [V.AS] Architecture Smell Scan (Standard+Full, end of Vision):**
Run AFTER architecture map is complete. Uses the dependency graph from C4/[V].
Scan 5 smells: AS-01 Cyclic, AS-02 Unstable Deps, AS-03 God Component,
AS-04 Scattered Concern, AS-05 Interface Segregation Violation.
FAST: AS-01 + AS-03 only.
Findings pre-seeded into [G.H] as H-AS-N hypotheses before generation begins.
→ Full protocol: `references/20-architecture-smells.md`

---

## [G] Generation

### [G.Pre] Differential Pre-mortem

Two temperatures:
- **COLD (first cycle):** "Assume complete failure in 6 months — list every way this could happen."
- **WARM (subsequent):** "Since last cycle, what new failure modes have emerged?"

🆕 **ENTERPRISE-specific pre-mortem question (add to all temperatures):**
"Assume this system failed due to an organizational problem, not a technical one.
What org-level failure caused it? (Teams didn't communicate? Incentives misaligned?
Compliance obligation unaddressed? SLA chain broke?)"

Track PM-accuracy: how many predictions materialized? Log to [KB].

---

### [G.H] Hypothesis Mapping + QBR

Generate hypotheses across **7 layers + L7.11**:
- L1: Input validation / contract violations
- L2: State management / data consistency
- L3: Cross-layer integration failures
- L4: Concurrency / timing — 🆕 **now 6 sub-layers: L4.1-L4.6**
  - L4.1 Data Races, L4.2 Deadlocks, L4.3 Atomicity Violations
  - L4.4 Order Violations, L4.5 Event-Race (mobile/async), L4.6 TOCTOU
  - Bare "L4" in new hypotheses = "L4-unclassified" — always prefer sub-layer
- L5: External dependency failures
- L6: Security / adversarial inputs (+ 🆕 **L6.7-L6.9** if AI_INTEGRATED = YES)
  - L6.7 Prompt Injection, L6.8 AI Output Injection, L6.9 Inference Mode Confusion
- L7: Cross-Cutting Concerns (rate limits, idempotency, timeouts, observability, resource lifecycle, authz, security headers, error cleanup, backpressure)
- 🆕 **L7.11: Compliance / Regulatory** (GDPR, PCI-DSS, HIPAA, SOC2 — see references/11-cross-cutting-layer.md)
- 🆕 **H-AS-N: Architecture Smell hypotheses** pre-seeded from [V.AS]

→ Full L7 + L7.11 + L6.7-L6.9 protocol in `references/11-cross-cutting-layer.md`

**[G.H.Replay] Bug Class Catalog Replay** (unchanged from 9.0):
Replay top HOT classes from [KB] Bug Class Catalog before generating new hypotheses.

**QBR Formula** (see Section 9).

🆕 **QBR calibration with ORG blast radius:**
After [G.ORG] probe (if applicable), re-check QBR's `blast_radius` input:
If BRS (Blast Radius Score) ≥ 8 and current `blast_radius` input < 3 → adjust to 3 and recalculate.

---

### 🆕 [G.FL] FMEA-lite Failure Ledger (v12.0)

**Trigger conditions (run when ANY apply):**
Safety-critical scope; hardware-software interface; state machine complexity ≥ 3 states;
any hypothesis with blast_radius = 3; POST-INCIDENT profile; [V.UT] Reliability scenario = NOT_SATISFIED.

**6-field format per failure mode:**
Component → Failure Mode → Cause → Local Effect → System Effect → RPN (S × O × D)
RPN ≥ 125 → MANDATORY | RPN 50-124 → REQUIRED | RPN 25-49 → RECOMMENDED

Apply only to critical-path components (Peeters 2018: selective application is key to
efficiency). Each failure mode above threshold → [G.H] hypothesis with `fmea_source: true`.
→ Full protocol: `references/24-fmea-lite.md`
Exit: HG-FL — FMEA-lite complete for triggered scope; findings seeded to [G.H]

---

### [G.CF] Compound Feature Decomposition (unchanged from 9.0)

→ Full protocol in `references/12-compound-feature-decomp.md`

---

### [G.SCR] Same-Cycle Re-Audit Protocol (unchanged from 9.0)

→ Full protocol in `references/02-bias-probes.md` → Bias Check 5 + SCR section

---

### 🆕 [G.INC] Incentive Misalignment Probe

**Triggered by**: Hypothesis confirmed in [E] (not a generation step — listed here for
discoverability, executed after [E] verification. See [E] section for execution trigger.)
**Required in:** Standard + Full modes. FAST: INC-1 (ownership) only.
**Mandatory in:** ENTERPRISE context mode.

Run the 3-question probe for each confirmed hypothesis:
1. **INC-1 Ownership**: "Who owns fixing this? Does it cross a boundary?"
2. **INC-2 Incentive**: "Does fixing this hurt the fixing team's metrics?"
3. **INC-3 Velocity**: "What's the realistic resolution time including org friction?"

BOUNDARY FLAG triggers:
- Automatic escalation of ADR to require handoff plan
- [G.ORG] Org Blast Radius probe
- ADR Owner + Boundary fields (mandatory)

→ Full protocol in `references/14-incentive-misalignment.md`

Exit gate: HG-INC — Incentive probe completed for all CONFIRMED hypotheses (Standard+Full)

---

### 🆕 [G.ORG] Organizational Blast Radius

**Triggered by**: Hypothesis confirmed as MANDATORY in [E], or BOUNDARY FLAG from [G.INC].
(Not a generation step — listed here for discoverability, executed after [E] verification.
See [E] section for execution trigger.)
**Required in:** Standard + Full modes.
**Mandatory in:** ENTERPRISE context mode.

Map 4 layers for the finding:
1. **Layer 1**: Immediate owner + on-call
2. **Layer 2**: Downstream teams and their impact
3. **Layer 3**: SLA chains at risk (internal + external + regulatory)
4. **Layer 4**: Regulatory / legal exposure

Calculate BRS (Blast Radius Score). If BRS ≥ 8: escalation triggered.

→ Full protocol in `references/15-org-blast-radius.md`

Exit gate: HG-ORG — ORG blast radius documented for all MANDATORY findings (Standard+Full)

---

### 🆕 [G.T] Temporal Scan Mode (v11.0)

**When:** After standard L1-L7 hypothesis generation. REQUIRED if persistent state in scope.
**Not a new layer:** Temporal is a re-read of existing layers through the time dimension.

5 temporal questions:
- **L-T.1 Accumulation:** "What state grows unboundedly? Where is it drained or capped?"
- **L-T.2 Date/Time Boundary:** "What happens at midnight, DST, year rollover, leap second?"
- **L-T.3 Session State Drift:** "Can state become stale between writes or sessions?"
- **L-T.4 Quota Exhaustion:** "What resets counters? Can reset fail to fire?"
- **L-T.5 Storage/Log Growth:** "What writes to storage? Is there an enforced retention policy?"

FAST: L-T.1 + L-T.4 only. Standard+Full: all 5.
Temporal hypotheses map to existing layers (L2, L7.5, L3, L1) — no new layer number.
L-T.3 has an async session lifecycle extension — see `references/16-temporal-scan-mode.md`
→ Full protocol: `references/16-temporal-scan-mode.md`

---

### 🆕 [G.CDOC] Changelog-to-Code Verification (v16.1)

**When:** Full mode (MANDATORY); Standard mode (RECOMMENDED); requires CHANGELOG or release
notes present in repo. Run once per audit cycle, after [G.H] generation, before [E].

**Gap closed:** Principle #2 "Documented ≠ Verified" existed as an axiom but not as a
procedure. VHEATM previously caught documentation-vs-code divergences by accident (pattern
matching while reading). [G.CDOC] makes this systematic: every CHANGELOG claim becomes
an explicit verification target.

```
Protocol:
  Step 1: Locate CHANGELOG / RELEASES.md / release notes for the version under audit.
          If absent: document "G.CDOC: no changelog found — skip" and continue.

  Step 2: Extract all feature and fix claims for this version.
          Claims include: "Added X", "Fixed Y", "Now supports Z", "Removed W",
          dependency changes, behavior changes.

  Step 3: For each claim, classify and verify:
    VERIFIED       — code found, behavior matches description in claim
    UNVERIFIABLE   — claim too vague to trace ("improved performance"),
                     or dependency not in audited codebase scope
    CONTRADICTED   — code found, behavior CONTRADICTS the claim
                     → generate L2/L4 hypothesis, cite CHANGELOG line as evidence

  Step 4: CONTRADICTED items enter [G.H] hypothesis pipeline with:
    evidence_anchor: "CHANGELOG [version] line [N]: claims X; code shows Y"
    layer: L2 (state) if data behavior differs; L4 (order/logic) if control flow differs
    confidence: HIGH (direct contradiction is T1 evidence — two sources disagree)
```

Output format:
```yaml
gdoc_verification:
  version: "[version audited]"
  changelog_location: "[file:line range]"
  claims_total: N
  verified: N
  unverifiable: N
  contradicted: N
  contradicted_items:
    - claim: "[exact claim text]"
      code_location: "[file:line]"
      discrepancy: "[what code actually does vs claim]"
      hypothesis_id: H-[N]  # generated hypothesis
```

FAST mode: skip unless CHANGELOG explicitly mentions the feature under audit.

---

### [G.I] Integration Shadow (unchanged from 9.0)

For each MANDATORY hypothesis: trace one layer up and one layer down.

---

### [G.I.R] Recursive Shadow (unchanged from 9.0)

For cascades from [G.I]: trace one more level.

---

### [G.Dep] Dependency DAG (unchanged from 9.0)

Map component dependencies. Flag circular deps and SPOFs.

---

### [G.U] Unknown Probe — 3 Techniques (unchanged from 9.0)

Techniques 1 (README vs Reality), 2 (Interface Contract), 3 (Spec Comparison).

🆕 **ENTERPRISE: Technique 4 — Org Boundary Gap Analysis**
"What does the system promise at team boundaries that isn't enforced?"
- What does Service A assume about Service B that isn't in the SLA?
- What does Team X think Team Y will handle that Team Y thinks X handles?
- Where is responsibility ambiguous or undocumented?

---

### [G.B] Auditor Bias Probe (now 6 checks)

→ Read `references/02-bias-probes.md` for full 6-check protocol.

| Check | Bias | When |
|---|---|---|
| 1 | Anchoring | Always |
| 2 | Confirmation | Always |
| 3 | Availability | Always |
| 4 | Automation | AI-assisted audits |
| 5 | Self-Audit Confirmation | SELF_AUDIT = YES |
| 🆕 6 | Organizational Capture | ORG-CONTEXT = own team |

FAST mode: Bias Check 1 (Anchoring) + Check 6 (Org Capture if applicable).

---

### [G.UX] User Journey Lens (unchanged from 9.0)

→ `references/03-ux-lens.md`

---

### [G.PG] Pattern Globalization (unchanged from 9.0)

→ `references/08-pattern-globalization.md`

---

### 🆕🆕 [G.AD] Auditor Defense Scan (v13.0 — HG-AD gate)

**Triggered when**: Standard + Full mode (always). FAST: inline check 6b.
**Run**: AFTER [G.H] hypothesis list formed, BEFORE [E] verification.
**Purpose**: Detect adversarial inputs in the artifact that may have biased hypothesis generation.

Five attack types scanned (AI-S5.1–5.5): instruction injection via comments, name-implementation
inconsistency, docstring lies, false fixture injection, salami validation claim chains.

Protocol:
1. Comment Pattern Scan — grep for audit-overriding instructions
2. Name-Implementation Consistency Check — security function names vs implementation
3. Docstring-to-Code Consistency Check — claimed behavior vs actual code
4. QBR Retroactive Audit — re-evaluate any MANDATORY hypothesis downgraded based on comment/docstring trust

→ Full protocol: `references/28-auditor-defense.md` Part 1
Exit: HG-AD — scan complete; YAML output produced; retroactive QBR adjustments made

---

### [G.SC] Scope Creep Protocol (unchanged from 9.0)

---

### 🆕🆕 [G.AB] Abductive Phase — 3-Mode (v14.0, expanded from v13.1)

> **Evidence basis**: Peirce CP 5.171: abduction = "the only logical operation which introduces
> any new idea." Schoenfisch et al. 2017 (ScienceDirect): MLN-based abductive RCA — "the most
> probable cause might not be the most obvious one." Magnani 2001: abduction is hypothesis-
> generative (not evaluative). VHEATM [G.H] is evaluative (IBE from taxonomy). [G.AB] is
> generative (from surprise or counterfactual).

**Three modes** (v14 — was single mode in v13.1):

---

**MODE 1: REACTIVE** (v13.1 — unchanged)

**Triggered when** (ANY of):
- POST-INCIDENT profile active AND incident not predicted by any H-ID in prior cycle
- Production anomaly reported: unexpected behavior, alert, symptom with no obvious cause
- [M.AT] abductive trigger fires: anomaly not in prior cycle's hypothesis vocabulary

**Protocol — four steps:**

```
Step 1: Symptom Decomposition
  Input: "[symptom description + context]"
  Decompose into observable properties:
    □ WHEN does it occur? (always / conditional: time, load, user type, data shape)
    □ WHAT is the scope? (all users / subset / single endpoint / specific region)
    □ WHAT VARIES between affected and unaffected cases?
    □ WHEN did it START? (after deploy / gradual / sudden)
  Output: symptom_properties YAML

Step 2: Abductive Candidate Generation
  For each symptom property identified in Step 1:
    □ Ask: "What system condition would make this property EXPECTED (not surprising)?"
    □ Generate candidates — prioritize by parsimony (fewest assumptions first)
    □ Do NOT filter by L1-L7 taxonomy at this step
  Output: candidate_list (unstructured, before taxonomy mapping)

Step 3: Taxonomy Routing
  For each abductive candidate:
    □ Map to nearest L1-L7 layer (or mark as OUT-OF-VOCABULARY if no match)
    □ In-vocabulary → generate H-AB-N hypothesis → feed into [G.H]
    □ OUT-OF-VOCABULARY → log as "OOV-AB-N" in [KB]
      → If ≥ 3 OOV-AB entries in this cycle → fire Paradigmatic Trigger (ref 30 Part 4)
      → Also trigger MODE 3 (Vocabulary Expansion) if ≥ 3 accumulated across recent cycles

Step 4: Parsimony Ranking
  Rank H-AB hypotheses: simpler explanations first (Ockham's razor)
  QBR: assign based on symptom severity, not unexpectedness of cause
```

Output format: `abductive_pass` YAML with symptom_properties, candidates, out_of_vocabulary.

---

**MODE 2: PROACTIVE** (v14.0 — NEW)

**Triggered when**: Standard + Full mode, AFTER [G.H] hypothesis list formed.
Not dependent on anomaly — runs proactively on top MANDATORY hypotheses.

**Purpose**: Test whether MANDATORY hypotheses survive counterfactual challenge.
"If this hypothesis is WRONG, what would we expect to see instead?"

```
Step 1: Select top 3 MANDATORY hypotheses from [G.H] by QBR score.

Step 2: For each MANDATORY hypothesis H:
  □ "If H is a FALSE POSITIVE, what alternative explanation fits the same code?"
  □ "What symptom would we expect if H is wrong?"
  □ "Is that alternative symptom or explanation present in the codebase?"

Step 3: Evaluate competing explanations
  □ If competing explanation IS present → H has credible alternative:
    → Downgrade confidence (not severity): flag for [E.HV] Step 2 mandatory
    → Note: competing explanation does NOT automatically downgrade QBR,
      it changes the CONFIDENCE that H is a true positive
  □ If competing explanation is NOT present → H's prediction is unchallenged:
    → Confidence maintained
  □ If competing explanation is STRONGER than H → H may be false positive:
    → Downgrade to RECOMMENDED and note: "abductive challenge — verify in [E]"

Output: H-AB-PRO-N entries with competing_explanation analysis
Time budget: 10 min (Standard), 20 min (Full)
```

**Output format:**

```yaml
abductive_proactive:
  hypotheses_challenged: 3
  results:
    - original_id: H-007
      original_qbr: 19
      competing_explanation: "Variable name 'sanitized_input' actually IS sanitized —
                              checked upstream in middleware"
      competing_evidence: "middleware.py:45 — input_validator() called before handler"
      verdict: COMPETING_STRONGER | COMPETING_WEAKER | NO_COMPETING
      action: DOWNGRADE_CONFIDENCE | MAINTAIN | FLAG_E_HV
    - ...
```

---

**MODE 3: VOCABULARY EXPANSION** (v14.0 — NEW)

**Triggered when**: Paradigmatic trigger from ref 30 Part 4 fires (≥ 3 OOV-AB entries
accumulated across recent cycles). This is the Kuhnian mechanism.

**Purpose**: When belt (L1-L7 taxonomy) can't absorb anomalies, propose core revision.

```
Step 1: Cluster OOV-AB entries by symptom type
  □ Group by: similar trigger conditions, similar system components, similar failure modes
  □ If cluster has ≥ 2 entries → viable candidate for new layer/sub-layer

Step 2: Draft layer proposal
  For each viable cluster:
    □ "What layer or sub-layer would NEED to exist to make these findings taxonomy-native?"
    □ Draft:
      - Layer name: L[X] or L[N].[M]
      - Trigger condition: when does this layer's hypothesis generation activate?
      - Hypothesis template: standard format for hypotheses in this layer
      - Search method: how to look for bugs in this category
      - Examples: at least 2 from OOV-AB cluster

Step 3: Evaluate layer necessity
  □ Is this genuinely new, or is it a sub-layer of an existing L1-L7?
  □ If sub-layer: propose as L[N].[M+1] addition to existing layer
  □ If genuinely new: propose as L8 (or L[N+1]) for v+1 framework discussion
  □ Document rationale: why does the existing taxonomy not cover this?

Output: layer_expansion_proposal for human review and v+1 discussion
```

**Output format:**

```yaml
vocabulary_expansion:
  trigger: "paradigmatic — ≥ 3 OOV-AB accumulated"
  clusters:
    - cluster_id: CLU-001
      oov_ab_entries: [OOV-AB-003, OOV-AB-007, OOV-AB-012]
      common_pattern: "[description of shared failure type]"
      proposed_layer:
        name: "L[X].[Y]: [descriptive name]"
        trigger: "[when to activate]"
        hypothesis_template: "H-[X]-NNN: [template]"
        search_method: "[how to find bugs of this class]"
        examples_from_oov: 2
      verdict: NEW_SUBLAYER | NEW_LAYER | REJECT_INSUFFICIENT_EVIDENCE
  recommendation: "Add to v+1 framework discussion | Defer — insufficient cluster size"
```

Exit condition: all active modes complete; H-AB-N and H-AB-PRO-N fed into [G.H];
OOV count logged; Mode 3 proposal documented if triggered.

---

### [G.D] Debate Gate + Cognitive Break (unchanged from 9.0)

Exit: HG-G + HG-CF + HG-PG + HG-INC (Standard+Full) + HG-ORG (Standard+Full, MANDATORY) + HG-AD (Standard+Full) + HG-G includes [G.AB] completion (all active modes)

---

## [E] Experiment (updated v11.0)

**Purpose:** Verify hypotheses with evidence. Every claim anchored.

After ANY hypothesis verified as real bug → immediately run [G.PG] (Pattern Globalization).
After ANY hypothesis verified as CONFIRMED (Standard+Full) → immediately run [G.INC].
After ANY hypothesis verified as MANDATORY → additionally run [G.ORG].

**🆕 Verify-Before-Claim (v11.0 — from ABG Guard 3):**
Before issuing any Evidence Anchor → confirm the file was actually read this session.
Reasoning about unread files → MEDIUM confidence → [E.HV] Step 2 required.

**Execution sequence per confirmed hypothesis:**
```
[E] verifies hypothesis
  → ABG Guard 3 (Verify-Before-Claim: was file READ or REASONED?)
  → [G.PG] Pattern Globalization (all modes — blocking for [A])
  → 🆕 [E.HV] Hybrid Verification (Standard+Full, MANDATORY — blocking for [A])
      Step 1: Rate static confidence HIGH/MEDIUM/LOW
      Step 2: if MEDIUM/LOW → dynamic confirmation before ADR
  → [G.INC] Incentive Probe (Standard+Full — blocking for [A])
  → [G.ORG] Org Blast Radius (Standard+Full, MANDATORY only — blocking for [A])
  → 🆕 [E.MS] Mutation Score Check (optional — add to ADR if run)
  → 🆕🆕🆕 [E.IJ] Independent Judge (Tier 3 mandatory; Tier 2 recommended for MANDATORY)
      Separate LLM call — clean context, no hypothesis trace, no taxonomy exposure.
      Judge receives: finding description + code snippet + claimed severity.
      Judge does NOT receive: H-ID, layer, QBR calc, prior cycle KB, Pre-mortem.
      Judge independently rates: real bug? severity? confidence?
      Reconciliation: if judge diverges from auditor → flag for human review.
  → proceed to [A] ADR
```

→ Full [E.HV] + [E.MS] protocol: `references/17-hybrid-verification.md`

### 🆕🆕🆕 [E.IJ] Independent Judge Protocol (v15.0)

> **Evidence basis**: Petri framework (Anthropic/Meridian Labs) — Auditor Agent and
> Judge are separate entities; Judge scores transcripts independently to reduce
> hallucination and eigenstate. Pattern proven in jurisprudence (prosecution ≠ judge),
> peer review (author ≠ reviewer), financial audit (preparer ≠ auditor).
>
> **Gap closed**: v13.1-v14.0 eigenstate detection ([M.EP], INVARIANT-06, [KB.CMA])
> all execute within the SAME entity that performed the audit. [E.IJ] introduces a
> genuinely independent assessment — different context, different entity, no prior reasoning.

**When required:**
- AUDIT_TARGET_TIER = 3: MANDATORY for all MANDATORY-level findings
- AUDIT_TARGET_TIER = 2: RECOMMENDED for MANDATORY findings, optional for others
- AUDIT_TARGET_TIER = 1: Skip
- SELF_AUDIT = YES: MANDATORY regardless of tier

**Protocol:**

```
[E.IJ] Independent Judge — 4 steps:

Step 1: Prepare judge input (strip auditor reasoning)
  For each finding to be judged:
    □ Extract: description, code_snippet, claimed_severity
    □ Strip: hypothesis ID (H-xxx), layer reference (L1-L7), QBR calculation,
      prior cycle KB data, pre-mortem predictions, taxonomy context
    □ Output: judge_input.yaml — clean finding description only

Step 2: Judge call (separate context)
  □ New LLM session — no conversation history from audit
  □ System prompt: "You are an independent code security reviewer.
    Assess each finding below. For each, state:
    1. Is this a real vulnerability/bug? (YES / UNCERTAIN / NO)
    2. If real, severity? (CRITICAL / HIGH / MEDIUM / LOW)
    3. Confidence in your assessment? (0-100%)
    Respond with ONLY your assessment. Do not explain methodology."
  □ No tools, no search, no framework — pure assessment from code + description

Step 3: Reconciliation
  For each finding:
    auditor_severity vs judge_severity:
    □ ALIGNED: auditor MANDATORY + judge CRITICAL/HIGH → CONFIRMED
    □ SOFT_DIVERGENCE: auditor MANDATORY + judge MEDIUM → FLAG (review recommended)
    □ HARD_DIVERGENCE: auditor MANDATORY + judge LOW/NO → ESCALATE
      → Mandatory human review before ADR
      → Log in [KB.AD] as judge_divergence_event
    □ UPGRADE: auditor RECOMMENDED + judge CRITICAL/HIGH → consider upgrade to MANDATORY

Step 4: Metrics
  judge_alignment_rate = ALIGNED / total_judged
  judge_divergence_rate = (SOFT + HARD) / total_judged
  Log in [KB.AD]:
    □ If judge_divergence_rate > 30% over 3 cycles → eigenstate signal
      (auditor is systematically miscalibrating — not just on individual findings)
    □ If judge_divergence_rate < 10% → judge may not be independent enough
      (check: is judge system prompt biased toward agreement?)
```

**Output format:**

```yaml
independent_judge:
  cycle: N
  findings_judged: N
  results:
    - finding: "[description]"
      auditor_severity: MANDATORY
      judge_verdict: YES
      judge_severity: HIGH
      judge_confidence: 85
      reconciliation: ALIGNED
    - finding: "[description]"
      auditor_severity: MANDATORY
      judge_verdict: UNCERTAIN
      judge_severity: MEDIUM
      judge_confidence: 45
      reconciliation: SOFT_DIVERGENCE
      action: "human review recommended"
  metrics:
    alignment_rate: "X%"
    divergence_rate: "X%"
    hard_divergence_count: N
```

**Anti-Patterns:**

🚫 **"Judge receives the full audit context for better assessment"** — defeats the purpose.
Judge independence requires IGNORANCE of auditor reasoning. If judge knows which layer
generated the hypothesis, judge will anchor to the same taxonomy.

🚫 **"Judge divergence means the judge is wrong"** — divergence is a SIGNAL, not a verdict.
It means auditor and judge see the same code differently. Human decides who's right.

🚫 **"Skip [E.IJ] because it costs an extra LLM call"** — the entire point of [E.IJ] is
that eigenstate can't be solved within the same context. One extra call per MANDATORY
finding is a small cost for genuine independence.

Exit: HG-E + HG-HV + HG-IJ (Tier 2-3: judge reconciliation complete; divergences logged)

---

## [A] Architecture Synthesis

**Purpose:** Convert verified hypotheses into actionable decisions.

### Opposing ADR (unchanged)

For each MANDATORY ADR: write strongest counter-argument first. Rebut with evidence.

### Signal-to-Noise Filter (unchanged)

→ `references/05-signal-noise-filter.md`

### Core ADR — 🆕 9 Parts (was 7 in 9.0)

```
ADR-[ID]: [Title]
Priority: MANDATORY | REQUIRED | RECOMMENDED | OPTIONAL
Bug Anchor: [file:line] or [Evidence Tier + source]
Fix Anchor: [PLACEHOLDER — filled at [T] after applying]

1. Context: What situation prompted this ADR?
2. Decision: What is being decided?
3. Rationale: Why this decision over alternatives?
4. Consequences: What changes as a result?
5. Revert Blast Radius: TRIVIAL | HARD | IMPOSSIBLE
6. Observable Success Criteria: [measurable condition]
7. Pattern Globalization: Siblings found? Fix cascade? Bug Class ID.
🆕 8. Owner: [Primary team/role responsible for implementing this fix]
     If boundary crossing: [Team A implements] + [Team B integrates/approves]
🆕 9. Boundary: YES / [type: TECHNICAL | HANDOFF | APPROVAL | DISPUTED] | NO
     If YES: Handoff plan: [explicit sequence — who does what in what order]
             Stakeholder sign-off required: YES | NO
             BRS: [N] — [CONTAINED | ELEVATED | HIGH | CRITICAL]
```

**Part 8 Owner rules:**
- "Unknown" is an acceptable entry but MANDATORY if unknown → add to debt register as EP-01 (Orphaned component)
- Joint ownership allowed: "Team A (primary) + Team B (sign-off)"
- Self-ownership declaration required: "Auditor's team owns this — Bias Check 6 applied"

**Part 9 Boundary rules:**
- If BOUNDARY = YES and MANDATORY → handoff plan is REQUIRED (not optional)
- If BOUNDARY = YES and DISPUTED → escalate to leadership before ADR proceeds
- If BRS ≥ 8 → leadership visibility note added

### Integration Cascade Map (unchanged)

→ Includes fix cascade from [G.PG] sibling instances.

Exit: HG-A — Opposing ADR + SNF + 9-part ADRs (with Owner + Boundary) + Revert BR

---

## [T] Transformation (unchanged from 9.0)

Red-Green Gate + Cascade Verification + [T.FV] Fix Verification.

🆕 **Boundary crossing check at [T]:**
Before marking any BOUNDARY=YES ADR as "applied":
```
□ Did Team B receive and acknowledge the change?
□ Did the handoff plan execute as documented?
□ Is Team B's integration confirmed (not just Team A's code change)?
```

Exit: HG-T + HG-FV

---

## [M] Closure

### Verifiable Metrics + Layer Feedback + Bug Class Catalog Update + AI FP Catalog Update

(Unchanged from v11)

### 🆕 [M.AM] Assurance Maturity Overlay (v12.0 — Full mode + recurring patterns)

**When:** Full mode AND (recurring pattern found OR ENTERPRISE mode OR POST-INCIDENT with systemic root cause).

For each MANDATORY/REQUIRED finding, run per-finding maturity delta:
1. Classify: ONE-OFF DEFECT / RECURRING PATTERN / MISSING CONTROL / PROCESS FAILURE
2. Map to SAMM function (Governance/Design/Implementation/Verification/Operations)
3. Map to NIST SSDF practice group (PO/PS/PW/RV)
4. Check against BSIMM 12 baseline activities — "is this below industry baseline?"
5. Issue improvement recommendation (IMMEDIATE/SHORT_TERM/LONG_TERM)

Output: maturity delta per finding — NOT a maturity score.
→ Full protocol: `references/25-assurance-maturity-overlay.md`

### 🆕 [AI-RMF] NIST AI RMF MANAGE + GOVERN Closure (v12.0 — Full mode + AI_INTEGRATED)

**When:** AI_INTEGRATED = YES AND Full mode.

MANAGE closure:
- For each AI-S finding: is there a monitoring/detection plan? If NO → Detection Gap ADR
- For each AI-S ADR: rollback plan? If NO → debt register as Rollback Gap
- Is there drift monitoring for AI model behavior change? If NO → flag

GOVERN closure:
- Review findings: do any reveal AI decisions without appropriate human oversight?
- Is governance process defined for AI policy updates? If NO → flag for [M.AM]

→ Full protocol: `references/26-nist-ai-rmf-overlay.md`

### Human Review Required Tag

When AI auditor encounters a finding that requires human judgment:
```
HUMAN-REVIEW REQUIRED conditions:
  □ Business logic depends on unstated company policy
  □ Safety/compliance impact is HIGH with no live metrics available
  □ Finding depends on organizational/political incentives not in scope
  □ Intent is unclear but decision is irreversible
  □ AI confidence is LOW and [E.HV] Step 2 not possible in this session

Tag format: HUMAN-REVIEW: [reason] — [what a human should verify]
```

This tag does NOT downgrade finding priority — it escalates human attention.

### 🆕 [M.AP] Adversarial Pass — Multi-perspective (Full mode)

**Standard mode**: 4 standard lenses (Pattern, Self, Cross-Cutting, Compound) — unchanged from 9.0.

**Full mode**: Each lens is now assigned a stakeholder perspective for multi-perspective coverage:

```
Full mode — Multi-Perspective Adversarial Pass:

Lens 1 — Pattern Lens (SRE perspective):
  "What failure modes would I as an SRE see that the developer missed?"
  → Focus: operational failure patterns, cascading failures, resource exhaustion

Lens 2 — Self Lens (Security perspective):
  "What attack surface does this code create that a security engineer would flag?"
  → Focus: new endpoints, data flows, auth changes, third-party integrations

Lens 3 — Cross-Cutting Lens (Compliance perspective):
  "What regulatory obligation could this code trigger or violate?"
  → Focus: L7.11 compliance scan, data retention, audit trails, breach notification

Lens 4 — Compound Lens (Product perspective):
  "What user-facing failure does this create that a product manager would be horrified by?"
  → Focus: UX failures, data correctness for business decisions, customer trust impacts
```

🆕 Lens 5 (ENTERPRISE mode only):

```
Lens 5 — ORG Lens (Cross-team perspective):
  "What bug would only be visible to someone sitting in a different team?"
  → Focus: ownership gaps, SLA boundary assumptions, cross-team integration failures
```

Time budget per lens (Full mode):
- Standard: 15 min total ÷ 4 lenses = ~3.75 min/lens
- Full: 30 min total ÷ 4 lenses = ~7.5 min/lens (or ÷ 5 for ENTERPRISE)

→ Full protocol in `references/10-adversarial-pass.md`

### Persona Rotation Stranger Review (updated for ENTERPRISE)

Standard 4 rotations (unchanged from 9.0):
1. Senior Engineer — "technically correct?"
2. Security Auditor — "attack surface?"
3. Junior Engineer — "implementable from ADR alone?"
4. Ops Engineer — "incident response impact at 2am?"

🆕 5th rotation for ENTERPRISE mode:
5. Team B Engineer (receives a BOUNDARY-crossing fix) — "does this ADR make sense from outside Team A? What's missing in the handoff?"

---

### 🆕🆕 [M.EF] Execution Fidelity Check (v13.0 — HG-EF gate)

**Run**: At [M] BEFORE issuing final handoff. Required in all modes.
**Purpose**: Mechanically enforce Ritual Suppression Rule; set accurate cycle_status.

1. **Ritual Suppression Checker**: Every phase section must have ≥1 output marker
   (DECISION / FINDING / DOWNGRADE / DEBT / NEXT_ACTION / SKIP_REASON / ADR).
   Sections with only prose → flag CEREMONY → revise or remove.
   Output density target: ≥ 85% sections with output markers.

2. **cycle_status**: Set accurately — COMPLETE | PARTIAL | HALTED | BUDGET_HALT.
   If ≠ COMPLETE: populate `interrupted_at_phase`, `phases_not_completed`,
   `critical_hypotheses_unverified`, `minimum_resume_action`.

3. **Finding Genealogy**: Each new ADR this cycle has `finding_genealogy` populated
   (research_basis.year, first_cycle, bug_class_id).

4. **Delta-Diff** (if prior cycle handoff exists): classify all prior H-IDs as
   RESOLVED / UNCHANGED / NEW / REGRESSED / SUPERSEDED / DOWNGRADED / NEVER_REACHED.
   REGRESSED findings → reopen at MANDATORY + trigger [G.SCR].

→ Full protocol: `references/29-execution-fidelity.md`
Exit: HG-EF — Ritual Suppression checked; cycle_status set; genealogy populated; delta-diff produced

---

### 🆕🆕 [M.AT] Mandatory Accuracy Tracker (v13.1)

**Run**: At [M] alongside [M.EF]. Required in Standard + Full mode.
**Purpose**: Track whether MANDATORY findings are empirically accurate in production.
Framework that produces uncalibrated MANDATORY findings misrepresents actual risk.

```
[M.AT] Protocol — three steps:

Step 1: Update prior findings
  □ For each MANDATORY finding closed in prior cycles with production_validated = unknown:
    □ Has a production incident been linked? → set production_validated: true, populate
      validation_source with incident/alert ID
    □ Has sufficient time passed (≥ 2 weeks post-deploy) with no incident? →
      set production_validated: unknown-aging (still unconfirmed, not a false positive)
    □ Has human reviewer determined finding would not have manifested? →
      set production_validated: false, set false_positive_retroactive: true

Step 2: Recalculate accuracy rate
  □ measured_accuracy_rate = validated_true / (validated_true + validated_false)
  □ Exclude unknown and unknown-aging from denominator
  □ Update mandatory_accuracy_tracker in [KB] YAML handoff

Step 3: Evaluate revision triggers (v14: calibrated threshold)
  □ Determine calibrated threshold per ref 13 + ref 30 Part 4:
    PRE-CALIBRATION (cycles 1-5): threshold = 60%
    EMPIRICAL (cycle 6+): threshold = (5-cycle baseline) × 0.85
    RECALIBRATED (annually): recalculated from last 12 months × 0.85
  □ If accuracy_rate < calibrated threshold: increment cycles_below_threshold
    □ If cycles_below_threshold ≥ 3: set current_status: TRIGGER
      → Debt-AT-[N]: "MANDATORY accuracy trigger — QBR threshold review required"
      → Human reviewer sign-off required before next v+1 discussion
  □ If this cycle's accuracy_rate ≥ threshold: reset cycles_below_threshold to 0

  □ If any production anomaly occurred not predicted by prior cycle H-IDs:
    → Log OOV-AB-[N] in [KB]; trigger [G.AB] REACTIVE in next cycle
  □ Update [KB.AD] accuracy dashboard (ref 30 Part 7)
```

→ Full protocol: `references/13-bug-class-catalog.md` [M.AT] section
   and `references/30-framework-lifecycle.md` Part 4
Exit: HG-M includes [M.AT] completion — accuracy_rate updated; triggers evaluated; KB YAML updated

---

### 🆕🆕🆕 [M.EP] Per-Cycle Eigenstate Probe (v14.0)

**Run**: At [M] AFTER [M.EF] and [M.AT], BEFORE final handoff.
**Cost**: 5 minutes (3 questions from 12-question rotating pool).
**Required in**: Standard + Full mode. Skip in FAST.
**Purpose**: Detect eigenstate formation between version releases. Lightweight complement
to INVARIANT-06 (which runs only at version release).

```
[M.EP] Protocol:

Step 1: Select 3 questions from rotation schedule (cycle mod 4):
  Cycle mod 4 = 1: Q1 (IGP isolation), Q5 (PG confirmation bias), Q9 (3 uncovered categories)
  Cycle mod 4 = 2: Q2 (QBR mechanical), Q6 (AP independence), Q10 (most likely error mode)
  Cycle mod 4 = 3: Q3 (AI-S5.3 self), Q7 (HV selection bias), Q11 (competing methodology)
  Cycle mod 4 = 0: Q4 (framework authority), Q8 (pre-mortem redundancy), Q12 (proto-abduction)

Step 2: Answer each question honestly. For each answer:
  □ Does the answer reveal an unverified assumption? → action: FLAG_DEBT
  □ Does the answer reveal a claim taken on framework authority? → action: REVERT_TO_E
  □ Does the answer confirm verified practice? → action: ACCEPT

Step 3: Set eigenstate_risk:
  LOW:    all answers show verified claims or honest uncertainty
  MEDIUM: 1 answer reveals unverified assumption → Debt-EP-[N]
  HIGH:   ≥2 answers reveal unverified assumptions →
          Debt-EP-[N] + flag for next cycle's [G.H] + consider [KB.CMA]

Step 4: Drift check (every 4th rotation — compare to same questions 4 cycles ago):
  □ Did the answer to the same question change substantively?
  □ If YES and shift toward more assumptions → eigenstate may be forming
  □ If YES and shift toward more verification → eigenstate may be dissolving
  □ If NO → stability check: is stability warranted or complacent?
```

→ Full question pool: `references/30-framework-lifecycle.md` Part 6
Exit: HG-M includes [M.EP] completion — eigenstate_risk set; drift check logged; [KB.AD] updated

---

### 🆕🆕🆕 [M.BA] Auditor Behavior Analysis (v15.0)

**Run**: At [M] AFTER [M.EP], BEFORE attestation and final handoff.
**Cost**: 5 minutes. Required in Standard + Full mode.
**Purpose**: Detect predictable auditor patterns that reduce audit effectiveness.

> **Evidence basis**: Petri framework (Anthropic/Meridian Labs) — early auditor versions
> fell into predictable patterns: leading questions, artificial scenarios, premature
> abandonment. Prompt refinement based on behavior transcript analysis fixed these issues.

```
[M.BA] Protocol — 4 checks:

Check 1: Layer distribution
  □ Which L1-L7 layers produced findings? Which produced nothing?
  □ If same layers produce nothing for 3+ consecutive cycles →
    Either: blind spot (layer relevant but auditor skipping)
    Or: layer irrelevant for this codebase (document + skip)

Check 2: Confirmation bias signal
  □ For each MANDATORY finding: was disconfirming evidence actively sought?
  □ Pattern to detect: "generate hypothesis → find confirming code → skip disconfirm"
  □ If > 50% of MANDATORY findings have no documented disconfirming search →
    FLAG: "leading question" anti-pattern (Petri terminology)

Check 3: Thread abandonment
  □ Count hypotheses generated vs hypotheses that reached [E] verification
  □ abandonment_rate = (generated - verified) / generated
  □ If abandonment_rate > 60% → auditor may be quitting too early
  □ If abandonment_rate < 20% → auditor may not be generating enough hypotheses

Check 4: Pattern staleness
  □ Compare this cycle's hypothesis types with prior 3 cycles
  □ If > 70% of hypothesis TYPES (not instances) are identical →
    "predictable pattern" signal — auditor needs prompt refinement
  □ Log: new_hypothesis_types_this_cycle / total_hypothesis_types
```

**Output format:**

```yaml
behavior_analysis:
  cycle: N
  layer_distribution:
    productive: [L1, L2, L4, L7]
    empty: [L3, L5, L6]
    empty_consecutive_cycles: {L5: 3, L6: 2}
    blind_spot_flags: ["L5 empty 3 cycles — investigate"]
  confirmation_bias:
    mandatory_findings_checked: N
    disconfirming_search_documented: N
    confirmation_bias_rate: "X%"
  thread_abandonment:
    hypotheses_generated: N
    hypotheses_verified: N
    abandonment_rate: "X%"
  pattern_staleness:
    hypothesis_types_this_cycle: N
    novel_types: N
    staleness_rate: "X%"
  overall_health: HEALTHY | WATCH | STALE
```

Exit: HG-M includes [M.BA] — behavior metrics logged; flags documented

---

### 🆕🆕🆕 Attestation Artifact (v15.0)

> **Evidence basis**: OSPS Baseline (OpenSSF) — tiered framework with point-in-time
> self-attestation: versioned, timestamped, with explicit expiration.

At [M] final exit, generate attestation:

```
VHEATM Attestation:
  project: [project_name]
  date: [audit date]
  framework_version: VHEATM [version]
  audit_target_tier: [1 | 2 | 3]
  audit_depth: [FAST | Standard | Full]
  findings:
    mandatory: N
    recommended: N
    informational: N
  accuracy:
    mandatory_accuracy_rate: "[X% | N/A if < 5 cycles]"
    calibration_phase: "[PRE-CALIBRATION | EMPIRICAL | RECALIBRATED]"
    judge_alignment_rate: "[X% | N/A if [E.IJ] not run]"
  heuristic_note: >
    ⚠️ VHEATM findings are heuristic. False positives and false negatives
    are expected. Treat findings as starting points for investigation,
    not definitive verdicts.
  attestation_expires: [date + 6 months]
```

---

### 🆕🆕🆕 Heuristic Acknowledgment (v15.0)

> **Evidence basis**: OpenSSF Scorecard V5 — "The checks themselves are heuristics;
> there are false positives and false negatives." Explicit acknowledgment increases
> trust calibration, not decreases it.

All VHEATM audit outputs MUST include at top of findings section:

```
⚠️ Methodology Note: VHEATM findings are heuristic. False positives and
false negatives are expected. MANDATORY accuracy rate for this project:
[X% | N/A if < 5 cycles]. Treat findings as starting points for
investigation, not definitive verdicts.
```

This is NOT a disclaimer to avoid accountability. It is intellectual honesty that
helps the reader calibrate trust correctly — same principle as confidence intervals
in statistics.

---

Exit: HG-M + HG-AP + HG-EF + HG-M includes [M.AT] + [M.EP] + [M.BA] + attestation

---

## [KB] Knowledge Base (updated from 9.0)

All existing elements unchanged. Additions:

🆕🆕 **[KB.EC] Evidence Currency Check (v13.0)**:
At end of every Full cycle, check research basis of all MANDATORY ADRs:
- Classify each citation: FRESH (≤2yr) | RECENT (3-4yr) | AGING (5-7yr) | STALE (8+yr) | CLASSIC (≥200 cit.)
- AI/ML domain: apply 1.5× aging rate
- STALE and non-CLASSIC: add to debt register "Debt-EC-N: update evidence for [finding]"
- Annual Evidence Refresh Cycle: search for retractions/superseding meta-analyses
→ Full protocol: `references/30-framework-lifecycle.md` Part 1

🆕🆕 **[KB.FST] Framework Self-Test Results (v13.0, updated v14.0)**:
Log Framework Self-Test results (INVARIANT-01..06 from ref 30 Part 3) here:
```
| Version | INV-01..06 | MAT-Rate | MAT-Phase | EP-Risk | CMA-Done | Pass/Fail |
```
INV-06 = Eigenstate Detection (protocol claim-vs-behavior consistency).
MAT-Rate = measured_accuracy_rate from [M.AT] tracker.
MAT-Phase = PRE-CALIBRATION | EMPIRICAL | RECALIBRATED.
EP-Risk = last [M.EP] eigenstate probe risk level.
CMA-Done = last [KB.CMA] comparative methodology audit cycle.

🆕🆕🆕 **[KB.AD] Unified Accuracy Dashboard (v14.0)**:
Merge PM-accuracy and [M.AT] into single dashboard per ref 30 Part 7:
```yaml
accuracy_dashboard:
  prediction: {pm_accuracy_rate, mandatory_accuracy_rate, weighted_score, status}
  calibration: {phase, threshold, evidence_tier, last_calibration, next_due}
  trend: {last_5_cycles, direction}
  eigenstate_health: {last_ep_risk, ep_high_count, last_cma_cycle}
```
→ Full schema: `references/30-framework-lifecycle.md` Part 7

🆕🆕🆕 **[KB.CMA] Comparative Methodology Audit (v14.0)**:
When triggered (empirical trigger, first cycle, every 10th, or ad hoc):
Run competing methodology independently → normalize findings → compare.
Metrics: overlap_rate, unique_rate, miss_rate, conflict_count.
→ Full protocol: `references/30-framework-lifecycle.md` Part 5

🆕 **Org-Context Performance Log**:
```
| Cycle | ENTERPRISE mode? | Teams in blast radius | BRS range | INC findings | Boundary crossings |
```

🆕 **Incentive Pattern Archive**:
Record recurring incentive misalignment patterns:
- EP-01 (Orphaned component) occurrences
- EP-02 (Cross-team no mandate) occurrences
- EP-03 (Compliance-gated) occurrences
- EP-04 (Incentive inversion) occurrences

Persistent patterns → systemic org recommendation (not just per-bug ADR).

---

## QBR Formula (Section 9) — updated calibration

```
QBR = (user_facing_impact × 4) + (data_integrity_risk × 4) +
      (security_risk × 3) + (blast_radius × 2)

Each input: 0 (none), 1 (low), 2 (medium), 3 (high)
Max QBR = 48

Context Mode adjustments:
  DESIGN mode: apply +20% skepticism to all QBR scores
  SELF_AUDIT = YES: additional +20% skepticism
  🆕 ORG_CAPTURE (Bias Check 6): additional +15% skepticism for own-team findings
  🆕 BRS ≥ 8: re-check blast_radius input → if < 3, set to 3 and recalculate
```

---

## ADR 🆕 9-Part Template (Section 10)

```markdown
### ADR-[N]: [Short Title]

**Priority:** MANDATORY | REQUIRED | RECOMMENDED | OPTIONAL
**QBR Score:** [N] | **Context Mode:** DESIGN | CODE | LIVE | LEGACY | ENTERPRISE

**Bug Anchor:**
- CODE/LIVE/ENTERPRISE: `path/to/file.ext:line_number`
- DESIGN: [Evidence Tier T1-T3] — [Source]

**Fix Anchor:** (filled at [T])
- `path/to/file.ext:line_number`

**Verification Anchor:** (filled at [T.FV])
- File:line range re-read + "Anti-pattern absent, correct pattern present, no siblings ±20 lines"

**1. Context**
[What situation prompted this ADR?]

**2. Decision**
[Exactly what is being decided?]

**3. Rationale**
[Why this over alternatives?]

**4. Consequences**
[What changes? What gets better/harder?]

**5. Revert Blast Radius**
TRIVIAL | HARD | IMPOSSIBLE

**6. Observable Success Criteria**
[Specific, measurable condition]

**7. Pattern Globalization**
- Pattern signature: "[anti-pattern shape]"
- Search command: "[grep/AST]"
- Siblings found: [count]
- Fix Cascade: [locations]
- Bug Class ID: BC-[NNN]

**🆕 8. Owner**
- Primary: [team/role]
- Secondary / sign-off: [team/role if applicable]
- Org Friction Multiplier: [from INC-3]
- Realistic Resolution Time: [CLI × multiplier]

**🆕 9. Boundary**
- Crosses org boundary: YES / NO
- If YES:
  - Type: TECHNICAL | HANDOFF | APPROVAL | DISPUTED
  - BRS: [N] — CONTAINED | ELEVATED | HIGH | CRITICAL
  - Handoff plan: [explicit: who does what in what order]
  - Stakeholder sign-off required: YES | NO
  - If BRS ≥ 8: Leadership visibility: [who must be notified]
```

---

*Phase Guide — VHEATM 10.0 | Updated for enterprise research additions*
