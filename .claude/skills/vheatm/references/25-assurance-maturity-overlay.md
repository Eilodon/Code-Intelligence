# Assurance Maturity Overlay — [M.AM] (v12.0)

> **Research Basis:**
> [Brasoveanu et al. 2022 (12 citations)](https://consensus.app/papers/details/4d9ebe6f0b59536ba762826164ae48c8/):
> SAMM + BSIMM combination for maturity self-assessment — validated in developer survey.
> Framework helps teams "understand security posture and identify which practices need improvement."
>
> [Fucci et al. 2024](https://consensus.app/papers/details/70a7a06bcd90530dbf4f8cccb17db5c5/):
> lightweight SAMM assessment (survey vs workshop) is viable in industry.
> Key finding: different roles perceive maturity differently — lightweight approach
> deviates only for specific practices. "Productive and efficient solution in industrial practice."
>
> [Weir et al. 2021 (21 citations, FSE)](https://consensus.app/papers/details/f7ea4b9a6f2a5a0f8a86ab7373df3aa0/):
> BSIMM study of 675,000 developers: identifies 12 developer security activities adopted
> early together, and more often than any others. Validation: activities related to
> detecting/responding to vulnerabilities are adopted earlier than prevention activities.
>
> NIST SSDF (Secure Software Development Framework): 4 practice groups:
> Prepare Organization (PO), Protect Software (PS), Produce Well-Secured Software (PW),
> Respond to Vulnerabilities (RV).
>
> **Design decision:** [M.AM] does NOT produce a maturity score.
> Scores require baseline data, multiple cycles, and calibration VHEATM doesn't have.
> Instead: per-finding maturity delta — "this finding indicates weakness in [SAMM practice]."
> This is actionable immediately; a maturity score is not.

---

## When to Run [M.AM]

```
MANDATORY:
  □ Full mode AND same bug type found in 2+ cycles (recurring pattern)
  □ ENTERPRISE mode + Full mode (org maturity is relevant to blast radius)
  □ POST-INCIDENT with systemic root cause (process/maturity failure, not just code)

RECOMMENDED:
  □ Full mode, any cycle (provides improvement recommendations)
  □ When stakeholder is Engineering Leadership or CISO (maturity language is relevant)

NOT APPLICABLE:
  □ FAST mode
  □ Standard mode unless explicitly requested
  □ Ad-hoc bug hunt with no process improvement intent
```

---

## SAMM 5 Business Functions (primary model)

[OWASP SAMM v2.0](https://owaspsamm.org/model/):

```
1. GOVERNANCE
   Security Practices: Policy & Compliance, Education & Guidance, Metrics & Feedback
   Finding signal: Missing policies, no developer security training, no metrics on bugs

2. DESIGN
   Security Practices: Threat Assessment, Security Requirements, Secure Architecture
   Finding signal: No threat model, missing security requirements in PRD,
                   architecture decisions not documented

3. IMPLEMENTATION
   Security Practices: Secure Build, Secure Deployment, Defect Management
   Finding signal: Dependency vulnerabilities, no SAST in CI/CD,
                   no defect tracking with severity/SLA

4. VERIFICATION
   Security Practices: Architecture Assessment, Requirements-Driven Testing, Security Testing
   Finding signal: Architecture never reviewed, no security-specific tests,
                   no penetration testing or fuzzing

5. OPERATIONS
   Security Practices: Incident Management, Environment Management, Operational Management
   Finding signal: No incident playbooks, config drift, no patch management process
```

---

## BSIMM 12 Early-Adoption Activities (descriptive overlay)

From Weir et al. 2021 (BSIMM data from 675,000 developers):
The 12 activities adopted earliest and most consistently across organizations:

```
1.  Security training for developers (Governance)
2.  Security feature review (Design)
3.  Penetration testing (Verification)
4.  Code review for security (Verification)
5.  Bug bar / severity classification (Implementation)
6.  Security champions program (Governance)
7.  Attack surface review (Design)
8.  Security testing in QA (Verification)
9.  SAST deployment (Implementation)
10. Dependency analysis (Implementation)
11. Security requirements in SDLC (Design)
12. Incident response plan (Operations)

When [M.AM] finds a missing practice from this list → REQUIRED improvement recommendation.
These are baseline expectations — not aspirational targets.
```

---

## NIST SSDF Practice Groups (secure producer assurance)

```
PO — Prepare the Organization:
  PO.1: Security requirements in roles and responsibilities
  PO.2: Implement supporting tooling
  PO.3: Provide security education and training

PS — Protect Software:
  PS.1: Protect code from unauthorized access
  PS.2: Provide mechanism to verify software integrity
  PS.3: Protect code from tampering during build

PW — Produce Well-Secured Software:
  PW.1: Design software to meet security requirements
  PW.4: Reuse existing, well-secured software
  PW.5: Create source code by following secure coding practices
  PW.6: Configure build process to improve security
  PW.7: Review and/or analyze software to identify vulnerabilities
  PW.8: Test software to identify vulnerabilities
  PW.9: Configure software to have secure settings by default

RV — Respond to Vulnerabilities:
  RV.1: Identify and confirm vulnerabilities
  RV.2: Assess, prioritize, and remediate vulnerabilities
  RV.3: Analyze vulnerabilities to identify root causes
```

---

## Per-Finding Maturity Delta Protocol

For each MANDATORY or REQUIRED finding in [A], run this analysis at [M.AM]:

```
Step 1: Classify the finding type
  □ ONE-OFF DEFECT: specific code error, unlikely to recur from same cause
  □ RECURRING PATTERN: same bug class found 2+ cycles (check KB Bug Class Catalog)
  □ MISSING CONTROL: a security/quality control is entirely absent (not just buggy)
  □ PROCESS FAILURE: a practice was followed incorrectly despite existing in policy

Step 2: Map to SAMM function
  ONE-OFF DEFECT:
    → Finding = weakness in SAMM Verification (tests didn't catch it)
    → Recommendation: add test for this failure class

  RECURRING PATTERN:
    → Finding = weakness in SAMM Implementation (defect management)
    → Recommendation: add to defect policy; automate detection via SAST

  MISSING CONTROL:
    → Finding = weakness in SAMM Design (control never specified) or
               SAMM Implementation (control specified but not built)
    → Recommendation: identify why control is missing; add to security requirements

  PROCESS FAILURE:
    → Finding = weakness in SAMM Governance (training/education) or
               SAMM Operations (process enforcement)
    → Recommendation: training gap or process enforcement review

Step 3: Map to NIST SSDF
  Security finding → PW.7 (review for vulnerabilities) or PW.8 (test for vulnerabilities)
  Defect recurring → RV.3 (analyze root causes) + PW.1 (design requirements)
  Missing control → PO.1 (security roles) + PW.1 (security design requirements)
  No incident response → RV.2 (remediation process) + Operations

Step 4: BSIMM baseline check
  Is the missing practice one of the 12 BSIMM baseline activities?
  YES → "This is a baseline expectation across [X]% of BSIMM organizations.
          Absence indicates maturity gap below industry baseline."
  NO  → "This is an advanced practice. Absence may be acceptable at current maturity level."
```

---

## [M.AM] Output Schema

```yaml
assurance_maturity_overlay:
  cycle: [N]
  findings_analyzed: [N]
  recurring_patterns_found: [count]
  missing_controls_found: [count]
  maturity_deltas:
    - finding_id: "ADR-[N]"
      finding_type: ONE-OFF | RECURRING | MISSING_CONTROL | PROCESS_FAILURE
      samm_function: GOVERNANCE | DESIGN | IMPLEMENTATION | VERIFICATION | OPERATIONS
      samm_practice: "[specific practice]"
      ssdf_mapping: "[PO/PS/PW/RV]-[N]"
      bsimm_baseline: true | false
      improvement_recommendation: "[specific, actionable recommendation]"
      priority: IMMEDIATE | SHORT_TERM | LONG_TERM
  org_improvement_recommendations:
    - "[Top 3 systemic improvements based on finding patterns]"
  next_maturity_review_trigger: "[condition — e.g., same pattern 3rd cycle, pre-SOC2 audit]"
```

---

## Output Sample

```
[M.AM] Assurance Maturity Delta

ADR-3 (Input validation bypass, RECURRING — found in cycles 8, 9, 10):
  Type: RECURRING PATTERN
  SAMM: Implementation → Defect Management (tracking recurring bugs with SLA)
  SSDF: RV.3 (Analyze root causes) + PW.5 (Secure coding practices)
  BSIMM: Bug bar / severity classification is a BSIMM baseline activity.
         This recurring pattern suggests classification and tracking process is absent.
  Recommendation: Establish input validation as a required security control
                  in code review checklist. Add SAST rule for this pattern.
                  Timeline: IMMEDIATE (3rd recurrence = systemic)

ADR-7 (No incident response procedure for auth failure, MISSING CONTROL):
  Type: MISSING CONTROL
  SAMM: Operations → Incident Management (L0: no documented process)
  SSDF: RV.2 (Remediation process missing)
  BSIMM: Incident response plan is a BSIMM baseline activity. Absence = below baseline.
  Recommendation: Define auth failure incident playbook with:
    (1) detection criteria, (2) escalation path, (3) communication template,
    (4) remediation steps.
    Timeline: SHORT_TERM (pre-launch requirement)
```

---

## Anti-Patterns

🚫 **"We'll produce a maturity score"** — Maturity scores require validated baseline data.
A single-cycle [M.AM] scan is a delta assessment, not a maturity measurement.
Don't claim "Maturity Level 2" without multi-cycle calibration.

🚫 **"All findings get maturity analysis"** — [M.AM] runs on MANDATORY and REQUIRED only.
RECOMMENDED and OPTIONAL findings don't trigger maturity analysis.

🚫 **"BSIMM says X% of companies do this, so it's optional"** — BSIMM is descriptive,
not prescriptive. Low BSIMM adoption means many companies are below baseline,
not that below-baseline is acceptable.

---

*Reference 25 — VHEATM 12.0 | Assurance Maturity Overlay
Research: Brasoveanu 2022 (12 cit.); Fucci 2024 SAMM; Weir 2021 (21 cit.) BSIMM; NIST SSDF*
