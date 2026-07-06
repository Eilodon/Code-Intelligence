# Context Modes — Full Adaptation Table (v10.0)

## Five Modes

| Mode | When to use |
|---|---|
| **DESIGN** | Auditing a product concept, PRD, specification, or proposal. No running code. |
| **CODE** | Auditing a codebase not yet in production. Tests may or may not exist. |
| **LIVE** | Auditing a production system with real users. Metrics and incidents available. |
| **LEGACY** | Codebase with significant technical debt. Partial docs, original authors unavailable. |
| 🆕 **ENTERPRISE** | Multi-system, multi-team, cross-regulatory, production at scale. Multiple teams own components. SLAs, compliance obligations, and organizational incentives are first-class audit concerns. Use when ALL of the following are true: (1) >2 teams own distinct parts of the system under audit, AND (2) at least one of: any regulatory framework applies (GDPR/PCI-DSS/HIPAA/SOC2), or a bug's fix would require changes by more than one team. **Note**: a LIVE system with external users is not automatically ENTERPRISE. The distinguishing signal is cross-team ownership of the fix path, not just regulatory exposure. If only one team owns all the code but the system is under GDPR, use LIVE with L7.11 activated. ENTERPRISE adds organizational blast radius modeling on top. |

---

## Adaptation Table by Phase

| Phase | DESIGN | CODE | LIVE | LEGACY | 🆕 ENTERPRISE |
|---|---|---|---|---|---|
| **[V] Architecture map** | Logical diagram | Actual file structure | Deployment topology | Reconstructed from code | Org ownership map + deployment topology + SLA chains |
| **Evidence Anchoring** | T1-T2 for MANDATORY | file:line required | file:line + metrics | file:line + uncertainty flag | file:line + team-confirmation + regulatory anchor |
| **[E] Execution Mode** | STATIC only | STATIC or LIVE | LIVE preferred | STATIC, no reliable baseline | LIVE preferred + multi-team verification |
| **[G.U] Unknown Probe** | Spec Comparison primary | All 3 techniques | Gap from metrics | README vs Reality primary | Org boundary gap analysis primary |
| **Red-Green Gate** | Conceptual only | STATIC or LIVE | Must use LIVE | STATIC with caveat | LIVE + cross-team sign-off |
| **CLI formula** | Document complexity units | Standard formula | Standard + incident log | LCC (see below) | Enterprise formula (see below) |
| **[G.UX] UX Lens** | Required | Recommended | Recommended | Optional | Recommended (multi-stakeholder UX) |
| **[G.INC]** | N/A | FAST signal only | FAST signal only | Recommended | **MANDATORY** |
| **[G.ORG]** | N/A | Recommended | Recommended | Recommended | **MANDATORY** |
| **L7.11 Compliance** | Future-looking | Recommended | Recommended | Recommended | **MANDATORY** |

---

## Evidence Tier (DESIGN mode only)

When there is no code to anchor to, use Evidence Tier instead of file:line:

| Tier | Label | Examples | MANDATORY ADR? |
|---|---|---|---|
| T1 | Peer-reviewed research / systematic review | Academic papers, meta-analyses | ✅ Acceptable |
| T2 | Industry data / known case studies | Post-mortems, engineering blogs, benchmarks | ✅ Acceptable |
| T3 | Expert consensus / documented best practices | Style guides, framework docs, RFCs | ✅ Acceptable (with note) |
| T4 | Logical inference / first-principles | Reasoning from known constraints | ❌ Downgrade to REQUIRED |
| T5 | Intuition / gut feeling | Unanchored assertions | ❌ Not acceptable |

🆕 **T2-Reg: Regulatory / Legal evidence** (applies in ENTERPRISE and DESIGN modes):
Legal text, audit reports, compliance officer documentation, case law.
MANDATORY ADR in compliance context: T2-Reg or better required.

---

## CLI Formula Adaptation

**Standard (CODE/LIVE):**
```
CLI = (lines_changed × 0.1) + (files_touched × 0.5) + (external_dependencies × 2)
CalibrationFactor (EMA-3) applied
```

**DESIGN mode adaptation:**
```
CLI = (sections_affected × 0.5) + (stakeholders_impacted × 1.0) + (downstream_systems × 2)
```

**🆕 LEGACY mode — Legacy Complexity Classifier (replaces × 1.5):**

Step 1: Classify the legacy system's complexity level:

| Level | Criteria | CLI Multiplier |
|---|---|---|
| **Level A** | Stand-alone system. No external service dependencies. At least one original author available. Partial but readable documentation. | × 1.5 |
| **Level B** | Part of a larger system. Data or API dependencies on other systems. Most original authors unavailable. Documentation incomplete or stale. | × 2.0 |
| **Level C** | Data-critical or multi-generation. Original authors unavailable. 3+ layers of patches by different teams. Critical business logic embedded in undocumented tribal knowledge. | × 3.0 |

Step 2: If Level C → **Tribal Knowledge Probe is MANDATORY**:

```
Tribal Knowledge Probe:
  □ Name 2 people who understand this subsystem most deeply.
  □ What critical knowledge lives only in their heads?
  □ What happens to this remediation if either is unavailable within the next sprint?
  □ What is the minimum documentation to transfer their knowledge before fix begins?
  □ If you cannot name 2 people → the system has no living knowledge owner.
    This is itself a MANDATORY finding (BC-010: undocumented-tribal-knowledge-system).
```

**Evidence base**: Ramasubbu & Balan (2015, 50 citations, Information Systems): 10-year
longitudinal study across 48 enterprise deployments found modular debt remediation was 53%
more effective than architectural, but increased vendor error probability by 83% — competing
remediation effects that a flat multiplier cannot model.

**🆕 ENTERPRISE mode formula:**
```
CLI_enterprise = CLI_standard
              + (ownership_boundaries_crossed × 3)
              + (regulatory_obligations_count × 2)
              + (sla_chains_at_risk × 1.5)

Where:
  ownership_boundaries_crossed = number of distinct team codebases that must change
  regulatory_obligations_count = number of distinct regulatory frameworks affected
  sla_chains_at_risk           = number of SLA commitments that could breach if bug hits prod
```

Example: A bug in a shared auth service at an enterprise, touching 3 teams' code,
under GDPR + PCI-DSS, with 2 SLA chains at risk:
```
CLI_enterprise = CLI_standard + (3 × 3) + (2 × 2) + (2 × 1.5)
               = CLI_standard + 9 + 4 + 3
               = CLI_standard + 16
```

---

## Context Mode — Self-Check

Before leaving [P]:
- [ ] Context Mode declared (DESIGN / CODE / LIVE / LEGACY / ENTERPRISE)
- [ ] Stakeholder declared
- [ ] Goal declared
- [ ] Org-Context declared (team that owns code, if applicable)
- [ ] Evidence Anchoring standard confirmed for this mode
- [ ] CLI formula variant noted (including LCC level if LEGACY)
- [ ] UX Lens requirement checked
- [ ] 🆕 ENTERPRISE activations checked: [G.INC] / [G.ORG] / L7.11 / LCC

---

## ENTERPRISE — What It Unlocks vs. LIVE

LIVE assumes a single production system owned by one team.
ENTERPRISE explicitly models the org layer.

**When to use LIVE vs ENTERPRISE:**

| Signal | Mode |
|--------|------|
| Single team owns all code; system happens to have external users | LIVE + L7.11 |
| Single team owns code; system under GDPR/PCI-DSS | LIVE + L7.11 |
| Multiple teams own code; any fix requires one team to change | ENTERPRISE |
| Bug's fix path crosses a team ownership boundary | ENTERPRISE |
| System under compliance AND multi-team ownership | ENTERPRISE |

The key discriminator is **fix path ownership**, not just regulatory scope.

| Concern | LIVE handles? | ENTERPRISE adds |
|---|---|---|
| Bug in single service | ✅ fully | Same |
| Bug that crosses team ownership | ⚠️ partial | [G.INC] + [G.ORG] mandatory |
| SLA chain breach risk | ⚠️ partial | ORG blast radius map |
| Regulatory exposure | ❌ not modeled | L7.11 mandatory |
| Incentive to defer fix | ❌ not modeled | [G.INC] probe mandatory |
| Legacy in multi-system context | ❌ flat × 1.5 | LCC with tribal knowledge |
| Multi-team adversarial pass | ❌ single perspective | 4-lens AP mandatory |

