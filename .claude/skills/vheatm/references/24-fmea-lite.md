# FMEA-lite Failure Ledger — [G.FL] (v12.0)

> **Research Basis:**
> [Crawley 2020](https://consensus.app/papers/details/5f2560d0a1f45d94a554ff39e9fc409d/):
> FMEA/FMECA applies to hardware, software, processes, services, and human actions.
> Core: identify failure modes → causes → effects → risk priority → corrective action.
>
> [Hassani et al. 2024 (15 citations, Design Society)](https://consensus.app/papers/details/37ff95dad75c55ae9309384961181270/):
> LLM-integrated FMEA framework demonstrates improved speed and accuracy vs manual.
> Human-in-loop validation improves reliability. Validates AI-executable FMEA.
>
> [Peeters et al. 2018 (246 citations, RESS)](https://consensus.app/papers/details/ad1db71fd060579aa08ad76a00aebda5/):
> recursive FTA+FMEA combination. Bottom-up (FMEA) + top-down (FTA) is more efficient.
> Key insight: apply FMEA at system level first → select critical failure modes only →
> then apply at function level. NOT every component needs full FMEA.
>
> [Subriadi et al. 2020 (96 citations, Heliyon)](https://consensus.app/papers/details/0558e3ec40535685b026862e60b86b2c/):
> FMEA for IT risk assessment — validates software/information system FMEA viability.
> Traditional FMEA consistency problems → improved FMEA framework shows near-perfect
> correlation (0.937) between teams.
>
> **Design decision:** Full FMEA = 12+ fields, workshop-based, days of effort.
> FMEA-lite = 6 fields + RPN, conditional trigger, feeds VHEATM hypothesis pipeline.
> Inspired by Peeters' recursive insight: only run FMEA-lite on HIGH-risk components.

---

## Trigger Conditions

```
REQUIRED (run [G.FL] when ANY of these):
  □ System is safety-critical (financial transaction processing, healthcare, infrastructure)
  □ Hardware-software interface in scope (embedded, IoT, device firmware)
  □ State machine with complexity ≥ 3 states (multiple valid state transitions)
  □ Any [G.H] finding has blast_radius = 3 (system-level effect suspected)
  □ POST-INCIDENT profile active (root cause must enumerate failure modes)
  □ [V.UT] identified a Reliability scenario with PARTIAL or NOT_SATISFIED state

RECOMMENDED:
  □ Complex workflows with multiple failure paths (e-commerce checkout, payment processing)
  □ Any module identified as God Component in [V.AS] (AS-03)

NOT APPLICABLE:
  □ FAST mode (skip entirely)
  □ Pure DESIGN mode without implementation detail
  □ Simple stateless CRUD without complex failure modes
```

---

## 6-Field FMEA-lite Format

Full FMEA has 12+ fields. FMEA-lite uses 6 that capture the essential risk information:

```
Field 1 — Component/Function
  "What is the system element being analyzed?"
  Scope: module, service, function, process step, or human action in the system
  Example: "Payment processor service — charge() method"

Field 2 — Failure Mode
  "How could this element fail?"
  Not the cause — the way it fails (what you observe, not why)
  Example: "Charge request succeeds but order state not updated"
           "Charge request times out after partial execution"
           "Charge executed twice (duplicate idempotency failure)"

Field 3 — Cause
  "Why would this failure mode occur?"
  Root cause — one cause per row (use separate rows for multiple causes)
  Example: "Race condition between payment callback and order state update"
           "Network timeout with non-idempotent retry logic"

Field 4 — Local Effect
  "What is the impact within this component?"
  What the component itself experiences
  Example: "Payment recorded as successful; order state = PENDING"
           "Charge processed; no acknowledgment received"

Field 5 — System Effect
  "What is the impact on the broader system and its users?"
  System-level consequence
  Example: "User charged but sees 'payment failed' → double charge on retry"
           "Inventory not decremented → oversell possible"

Field 6 — RPN (Risk Priority Number)
  RPN = Severity × Occurrence × Detectability
  Each rated 1-10:
    Severity (S):     1=cosmetic, 5=user impacted, 10=data loss/safety risk
    Occurrence (O):   1=rare, 5=occasional, 10=frequent/certain
    Detectability (D): 1=certain to detect, 5=may detect, 10=unlikely to detect
  RPN range: 1-1000
  Threshold: RPN ≥ 125 → MANDATORY finding
             RPN 50-124 → REQUIRED finding
             RPN < 50 → RECOMMENDED or OPTIONAL
```

---

## FMEA-lite Execution Protocol

### Step 1: Component Selection (Peeters' insight)

**Don't run FMEA-lite on every component.** Select candidates:

```
Select components for FMEA-lite scan when:
  □ Component is in the critical path (failure stops system function)
  □ Component has highest fan-in from [V.AS] (many callers depend on it)
  □ Component was flagged in [G.H] as MANDATORY or triggered blast_radius = 3
  □ Component is identified as God Component (AS-03 from [V.AS])
  □ Component involves state transition or external payment/financial operations

Run FMEA-lite only on selected components. Document which were excluded and why.
```

---

### Step 2: Failure Mode Enumeration

For each selected component, enumerate failure modes:

```
Failure mode prompts (use these to ensure coverage):
  "What happens if [component] returns success but side effects fail?"
  "What happens if [component] times out mid-operation?"
  "What happens if [component] is called twice with same input?"
  "What happens if [component] receives malformed input?"
  "What happens if [component] fails silently (no error, no result)?"
  "What happens if [component] succeeds for the caller but fails for downstream?"
  "What happens if [component] is unavailable for [N] seconds?"
  "What happens if [component]'s state is corrupted by a prior partial failure?"
```

---

### Step 3: RPN Calculation and Threshold

```
For each failure mode (Field 2), determine:

Severity (S): What is the worst-case impact on users/system?
  1-2: cosmetic / no user impact
  3-4: user inconvenience, easily recoverable
  5-6: user-visible failure, requires support
  7-8: data integrity risk or partial data loss
  9-10: data loss, financial impact, or safety risk

Occurrence (O): How often would this failure mode trigger?
  1-2: rare/theoretical (requires unusual conditions)
  3-4: possible (observed in similar systems)
  5-6: occasional (known edge case, happens sometimes)
  7-8: frequent (happens under normal load periodically)
  9-10: high probability / known to occur

Detectability (D): How easily can this failure be detected?
  1-2: definitely detected (immediate error, clear alert)
  3-4: usually detected (test coverage, monitoring exists)
  5-6: sometimes detected (sporadic tests, partial monitoring)
  7-8: rarely detected (no tests, no alerts for this path)
  9-10: undetectable (silent failure, no observable signal)

RPN = S × O × D

Thresholds:
  RPN ≥ 125 → MANDATORY
  RPN 50-124 → REQUIRED
  RPN 25-49 → RECOMMENDED
  RPN < 25 → OPTIONAL or dismissed
```

---

### Step 4: Integration with VHEATM Hypothesis Pipeline

Each FMEA-lite failure mode above threshold → [G.H] hypothesis:

```yaml
fmea_hypothesis:
  id: H-FL-[N]
  component: "[component name]"
  failure_mode: "[description]"
  cause: "[root cause]"
  local_effect: "[component impact]"
  system_effect: "[system impact]"
  rpn:
    severity: [1-10]
    occurrence: [1-10]
    detectability: [1-10]
    score: [S × O × D]
  layer: "[L1-L7 or AI-S1-4]"  # assign VHEATM layer from failure mode type
  fmea_source: true
  qbr_note: "FMEA RPN drives QBR inputs: S→data_integrity/user_facing, O→occurrence, D→blast_radius"
```

**QBR calibration from FMEA fields:**
```
user_facing_impact:
  Severity 1-4 → 0-1
  Severity 5-7 → 2
  Severity 8-10 → 3

data_integrity_risk:
  System effect = data loss/corruption → 3
  System effect = partial impact → 2
  System effect = operational only → 1

security_risk:
  Cause = auth bypass or injection → 3
  Cause = data exposure → 2
  Cause = resource exhaustion → 1

blast_radius:
  Map from FMEA system_effect scope:
  5+ downstream components/teams → 3
  2-4 downstream components → 2
  local only → 1
  ⚠️ v13 correction: do NOT use Detectability for blast_radius.
     Detectability → adjust data_integrity_risk and security_risk instead:
       D 8-10 (undetectable) → add +1 to data_integrity_risk (silent corruption window)
       D 8-10 AND cause=security → add +1 to security_risk (exploitation window)
     See VHEATM 13.0 SKILL.md: "Corrected FMEA RPN → QBR Mapping" 
```

---

## FMEA-lite Output Template

```markdown
## FMEA-lite Failure Ledger — [Component Name]

| # | Failure Mode | Cause | Local Effect | System Effect | S | O | D | RPN | Priority | H-ID |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | [mode] | [cause] | [local] | [system] | 8 | 5 | 7 | 280 | MANDATORY | H-FL-001 |
| 2 | [mode] | [cause] | [local] | [system] | 5 | 3 | 4 | 60 | REQUIRED | H-FL-002 |
| 3 | [mode] | [cause] | [local] | [system] | 3 | 2 | 3 | 18 | OPTIONAL | — |

Components scanned: [list]
Components excluded: [list + reason]
Total failure modes identified: [N]
Promoted to hypotheses: [M]
```

---

## Anti-Patterns

🚫 **"Running FMEA-lite on every function"** — Peeters 2018 (246 citations): run on critical
components only. Over-application produces hundreds of rows and kills the signal.

🚫 **"RPN = 100 so it's fine"** — High severity with low occurrence can produce RPN = 100
but still be MANDATORY if S=10, O=2, D=5. Always check severity independently.

🚫 **"We have tests so Detectability = 1"** — Detectability = 1 means a failure is
*certain* to be detected. That requires: tests that cover this exact failure mode + alerts
that fire in production. If only tests exist (no production detection) → D ≥ 4.

🚫 **"FMEA is for hardware"** — IEC 60812:2018 explicitly covers software, processes,
and human actions. Subriadi 2020 (96 citations) validates IT risk FMEA.

---

*Reference 24 — VHEATM 12.0 | FMEA-lite Failure Ledger
Research: Crawley 2020; Hassani 2024 (15 cit.); Peeters 2018 (246 cit.); Subriadi 2020 (96 cit.)*
