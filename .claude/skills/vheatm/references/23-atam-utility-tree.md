# ATAM Utility Tree — [V.UT] (v12.0)

> **Research Basis:**
> [ATRAF (Ben Hassouna, 2025, 2 citations)](https://consensus.app/papers/details/b8a43af1efc952bc9935803fc232fb92/):
> extends ATAM with iterative spiral process for tradeoff analysis across architectural levels —
> sensitivity points, tradeoff points, and risks. ATRAM → concrete system evaluation.
>
> [Lightweight SAEM review (Sahlabadi, 2022, 18 citations)](https://consensus.app/papers/details/851ccf213dc15ea7a398c86e686dd854/):
> practitioners avoid heavyweight ATAM due to time cost; lightweight factors are key to
> industrial adoption. Identifies what practitioners actually need from architecture evaluation.
>
> [Capilla et al. 2025](https://consensus.app/papers/details/f65893e6a2e5592585a68d3eaf883109/):
> LLMs produce better, more accurate quality scenario analysis than humans in most cases —
> validates AI-executable ATAM-lite with good quality output.
>
> ISO/IEC 25010:2023 — 8 quality characteristic vocabulary (standard QA naming).
>
> **Design decision:** Full ATAM takes 3-4 days with a trained evaluation team.
> [V.UT] is ATAM-lite: 15-30 minutes, 3-5 scenarios, focused on sensitivity points
> and tradeoff points that affect [V.AS] and [G.H] weighting.
> It is NOT a formal ATAM evaluation — it is architectural context that makes
> subsequent audit phases more precise.

---

## When to Run [V.UT]

```
REQUIRED:
  □ Audit scope covers multiple components or services
  □ Architecture is in scope for evaluation (not just code-level audit)
  □ QA tradeoffs are explicitly in scope ("performance vs. maintainability?")
  □ ENTERPRISE mode (multiple team ownership means implicit QA conflicts)

RECOMMENDED:
  □ PRE-LAUNCH audit (Final architecture assessment before shipping)
  □ LEGACY mode (debt remediation requires QA priority to sequence work)

FAST: skip [V.UT] entirely — run brief inline note:
  "QA priority assumed: Reliability > Security > Maintainability (default).
   Override in next cycle if different."

NOT APPLICABLE:
  □ Single-function audit (no architecture-level concerns)
  □ DESIGN mode where no architecture exists yet (defer to later cycle)
```

---

## ISO/IEC 25010 Quality Attribute Vocabulary

Use these standard terms to ensure consistency across cycles and across team members:

| Characteristic | What it measures | Common sub-characteristics |
|---|---|---|
| **Functional Suitability** | Does it do what it should? | Completeness, correctness, appropriateness |
| **Performance Efficiency** | Resource use under conditions | Time behavior, resource utilization, capacity |
| **Compatibility** | Co-existence with other systems | Co-existence, interoperability |
| **Usability** | User experience | Learnability, operability, error protection |
| **Reliability** | Stable under conditions | Maturity, availability, fault tolerance, recoverability |
| **Security** | Protects against attacks | Confidentiality, integrity, non-repudiation, authenticity |
| **Maintainability** | Ease of modification | Modularity, reusability, analyzability, modifiability, testability |
| **Portability** | Ease of environment transfer | Adaptability, installability, replaceability |

---

## [V.UT] 4-Step Mini-Pass

### Step 1: Business Driver Identification

```
"What is the primary business goal this system serves?"

□ Revenue/transaction processing → Reliability + Performance priority
□ Compliance/regulatory → Security + Functional Suitability priority
□ Developer platform/API → Maintainability + Compatibility priority
□ Consumer product → Usability + Reliability priority
□ Data processing pipeline → Performance + Reliability priority
□ Safety-critical system → Reliability + Security + Functional Suitability priority
□ Internal tooling → Maintainability + Usability priority

Note: business driver ≠ primary stakeholder. Both can be declared.
```

---

### Step 2: Quality Attribute Priority Ranking

```
Using ISO/IEC 25010 vocabulary, rank the top 3-5 QAs for this system:

Priority 1 (most important): [QA from ISO 25010 list]
Priority 2: [QA]
Priority 3: [QA]
Priority 4 (optional): [QA]
Priority 5 (optional): [QA]

Ranking basis: business driver from Step 1 + stakeholder declarations from [P]

Example for e-commerce checkout service:
  P1: Reliability (downtime = lost revenue)
  P2: Security (payment data protection)
  P3: Performance Efficiency (cart abandonment at >3s load)
  P4: Maintainability (high change rate, multiple teams)
```

---

### Step 3: Utility Tree Scenarios

For each top-3 QA, define one concrete scenario:

```
Scenario format (adapted from ATAM utility tree):
  QA:          [from ISO 25010]
  Scenario:    [concrete situation: who does what, in what environment]
  Stimulus:    [what triggers the quality concern]
  Environment: [normal / peak load / degraded / attack]
  Response:    [what the system should do]
  Measure:     [how to know if response succeeded — must be numeric]
  Current:     [does the architecture currently satisfy this? YES / NO / PARTIAL]
  Risk:        [if current = NO or PARTIAL — what is the architectural risk?]
```

**Example:**
```
QA:          Reliability
Scenario:    Checkout service during Black Friday peak load
Stimulus:    5× normal request volume, one database node fails
Environment: Peak load + degraded infrastructure
Response:    Service continues processing payments; affected transactions
             automatically retry; no data loss
Measure:     P99 latency < 2s, 0 transactions lost, recovery < 30s
Current:     PARTIAL — retry exists but no graceful degradation for DB failure
Risk:        Complete checkout unavailability if primary DB fails, estimated
             ~$12k/minute revenue impact
```

---

### Step 4: Sensitivity and Tradeoff Points

```
Sensitivity point: an architectural decision where a small change significantly
affects one QA.

Example: "Adding Redis caching (sensitivity point) greatly improves Performance
but introduces Reliability risk (cache invalidation bugs, Redis failure)."

Tradeoff point: a decision that simultaneously affects two or more QAs,
often in opposing directions.

Example: "Strong encryption of all PII (Security+) comes at Performance cost
for high-throughput data pipeline (Performance-)."

For each scenario from Step 3:
  □ Identify 1-2 sensitivity points
  □ Identify 1 tradeoff point (if any)
  □ Is the current architecture's tradeoff decision explicit and documented?
    If NO → generate H-AS hypothesis for "undocumented tradeoff decision"
```

---

## [V.UT] → [V.AS] Weighting Feedback

The QA priority ranking from Step 2 feeds [V.AS] architecture smell weighting:

```
QA priority → [V.AS] smell weighting:

If P1 = Maintainability:
  AS-01 (Cyclic deps) → weight × 1.5
  AS-03 (God component) → weight × 1.5

If P1 = Performance:
  AS-02 (Unstable deps) → weight × 1.5 (volatile dependencies affect perf)
  AS-04 (Scattered concern) → weight × 1.2 (scattered = harder to optimize)

If P1 = Reliability:
  AS-01 (Cyclic) → weight × 1.5 (cascading failure risk)
  AS-03 (God component) → weight × 1.3 (single point of failure risk)

If P1 = Security:
  AS-04 (Scattered concern) → weight × 1.5
    (scattered auth/validation = bypass risk)
  AS-01 (Cyclic) → weight × 1.2

Default weighting: all smells equal.
Adjusted weighting feeds QBR blast_radius for [V.AS] findings.
```

---

## [V.UT] Output Schema

```yaml
utility_tree:
  business_driver: "[primary driver]"
  quality_attributes:
    - rank: 1
      qa: "[ISO 25010 characteristic]"
      scenario: "[concrete scenario]"
      stimulus: "[trigger]"
      environment: "[context]"
      response: "[expected behavior]"
      measure: "[numeric criterion]"
      current_state: SATISFIED | PARTIAL | NOT_SATISFIED
      risk: "[if PARTIAL or NOT_SATISFIED]"
      sensitivity_points: ["[decision A affects QA X significantly]"]
      tradeoff_points: ["[decision B improves X but degrades Y]"]
    - rank: 2
      ...
  vas_weighting_adjustments:
    - smell: "AS-01"
      multiplier: 1.5
      reason: "P1 QA = Reliability; cyclic deps increase cascading failure risk"
  undocumented_tradeoffs: [count]
  hypotheses_seeded: ["H-AS-N: undocumented tradeoff in [component]"]
```

---

## FAST Mode Inline

When [V.UT] is skipped (FAST mode), add this inline note to output:

```
[V.UT] FAST inline:
  Default QA priority: Reliability > Security > Maintainability
  Override: declare correct priority at [P] in next cycle if this assumption is wrong.
  No sensitivity/tradeoff analysis performed.
  [V.AS] runs with default weighting.
```

---

*Reference 23 — VHEATM 12.0 | ATAM utility tree mini-pass + ISO/IEC 25010 QA vocabulary
Research: ATRAF Ben Hassouna 2025; Lightweight SAEM review 2022; Capilla LLM-ATAM 2025*
