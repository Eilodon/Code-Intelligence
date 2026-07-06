# NIST AI RMF Overlay — [AI-RMF] (v12.0)

> **Research Basis:**
> [NIST AI RMF (Tabassi, 2023, 145 citations)](https://consensus.app/papers/details/77687382dc795819b10f866cb132cc3e/):
> voluntary framework for trustworthy AI: Govern, Map, Measure, Manage.
> Context-specific, lifecycle-aware, continuous improvement oriented.
>
> [AAGATE (Huang, 2025)](https://consensus.app/papers/details/3bbc4f5851c255bdbeb210e4d731a208/):
> Kubernetes-native operationalization of NIST AI RMF. Key integration mapping:
> MAESTRO → Map function; OWASP AIVSS + SEI SSVC → Measure; CSA Agentic Red Teaming → Manage.
> This is the blueprint for v12's AI RMF integration.
>
> [Dotan et al. 2024 (14 citations)](https://consensus.app/papers/details/889fbf708b39583aa09f532fe604590e/):
> maturity model based on NIST AI RMF. Private sector lags behind framework consensus.
> Implementation is "sporadic and selective" — risk of providing "misleading veneer of legitimacy."
> Validates VHEATM's evidence-gated approach: no ceremonial compliance checks.
>
> [Ee et al. 2024 (13 citations)](https://consensus.app/papers/details/bd64de62d84f5891a48ef52241991f46/):
> defense-in-depth for AI risks: functional approach (NIST AI RMF) + lifecycle approach
> + threat-based approach (MITRE ATLAS). All three needed.
>
> **Design decision:** AI RMF is governance overlay, not audit methodology.
> It maps ONTO existing VHEATM phases rather than adding new phases.
> GOVERN → [P] declarations. MAP → [G.H] + [V]. MEASURE → [E.HV] + QBR.
> MANAGE → [A] ADRs + [T] + [M]. This is how AAGATE operationalizes it.

---

## When to Activate [AI-RMF]

```
MANDATORY:
  □ AI_INTEGRATED = YES AND Full mode
  □ AI system serving high-stakes decisions (hiring, credit, medical, legal)
  □ Agentic AI with autonomous tool use in scope

RECOMMENDED:
  □ AI_INTEGRATED = YES AND Standard mode
  □ AI component in enterprise system with compliance obligations

NOT APPLICABLE:
  □ FAST mode (use AI-S1 to AI-S4 only)
  □ No AI components in scope
```

---

## 4-Function Mapping onto VHEATM Phases

### GOVERN → [P] Pre-conditions + [M] Governance Review

GOVERN = policies, accountability, transparency, workforce practices for AI.

**At [P], add governance declarations:**
```
AI_GOVERNANCE declarations (when AI_INTEGRATED = YES):
  □ Is there a documented AI use policy for this system? YES / NO / PARTIAL
  □ Who is accountable for AI outputs? [role / team]
  □ Are AI outputs subject to human review for high-stakes decisions? YES / NO / N/A
  □ Is there documentation of the AI system's intended use and known limitations?
  □ Is there a training/awareness program for teams deploying this AI?
  □ Is there a stakeholder feedback mechanism for AI errors?

If any = NO → governance gap identified → REQUIRED ADR in [A]
```

**At [M] Full mode, add governance review:**
```
GOVERN closure check:
  □ Review findings from this cycle: do any reveal governance failures
    (decisions made by AI without appropriate oversight)?
  □ Any finding where AI output was trusted without human validation
    in a high-stakes context? → GOVERNANCE FAILURE ADR
  □ Is there a governance process for updating AI policies when
    risk profile changes? If NO → flag for [M.AM]
```

---

### MAP → [V] + [G.H] Risk Identification

MAP = identify AI-specific risks, context, and potential impacts.

**At [V] Vision, add AI system context mapping:**
```
AI System Context Map (required for MAP function when AI_INTEGRATED = YES):
  □ AI model(s) in scope: [model name, version, provider]
  □ AI inputs: [what data flows into AI — user input, DB query, documents?]
  □ AI outputs: [what does AI return — text, structured data, actions, decisions?]
  □ AI usage context: [informational / decision-support / autonomous action]
  □ Data sensitivity: [does AI process PII / financial / health data?]
  □ Failure modes at AI boundary: [what happens if AI returns wrong, hallucinated, or malicious output?]
  □ Human oversight: [where does a human review/override AI output?]
```

**[G.H] MAP integration:**
The AI System Context Map feeds [G.H] AI-S1 to AI-S4 (via Specialist Lens Router).
MAP ensures the scope is defined before AI threats are enumerated.

---

### MEASURE → [E.HV] + QBR

MEASURE = quantify AI-specific risks with evidence.

```
MEASURE applies to AI-related hypotheses in [E]:

For each AI-related hypothesis (layer AI-S1 to AI-S4):
  □ Can the risk be measured/observed? What metric or signal?
  □ Is there a baseline for "normal" AI behavior that this deviates from?
  □ What would trigger detection of this risk in production?

QBR calibration for AI risks:
  user_facing_impact: How often do real users encounter this AI failure mode?
  data_integrity_risk: Does this AI risk produce incorrect data used in decisions?
  security_risk: Does this risk enable prompt injection, data extraction, or unauthorized action?
  blast_radius: If this AI risk triggers, how many users/services are affected?

[E.HV] for AI-S hypotheses:
  AI attack success rates > 84% (ASB benchmark Zhang 2024).
  Static analysis alone insufficient for AI attack surface.
  For AI-S1 to AI-S4: static confidence = MEDIUM by default.
  [E.HV] Step 2 = adversarial test required (prompt injection test, output validation test).
```

---

### MANAGE → [A] ADRs + [T] + [M.AP]

MANAGE = prioritize and address AI risks; implement controls; monitor.

**[A] ADR additions for AI risks:**
```
AI-specific ADR fields (add to 9-part ADR when AI-related finding):
  Part 10: AI Risk Treatment:
    Risk type: [OWASP LLM / Agentic category]
    Treatment: MITIGATE | TRANSFER | ACCEPT | AVOID
    Control: [specific technical control — e.g., input sanitization, output schema validation,
              privilege separation, rate limiting for AI calls]
    Monitoring: [how to detect if this risk materializes in production]
    Human oversight: [what human review is triggered if control fails?]
```

**[M] MANAGE closure check:**
```
At [M] Full mode for AI-integrated systems:
  □ For each AI-S finding: is there a monitoring plan?
    If NO → Detection Gap ADR (same as POST-INCIDENT profile detection gap)
  □ For each AI-S ADR: is there a rollback plan if AI misbehaves?
    If NO → add to debt register as Rollback Gap
  □ Is there a plan for model drift / capability change?
    AI models change behavior over time without code changes.
    → Flag if no monitoring for behavioral drift is in place.
```

---

## NIST AI RMF × MAESTRO Integration (from AAGATE)

Following AAGATE's integration blueprint:

```
MAESTRO 7-layer → MAP function:
  Layer 1: Foundation Model → MAP: "What AI model is this? What are its known failure modes?"
  Layer 2: Data Operations → MAP: "What data flows in/out? What quality/poisoning risks?"
  Layer 3: Agent Frameworks → MAP: "What agent orchestration? Tool inventory?"
  Layer 4: Deployment/Infra → MAP: "How is the AI deployed? What infrastructure dependencies?"
  Layer 5: Evaluation/Observability → MEASURE: "How is AI performance measured?"
  Layer 6: Security/Compliance → GOVERN + MANAGE: "What controls exist? Are they enforced?"
  Layer 7: Agent Ecosystem → MAP: "What third-party agents/plugins? What trust model?"

OWASP LLM/Agentic → MEASURE function:
  Each OWASP vulnerability = a measurable risk with known attack patterns.
  MEASURE: "Does monitoring exist to detect [LLM01-Prompt Injection] attempts?"

MITRE ATLAS → MAP + MANAGE:
  MAP: "What adversarial techniques could target this AI system?"
  MANAGE: "What controls mitigate each mapped technique?"
```

---

## AI RMF Output at [M]

```yaml
nist_ai_rmf_overlay:
  ai_system_scope: "[name / model / version]"
  govern:
    policy_exists: true | false
    accountability_documented: true | false
    human_review_for_high_stakes: true | false
    governance_gaps: ["[description]", ...]
    governance_adrs_generated: [count]
  map:
    ai_inputs_documented: true | false
    ai_outputs_documented: true | false
    human_oversight_points: [count]
    ai_threat_surface_mapped: true | false
    maestro_layers_covered: [1-7 list]
  measure:
    ai_hypotheses_generated: [count]
    owasp_categories_triggered: ["LLM01", "A02", ...]
    evidence_quality_ai_findings: HIGH | MEDIUM | LOW
    monitoring_coverage: "[% of AI-S findings with detection plan]"
  manage:
    ai_adrs_issued: [count]
    treatment_breakdown:
      mitigate: [count]
      accept: [count]
      transfer: [count]
      avoid: [count]
    detection_gaps: [count]
    rollback_gaps: [count]
    drift_monitoring_plan: true | false
```

---

## Important Caveat

Dotan et al. 2024 warns: AI RMF compliance without substance creates "misleading veneer of legitimacy."
VHEATM's evidence-gated protocol prevents this:
- GOVERN checks are concrete YES/NO declarations, not narrative claims
- MAP requires actual AI system documentation, not checkbox
- MEASURE requires QBR scores and [E.HV] verification for AI findings
- MANAGE requires ADRs with specific controls and monitoring plans

Ceremonial AI RMF compliance (declaring "we do AI risk management" without evidence)
violates VHEATM's Ritual Suppression Rule.

---

*Reference 26 — VHEATM 12.0 | NIST AI RMF Overlay
Research: Tabassi 2023 (145 cit.); AAGATE Huang 2025; Dotan 2024 (14 cit.); Ee 2024 (13 cit.)*
