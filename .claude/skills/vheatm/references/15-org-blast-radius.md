# Organizational Blast Radius — [G.ORG] (v10.0)

> **Research Basis**: Capital One breach case study (Khan et al., 2022, 43 citations,
> ACM TPAS): failure spanned control levels from technical issues to top management,
> Board of Directors, and Government regulators — a hierarchical failure no single-layer
> audit surfaces. Architecture Risk Analysis industry practice: blast radius modeling
> with telemetry, dependency maps, and ownership tags is standard in production-grade
> enterprise review. ITIL Post-Incident Reviews: SLA chains and escalation paths must be
> mapped before incidents, not discovered during them.
>
> **Implication**: VHEATM 9.0 maps technical blast radius (what code breaks).
> [G.ORG] maps organizational blast radius (what teams, SLAs, and legal exposure
> are touched when this bug reaches production).

Run AFTER a hypothesis is confirmed as **MANDATORY** in [E], BEFORE issuing ADR in [A].
Also runs at [G.INC] when BOUNDARY FLAG = YES.

---

## The Core Question

**"If this bug reaches production, what organizational layers does the failure propagate through?"**

Technical blast radius (in VHEATM since v1): "What code breaks?"
Organizational blast radius (new in v10): "What teams, SLAs, customers, and regulatory
obligations are affected when this failure manifests at scale?"

---

## The 4-Layer ORG Map

For each MANDATORY finding, map all 4 layers:

### Layer 1: Immediate Owner

```
Who owns the code/system where the bug lives?
  Primary team: [team name]
  Secondary owners (co-owners): [list]
  On-call rotation: [who gets paged if this fires at 3am?]
```

---

### Layer 2: Downstream Teams

```
Which teams are affected if this bug causes a production incident?
Map explicitly:

Direct consumers of this service:
  Team A — [how they're affected]
  Team B — [how they're affected]

Indirect consumers (one hop):
  Team C → through Team A → [how they're affected]

Infrastructure/platform teams:
  [SRE / DevOps / Platform] — [escalation and incident management burden]

Business teams:
  [Support / Sales / Legal] — [customer impact and business exposure]
```

**Key signal**: The more teams appear at Layer 2, the higher the REAL blast radius.
QBR's `blast_radius` input (0-3) should be calibrated against this map:
- 0-1 teams affected → blast_radius = 1
- 2-4 teams → blast_radius = 2
- 5+ teams or any business/legal team → blast_radius = 3

---

### Layer 3: SLA Chains

```
Which SLA commitments could breach if this bug fires?

Internal SLAs (team-to-team):
  [Service A] commits [99.9% uptime] to [Service B consumers]
  This bug could cause: [degradation / outage / data inconsistency]
  SLA breach probability if bug fires: HIGH / MEDIUM / LOW

External SLAs (customer-facing):
  [Product X] commits [uptime / response time / data accuracy] to [customer segment]
  SLA breach probability if bug fires: HIGH / MEDIUM / LOW
  Financial exposure: [penalties / credits / churn risk]

Regulatory SLAs:
  [Data processing within X hours per GDPR Art.12]
  [Audit trail completeness per SOC2]
  Compliance breach probability if bug fires: HIGH / MEDIUM / LOW
```

**When SLA information is unavailable:**
SLA documentation is often not accessible to a code auditor. If Layer 3 cannot be
fully mapped, use this fallback:
```
SLA_chains_at_risk: UNKNOWN
Action:
  □ Note "SLA mapping blocked — no access to team SLA documentation"
  □ Set sla_chains_at_risk = 1 as conservative floor (not 0)
  □ Flag in ADR: "BRS may be understated — SLA chains not verified"
  □ Recommended: request SLA info from team lead before marking ADR resolved
```
UNKNOWN is not the same as zero. Default to conservative assumption.

---

### Layer 4: Regulatory / Legal Exposure

```
Does this bug create regulatory or legal exposure if it fires?

□ Data exposure: personal data accessible to unauthorized parties?
  → GDPR Art.33 breach notification obligation (72 hours)
  → PCI-DSS §12.10 incident response requirements

□ Data integrity: financial or health data could be incorrect?
  → SOX audit trail requirements
  → HIPAA accuracy requirements

□ Data availability: required audit logs could be lost?
  → SOC2 CC7.2 (logical access controls and audit trails)

□ Service availability SLA breach creating contractual liability?
  → Review customer contracts for SLA penalty clauses

For each YES: document the specific obligation and the specific way
this bug could trigger it.
```

---

## Blast Radius Score (BRS)

After mapping all 4 layers, assign a Blast Radius Score:

```
BRS = teams_directly_affected + (sla_chains_at_risk × 2) + (regulatory_obligations × 3)

BRS Thresholds:
  BRS 0-3:   Contained blast radius — standard ADR, Owner field sufficient
  BRS 4-7:   Elevated blast radius — ADR requires handoff plan + SLA mitigation strategy
  BRS 8-12:  High blast radius — escalate to leadership visibility; consider incident pre-brief
  BRS 13+:   Critical blast radius — treat as pre-incident; leadership sign-off before any change
```

⚠️ **Calibration caveat (Evidence Tier: T4 — first-principles)**
The weights (×2 for SLA chains, ×3 for regulatory obligations) and thresholds (4/8/13)
are derived from first-principles reasoning, not empirical data. They reflect the relative
severity judgment that regulatory obligations carry higher organizational cost than SLA chains,
which carry higher cost than team count alone. These weights should be treated as starting
defaults and calibrated against your organization's actual incident history.

**How to calibrate**: After 3+ cycles using BRS, compare BRS tier against actual escalation
outcomes. If BRS=6 findings consistently required leadership involvement → lower the
"Elevated → High" threshold from 8 to 6 for your context. Log calibration to [KB].

The BRS feeds back into QBR calibration:
- If BRS ≥ 8 and QBR's `blast_radius` input was < 3 → adjust QBR upward, re-evaluate priority.

---

## ORG Map Output Template

```yaml
org_blast_radius:
  finding_id: ADR-[N]
  layer_1_immediate_owner:
    primary_team: "[team]"
    secondary_owners: ["[team]", ...]
    oncall_owner: "[person/rotation]"
  layer_2_downstream_teams:
    direct_consumers:
      - team: "[name]"
        impact: "[description]"
    indirect_consumers:
      - chain: "[A → B → C]"
        impact: "[description]"
    infrastructure_teams:
      - team: "[SRE/Platform]"
        impact: "incident management burden"
    business_teams:
      - team: "[Support/Legal]"
        impact: "[customer-facing / legal exposure]"
  layer_3_sla_chains:
    internal:
      - service: "[name]"
        commitment: "[SLA]"
        breach_probability: HIGH | MEDIUM | LOW
    external:
      - product: "[name]"
        commitment: "[SLA]"
        breach_probability: HIGH | MEDIUM | LOW
        financial_exposure: "[estimate or 'unknown']"
    regulatory:
      - framework: "[GDPR/SOC2/PCI-DSS]"
        obligation: "[specific article/requirement]"
        breach_probability: HIGH | MEDIUM | LOW
  layer_4_regulatory_legal:
    data_exposure: true | false
    data_integrity: true | false
    data_availability: true | false
    contractual_liability: true | false
    obligations_triggered: ["[obligation 1]", ...]
  blast_radius_score: [N]
  brs_tier: CONTAINED | ELEVATED | HIGH | CRITICAL
  escalation_required: true | false
  adr_requirements:
    handoff_plan_required: true | false
    sla_mitigation_required: true | false
    leadership_visibility: true | false
    leadership_signoff: true | false
```

---

## Integration with ADR

Every MANDATORY ADR's Owner field and Boundary field are populated from [G.ORG]:

```
Owner:    [from Layer 1 — primary team]
Boundary: [YES/[type] if Layer 2 has cross-team impact; NO if contained]

Escalation note (if BRS ≥ 8):
  "This finding has BRS=[N]. Leadership visibility required before implementation.
   SLA chains at risk: [list]. Regulatory obligations: [list]."
```

---

## Mode Adaptation

### FAST Mode

Layer 1 only: "Who owns this? Who gets paged if it fires?"
Document: "ORG-FAST: ownership map only. Layers 2-4 deferred."
If anything suggests cross-team or regulatory → note "ORG-FAST: elevated probe recommended."

### Standard Mode

Layers 1 + 2 + 3 (SLA chains). Layer 4 (regulatory) if scope includes user/financial data.
BRS calculated. ADR owner/boundary fields populated.

### Full Mode

All 4 layers. BRS calculated. Escalation triggered if BRS ≥ 8.
Cross-referenced with [G.INC] org friction data for realistic remediation timeline.

---

## ENTERPRISE-Specific: Ownership Map at [P]

In ENTERPRISE mode, the organizational blast radius mapping BEGINS AT [P], not [G.ORG].
The [V] Vision phase should produce an **Org Ownership Map** alongside the C4 architecture map:

```
Org Ownership Map (ENTERPRISE mode, required at [V]):
  Component A → owned by Team X → on-call: [person]
  Component B → owned by Team Y → on-call: [person]
  Component C → shared ownership: Teams X + Z → escalation: [process]
  API Gateway → owned by Platform team → SLA: [99.95% uptime]
  Data store → owned by Data team → regulatory: GDPR + SOC2
```

This map makes [G.ORG] probes much faster — auditor looks up the map rather than
reconstructing it mid-audit.

---

## Calibration: When ORG Blast Radius Reveals More Than the Bug

Sometimes [G.ORG] reveals that the blast radius itself is a finding, independent
of the specific bug being audited:

**Example**: Bug H-031 (minor input validation gap) has:
- Layer 2: 7 downstream teams
- Layer 3: 3 external SLA chains
- Layer 4: GDPR notification obligation

The bug is QBR = 12 (REQUIRED). But the BRS = 14 (CRITICAL).

**Conclusion**: The *architecture* of this component is the real problem — a minor bug
propagates to 7 teams and triggers GDPR obligations. The ADR for H-031 should include
a RECOMMENDED ADR for architectural isolation of this component.

→ This is the integration of [G.ORG] with [V] Vision — blast radius findings feed back
into architectural recommendations.

---

## Anti-Patterns

🚫 **"Blast radius is covered by QBR's blast_radius input"** — QBR blast_radius is
a 0-3 estimate of code-level impact. [G.ORG] is an organizational map.
They're measuring different things. Both are needed.

🚫 **"We'll figure out the SLA impact during the incident"** — That's the definition
of a bad incident. Map it now.

🚫 **"Regulatory exposure is Legal's problem, not audit's"** — The audit is the last
structured review before deployment. If regulatory exposure isn't flagged here,
it won't be flagged until after breach.

🚫 **"Only MANDATORY findings need ORG map"** — Correct for Standard mode.
But in ENTERPRISE mode, REQUIRED findings with BRS ≥ 4 should also get Layer 1-2 maps.

---

*Reference 15 — VHEATM 10.0 | Research basis: Capital One case study (Khan et al., 2022),
Architecture Risk Analysis industry practice, ITIL Post-Incident Reviews, SLA chain modeling*
