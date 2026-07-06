# Cross-Cutting Concerns — L7 Scan Layer (v10.0)

> **v10.0 addition**: L7.11 Compliance / Regulatory added as 11th sub-category.
> Research basis: DevSecOps compliance studies show "bolt-on compliance" (added after
> development) results in 67% higher incident rate than native integration (Essien et al.,
> 2021). GDPR Formal Concept Analysis (Tamburri, 2020, 102 citations) demonstrates
> 144,000+ attribute implications — compliance is not enumerable as a checklist.
> Three Lines of Defense model (COSO/COBIT) provides the governance structure.
>
> L7.11 is MANDATORY in ENTERPRISE context mode. REQUIRED in LIVE/CODE when user,
> financial, or health data is in scope. Available in FAST mode as a 1-question signal flag.

(L7.1 through L7.10 unchanged from 9.0. Only additions documented here.)

---

## What L7 Covers (updated summary)

```
L7.1  Rate Limiting & Quota Enforcement
L7.2  Idempotency on Retried Operations
L7.3  Timeout & Deadline Propagation
L7.4  Observability (Logs / Metrics / Traces)
L7.5  Resource Lifecycle (Connections, Files, Locks)
L7.6  Concurrency Safety on Shared State
L7.7  Security Headers & Content Policies
L7.8  Authorization (Beyond Authentication)
L7.9  Error Path Cleanup (Rollback, Compensating Actions)
L7.10 Backpressure & Load Shedding
🆕 L7.11 Compliance / Regulatory
```

---

## 🆕 L7.11 Compliance / Regulatory Scan

### Applicability

```
MANDATORY (always include):
  □ CONTEXT_MODE = ENTERPRISE
  □ System touches personally identifiable information (PII)
  □ System processes financial transactions or card data
  □ System stores or processes health data
  □ System is deployed in EU (GDPR applies by jurisdiction)

REQUIRED (strong signal, include unless scoped out):
  □ CONTEXT_MODE = LIVE and any external users
  □ Audit scope includes data storage, user authentication, or billing
  □ Pre-launch scope intent

OPTIONAL:
  □ Internal tooling with no external user data
  □ Pure analysis / reporting with read-only access to already-regulated data
```

---

### Step 1: Regulatory Framework Identification

```
Identify applicable frameworks for this system:

□ GDPR — EU/UK personal data processing
  Triggers: any personal data from EU/UK residents, regardless of company location
  Key obligations: lawful basis for processing, data minimization, right to erasure,
                   data subject access requests, breach notification (72h)

□ CCPA — California Consumer Privacy Act
  Triggers: California residents' personal data, company > $25M revenue or 100k users
  Key obligations: right to know, right to delete, opt-out of data sale

□ PCI-DSS — Payment Card Industry Data Security Standard
  Triggers: storing, processing, or transmitting cardholder data
  Key obligations: encrypt cardholder data, access controls, penetration testing,
                   audit logs, vulnerability management program

□ HIPAA — Health Insurance Portability and Accountability Act
  Triggers: protected health information (PHI), US healthcare context
  Key obligations: minimum necessary access, audit controls, data integrity,
                   business associate agreements, breach notification

□ SOC2 (Type I / II) — Service Organization Control
  Triggers: cloud services used by other companies' data
  Trust Service Criteria: Security, Availability, Processing Integrity,
                          Confidentiality, Privacy

□ ISO 27001 — Information Security Management System
  Triggers: formal ISMS certification required by contracts/customers
  Key obligations: risk assessment, controls implementation, audit and review

□ Other: [jurisdiction-specific, sector-specific frameworks]

For each applicable framework: note the version/year of the standard in use.
```

---

### Step 2: Per-Component Compliance Scan

For each component/endpoint in scope:

```
Component: [name/path]
Data handled: [PII / financial / health / internal-only]
Applicable frameworks: [list from Step 1]

For each framework:
  □ Obligation: [specific article/requirement — not the framework name, the SPECIFIC clause]
  □ Current implementation: [what the code does]
  □ Compliant? YES / NO / PARTIAL
  □ Evidence anchor: [file:line or document reference]
  □ If NO or PARTIAL → hypothesis with compliance_gap type

Common compliance-code mappings:
  GDPR Art.5(e) Storage limitation:
    → Is there a data retention policy enforced in code? Is stale data auto-deleted?
  GDPR Art.17 Right to erasure:
    → Can user data be completely deleted on request? Cascades to backups?
  GDPR Art.33 Breach notification:
    → Is there automated detection + notification pipeline for data breaches?
  PCI-DSS Req.3: Protect stored cardholder data:
    → Is card data encrypted at rest? Is it stored at all? (Most systems shouldn't store raw PAN)
  PCI-DSS Req.10: Track all access to network resources / cardholder data:
    → Full audit log of all access to payment data?
  SOC2 CC6.1: Logical access controls:
    → Role-based access, least privilege enforced?
  HIPAA §164.312: Technical safeguards:
    → Access controls, audit controls, integrity, authentication all implemented?
```

---

### Step 3: Audit Trail Scan

```
Across the entire system in scope:

□ Are all data access events (reads of PII/financial/health data) logged?
  Minimum log fields: timestamp, user_id, resource_type, resource_id, action
□ Are all mutations (create/update/delete of sensitive data) logged with before/after?
□ Are logs tamper-resistant (append-only store or signed)?
□ Log retention period: [documented? matches regulatory requirements?]
□ Log access controlled (not all engineers can read production logs with PII)?
□ Audit log export capability for regulatory requests?

If ANY of the above = NO for sensitive data → hypothesis with L7.11 compliance_gap
QBR: data_integrity_risk = 3 (compliance failure = data integrity risk by definition)
```

---

### Step 4: Data Subject Rights Scan (GDPR / CCPA)

```
For systems with EU/CA users:

Right to Access (GDPR Art.15, CCPA):
  □ Can you export all data for a given user_id on request within 30 days?
  □ Is the export complete (includes backups, logs, derived data)?
  □ Process documented and tested?

Right to Erasure (GDPR Art.17):
  □ Can you delete all data for a user_id on request?
  □ Cascades through: main DB, caches, CDN, backups, analytics, third-party integrations?
  □ Retention exceptions documented (legal holds, fraud prevention)?

Right to Portability (GDPR Art.20):
  □ Can data be exported in machine-readable format (JSON/CSV)?

If process exists only in documentation but not in code → PARTIAL compliance.
```

---

### L7.11 QBR Calibration

L7.11 compliance findings use standard QBR formula but with these calibration notes:

```
data_integrity_risk: High by default for any compliance gap that could result in
  incorrect data state (misprocessed financial transaction, incorrect PHI,
  wrong consent state).

user_facing_impact: High for any gap affecting user rights (erasure, access, portability).
  Medium for audit trail gaps (user not directly impacted, but regulator can fine).

security_risk: High for PCI-DSS and HIPAA violations (overlap with L6).
  Medium for GDPR/SOC2 gaps that don't directly expose data.

blast_radius: Use BRS from [G.ORG] if available.
  Otherwise: estimate teams affected by compliance remediation.
```

**Compliance gaps are not downgraded by SNF:**
Standard SNF Question 3 (detect-and-fix-later) does NOT apply to regulatory obligations.
A compliance gap that materializes as a regulatory audit finding has:
- Mandatory regulatory disclosure
- Fines (GDPR: up to 4% annual revenue or €20M)
- Customer trust damage
These cannot be "monitored and fixed quickly."

---

## When to Escalate L7.11 Findings

```
Escalate immediately (out-of-band, do not wait for ADR cycle):
  □ Active data breach where L7.11 gap enabled the breach
  □ Audit log gap covering past N days (retroactive compliance failure)
  □ Card data stored in plaintext (PCI-DSS immediate violation)
  □ PHI accessible without access controls (HIPAA §164.312)

Standard ADR cycle:
  □ Missing retention policy (risk manifests over months)
  □ Missing right-to-erasure implementation (risk materializes on first request)
  □ Incomplete audit logs (risk materializes at next audit)
  □ Third-party data sharing without documented lawful basis
```

---

## Mode Adaptations (updated)

### FAST Mode — L7.11

Single question:
```
L7.11-FAST: "Does this system process user PII, financial data, or health data?
  If YES: Are there any obvious compliance gaps (no audit logs, no data deletion,
  no consent management, card data in plaintext)? YES/NO"
If YES to gap → note "L7.11-FAST: compliance gap flagged — escalate to Standard"
```

### Standard Mode — L7.11

Steps 1 + 2 + 3. Step 4 if GDPR/CCPA applicable.
Compliance gaps generate hypotheses with compliance_gap type.

### Full Mode — L7.11

All 4 steps + regulatory framework deep-scan + BRS calculation for compliance blast radius.
For each major compliance gap: [G.ORG] Layer 4 (regulatory exposure) is mandatory.

### Updated Standard Mode Summary (all L7 sub-categories)

```
Standard: L7.1 (rate limits), L7.2 (idempotency), L7.3 (timeouts),
           L7.4 (observability), L7.5 (resource lifecycle), L7.8 (authz),
           🆕 L7.11 (compliance — if scope includes user/financial/health data)
Defer: L7.6, L7.7, L7.9, L7.10 to Full or specialized audit
```

### Updated Full Mode Summary

```
Full: All 10 sub-categories (L7.1-L7.10) + 🆕 L7.11
For each: produce explicit hypothesis (even if "no findings")
```

---

## Output Integration

L7.11 findings integrate into [G.H] output:

```yaml
hypotheses:
  - id: H-042
    layer: L7.11
    description: "User PII exported to analytics service without documented lawful basis"
    compliance_framework: "GDPR Art.6 (lawful basis for processing)"
    compliance_gap_type: "missing_lawful_basis_documentation"
    qbr_data_integrity_risk: 3
    qbr_user_facing_impact: 2
    qbr_score: 22  # MANDATORY
    triggers_ORG: true  # Layer 4 blast radius required
    evidence_anchor: "analytics_service.py:147"
```

---

*Reference 11 — VHEATM 10.0 | L7.1-L7.10 unchanged from 9.0; L7.11 new
Research basis: Essien et al. 2021 fintech compliance study; Tamburri 2020 GDPR analysis;
COSO/COBIT Three Lines of Defense; DevSecOps GDPR/SOC2/HIPAA systematic reviews*

---

## 🆕 AI Security Sub-Layers: AI-S1 to AI-S4 (v12.0 — replaces L6.7-L6.9)

> Activates when AI_INTEGRATED = YES.
> L6.7-L6.9 from v11 were self-invented categories. v12 replaces them with
> established taxonomies that have research backing and community maintenance.
> → Full routing protocol: `references/22-specialist-lens-router.md`

### AI-S1: OWASP LLM Top 10 2025 (Model / Prompt / Output Layer)

Scan for these 10 vulnerability classes in any AI-integrated system:

```
LLM01 Prompt Injection
  "Does any user-controlled input flow unsanitized into an AI prompt?"
  Research: McHugh 2025 (9 cit.) — Prompt Injection 2.0 hybrid AI threats.
  Ferrag 2025 (10 cit.) — end-to-end threat model covering 30+ attack techniques.
  Evidence: attack success rates > 84% even with defenses (ASB Zhang 2024, 101 cit.)

LLM02 Insecure Output Handling
  "Does AI output flow directly to DB, UI, or API without schema validation?"

LLM03 Training Data Poisoning
  "Is any training data sourced from user-controllable input?"

LLM04 Model Denial of Service
  "Can an attacker exhaust AI compute budget via crafted inputs?"

LLM05 Supply Chain Vulnerabilities
  "Are AI model weights, plugins, or prompts sourced from untrusted third parties?"

LLM06 Sensitive Information Disclosure
  "Can the model be prompted to reveal training data, system prompts, or PII?"

LLM07 Insecure Plugin Design
  "Do AI plugins have excessive permissions or lack input validation?"

LLM08 Excessive Agency
  "Does the AI agent have more permissions than needed for its task?"

LLM09 Overreliance
  "Are there human review gates for high-stakes AI decisions?"

LLM10 Model Theft
  "Can the model be extracted via repeated queries?"
```

### AI-S2: OWASP Agentic Top 10 2026 (Agent Action / Tool / Identity Layer)

```
A01 Agent Goal Hijack: "Can external input change the agent's goal mid-task?"
A02 Tool Misuse: "Can the agent be tricked into using tools for unintended actions?"
A03 Identity/Privilege Abuse: "Can the agent act with higher privilege than intended?"
A04 Agentic Supply Chain: "Are third-party agents in the orchestration trusted?"
A05 Unexpected Code Execution: "Can the agent write and execute arbitrary code?"
A06 Memory/Context Poisoning: "Can past context be injected with malicious instructions?"
A07 Insecure Inter-Agent Communication: "Is trust between agents authenticated?"
A08 Cascading Failures: "Does one agent's failure propagate to dependent agents?"
A09 Human-Agent Trust Exploitation: "Does the system over-trust agent outputs?"
A10 Rogue Agents: "Can an agent detach from its goal and pursue autonomous objectives?"
```

### AI-S3: MAESTRO 7-Layer Threat Model (Layered Architecture)

```
Layer 1 Foundation Model: "What model? Known failure modes? Version pinned?"
Layer 2 Data Operations: "Input quality, poisoning resistance, data lineage?"
Layer 3 Agent Frameworks: "Framework security posture? Tool inventory controlled?"
Layer 4 Deployment/Infra: "Isolation? Secrets management? Network exposure?"
Layer 5 Evaluation/Observability: "Can failures be detected? Metrics for drift?"
Layer 6 Security/Compliance: "Controls in place? Audit logs? Compliance obligations?"
Layer 7 Agent Ecosystem: "Third-party agents trusted? Inter-agent auth?"
```

### AI-S4: MITRE ATLAS Adversarial Technique Mapping

```
For each confirmed AI-S finding, map to ATLAS tactic:
  Reconnaissance: attacker gathers info about AI system
  Resource Development: attacker builds capability to attack AI
  Initial Access: attacker gains foothold via AI interface
  ML Attack Staging: attacker prepares adversarial inputs/data
  Model Evasion: attacker crafts inputs to bypass detection
  Exfiltration: attacker extracts data/model knowledge
  Impact: attacker damages AI system or its outputs

ADR for AI-S4 findings should include ATLAS technique ID where applicable.
```

### Evidence Adapter for AI-S Findings

```
AI-S hypotheses use VHEATM Evidence Tiers:
  T1: Test result (adversarial prompt successfully injected / output validated against schema)
  T2: Code review evidence (data flow traced: user input → AI prompt → unsanitized)
  T3: Architectural evidence (design shows no output validation layer)
  T4: Inferred (AI integration pattern known to be vulnerable by default)
  T5: Intuition / unanchored → NOT acceptable for MANDATORY finding

Note: static confidence for AI-S findings is MEDIUM by default.
[E.HV] Step 2 = adversarial test required for MANDATORY AI-S findings.
```
