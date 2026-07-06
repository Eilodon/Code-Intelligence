# Specialist Lens Router (v12.0)

> VHEATM v12 is the orchestration layer. Specialist frameworks are expert lenses.
> This router defines: when to invoke, what scope to use, how to integrate output.
>
> Core principle: "Deep by routing, not by bloating."
> VHEATM core does not replicate specialist depth. It triggers the right lens,
> frames the right question, and integrates the finding into VHEATM output format.
>
> Framework coverage: STRIDE/PASTA, LINDDUN, ATAM, SAMM/BSIMM/SSDF, FMEA/FMECA,
> ISO/IEC 25010, MAESTRO, OWASP LLM Top 10, OWASP Agentic Top 10, MITRE ATLAS,
> NIST AI RMF, Chaos Engineering.

---

## Router Trigger Table

| Trigger Condition | Framework | Mode | When in Cycle |
|---|---|---|---|
| Any security threat / attack surface concern | STRIDE or PASTA | Standard+Full | [G.H] L6 |
| Personal data / privacy obligations | LINDDUN | Standard+Full (if PII in scope) | [G.H] L7.11 |
| Architecture quality attribute tradeoffs | ATAM-lite + ISO 25010 | Standard+Full | [V.UT] |
| Secure SDLC process / maturity measurement | SAMM-lite | Full | [M.AM] |
| Observed practices measurement (benchmarking) | BSIMM overlay | Full (optional) | [M.AM] |
| Secure software producer assurance | NIST SSDF | Full | [M.AM] |
| Failure mode enumeration (safety-critical, state machine) | FMEA-lite | Standard+Full when triggered | [G.FL] |
| AI model / prompt / output security | OWASP LLM Top 10 | AI_INTEGRATED=YES | [G.H] AI-S1 |
| AI agent action / tool / identity security | OWASP Agentic Top 10 | AI_INTEGRATED=YES | [G.H] AI-S2 |
| AI layered architecture threat model | MAESTRO | AI_INTEGRATED=YES | [G.H] AI-S3 |
| AI adversarial technique mapping | MITRE ATLAS | AI_INTEGRATED=YES | [G.H] AI-S4 |
| AI governance / risk management | NIST AI RMF | AI_INTEGRATED=YES, Full | [M] |
| Resilience / recovery testing | Chaos Engineering principles | LIVE + ENTERPRISE | [E.HV] |

---

## Framework Invocation Protocols

### 1 — STRIDE / PASTA (Security Threat Modeling)

**Trigger**: Any finding touching L6 Security, auth, data flow, or external API.
**Scope**: Data flow segments in scope for this audit cycle.

**STRIDE mini-scan** (integrate into [G.H] L6):
```
For each data flow or API boundary in scope, ask:
  S — Spoofing:    "Can an attacker impersonate a legitimate identity here?"
  T — Tampering:   "Can data be modified in transit or at rest without detection?"
  R — Repudiation: "Can an actor deny performing an action? Is there an audit trail?"
  I — Info Disc:   "Can sensitive data be exposed to unauthorized parties?"
  D — DoS:         "Can this service be made unavailable by an attacker?"
  E — Elevation:   "Can an attacker gain higher privilege than intended?"

For each YES → generate L6 hypothesis with STRIDE category tag.
```

**PASTA integration** (Full mode only, when security is primary concern):
```
P1 Define business objectives: what business risk does a security breach cause?
P2 Define technical scope: which components are in this audit's security perimeter?
P3 Application decomposition: data flows, trust boundaries, entry points
P4 Threat analysis: what threats exist? (use STRIDE results from above)
P5 Vulnerability discovery: which threats have corresponding weaknesses in code?
   → These become [G.H] L6 hypotheses
P7 Risk/impact analysis: business impact of each threat materializing
   → This feeds QBR blast_radius and user_facing_impact inputs
```

**Integration output**: L6 hypothesis with `threat_model: STRIDE` or `threat_model: PASTA` tag.
STRIDE findings → standard [G.H] processing.

---

### 2 — LINDDUN (Privacy Threat Modeling)

**Trigger**: Any scope touching personal data from EU/UK/CA users, or L7.11 compliance flag.
**Scope**: Data flows involving personal data.

```
LINDDUN 7 categories mini-scan:
  LI — Linkability:      "Can data be linked across contexts without consent?"
  Id — Identifiability:  "Can a person be identified from 'anonymous' data?"
  Nr — Non-repudiation:  "Are personal actions logged in ways that prevent denial?"
  De — Detectability:    "Can attackers detect whether a data subject exists?"
  Di — Disclosure:       "Can personal data be disclosed to unauthorized parties?"
  Un — Unawareness:      "Are data subjects unaware of how their data is used?"
  Nc — Non-compliance:   "Does the system violate applicable privacy regulation?"

For each YES → generate L7.11 hypothesis with LINDDUN category tag.
Cross-reference with GDPR obligations from [L7.11] scan.
```

**Integration output**: L7.11 hypothesis with `privacy_threat: LINDDUN-[category]` tag.

---

### 3 — ATAM-lite + ISO/IEC 25010 (Architecture Quality)

**Trigger**: Audit scope includes architectural decisions, multi-component system,
or QA tradeoffs are in scope. Runs at [V.UT].
**Full protocol**: See `references/23-atam-utility-tree.md`

```
ISO/IEC 25010 Quality Attribute vocabulary (use these exact terms):
  Functional Suitability  — does it do what it should?
  Performance Efficiency  — response time, throughput, resource use
  Compatibility           — co-existence with other systems, interoperability
  Usability               — learnability, operability, error protection
  Reliability             — maturity, availability, fault tolerance, recoverability
  Security                — confidentiality, integrity, authenticity, accountability
  Maintainability         — modularity, reusability, analyzability, modifiability, testability
  Portability             — adaptability, installability, replaceability

For each QA: rate importance (HIGH/MEDIUM/LOW) and define one concrete scenario.
```

---

### 4 — SAMM-lite / BSIMM / NIST SSDF (Process Maturity)

**Trigger**: Full mode [M.AM], or when recurring pattern is found (same bug type 2+ cycles).
**Full protocol**: See `references/25-assurance-maturity-overlay.md`

```
SAMM 5 business functions:
  Governance      → policies, compliance, education
  Design          → threat assessment, security requirements, secure architecture
  Implementation  → secure build, secure deploy, defect management
  Verification    → architecture assessment, requirements-driven testing, security testing
  Operations      → incident management, environment management, operational management

For each MANDATORY/REQUIRED finding:
  → Map to SAMM function: "This finding indicates missing/weak [SAMM function]"
  → BSIMM context: "This pattern is observed in [X]% of BSIMM organizations"
    (use BSIMM descriptive data for benchmarking, not prescriptive target)
  → NIST SSDF: "This gap maps to SSDF practice [PS/PO/RV/RD]-[N]"
```

---

### 5 — FMEA-lite (Failure Mode Enumeration)

**Trigger**: Safety-critical system, hardware-software interface, state machine complexity ≥ 3 states,
blast_radius = 3 finding, or POST-INCIDENT scope.
**Full protocol**: See `references/24-fmea-lite.md`

```
FMEA-lite 6-field scan per component:
  Component   → [module / service / function]
  Failure Mode → [how it could fail]
  Cause        → [why it would fail]
  Local Effect → [impact within this component]
  System Effect → [impact on the broader system]
  RPN          → Severity (1-10) × Occurrence (1-10) × Detectability (1-10)
                 RPN ≥ 125 → MANDATORY finding

Output feeds [G.H] as hypotheses with fmea_source: true tag.
```

---

### 6 — AI Security 4 Sub-lenses (AI-S1 to AI-S4)

**Trigger**: AI_INTEGRATED = YES.
**Full protocol**: See `references/11-cross-cutting-layer.md` AI Security section.

```
AI-S1: OWASP LLM Top 10 2025 → model/prompt/output layer
  LLM01: Prompt Injection
  LLM02: Insecure Output Handling
  LLM03: Training Data Poisoning
  LLM04: Model Denial of Service
  LLM05: Supply Chain Vulnerabilities
  LLM06: Sensitive Information Disclosure
  LLM07: Insecure Plugin Design
  LLM08: Excessive Agency
  LLM09: Overreliance
  LLM10: Model Theft

AI-S2: OWASP Agentic Top 10 2026 → agent action/tool/identity layer
  A01: Agent Goal Hijack
  A02: Tool Misuse
  A03: Identity/Privilege Abuse
  A04: Agentic Supply Chain
  A05: Unexpected Code Execution
  A06: Memory/Context Poisoning
  A07: Insecure Inter-Agent Communication
  A08: Cascading Failures
  A09: Human-Agent Trust Exploitation
  A10: Rogue Agents

AI-S3: MAESTRO 7-layer architecture threat model
  Layer 1: Foundation Model
  Layer 2: Data Operations
  Layer 3: Agent Frameworks
  Layer 4: Deployment/Infrastructure
  Layer 5: Evaluation/Observability
  Layer 6: Security/Compliance
  Layer 7: Agent Ecosystem

AI-S4: MITRE ATLAS adversarial technique mapping
  For each confirmed AI security finding: map to ATLAS tactic/technique
  Tactics: Reconnaissance, Resource Dev, Initial Access, ML Attack Staging,
            Model Evasion, Exfiltration, Impact
```

---

### 7 — NIST AI RMF Overlay (AI Governance)

**Trigger**: AI_INTEGRATED = YES, Full mode, or when AI governance is explicitly in scope.
**Full protocol**: See `references/26-nist-ai-rmf-overlay.md`

```
4-function overlay maps onto VHEATM phases:
  GOVERN  → [P] AI governance declarations + [M] governance review
  MAP     → [G.H] + [G.T] + [V.AS] risk identification
  MEASURE → [E.HV] + QBR scoring (risk measurement with evidence)
  MANAGE  → [A] ADRs + [T] + [M.AP] (risk treatment and monitoring)

At [M] Full mode: run GOVERN check:
  □ Is there a documented AI use policy for this system?
  □ Are AI outputs subject to human review for high-stakes decisions?
  □ Is there a feedback mechanism for AI errors?
  □ Is there a monitoring plan for model drift/degradation?
```

---

### 8 — Chaos Engineering (Resilience Testing)

**Trigger**: LIVE or ENTERPRISE mode with resilience concerns; explicit chaos/failure injection scope.

```
Chaos Engineering principles (from Netflix/Google DiRT / Chaos Engineering MLR):
  1. Define steady state: "What does normal look like?" → metrics baseline
  2. Hypothesize steady state will continue in both control + experimental group
  3. Introduce variables: inject realistic events (node failure, high latency, network partition)
  4. Try to disprove hypothesis

In VHEATM context (no live execution):
  → Frame as: "If we WERE to run chaos experiment X, what would we expect?"
  → Translate to hypotheses: "What failure mode would chaos test X expose?"
  → Each chaotic scenario → [G.H] L5 hypothesis (external dependency failure)
  → Evidence: "Chaos hypothesis — requires live execution to confirm."
               Static confidence = MEDIUM by definition.
               [E.HV] Step 2 = chaos experiment required.
```

---

## Router Decision Tree (condensed)

```
START → Is this audit touching AI-integrated system?
  YES → Activate AI-S1 + AI-S2 + AI-S3 + AI-S4 (mandatory if AI_INTEGRATED=YES)
  NO  → Continue

→ Does scope include personal data / GDPR / privacy?
  YES → Activate LINDDUN
  NO  → Continue

→ Does scope include security threats / auth / API boundaries?
  YES → Activate STRIDE (always) + PASTA (Full mode)
  NO  → Continue

→ Does scope include architecture quality / QA tradeoffs?
  YES → Activate ATAM-lite + ISO 25010 at [V.UT]
  NO  → Continue

→ Is this Full mode AND finding recurring pattern / same bug 2+ cycles?
  YES → Activate SAMM-lite at [M.AM]
  NO  → Continue

→ Is scope safety-critical / state machine / blast_radius=3?
  YES → Activate FMEA-lite at [G.FL]
  NO  → Continue

→ Is this AI system in Full mode?
  YES → Activate NIST AI RMF overlay at [M]
  NO  → Continue

→ Is this LIVE/ENTERPRISE with resilience concerns?
  YES → Activate Chaos Engineering framing in [G.H] L5
  NO  → Done — proceed with standard VHEATM without specialist lens
```

---

## Integration Output Format

When specialist lens produces findings, integrate into standard VHEATM format:

```yaml
specialist_lens_finding:
  lens: STRIDE | PASTA | LINDDUN | ATAM | FMEA | SAMM | OWASP-LLM | OWASP-Agentic | MAESTRO | ATLAS | NIST-AIRLMF
  category: "[lens-specific category — e.g. STRIDE-T, LINDDUN-De, OWASP-LLM01]"
  hypothesis_id: H-[N]  # maps into standard VHEATM H-ID
  layer: L1 | L2 | ... | L7.11 | AI-S1 | AI-S2 | AI-S3 | AI-S4
  description: "[finding description in VHEATM terms]"
  evidence_anchor: "[file:line or Evidence Tier]"
  qbr_score: [N]
  adr_generated: true | false
```

All specialist lens findings enter standard VHEATM hypothesis lifecycle.
No specialist lens bypasses QBR, SNF, Evidence Anchor, or ADR requirements.

---

## Activation Profile: ASYNC_WORKER

> **Purpose**: Auto-activate a coordinated bundle of protocols when codebase uses
> background worker architecture. Prevents the "lens scatter" failure mode where
> async Python stack bugs fall between VHEATM lenses because no single lens owns
> the full failure class.
>
> **Evidence basis**: Tikai field evidence — async session commit failures, blocking
> pandas in FastAPI handlers, divergent worker/API session semantics. T4 evidence;
> upgrade path: 3 independent codebases confirmed → T2.

**Auto-detect trigger** (ANY of the following in pyproject.toml, requirements.txt,
or setup.cfg):
```
arq, celery, dramatiq, rq, huey, taskiq
```

Or code-level detection:
```
from arq import ...  |  from celery import ...  |  import dramatiq
```

**When triggered, ALL of the following activate automatically:**

| Protocol | Ref | Activation level | What it does |
|---|---|---|---|
| PY-07 AsyncSession commit | ref 19 | MANDATORY scan | All async session usages audited for commit |
| PY-08 Pandas/IO blocking | ref 19 | MANDATORY scan | All pd.read_* in async def flagged |
| PY-09 dataclasses.replace | ref 19 | RECOMMENDED | All replace() callsites verified |
| [G.CF] Background Job Pipeline | ref 12 | MANDATORY template | Every detected job decomposed with pipeline template |
| [G.CF] DB Write Pipeline | ref 12 | MANDATORY template | Every DB write path decomposed |
| [G.PG] Divergent Implementation | ref 08 | MANDATORY extension | Session factories, commit patterns checked for divergence across API and worker |
| L-T.3 Async Session Extension | ref 16 | MANDATORY | All async context managers wrapping sessions audited |
| [G.CPT] Code Path Trace | ref 31 | MANDATORY | All write chains with ≥2 components traced to terminal state |
| [E.IJ] upgrade | ref 01 | MANDATORY (Tier 2+) | Two independent session lifetimes = eigenstate risk doubled; external judge not optional |

**Execution order** (integrated into standard VHEATM cycle):

```
[P] Pre-conditions:
  □ Detect ASYNC_WORKER: check deps as above
  □ If detected: declare ASYNC_WORKER=YES in session header
  □ All activations in table above are now blocking gates

[G] Generation:
  □ [G.T] L-T.3 Extended fires on all async context managers (autoflush, begin(), commit())
  □ [G.CF] Background Job Pipeline template applied to every enqueue point found
  □ [G.CF] DB Write Pipeline template applied to every session.add() / execute() chain
  □ [G.PG] Divergent Implementation: enumerate all session factories, compare commit semantics
  □ [G.CPT]: trace each write chain identified by [G.CF] to terminal state

[E] Experiment:
  □ [E.IJ] mandatory for all MANDATORY findings (not just Tier 3 — see upgrade above)

[M] Metrics:
  □ async_worker_coverage: "N write chains traced / M total detected"
  □ commit_boundary_pass_rate: "X / N chains reached COMMITTED terminal state"
```

**Note on [E.IJ] upgrade rationale**: In async worker architecture, two independent
session factories exist (API dependency injection + worker startup context). Each has
its own commit lifecycle. An auditor who verified the API session correctly cannot
assume worker session has same semantics — they are genuinely independent code paths.
This doubles the eigenstate surface: the auditor may have correct understanding of one
path while the other is broken. [E.IJ] external judge is the only mechanism that
evaluates both paths without inheriting the auditor's assumptions about either.

---

*Reference 22 — VHEATM 12.0 | Specialist Lens Router covering 12 frameworks*
