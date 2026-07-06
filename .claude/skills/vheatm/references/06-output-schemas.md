# Output Schemas — FAST / Standard / Full (v10.0)

Changes from 9.0:
- All schemas: Owner + Boundary fields in ADR section
- All schemas: L7.11 compliance flag in cross-cutting layer
- Standard + Full: INC probe output section
- Standard + Full: ORG blast radius section for MANDATORY findings
- Full: Multi-perspective adversarial pass output
- New context mode: ENTERPRISE in context declaration

---

## FAST Schema (v10.0)

```yaml
mode: FAST
cycle: [N]
context:
  mode: DESIGN | CODE | LIVE | LEGACY | ENTERPRISE   # 10.0: ENTERPRISE added
  stakeholder: [who]
  goal: [decision being informed]
  self_audit: YES | NO
  🆕 org_context: "[own-team | external-team | N/A]"

summary:
  verdict: "[One clear sentence]"
  confidence: low | medium | high

top_findings:           # exactly 3, ordered by QBR
  - id: F1
    conclusion: "[Finding]"
    severity: critical | high | medium | low
    layer: L1 | L2 | L3 | L4 | L5 | L6 | L7 | L7.11   # 10.0: L7.11 added
    bug_anchor: "[file:line or Evidence Tier + source]"
    fix_anchor: "[file:line if applied, else PENDING]"
    pattern_globalization: "[siblings searched? count? bug class id?]"
    🆕 owner: "[team or 'unclear']"
    🆕 boundary: "YES/[type] or NO"
  - id: F2
    ...
  - id: F3
    ...

bias_probe:
  anchoring_check: "[adjustment if any]"
  self_audit_check: "[if triggered]"
  🆕 org_capture_check: "[if triggered: risk level + skepticism applied]"

# INC-FAST: ownership check only
🆕 incentive_quick_check:
  boundary_flags_found: [count]
  boundary_items: ["[H-ID]: [crossing type]", ...]

automation_bias_guard:
  blind_spots: "[declared]"

signal_noise_filter:
  signal: ["[real risk]"]
  noise: ["[deprioritized + why]"]

adversarial_pass:
  lens_used: "[lens name] — [perspective in Full mode]"
  l7_11_flag: "[any regulated data gaps? YES/NO]"
  candidate_findings: [list]
  routed_to: [NEW_HYPOTHESIS | DEFER_DEBT | DISMISS]

recommendation:
  decision: "[Approve / Reject / Revise / Delay]"
  next_step: "[specific concrete action]"

next_cycle_trigger: "[specific condition]"
```

---

## Standard Schema (v10.0)

```yaml
mode: Standard
cycle: [N]
context:
  mode: DESIGN | CODE | LIVE | LEGACY | ENTERPRISE
  stakeholder: [who]
  goal: [decision]
  audit_scope_intent: BUG_HUNT | GAP_MAP | PRE_LAUNCH | POST_INCIDENT | FULL_SPECTRUM
  self_audit: YES | NO
  🆕 org_context: "[team that owns this code]"
  🆕 enterprise_activations:
    g_inc: true | false
    g_org: true | false
    l7_11: true | false
    lcc: true | false

pre_mortem:
  temperature: COLD | WARM
  top_failure_modes:
    - "[Failure mode 1]"
    - "[Failure mode 2]"
    - "[Failure mode 3]"
  🆕 enterprise_failure_mode: "[if ENTERPRISE: org-level failure mode declared]"

catalog_replay:
  classes_replayed: ["BC-001", ...]
  new_instances_found: [count]
  hypotheses_added: ["H-ID", ...]

compound_features:
  - name: "[Feature]"
    components_expected: [N]
    components_present: [N]
    missing: ["component-name", ...]

findings:
  - id: F1
    conclusion: "[Finding]"
    severity: critical | high | medium | low
    priority: MANDATORY | REQUIRED | RECOMMENDED | OPTIONAL
    layer: L1 | L2 | L3 | L4 | L5 | L6 | L7 | L7.11
    qbr_score: [N]
    bug_anchor: "[file:line or Tier + source]"
    fix_anchor: "[file:line if applied, else PENDING]"
    verification_anchor: "[file:line, else PENDING]"
    pattern_globalization:
      pattern_signature: "[anti-pattern shape]"
      search_command: "[grep/AST command]"
      siblings_found: [count]
      fix_cascade: ["file:line", ...]
      bug_class_id: BC-[NNN]
    🆕 incentive_probe:   # if Standard+Full mode
      owner: "[team]"
      boundary_flag: true | false
      boundary_type: TECHNICAL | HANDOFF | APPROVAL | DISPUTED | N/A
      incentive_risk: true | false
      org_friction_multiplier: [1.0 | 1.5 | 2.0 | 3.0 | 4.0 | 5.0]
      realistic_resolution_time: "[estimate]"
    🆕 org_blast_radius:   # if MANDATORY finding
      brs: [N]
      brs_tier: CONTAINED | ELEVATED | HIGH | CRITICAL
      layer_1_owner: "[team]"
      layer_2_teams_affected: [count]
      layer_3_sla_chains_at_risk: [count]
      layer_4_regulatory_exposure: true | false
      escalation_required: true | false

bias_probes:
  anchoring: "[result + adjustment]"
  confirmation: "[counter-evidence result]"
  availability: "[incident + discount]"
  self_audit: "[SCR output if triggered]"
  🆕 org_capture: "[if triggered: risk=HIGH/MED/LOW, skepticism=+15/20%, scope=[...]]]"

automation_bias_guard:
  self_challenge: "[top finding challenged + verdict]"
  confidence_calibration: "[evidence vs inferred]"
  blind_spots: "[declared]"

signal_noise_filter:
  applied_to: [N] hypotheses
  maintained: [N]
  downgraded: [N]
  removed: [N]

adrs:
  - id: ADR-1
    title: "[Title]"
    priority: MANDATORY
    bug_anchor: "[anchor]"
    fix_anchor: "[anchor or PENDING]"
    verification_anchor: "[anchor or PENDING]"
    decision: "[what]"
    rationale: "[why]"
    consequences: "[what changes]"
    revert_blast_radius: TRIVIAL | HARD | IMPOSSIBLE
    observable_success_criteria: "[measurable condition]"
    pattern_globalization: "[summary or N/A]"
    🆕 owner: "[team/role]"
    🆕 boundary: "YES/[type] or NO"
    🆕 handoff_plan: "[if BOUNDARY=YES: explicit sequence]"
    🆕 stakeholder_signoff_required: true | false
    🆕 brs: [N]
    🆕 escalation: "[if BRS ≥ 8: leadership to notify]"

adversarial_pass:
  assumed_remaining_bugs_N: [N]
  lenses_run: [pattern, self, cross_cutting, compound]
  🆕 l7_11_compliance_checked: true | false
  routing:
    new_hypotheses: [count]
    deferred_debt: [count]
    dismissed: [count]
  closure_verdict: CLEAN | DEFERRED_DEBT_LOGGED | REOPENED_CYCLE

bug_class_catalog_update:
  new_classes_added: ["BC-NNN: name", ...]
  existing_classes_updated: ["BC-NNN: +X instances", ...]

recommendation:
  status: "Approve | Reject | Revise | Delay"
  required_revisions: ["[specific revision]"]
  confidence:
    level: low | medium | medium-high | high
    basis: "[grounding]"
    caveat: "[what would change it]"

next_cycle:
  trigger: "[specific condition]"
  focus: "[specific questions]"
  debt_carried: ["Debt-[ID]: [description], age: [N] cycles"]
  🆕 org_friction_blocked_items: ["ADR-[N]: org multiplier × [N], expected resolution [date]"]
  bug_classes_to_replay: ["BC-NNN", ...]
```

---

## Full Schema (v10.0)

```yaml
mode: Full
cycle: [N]
context:
  mode: DESIGN | CODE | LIVE | LEGACY | ENTERPRISE
  stakeholders:
    - [stakeholder 1]
    - [stakeholder 2]
  goal: [decision]
  audit_scope_intent: [intent]
  risk_class: low | medium | high | critical
  🆕 org_ownership_map: "[reference to [V] Org Ownership Map]"
  🆕 enterprise_activations:
    g_inc: true | false
    g_org: true | false
    l7_11: true | false
    lcc_level: A | B | C | N/A

scope_statement:
  included: ["[topic]"]
  excluded: ["[topic — with reason]"]

executive_judgment:
  outcome: "[verdict]"
  rationale: "[grounded in findings]"

pre_mortem:
  temperature: COLD | WARM
  pm_accuracy_last_cycle: "[N/M predictions materialized]"
  failure_modes:
    - id: PM-1
      scenario: "[failure]"
      probability: low | medium | high
      lead_indicator: "[early warning]"
  🆕 enterprise_pre_mortem:
    org_failure_scenario: "[organizational failure that causes this system to fail]"
    incentive_failure_scenario: "[incentive misalignment that prevents fix]"

hypotheses_summary:
  total_generated: [N]
  after_snf: [M]
  mandatory: [N]
  required: [N]
  recommended: [N]
  🆕 l7_11_compliance_gaps: [N]
  🆕 boundary_crossings: [N]
  🆕 incentive_risk_flags: [N]

evidence_anchored_findings:
  - id: R1
    severity: critical | high | medium | low
    conclusion: "[Finding]"
    anchor: "[file:line or Evidence Tier]"
    why_it_matters: "[impact on stakeholders]"
    snf_verdict: MAINTAINED | DOWNGRADED
    🆕 owner: "[team]"
    🆕 boundary: "YES/[type] or NO"
    🆕 brs_tier: CONTAINED | ELEVATED | HIGH | CRITICAL

🆕 enterprise_blast_radius_summary:   # ENTERPRISE mode
  highest_brs_finding: [ADR-N]
  highest_brs_score: [N]
  total_teams_in_blast_zone: [N]
  sla_chains_at_risk: [N]
  regulatory_exposures: ["GDPR Art.33", "PCI-DSS Req.10", ...]
  escalation_required: true | false
  leadership_to_notify: ["[person/team]", ...]

stakeholder_view:
  [stakeholder_1]:
    upside: "[gains]"
    risk: "[exposure]"
    recommendation: "[specific]"

bias_probes:
  - type: "[bias type]"
    finding_affected: "[H-ID]"
    question_asked: "[counter-question]"
    result: "[finding]"
    adjustment: "[if any]"
  🆕 org_capture:
    triggered: true | false
    risk_level: HIGH | MEDIUM | LOW
    skepticism_applied: "+15% | +20%"
    scope: "[components reviewed]"

adrs:
  - id: ADR-1
    title: "[Title]"
    priority: MANDATORY | REQUIRED | RECOMMENDED
    qbr_score: [N]
    evidence_anchor: "[anchor]"
    context: "[situation]"
    decision: "[what]"
    rationale: "[why over alternatives]"
    consequences: "[what changes]"
    revert_blast_radius: TRIVIAL | HARD | IMPOSSIBLE
    observable_success_criteria: "[measurable condition]"
    opposing_argument: "[strongest counter]"
    rebuttal: "[why maintained]"
    pattern_globalization: "[summary]"
    🆕 owner: "[team]"
    🆕 boundary: "YES/[type] or NO"
    🆕 handoff_plan: "[if YES: explicit sequence]"
    🆕 stakeholder_signoff_required: true | false
    🆕 brs: [N]
    🆕 org_friction_multiplier: [N]
    🆕 realistic_resolution_time: "[estimate]"

adversarial_pass:
  mode: Full
  multi_perspective: true
  assumed_remaining_bugs_N: [N]
  lenses:
    pattern:
      perspective: "SRE"
      candidates_found: [count]
      findings: [...]
    self:
      perspective: "Security"
      candidates_found: [count]
      findings: [...]
    cross_cutting:
      perspective: "Compliance"
      l7_11_compliance_checked: true
      candidates_found: [count]
      findings: [...]
    compound:
      perspective: "Product"
      candidates_found: [count]
      findings: [...]
    org:  # ENTERPRISE only
      perspective: "Team B / cross-team"
      candidates_found: [count]
      findings: [...]
  closure_verdict: CLEAN | DEFERRED_DEBT_LOGGED | REOPENED_CYCLE

decision:
  recommendation: "[Approve / Reject / Revise / Delay / Pilot]"
  conditions: ["[condition]"]

confidence:
  level: low | medium | medium-high | high
  basis: "[grounding]"
  open_questions: ["[question]"]

next_cycle:
  triggers:
    automatic: ["[condition]"]
    data_driven: ["[metric threshold]"]
    calendar: "[fallback date]"
  focus: "[questions]"
  debt_register: ["Debt-[ID]: [description], age: [N] cycles, priority: HIGH|MEDIUM|LOW"]
  🆕 org_friction_register: ["ADR-[N]: blocked by [org constraint], owner: [team], unblock: [condition]"]
  cross_cycle_synthesis: "[if N≥3: pattern analysis]"
```

