<!-- VHEATM 10.0: unchanged from 9.0 -->
# Bug Class Catalog — [KB] Persistent Structure

> **Tikai Evidence**: The same Pattern Globalization gap was missed twice within the same audit chain (Round 1 missed actions.py for ARQ pool; Round 3 missed 7 endpoints for rate limit). Without a pattern memory, the same blind spot repeats indefinitely.

A persistent log of bug-class patterns identified across cycles. Replayed at every new cycle's [G.H].

---

## Why Patterns, Not Just Hypotheses?

VHEATM 8.0 had `Hypothesis Lifecycle Archive` — but archives are per-instance, not per-class.

If `H-007: imports.py missing rate limit` is archived as CLOSED, a future cycle won't think to grep for similar patterns. The archive doesn't generalize.

A Bug Class Catalog stores the SHAPE of the bug, not the instance. Next cycle can replay the search.

---

## Catalog Structure

```yaml
bug_class_catalog:
  - class_id: BC-001
    name: "mutation-endpoint-without-rate-limit"
    first_discovered_cycle: 3
    severity_typical: HIGH    # QBR usually in 9-16 range
    pattern_signature:
      trigger: "HTTP mutation endpoint (POST/PATCH/PUT/DELETE)"
      anti_pattern_code: |
        @router.post(...)
        async def handler(...):
            ...
      correct_pattern_code: |
        @router.post(...)
        @limiter.limit("N/period")
        async def handler(request: Request, ...):
            ...
    search_method:
      type: AST_TRAVERSAL
      command: |
        # Walk all router files, find AsyncFunctionDef with @router.post|patch|put|delete
        # decorators, assert @limiter.limit also present
      grep_fallback: |
        grep -B 2 -A 5 "^@router\.(post|patch|put|delete)" path/to/routers/
    test_to_close: tests/test_rate_limit_coverage.py
    instances:
      - cycle: 3
        location: "app/api/v1/insights.py:recompute_insight"
        closed: true
        fix_anchor: "app/api/v1/insights.py:157"
        production_validated: true         # 🆕 v13.1: was this confirmed real in prod?
        validation_source: "incident-INS-042"  # link to incident/monitoring evidence
        false_positive_retroactive: false  # set true if prod showed bug never manifested
      - cycle: 4
        location: "app/api/v1/cogs.py:upsert_cogs"
        closed: true
        fix_anchor: "app/api/v1/cogs.py:72"
        production_validated: unknown      # 🆕 v13.1: not yet confirmed in prod
        validation_source: null
        false_positive_retroactive: null   # pending
      # ... etc
    open_instances: 0
    last_replay_cycle: 4
    replay_recommended_next: true  # always replay until class is conclusively closed
```

---

## Anatomy of a Catalog Entry

### `class_id`
Stable identifier `BC-NNN` for cross-cycle reference.

### `name`
Descriptive slug. Should be specific enough that the bug class is clear from the name alone.

✅ Good: `mutation-endpoint-without-rate-limit`, `inline-redis-connection-no-singleton`, `first-match-by-rule-id-for-entity-lookup`

❌ Bad: `bug-class-1`, `security-issue`, `code-quality`

### `pattern_signature`
The shape of the bug. Must include:
- `trigger`: when does this bug class manifest?
- `anti_pattern_code`: small code snippet showing the bug shape
- `correct_pattern_code`: small code snippet showing the fix shape

### `search_method`
HOW to find instances. Options:
- `AST_TRAVERSAL`: Python AST or similar — best for complex patterns
- `REGEX_GREP`: simple text search — works for many patterns
- `STATIC_RULE`: dedicated linter rule (e.g., ruff, custom checker)
- `MANUAL_REVIEW`: pattern requires human judgment (declare this explicitly)

Always include the command/code so a future cycle can re-run it.

### `test_to_close`
Path to an automated test that PROVES the class is closed across the codebase. This is the strongest closure signal — re-running the test verifies no new instances appeared.

### `instances`
Historical record of each instance found and its fix. Closed instances stay in the log (don't delete).

### `last_replay_cycle` + `replay_recommended_next`
When was the search last run? Should it run again? Catalog entries are not "closed forever" — they should be replayed periodically.

---

## Replay Protocol

At every new cycle's [G.H], add this step:

```
[G.H.Replay] Bug Class Catalog Replay

For each entry in catalog where replay_recommended_next == true:
  □ Re-run the search_method
  □ Compare instances found vs catalog's known instances
  □ Any NEW instance → new hypothesis in [G.H]
  □ Any STILL-OPEN instance from past cycle → escalate to MANDATORY
  □ If automated test exists: run it → green = no new instances
```

**Replay frequency** by class age (v13 adds PERMANENT tier):

| Age | Tier | Replay frequency |
|----|------|------------------|
| 1-2 cycles | HOT | Every cycle |
| 3-5 cycles | WARM | Every other cycle |
| 5+ cycles, 0 new instances last 3 | COLD | Every 3rd cycle |
| 10+ cycles, 0 new instances last 5 | ARCHIVE | Only on demand |
| Any age — if MANDATORY+security | PERMANENT | Every cycle, NEVER archived |

**PERMANENT tier criteria (v13):**
A class becomes PERMANENT when ANY of:
  □ Pattern was ever confirmed at QBR ≥ 17 (MANDATORY) AND involves security or data loss
  □ Pattern is exploitable by an external attacker (L6 layer)
  □ Pattern has audit-resistant nature (hard to detect via static analysis alone)
  □ Explicitly elevated by human reviewer

Rationale: a quiet MANDATORY security pattern (0 new instances for 10 cycles) may
resurface with any new external dependency. Silence ≠ closed. PERMANENT classes
require automated test in CI to justify COLD or ARCHIVE — not just absence of instances.

---

## When to Add a New Class

Add to catalog when:

```
□ Bug found via [G.PG] sibling search across files
□ Same pattern identified in 2+ files within one cycle
□ Pattern is generalizable (not unique to one specific function)
□ Pattern is testable (you can describe how to find it programmatically)
```

Don't add to catalog when:

```
□ Single-file logic bug with no replicable pattern
□ Pattern is too narrow (only matches one function)
□ Pattern is environmental (e.g., specific to one cloud provider version)
```

---

## Example: Initial Bug Class Catalog seeded from Tikai

```yaml
bug_class_catalog:
  - class_id: BC-001
    name: "mutation-endpoint-without-rate-limit"
    trigger: "HTTP mutation endpoint without @limiter.limit"
    search_method: AST_TRAVERSAL
    test_to_close: "tests/test_rate_limit_coverage.py"

  - class_id: BC-002
    name: "inline-redis-connection-no-singleton"
    trigger: "Router-level ArqRedis.from_url() instead of shared pool"
    search_method: REGEX_GREP
    command: 'grep -rn "ArqRedis.from_url" app/api/'

  - class_id: BC-003
    name: "first-match-by-rule-id"
    trigger: "Loop over triggers, return first match where rule_id matches"
    search_method: MANUAL_REVIEW
    notes: "Pattern too contextual for grep; flag in code review"

  - class_id: BC-004
    name: "hardcoded-zero-near-calc-field"
    trigger: "Decimal('0') as literal value passed to a field that should be calculated"
    search_method: REGEX_GREP
    command: 'grep -rn "Decimal(\"0\")" app/ | grep -v default'
    notes: "High false-positive rate — manual review of each match required"

  - class_id: BC-005
    name: "compound-feature-incomplete"
    trigger: "Feature declared 'done' but missing components per catalog template"
    search_method: MANUAL_REVIEW
    related_reference: "references/12-compound-feature-decomp.md"

  - class_id: BC-006
    name: "documented-fix-without-code-change"
    trigger: "Comment says 'FIX X' but surrounding code unchanged"
    search_method: MANUAL_REVIEW
    notes: "Tikai BUG-H4. Mitigated by [T.FV] Fix Verification."

  - class_id: BC-007
    name: "external-api-call-without-timeout"
    trigger: "httpx/Anthropic/OpenAI client without explicit Timeout()"
    search_method: REGEX_GREP
    command: 'grep -rn "AsyncClient\|AsyncAnthropic" app/ | grep -v timeout'
```

This catalog is SEEDED, not exhaustive. Each new cycle should add new classes.

---

## Catalog Maintenance

End of every cycle [M]:

```
□ New classes added this cycle? List them with class_id
□ Existing classes — any new instances found? Update instance list
□ Any class fully closed (0 instances, test in place)? Mark COLD
□ Persist catalog to durable storage
```

---

## Anti-Patterns

🚫 **Catalog becomes a graveyard**: If every entry is COLD, you're not finding new patterns. Either you're truly clean, or your replay protocol is broken.

🚫 **Catalog becomes a wall of text**: If 200 entries, no one will replay them all. Cap at ~30 HOT entries; archive the rest.

🚫 **Catalog grows but tests don't**: For each class, an automated test should be the eventual destination. Manual replay is interim.

🚫 **Catalog used as a checklist instead of a generator**: The catalog tells you what TO LOOK FOR, but [G.H] must still produce hypotheses that the catalog doesn't predict.

---

## Integration with Hypothesis Lifecycle Archive

Catalog (class) and Archive (instance) are complementary:

```
Bug Class Catalog: BC-001 mutation-endpoint-without-rate-limit
  ├── Instance: H-027 (cycle 3, insights.py recompute) → CLOSED
  ├── Instance: H-041 (cycle 4, cogs.py upsert) → CLOSED
  ├── Instance: H-042 (cycle 4, livestream.py create) → CLOSED
  └── ... etc

Hypothesis Lifecycle Archive: H-027
  status: CLOSED
  related_bug_class: BC-001
  closed_in_cycle: 3
  fix_anchor: insights.py:157
```

---

*Reference 13 — VHEATM 9.0 | Direct response to Tikai recurring-pattern-miss failure*

---

## 🆕 Enterprise Bug Class Seeds (v10.0)

Added in v10.0 to cover organizational-layer failures:

```yaml
  - class_id: BC-008
    name: "compliance-gap-no-audit-trail"
    trigger: "Data mutation or access with no structured log entry"
    severity_typical: HIGH
    search_method: MANUAL_REVIEW
    related_reference: "references/11-cross-cutting-layer.md → L7.11"
    replay_recommended_next: true

  - class_id: BC-009
    name: "ownership-boundary-crossing-without-handoff"
    trigger: "ADR requires Team A to fix + Team B to integrate, handoff plan absent"
    severity_typical: HIGH
    search_method: MANUAL_REVIEW
    notes: "Detected by [G.INC] probe. Manifests as fixes that 'land' in code but never deploy"
    related_reference: "references/14-incentive-misalignment.md"
    replay_recommended_next: true

  - class_id: BC-010
    name: "undocumented-tribal-knowledge-system"
    trigger: "Legacy system with no named owner and no one who can describe its full behavior"
    severity_typical: CRITICAL
    search_method: MANUAL_REVIEW
    notes: "Detected by LCC Tribal Knowledge Probe at [P]. Risk: any change to this system
            has CLI × 3.0 + unknown failure modes"
    related_reference: "references/00-context-modes.md → LCC Level C"
    replay_recommended_next: true

  - class_id: BC-011
    name: "incentive-inversion-quality-vs-velocity"
    trigger: "Team measured on feature velocity; quality bugs in their code cost another team"
    severity_typical: REQUIRED
    search_method: MANUAL_REVIEW
    notes: "Organizational bug class — not fixable in code. Requires org-level ADR (incentive
            alignment recommendation). Detected by [G.INC] INC-2 probe"
    related_reference: "references/14-incentive-misalignment.md → EP-04"
    replay_recommended_next: true
```

---

## 🆕 v11.0 Technical Bug Class Seeds

```yaml
  - class_id: BC-012
    name: "temporal-accumulation-no-drain"
    trigger: "State grows without enforced drain/eviction/rotation"
    layer: "L2 / L7.5"
    temporal_subclass: "accumulation"
    severity_typical: REQUIRED
    search_method: "grep for write paths; check for corresponding delete/evict"
    related_reference: "references/16-temporal-scan-mode.md → L-T.1"
    replay_recommended_next: true

  - class_id: BC-013
    name: "date-boundary-reset-not-guaranteed"
    trigger: "Counter or limit resets at time boundary via event-triggered path"
    layer: "L2"
    temporal_subclass: "date-boundary"
    severity_typical: HIGH
    search_method: "grep for reset logic; check if reset fires on boundary OR only on user action"
    notes: "Canonical instance: Aletheia BUG-01 — daily limit persisted past midnight"
    related_reference: "references/16-temporal-scan-mode.md → L-T.2"
    replay_recommended_next: true

  - class_id: BC-014
    name: "ai-output-unvalidated-to-storage"
    trigger: "AI response written to DB/file/state without schema validation"
    layer: "L6.8"
    severity_typical: HIGH
    search_method: "trace AI response → storage write path; check for validation layer"
    related_reference: "references/11-cross-cutting-layer.md → L6.8"
    replay_recommended_next: true

  - class_id: BC-015
    name: "cyclic-dependency-class-level"
    trigger: "Module A depends on B which depends (transitively) on A"
    layer: "L3"
    smell_type: "AS-01"
    severity_typical: REQUIRED
    search_method: "dependency-cruiser / cargo-deps / manual import chain trace"
    related_reference: "references/20-architecture-smells.md → AS-01"
    replay_recommended_next: true

  - class_id: BC-016
    name: "l4-unclassified-concurrency"
    trigger: "Concurrency bug found but sub-type not yet determined"
    layer: "L4"
    severity_typical: HIGH
    search_method: "Apply L4.1-L4.6 sub-scan to classify"
    notes: "BC-016 is a temporary classification — always resolve to L4.1-L4.6 before ADR"
    related_reference: "references/01-phase-guide.md → L4.1-L4.6"
    replay_recommended_next: false
```

---

## 🆕 v12.0 Bug Class Seeds

```yaml
  - class_id: BC-017
    name: "fmea-high-rpn-undetected-failure"
    trigger: "Component failure mode with RPN ≥ 125 and Detectability ≥ 8"
    layer: "L2 / L5"
    fmea_source: true
    severity_typical: MANDATORY
    search_method: "FMEA-lite scan on critical path components"
    notes: "High severity + low detectability = silent system-level failure.
            Common in payment processing, state transitions, external dependencies."
    related_reference: "references/24-fmea-lite.md"
    replay_recommended_next: true

  - class_id: BC-018
    name: "ai-prompt-injection-unsanitized-user-input"
    trigger: "User input flows into AI prompt without sanitization boundary"
    layer: "AI-S1 (LLM01)"
    severity_typical: HIGH
    search_method: "Trace data flow: user input → AI request body"
    notes: "Attack success rates > 84% even with defenses (ASB Zhang 2024).
            Cannot be treated as theoretical. Architecture-level mitigations required."
    related_reference: "references/11-cross-cutting-layer.md → AI-S1"
    replay_recommended_next: true

  - class_id: BC-019
    name: "samm-missing-defect-management"
    trigger: "Same bug class found in 2+ audit cycles without process-level remediation"
    layer: "process"
    severity_typical: REQUIRED
    search_method: "[KB] Bug Class Catalog recurrence count"
    notes: "SAMM: Implementation → Defect Management gap. BSIMM baseline activity.
            Absence of recurring-bug tracking = below industry baseline."
    related_reference: "references/25-assurance-maturity-overlay.md"
    replay_recommended_next: true

  - class_id: BC-020
    name: "atam-undocumented-tradeoff-decision"
    trigger: "Architecture has implicit QA tradeoff with no documented decision rationale"
    layer: "L3"
    severity_typical: RECOMMENDED
    search_method: "[V.UT] sensitivity/tradeoff point analysis"
    notes: "Undocumented tradeoffs become debt: next engineer re-opens the decision
            without knowing why it was made. ATAM utility tree surfaces these."
    related_reference: "references/23-atam-utility-tree.md"
    replay_recommended_next: false
```

---

## 🆕 [M.AT] MANDATORY Accuracy Tracker (v13.1)

> **Gap closed**: Bug Class Catalog previously tracked whether fixes were applied
> (`closed: true`) but never tracked whether findings were confirmed real in production.
> A framework that produces 40% false positives at MANDATORY level is optimizing for
> process rigor while misleading stakeholders about actual risk. [M.AT] adds the
> feedback loop between audit output and production reality.

### Instance Fields (v13.1 additions)

Every instance entry in the catalog now requires three new fields:

```yaml
production_validated: true | false | unknown
  # true    = confirmed real bug via prod incident, monitoring alert, or post-deploy test
  # false   = bug never manifested in prod — retroactively a false positive
  # unknown = not yet enough production time to determine (default for recent cycles)

validation_source: "incident-ID | monitoring-alert-ID | post-deploy-test | null"
  # concrete evidence anchor for production_validated = true/false

false_positive_retroactive: true | false | null
  # Set true if production showed the MANDATORY designation was incorrect.
  # This field feeds the aggregate accuracy tracker below.
```

### KB-Level Accuracy Tracker

Add to [KB] persistent handoff YAML:

```yaml
mandatory_accuracy_tracker:
  cycles_measured: N                  # number of cycles with ≥1 MANDATORY finding
  mandatory_designated_total: N       # all MANDATORY findings across measured cycles
  production_validated_true: N        # confirmed real in prod
  production_validated_false: N       # confirmed false positive in prod
  production_validated_unknown: N     # not yet determined
  measured_accuracy_rate: "X%"        # validated_true / (validated_true + validated_false)
  # Note: exclude "unknown" from rate calculation — they haven't resolved yet

  revision_trigger:
    calibration_phase: "PRE-CALIBRATION | EMPIRICAL | RECALIBRATED"  # 🆕 v14
    threshold: null                   # 🆕 v14: NOT a fixed value — derived from calibration
    threshold_evidence_tier: "T3 | T2"  # T3 = industry benchmark; T2 = empirical from own data
    window: 3                         # consecutive cycles below threshold → trigger
    current_status: "NOMINAL | WATCH | TRIGGER"
    cycles_below_threshold: 0         # resets when a cycle returns above threshold

    # 🆕 v14 Threshold Calibration Protocol (Track N):
    #
    # PRE-CALIBRATION (cycles 1-5, no empirical data yet):
    #   threshold = 60% (industry benchmark floor)
    #   Evidence: SOC analyst 53% FP avg (Devo 2024), Prophet Security <25% = "critical tier"
    #   VHEATM with human-in-loop should outperform raw SOC alerts significantly
    #   threshold_evidence_tier: T3
    #
    # EMPIRICAL (cycle 6+, after 5 cycles of M.AT data):
    #   threshold = (empirical accuracy rate from cycles 1-5) × 0.85
    #   = trigger when accuracy drops 15% below YOUR observed baseline
    #   If baseline already < 50%: threshold = 50% (absolute floor — random chance)
    #   threshold_evidence_tier: T2
    #
    # RECALIBRATED (annually):
    #   threshold = recalculated from last 12 months of M.AT data × 0.85
    #   threshold_evidence_tier: T2
    #   Log calibration drift in [KB.AD]
    #
    # Evidence anchors:
    #   Du et al. 2025 (arxiv 2601.18844): SAST FP rates >95% on large codebases
    #   Devo SOC Performance Report 2024: 53% FP rate industry average
    #   Prophet Security 2026: <25% FP = "critical" (well above average)
    #   Massacci et al. 2020: CVSS inter-rater reliability (Empirical SE, Springer)
```

### Accuracy Tracker Protocol at [M]

Run at [M] alongside [M.EF], after closing all ADRs:

```
[M.AT] Mandatory Accuracy Update:
  □ For each MANDATORY finding closed in prior cycles:
    □ Is production_validated still "unknown"?
    □ If ≥ 2 weeks post-deploy and no incident observed → set to "unknown-aging"
    □ If incident/alert linked → set to "true" + populate validation_source
    □ If human reviewer confirms false positive → set to "false" + note reason

  □ Recalculate measured_accuracy_rate

  □ 🆕 v14 Threshold evaluation (replaces fixed 65%):
    □ Determine current calibration_phase:
      If cycles_measured < 5 → PRE-CALIBRATION: use threshold = 60%
      If cycles_measured ≥ 5 AND no empirical calibration done → run calibration:
        → empirical_baseline = average measured_accuracy_rate across first 5 cycles
        → threshold = empirical_baseline × 0.85
        → If empirical_baseline < 50%: threshold = 50% (absolute floor)
        → Set calibration_phase = EMPIRICAL; log derivation in [KB.AD]
      If last calibration > 12 months → RECALIBRATE from last 12 months data

    □ If accuracy_rate < threshold in current cycle:
      □ Increment cycles_below_threshold
      □ If cycles_below_threshold ≥ 3 → set current_status: TRIGGER
        → Debt-AT-[N]: "MANDATORY accuracy trigger — QBR threshold review required"
        → Flag for human reviewer sign-off before next v+1 discussion

  □ Log update in [KB] YAML handoff + [KB.AD] accuracy dashboard
```

### Anti-Patterns

🚫 **"closed: true means the bug was real"** — `closed` means the fix was applied.
It says nothing about whether the bug would have manifested in production.

🚫 **"we can't track production outcomes from an audit tool"** — Production
validation doesn't require integration with monitoring systems. A human reviewer
noting "this incident matches ADR-N" or "6 weeks post-deploy, no incident = unknown"
is sufficient for accuracy tracking. Low fidelity beats no fidelity.

🚫 **"accuracy tracking will discourage auditors from flagging MANDATORY"** — The
tracker measures the framework's calibration, not individual auditor performance.
If MANDATORY accuracy is high, the framework is well-calibrated. If low, the QBR
thresholds or severity heuristics need adjustment — not the auditors.

---

*Bug Class Catalog — v14.0 additions: evidence-anchored threshold calibration (Track N),
[KB.AD] accuracy dashboard integration (Track R).
v13.1 additions: production_validated fields + [M.AT] accuracy tracker.*
