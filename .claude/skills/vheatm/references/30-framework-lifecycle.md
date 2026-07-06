# Framework Lifecycle Intelligence — [KB.EC], [P.FD], [M.AM] (v13.0)

> **Gap closed**: VHEATM v12.0 has excellent mechanisms for auditing CODE artifacts
> but no mechanisms for the framework's own lifecycle health:
> (1) Research citations can become stale, retracted, or superseded — no refresh protocol.
> (2) When framework version changes (v12→v13), new gates/phases may be unknown to
>     executor using prior cycle's training — no framework diff check.
> (3) SAMM/BSIMM maturity expectations are calibrated for medium-large organizations
>     but applied without size context — incorrect baselines for startups or enterprises.
>
> These three gaps are related: the framework is its own system that needs maintenance.

---

## Part 1 — Evidence Currency Protocol ([KB.EC])

### Why Evidence Ages

VHEATM references academic papers by citation count and year. But:
- Papers can be retracted (rare but happens)
- Findings can be superseded by larger studies
- Technology landscape changes (2018 benchmark may not apply to 2025 LLMs)
- Evidence Tiers depend on freshness: T2 (industry case study) from 2015 has lower
  authority than T2 from 2024 for a rapidly-evolving domain

### Evidence Age Classification

```
FRESH    (≤2 years from current audit date): full weight — T-tier stands
RECENT   (3-4 years): apply 10% confidence discount — document in finding
AGING    (5-7 years): apply 25% confidence discount — flag for verification
STALE    (8+ years): apply 50% confidence discount — recommend finding replacement evidence
CLASSIC  (10+ years, high citations ≥200): weight stays — foundational research
         🆕 EXCEPTION: CLASSIC weight does NOT apply if:
           (a) Paper has been directly retracted or issued an erratum affecting core claims, OR
           (b) A superseding meta-analysis (≥100 citations, published after original)
               demonstrates original claim was confounded, underpowered, or domain-limited.
         Annual Evidence Refresh must explicitly check CLASSIC papers for retraction/superseding.
         Verification format: "CLASSIC status confirmed [year]: no retraction, no superseding
           meta-analysis found via [Consensus / Google Scholar / Retraction Watch search]."
```

**CLASSIC exception**: Papers with ≥200 citations that establish foundational principles
(Just et al. 2014 mutation testing, Mohanani et al. 2017 cognitive biases) remain at
full weight regardless of age. Their principles are structurally sound.

**Domain-adjusted aging for AI/ML research**:
AI field moves rapidly. Apply 1.5× aging rate:
- FRESH: ≤1 year (instead of 2)
- AGING: 3-5 years (instead of 5-7)
- STALE: 6+ years (instead of 8+)

### Evidence Currency Check at [KB]

```
At end of every Full cycle, check research basis of all MANDATORY ADRs:
  □ For each research citation in active MANDATORY findings:
    □ Evidence age: FRESH | RECENT | AGING | STALE | CLASSIC
    □ Apply confidence discount if applicable
    □ If STALE and NOT CLASSIC: flag "Evidence Refresh Recommended"
      → Add to debt register: "Debt-EC-[N]: update evidence for [finding]"
    □ AI-domain research: apply 1.5× aging rate

Annual Evidence Refresh Cycle (once per calendar year):
  □ Review all [KB] research citations
  □ Run Consensus/Google Scholar search for each citation: "any updates or retractions?"
  □ Check for superseding meta-analyses in the same domain
  □ Update evidence_basis.year field after verification
  □ Promote or demote evidence tier based on current authority
```

### Adding Evidence Currency to Reference Headers

Each VHEATM reference file should carry:

```yaml
# Evidence Currency Header (add to top of each reference file):
research_summary:
  - source: "[Paper name, Author, Year]"
    citations: [N]
    evidence_tier: T1 | T2 | T3
    domain: "software-engineering | AI | security | org-behavior | ..."
    last_verified: [year]  # when this citation was last checked for retraction/update
    age_class: FRESH | RECENT | AGING | STALE | CLASSIC
```

---

## Part 2 — Org-Size Calibration for [M.AM]

### Why Size Matters for Maturity Models

SAMM, BSIMM, and NIST SSDF were developed with and for medium-to-large software
organizations. BSIMM data comes primarily from organizations with 100+ developers.
SAMM's "Level 2" practices assume dedicated security resources.

Applying BSIMM "baseline activities" to a 3-person startup as "baseline expectations"
miscalibrates maturity assessment:
- False negative: startup meets startup-appropriate baseline but appears "below baseline"
  by BSIMM norms → creates discouragement / irrelevant ADRs
- False positive: enterprise with formal security team appears "mature" but has gaps
  in practices relevant at their scale → misses real issues

### Size-Calibrated SAMM Expectations

#### <10 people (micro team / startup)

```
Expected baseline (reasonable for this size):
  Governance:    □ Basic security policy documented (even 1 page)
                 □ Team awareness of OWASP Top 10
  Design:        □ Threat model for core features (informal)
  Implementation:□ SAST in CI/CD (free tools: semgrep, bandit)
                 □ Dependency vulnerability check (Dependabot or equivalent)
  Verification:  □ Security test cases for authentication/authorization
  Operations:    □ Basic incident response: "who do we call, what do we do"

NOT expected at this size (defer to scale):
  □ Dedicated security team
  □ Formal SDLC process with security stage gates
  □ Security champions program
  □ Penetration testing (annual → yes; weekly → no)
```

#### 10-100 people (growth stage)

```
Expected baseline:
  Governance:    □ Security policy reviewed annually
                 □ Developer security training (annual, any format)
                 □ Security ownership role (can be part-time)
  Design:        □ Threat model for new features (lightweight, documented)
                 □ Security requirements in feature specs
  Implementation:□ SAST in CI/CD + DAST for APIs
                 □ Dependency scanning + automated vulnerability alerts
                 □ Secure coding standards documented
  Verification:  □ Security review for high-risk changes
                 □ Penetration testing: annual for external-facing features
  Operations:    □ Incident response plan documented + practiced
                 □ Security logging/monitoring for auth events

BSIMM calibration note: BSIMM L1 activities apply at this tier.
```

#### 100-1000 people (scale)

```
Expected baseline: full BSIMM L1 + selected L2 activities based on risk profile.
Apply v12.0 [M.AM] as-is — calibration matches this tier.
```

#### Enterprise (1000+)

```
Expected baseline: BSIMM L2-L3 for core practices.
Apply BSIMM descriptive data at face value.
Add enterprise-specific concerns: M&A security due diligence, vendor risk management,
regulatory audit programs, security SLAs in vendor contracts.
```

### [M.AM] Output with Size Calibration

```yaml
assurance_maturity_overlay:
  org_size: "<10 | 10-100 | 100-1000 | enterprise"
  calibration_tier: "micro | growth | scale | enterprise"
  maturity_deltas:
    - finding_id: ADR-[N]
      calibrated_expectation: "[what IS expected at this org size]"
      gap_against_calibrated: true | false  # gap against SIZE-appropriate, not BSIMM max
      improvement_recommendation: "[specific to org size and context]"
      priority: IMMEDIATE | SHORT_TERM | LONG_TERM
  size_specific_notes:
    - "[e.g., 'Penetration testing annually is expected at growth stage — not in scope for micro']"
```

---

## Part 3 — Framework Version Audit ([P.FD])

### The Problem

When VHEATM is upgraded (v12.0 → v13.0), the executor may:
1. Not know about new Hard Gates (e.g., HG-AD, HG-EF in v13)
2. Apply v12 mental model with v13 framework, missing new phases
3. Not know new reference files exist

### Framework Diff Check at [P]

When `FRAMEWORK_VERSION` declared at [P] and ≠ current:

```
[P.FD] Protocol:
  □ Identify version delta: [prior version] → [current version]
  □ Check SKILL.md changelog for new additions since prior version
  □ Generate framework-diff hypotheses:
    "HG-[new gate] was not present in [prior version].
     Was this gate satisfied in this cycle? If no → add to [G.H] as REQUIRED."

Framework diff hypothesis format:
  id: H-FD-[N]
  type: NEW_GATE | NEW_REFERENCE | MODIFIED_PROTOCOL | CORRECTED_MAPPING
  description: "[what changed between versions]"
  question: "[did this cycle cover the new content?]"
  layer: "FRAMEWORK" (special layer for framework-level hypotheses)
  qbr: REQUIRED (framework compliance) | RECOMMENDED (enhancements)
  evidence_anchor: "SKILL.md changelog entry"
```

### Framework Self-Test Catalog

A set of invariants that SHOULD be true of any valid VHEATM document:

```
Self-test catalog (run at [KB] after each full cycle):

INVARIANT-01: Reference completeness
  □ Every file listed in SKILL.md reference table exists in references/ directory
  □ Every file in references/ directory is listed in SKILL.md reference index
  Command (bash):
    # SKILL.md table format: "| N | filename.md | when |"
    # Extract filename from column 2 (not combined "N-filename.md")
    grep -oE '\| [0-9]+ \| [0-9]{2}-[a-z-]+\.md' SKILL.md | grep -oE '[0-9]{2}-[a-z-]+\.md' | sort -u > /tmp/skill_refs.txt
    ls references/*.md | xargs -I{} basename {} | sort -u > /tmp/dir_refs.txt
    echo "In SKILL.md but not in dir:"; comm -23 /tmp/skill_refs.txt /tmp/dir_refs.txt
    echo "In dir but not in SKILL.md:"; comm -13 /tmp/skill_refs.txt /tmp/dir_refs.txt
  Note: pattern "| N | filename.md" extracts filename from column 2 of table.
  PASS = both comm outputs empty.

INVARIANT-02: Hard Gate coverage
  □ Count of "| HG-" rows in SKILL.md matches declared gate count in changelog
  □ Every Hard Gate has a corresponding exit condition in ref 01 phase guide
  Command:
    grep -c "^| HG-" SKILL.md   # must equal number in changelog
    grep "Hard Gates:" SKILL.md  # verify declared count
  Note: all Gate rows must use "| HG-NAME |" format (not bold "| **HG-NAME** |").
  Bold formatting breaks the grep — new gates added in standard format.

INVARIANT-03: ADR part count consistency
  □ SKILL.md says "9-part ADR (+Part 10 for AI)"
  □ ref 01 ADR template section has exactly 9 parts + optional Part 10
  □ ref 06 output schemas have 9 ADR fields (+Part 10)
  Command: grep "9-part\|Part 10" SKILL.md references/01-phase-guide.md references/06-output-schemas.md

INVARIANT-04: Evidence tier coverage
  □ Every MANDATORY ADR recommendation in framework has evidence ≥ T3
  □ No MANDATORY recommendation based solely on T4/T5 (first principles / intuition)
  Manual check: scan ref 01 phase-guide for MANDATORY-level protocols without citations.

INVARIANT-05: Cross-reference validity
  □ All "→ ref [N]" citations in SKILL.md point to files that exist in references/
  □ All "ref [N] Part [M]" citations point to sections that exist in that file
  Command:
    for n in $(grep -oE 'ref [0-9]+' SKILL.md | grep -oE '[0-9]+' | sort -nu); do
      f=$(printf "%02d" $n)
      ls references/${f}-*.md 2>/dev/null | head -1 || echo "MISSING: ref $n"
    done

INVARIANT-06: Protocol claim-vs-behavior consistency (Eigenstate Detection) 🆕 v13.1
  □ For each MANDATORY-level protocol in SKILL.md that makes a behavioral claim:
    □ Identify the core claim (e.g., "YAML-only cross-visibility = bounded contamination")
    □ Identify the evidence anchor supporting the claim
    □ If no evidence anchor exists → FLAG as AI-S5.3 analog: "spec claims X, implementation unverified"
  Priority checklist (run every version release):
    □ IGP claim: "YAML-only cross-lens visibility achieves bounded cross-contamination"
      → Required evidence: empirical test or research citation showing YAML constraint
        reduces cross-contamination to acceptable level. SPP Wang 2023 supports
        "context constraints" in general but does not validate YAML-only specifically.
    □ QBR threshold 17 = MANDATORY designation
      → Required evidence: production accuracy data (see [M.AT]) OR explicit calibration
        rationale from historical incidents. If neither: T4 evidence — must be flagged.
    □ Hybrid Verification "72-96% FP reduction" claim (ref 17)
      → Required evidence: current-cycle verification that this holds for this codebase.
        Not just citation — active confirmation.
    □ Pre-mortem PM-accuracy claim (ref 01 [G.Pre])
      → Required evidence: [KB] PM-accuracy log shows predictions tracked ≥ 3 cycles.
  Command (for grep-detectable claims):
    grep -n "achieves\|guarantees\|ensures\|eliminates\|prevents\|proven\|confirmed" \
      SKILL.md references/*.md | \
    while read line; do
      # For each match: manually verify evidence anchor in same paragraph
      echo "CLAIM CHECK: $line"
    done
  PASS = every MANDATORY-level behavioral claim has T1-T3 evidence anchor.
  CONDITIONAL PASS = claim has T4 evidence (first principles / design intent) — must be documented.
  FAIL = MANDATORY behavioral claim with no evidence anchor found.

INVARIANT-07: Self-Application 🆕 v16.0
  □ Run VHEATM as if auditing its own SKILL.md as a target artifact (DESIGN context mode).
  □ Apply Anti-Patterns checklist to SKILL.md: any violations of framework's own rules?
  □ Apply Pattern Globalization: any naming/structural inconsistencies grep-able across SKILL.md?
  □ Apply Fix Verification: do referenced section titles/headers match what they label?
    Specifically check:
      - Hard Gate count in section header matches actual row count in table
      - "Reference Files — Full Index (vN)" version label matches current release version
      - "FAST Mode — Inline (vN, ...)" version label matches current release
      - INVARIANT-01 count claim matches `ls references/*.md | wc -l`
      - All "🆕 (vN.0)" version tags reflect actual introduction version, not last touch
  □ Apply Execution Fidelity [M.EF]: every section header has ≥1 of decision/finding/...?
  □ Apply [M.BA] behavior pattern check: are findings spread across L1-L7 or clustered?
  Command (bash):
    # Header-vs-table count mismatch check (SKILL.md):
    declared=$(grep -oE "^## [0-9]+ Hard Gates" SKILL.md | grep -oE "[0-9]+" | head -1)
    actual=$(grep -cE "^\| \*\*HG-" SKILL.md)
    [ "$declared" = "$actual" ] || echo "MISMATCH: declared=$declared actual=$actual"
    # Version label staleness check:
    grep -nE "(Full Index|FAST Mode — Inline) \(v[0-9]+" SKILL.md
    # Reference file count:
    ls references/*.md | wc -l   # compare against INVARIANT-01 stated count
  PASS = framework's own SKILL.md survives audit by framework's own rules.
  FAIL = ≥1 anti-pattern violation in SKILL.md OR ≥1 stale label OR count mismatch.

  Rationale: v15.0 framework had 5 self-application failures (stale "v13" labels, count
  mismatch, missing Mode Router, dropped trust principles, anti-pattern section absent)
  that the v15 self-test (INVARIANT-01..06) did not catch because none of them checked
  the framework's adherence to its own rules. INVARIANT-07 closes that gap.

Test schedule: INVARIANT-01 through INVARIANT-07 before each framework version release.
Results logged in [KB.FST]: "Framework Self-Test vX.Y: [N]/7 invariants passed. Failures: [list]."
```

### When to Run Framework Self-Test

```
MANDATORY:
  □ Before releasing a new framework version (vX.0)
  □ After adding or removing any reference file
  □ After modifying the Hard Gate table

RECOMMENDED:
  □ At start of first cycle with a new framework version
  □ When [P.FD] detects version delta

OPTIONAL:
  □ Any time framework consistency is uncertain
```

---

## Anti-Patterns

🚫 **"Evidence currency doesn't matter if the findings are still correct"** — The
finding might be correct but the authority of the recommendation changes if based
on retracted or superseded research. Transparent evidence aging lets stakeholders
assess the research basis honestly.

🚫 **"We're a startup, SAMM doesn't apply to us"** — Size-calibrated SAMM DOES
apply. The calibration says "here's what's appropriate for YOUR size" — not
"this isn't for you." Micro teams still need incident response and dependency scanning.

🚫 **"Framework diff check is only needed for major versions"** — Minor version
changes can introduce new Hard Gates (v13 added HG-AD and HG-EF from v12).
Any FRAMEWORK_VERSION change triggers a diff check.

🚫 **"Framework self-tests are meta-circular"** — They ARE meta-circular, and that's
the point. A framework that advocates for automated invariant testing should enforce
invariants on itself. Dogfooding principle: if you recommend it for code, apply it
to the framework spec.

🚫 **"The framework's own protocol claims don't need evidence anchors — they're design decisions"**
— A MANDATORY-level protocol claim without an evidence anchor is exactly what AI-S5.3
describes: a docstring that claims behavior the implementation may not deliver. The framework
holds external code to T3+ evidence. It must hold itself to the same standard. Unverified
behavioral claims in SKILL.md are Eigenstate conditions: the framework is stable with its
self-description precisely because it lacks a mechanism to challenge that description.
(INVARIANT-06 closes this gap.)

---

## Part 4 — Empirical Accuracy Tracking and Revision Triggers (v13.1)

> **Gap closed**: Framework Self-Test (Part 3) verifies structural consistency —
> reference completeness, gate counts, cross-reference validity. It does NOT verify
> whether the framework's outputs are empirically accurate: are MANDATORY findings
> actually mandatory in production? Part 4 closes this gap.
>
> **Relationship to [M.AT] in ref 13**: The Bug Class Catalog (ref 13) holds the
> per-instance accuracy data (`production_validated`, `false_positive_retroactive`).
> This section defines the framework-level protocol for when to act on that data.

### Accuracy Threshold and Revision Protocol

```
Framework Accuracy Revision Protocol — four trigger types (v14 update):

  Empirical trigger (Track N — evidence-anchored thresholds):
    □ measured_accuracy_rate < calibrated threshold across ≥ 3 consecutive cycles
    □ Threshold is NOT a fixed value — derived from calibration protocol:
      PRE-CALIBRATION (cycles 1-5): threshold = 60% (T3 industry benchmark)
      EMPIRICAL (cycle 6+): threshold = (5-cycle baseline) × 0.85 (T2 own data)
      RECALIBRATED (annually): threshold from last 12 months × 0.85 (T2)
      Absolute floor: 50% (below = random chance territory)
    □ Action: core review of QBR threshold (currently 17 = MANDATORY)
    □ Review question: "Is QBR ≥ 17 calibrated to actual severity, or to
      framework designer heuristics that have never been validated in production?"
    Evidence anchors:
      Du et al. 2025 (arxiv 2601.18844): SAST tools >95% FP on large codebases
      Devo SOC Performance Report 2024: 53% FP rate industry average
      Prophet Security 2026: <25% FP = "critical" (well above average)
      Massacci et al. 2020: CVSS inter-rater reliability (Empirical SE, Springer)

  Comparative trigger (→ full protocol in Part 5 [KB.CMA]):
    □ Same codebase audited by competing methodology AND finding divergence > 40%
    □ OR: [M.AT] empirical trigger fires → comparative audit recommended
    □ OR: first cycle of new project (baseline establishment)
    □ OR: every 10th cycle (periodic validation)
    □ Action: run full Comparative Methodology Audit Protocol (Part 5)

  Abductive trigger (→ [G.AB] 3-mode protocol in ref 01):
    □ Production anomaly not predicted by any H-ID in prior cycle's [G.H]
    □ Action: mandatory post-mortem using [G.AB] REACTIVE mode
    □ Ask: "why did our hypothesis vocabulary miss this symptom?"
    □ Output: new hypothesis template + OOV-AB-N entry if out of vocabulary

  Paradigmatic trigger:
    □ ≥ 3 out-of-vocabulary anomalies in a single cycle (abductive trigger fired 3×)
    □ Action: [G.AB] VOCABULARY EXPANSION mode → layer proposal for v+1
    □ This is the Kuhnian signal: belt absorption is failing, core may need revision
```

### Integration with Framework Self-Test

[KB.FST] table in ref 01 now includes accuracy and eigenstate columns:

```
| Version | INV-01..06 | MAT-Rate | MAT-Phase | EP-Risk | CMA-Done | Pass/Fail |
```

Where:
- `MAT-Rate` = measured_accuracy_rate from [M.AT] tracker (ref 13)
- `MAT-Phase` = PRE-CALIBRATION | EMPIRICAL | RECALIBRATED
- `EP-Risk` = last eigenstate probe risk level from [M.EP] (LOW | MEDIUM | HIGH)
- `CMA-Done` = last comparative methodology audit cycle number, or "N/A"

---

## Part 5 — Comparative Methodology Audit Protocol [KB.CMA] (v14.0)

> **Gap closed**: v13.1 added a "Comparative trigger" as a one-line condition.
> This was a trigger without a protocol — no guidance on HOW to run a cross-methodology
> comparison, normalize findings, or interpret results. Part 5 provides the full protocol.
>
> **Evidence basis**: SPECA (Zhou et al. 2025, arxiv 2602.07513) showed that cross-
> implementation checks account for 76.5% of valid findings in multi-methodology audits,
> and 56.8% of false positives came from threat model misalignment, not accuracy differences.

### When to Run

```
MANDATORY:
  □ [M.AT] Empirical trigger fires (accuracy below threshold for 3+ cycles)
RECOMMENDED:
  □ First cycle of a new project (baseline establishment)
  □ Every 10th cycle (periodic validation)
  □ Human reviewer requests (ad hoc)
```

### Competing Methodology Selection

```
Select ONE methodology most appropriate for the codebase:
  Web application          → OWASP ASVS v4.0 checklist
  System architecture      → PASTA 7-step process
  Threat enumeration       → STRIDE per-component
  Compliance-focused       → NIST SP 800-53 control mapping
  AI/ML system             → OWASP ML Top 10 + NIST AI RMF MAP
  API-focused              → OWASP API Security Top 10

Document selection rationale in [KB.CMA] entry.
```

### Execution Protocol

```
Phase 1: Independent execution
  □ Run competing methodology INDEPENDENTLY
  □ For AI-assisted audit: use separate context window or session
  □ For human audit: assign different auditor if possible
  □ Do NOT look at VHEATM findings during competing methodology run
  □ Output: finding list in competing methodology's native format

Phase 2: Finding normalization
  Normalize BOTH finding sets to common format:
    finding_id: [methodology-prefix]-NNN
    description: [what was found]
    severity: CRITICAL | HIGH | MEDIUM | LOW
    category: CWE-ID (preferred) | methodology-native category
    location: file:line | component | API endpoint
    evidence: code snippet | configuration | reference

Phase 3: Comparison matrix
  For each finding in either set:
    OVERLAP:          same finding, same location in both
    PARTIAL_OVERLAP:  same category, different location
    VHEATM_UNIQUE:   in VHEATM only — VHEATM's added value
    VHEATM_MISS:     in competing only — VHEATM blind spot
    CONFLICT:         opposite conclusion on same location — scope mismatch

Phase 4: Metrics
  overlap_rate      = OVERLAP / total_unique_findings
  unique_rate       = VHEATM_UNIQUE / total_unique_findings
  miss_rate         = VHEATM_MISS / total_unique_findings
  conflict_count    = CONFLICT count

Phase 5: Interpretation and action
  overlap_rate > 80%:   VHEATM tracks well with established methodology
  unique_rate > 20%:    VHEATM adding genuine value beyond competing methodology
  miss_rate > 30%:      VHEATM has systematic blind spot → investigate:
                        □ Is the miss in a layer VHEATM covers? → hypothesis generation gap
                        □ Is the miss outside L1-L7? → potential new layer needed (→ [G.AB] Mode 3)
  conflict_count > 0:   Scope assumption mismatch → clarify threat model boundaries
                        (SPECA found 56.8% of FP from threat model misalignment)
```

### [KB.CMA] Output Template

```yaml
comparative_methodology_audit:
  cycle: N
  competing_methodology: "OWASP ASVS v4.0 | PASTA | STRIDE | ..."
  selection_rationale: "[why this methodology for this codebase]"
  metrics:
    total_unique_findings: N
    overlap: N
    overlap_rate: "X%"
    vheatm_unique: N
    unique_rate: "X%"
    vheatm_miss: N
    miss_rate: "X%"
    conflicts: N
  analysis:
    blind_spots: ["[layer/category VHEATM missed]"]
    value_added: ["[finding types only VHEATM found]"]
    scope_mismatches: ["[where threat model assumptions differed]"]
  action:
    - "[specific improvement for VHEATM based on comparison]"
  next_comparison_due: "cycle N+10 | on empirical trigger"
```

### Anti-Patterns

🚫 **"VHEATM already includes OWASP as a lens — comparison is redundant"** — VHEATM ROUTES to
OWASP as a specialist lens. Running OWASP ASVS independently tests whether VHEATM's routing
actually invokes the right checks. Routing ≠ executing.

🚫 **"The competing methodology will always find different things — comparison is meaningless"** —
Different doesn't mean meaningless. SPECA (2025) showed 76.5% of valid findings came from
cross-methodology checks. The comparison reveals systematic blind spots, not random noise.

🚫 **"We don't have capacity for a full competing methodology run"** — Use the lightest option:
OWASP Top 10 checklist (not full ASVS) takes ~30 minutes for a focused web application.
Low fidelity comparison beats no comparison.

---

## Part 6 — Continuous Eigenstate Detection [M.EP] (v14.0)

> **Gap closed**: v13.1 INVARIANT-06 runs only at version release. Von Foerster's eigenforms
> emerge continuously through recursive self-observation: "The recursive process converges
> over time to a stable concept" (Pangaro/Glanville 2002). A framework that self-audits
> every cycle forms eigenforms every cycle — not just at releases.
>
> **Evidence basis**:
> Von Foerster (1965, 1969, 1977): Eigenvalues of observing systems — stable structures
> maintained in an organization's operational dynamics (CLASSIC, foundational).
> Glanville 2008 ("Second Order Cybernetics"): eigenfunctions "applied recursively, reach
> stable and (dynamically) self-perpetuating states."
> Chavalarias 2016 (SAGE Publications): stable attractors in self-referential systems;
> observer inside the system tends to interpret dynamics as "self-organized order."

### [M.EP] Per-Cycle Eigenstate Probe

```
Run at: [M] phase, after [M.EF] and [M.AT], before final handoff
Cost: 5 minutes (3 questions from rotating pool)
Required in: Standard + Full mode (skip in FAST)

QUESTION POOL (12 questions, rotate 3 per cycle):

Protocol Claim Questions:
  Q1:  "Did this cycle rely on IGP to isolate lens outputs? If yes: was the
       isolation actually effective, or did prior lens reasoning visibly leak?"
  Q2:  "Did QBR ≥ 17 designation feel justified by the evidence, or was it
       a mechanical output of the formula without judgment?"
  Q3:  "Did any finding rely on docstring/comment trust without code verification?
       (AI-S5.3 applied to THIS audit's own reasoning)"
  Q4:  "Was any hypothesis downgraded because 'the framework says it's RECOMMENDED
       not MANDATORY' without independent severity assessment?"

Process Assumption Questions:
  Q5:  "Did Pattern Globalization actually find new instances, or did it confirm
       what was already expected? (confirmation bias in PG)"
  Q6:  "Did the Adversarial Pass produce genuinely new findings, or did it
       rediscover findings already in [G.H]? (independence failure)"
  Q7:  "Was Hybrid Verification applied to the findings that needed it most, or
       to the ones that were easiest to verify? (selection bias)"
  Q8:  "Did the Pre-mortem produce predictions that were DIFFERENT from [G.H]
       hypotheses, or was it redundant?"

Meta Questions:
  Q9:  "What did this cycle NOT look for? Name 3 categories of bugs that the
       layer taxonomy doesn't cover for this specific codebase."
  Q10: "If this audit is wrong, what is the most likely way it's wrong?"
  Q11: "What would a competing methodology (PASTA/ASVS) have found that we didn't?"
  Q12: "Is there a production symptom we know about but didn't generate a
       hypothesis for? (proto-abductive check)"

Rotation schedule:
  Cycle 1: Q1, Q5, Q9       Cycle 2: Q2, Q6, Q10
  Cycle 3: Q3, Q7, Q11      Cycle 4: Q4, Q8, Q12
  Cycle 5+: restart rotation — compare answers to same questions from 4 cycles ago
  → Drift check: did the same question produce a substantively different answer?
    If yes → eigenstate may have shifted → flag for review
    If no → stable eigenstate — check if stability is warranted or complacent
```

### [M.EP] Output Template

```yaml
eigenstate_probe:
  cycle: N
  questions_asked: [Q3, Q7, Q11]
  answers:
    - question: Q3
      answer: "[honest assessment]"
      action: REVERT_TO_E | ACCEPT | FLAG_DEBT
    - question: Q7
      answer: "[honest assessment]"
      action: REVERT_TO_E | ACCEPT | FLAG_DEBT
    - question: Q11
      answer: "[honest assessment]"
      action: REVERT_TO_E | ACCEPT | FLAG_DEBT
  eigenstate_risk: LOW | MEDIUM | HIGH
    # LOW: all answers show honest uncertainty or verified claims
    # MEDIUM: 1 answer reveals unverified assumption → flag as debt
    # HIGH: ≥2 answers reveal unverified assumptions → add Debt-EP-[N],
    #        flag for next cycle's [G.H], consider [KB.CMA] comparative run
  drift_check:
    compared_to_cycle: N-4  # or null if first rotation
    drift_detected: true | false
    drift_description: "[what changed in the answer]"
```

### Relationship to INVARIANT-06

INVARIANT-06 (version-release, Part 3) = **deep scan** of all protocol claims.
[M.EP] (per-cycle, Part 6) = **lightweight probe** of 3 rotating questions.

Both serve eigenstate detection. They are complementary, not redundant:
- [M.EP] catches eigenstate formation DURING cycles (early detection)
- INVARIANT-06 catches accumulated drift at version boundary (comprehensive check)
- [M.EP] answers feed into INVARIANT-06: if [M.EP] flagged HIGH risk ≥2 times
  between releases, INVARIANT-06 should prioritize those protocol claims.

---

## Part 7 — Accuracy Dashboard [KB.AD] (v14.0)

> **Gap closed**: PM-accuracy (ref 01 [G.Pre]) and [M.AT] (ref 13) are two separate
> empirical feedback streams. [KB.AD] unifies them into a single dashboard that shows
> prediction accuracy AND classification accuracy together.

### [KB.AD] Unified Accuracy Dashboard

```yaml
accuracy_dashboard:
  cycle: N
  prediction_accuracy:
    pre_mortem:
      predictions_made: N
      predictions_materialized: N
      pm_accuracy_rate: "X%"
    mandatory_classification:
      mandatory_designated: N
      production_validated_true: N
      production_validated_false: N
      production_validated_unknown: N
      mandatory_accuracy_rate: "X%"  # exclude unknown from denominator
    combined_accuracy:
      weighted_score: "X%"  # (pm_accuracy × 0.3) + (mandatory_accuracy × 0.7)
      # Weighting: MANDATORY accuracy matters more than PM prediction
      status: "NOMINAL | WATCH | TRIGGER"

  calibration:
    calibration_phase: "PRE-CALIBRATION | EMPIRICAL | RECALIBRATED"
    current_threshold: "X%"
    threshold_evidence_tier: "T3 | T2"
    last_calibration_cycle: N
    next_calibration_due: "cycle N+M | annual"

  trend:
    last_5_cycles: ["X%", "Y%", ...]  # mandatory_accuracy_rate per cycle
    direction: "IMPROVING | STABLE | DECLINING"
    # DECLINING for 3+ cycles → automatic flag for review
    # IMPROVING for 5+ cycles → consider tightening threshold

  eigenstate_health:
    last_ep_risk: "LOW | MEDIUM | HIGH"
    ep_high_count_since_release: N  # if ≥2 → flag for INVARIANT-06 priority
    last_cma_cycle: N               # last comparative methodology audit

  comparative:
    last_cma_overlap_rate: "X%"
    last_cma_miss_rate: "X%"
    last_cma_methodology: "[name]"
```

---

*Reference 30 — VHEATM 14.0 | Framework Lifecycle Intelligence
v13.0 gaps: evidence currency, framework version drift, SAMM/BSIMM org-size miscalibration.
v13.1 additions: INVARIANT-06, Part 4 (Revision Triggers).
v14.0 additions: Part 4 updated with evidence-anchored thresholds (Track N),
Part 5 [KB.CMA] Comparative Methodology Protocol (Track O),
Part 6 [M.EP] Continuous Eigenstate Detection (Track P),
Part 7 [KB.AD] Unified Accuracy Dashboard (Track R).
Evidence anchors: Du et al. 2025, Devo 2024, Prophet Security 2026, Massacci et al. 2020,
Von Foerster 1965-1977, Glanville 2008, Chavalarias 2016, SPECA Zhou et al. 2025.*
