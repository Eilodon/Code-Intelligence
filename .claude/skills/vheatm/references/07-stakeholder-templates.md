# Stakeholder Templates — Full Mode (v10.0)

Changes from 9.0:
- Added ENTERPRISE stakeholder types: Platform team, Compliance/Legal, SRE, Data team
- Added Org Ownership Map template for ENTERPRISE mode [V] phase
- Added Team B perspective to Persona Rotation for boundary-crossing ADRs
- Added Regulatory Stakeholder lens

---

## Stakeholder Declaration Template (updated)

```
Primary stakeholder (this audit primarily serves):
  □ Builder / Engineering — optimize for code quality + velocity
  □ Product / Business — optimize for user outcomes + business risk
  □ Security / Compliance — optimize for attack surface + regulatory risk
  □ End users — optimize for experience + safety + trust
  □ 🆕 SRE / Operations — optimize for reliability, incident response, runbooks
  □ 🆕 Data / Analytics team — optimize for data correctness, pipeline reliability
  □ 🆕 Legal / Regulatory — optimize for compliance, liability exposure, audit readiness
  □ 🆕 Platform team — optimize for stability, backward compatibility, contract adherence
  □ Multiple: [list with weighting]

Secondary stakeholders (affected but not primary target):
  [List — findings affecting them noted in handoff, not escalated by default]

🆕 Org-Context declaration (ENTERPRISE mode):
  Teams with code in scope: [list]
  Auditor's team: [team name or "external"]
  Org Capture risk: HIGH | MEDIUM | LOW | N/A
  Teams that will receive ADRs (boundary crossings): [list]
```

---

## Common Stakeholder Lenses (updated)

### Engineering / Builder Lens (unchanged)
Correctness, velocity, maintainability, testability.
MANDATORY threshold: breaks existing functionality or blocks development.

### Product / Business Lens (unchanged)
User value, time-to-market, revenue risk.
MANDATORY threshold: blocks launch or user-facing failure.

### Security / Compliance Lens (updated)
Attack surface, data exposure, regulatory requirements.
MANDATORY threshold: any exploitable vulnerability or compliance gap.
🆕 Key question: "Is this safe to deploy, compliant with regulations, AND auditable?"

### End User Lens (unchanged)
Usability, reliability, trust, safety.
MANDATORY threshold: user cannot complete core task or is harmed.

### Legal / Risk Lens (updated)
Liability, IP, contractual obligations, regulatory exposure.
MANDATORY threshold: clear legal risk or contract violation.
🆕 Key question: "Does this create a regulatory notification obligation? A fine exposure?
                   A contractual breach?"

### 🆕 SRE / Operations Lens
Reliability, observability, incident response.
MANDATORY threshold: undetectable failure mode, missing runbook, cascading failure risk.
Key question: "If this fails at 3am, can I detect it, understand it, and fix it within SLA?"

### 🆕 Platform / Infrastructure Lens
Backward compatibility, shared service stability, API contracts.
MANDATORY threshold: breaks consumers downstream without migration path.
Key question: "Does this change break an implicit or explicit contract with consumers?"

### 🆕 Cross-Team Consumer Lens (ENTERPRISE mode)
Integration correctness, assumption validity, handoff clarity.
MANDATORY threshold: integration break that Team A won't discover in testing.
Key question: "From Team B's perspective, does this ADR make sense? Is the handoff complete?"

---

## 🆕 Org Ownership Map Template (ENTERPRISE mode, [V] phase)

```markdown
# Org Ownership Map — Cycle [N]

## Component Ownership
| Component / Service | Owning Team | On-call | Slack/channel | Escalation |
|---|---|---|---|---|
| [Service A] | [Team X] | [person] | [#channel] | [process] |
| [Service B] | [Team Y] | [person] | [#channel] | [process] |
| [Shared DB] | [Team X + Z] | [rotation] | [#channel] | [DRI: Team X] |

## SLA Commitments
| Service | Committed to | Uptime SLA | Response Time SLA | Breach consequence |
|---|---|---|---|---|
| [API Gateway] | All consumers | 99.95% | P95 < 500ms | [incident + credits] |
| [Auth Service] | Product teams | 99.99% | P99 < 100ms | [escalation] |

## Regulatory Scope
| Component | Data type | Applicable frameworks | DPO contact |
|---|---|---|---|
| [User service] | PII | GDPR, CCPA | [contact] |
| [Payment service] | Financial | PCI-DSS | [contact] |

## Known Ownership Gaps
| Component | Gap description | Last owner | Status |
|---|---|---|---|
| [Legacy service X] | No current owner | [former team] | EP-01: orphaned |
```

---

## Persona Rotation — [M] Stranger Review (updated for v10.0)

Standard 4 rotations (unchanged):
1. Senior Engineer — "Is this technically correct and implementable?"
2. Security Auditor — "What attack surface does this expose or close?"
3. Junior Engineer — "Can I implement this from the ADR alone?"
4. Ops Engineer — "What does this do to incident response at 2am?"

🆕 5th rotation for ENTERPRISE mode or any BOUNDARY=YES ADR:
5. **Team B Engineer** — reads ADR from the receiving team's perspective:
   "Does this ADR make sense from outside Team A?
    What is missing in the handoff?
    What assumption is Team A making about our system that isn't stated here?
    What would block me from completing my side of this handoff?"

If Team B rotation surfaces a gap → escalate to new REQUIRED ADR before closing.

🆕 6th rotation for L7.11 compliance findings:
6. **Compliance Officer / DPO** — reads ADR from regulatory perspective:
   "Does this fix actually address the regulatory obligation, not just the symptom?
    What evidence would I present to a regulator that this was remediated?
    Is the audit trail updated to show the fix?"

---

## ENTERPRISE: Stakeholder Matrix

For high-BRS findings in ENTERPRISE mode, use this matrix:

```
| Finding | Engineering | Product | Security | SRE | Legal | Compliance | Affected Team B |
|---|---|---|---|---|---|---|---|
| ADR-1 (BRS=14) | IMPLEMENT | AWARE | APPROVE | RUNBOOK | REVIEW | SIGN-OFF | INTEGRATE |
| ADR-2 (BRS=6) | IMPLEMENT | AWARE | — | RUNBOOK | — | — | — |
| ADR-3 (BRS=3) | IMPLEMENT | — | — | — | — | — | — |

Action codes:
  IMPLEMENT: responsible for code change
  APPROVE: must sign off before deployment
  AWARE: notified, no action required
  REVIEW: must review for correctness in their domain
  SIGN-OFF: formal approval required (compliance gate)
  RUNBOOK: must update/create runbook
  INTEGRATE: receiving team — must confirm integration works
```

---

*Reference 07 — VHEATM 10.0 | v9.0 + ENTERPRISE stakeholder additions*
