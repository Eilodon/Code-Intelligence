# Runtime Controller (v12.0)

> The Runtime Controller is the AI executor's decision layer.
> It answers: "What should I do next, given what I know right now?"
> Framework spec answers WHAT. Runtime Controller answers WHEN, WHETHER, and HOW MUCH.
>
> Design principle: fail toward precision, not coverage.
> Better to audit less with high confidence than audit everything with low confidence.

---

## Part 1 — Mode Inference (when CONTEXT_BUDGET is unknown or user hasn't declared)

If MODE was not explicitly declared, infer from input signals:

```
Input contains: code files, PRDs with technical detail, ADRs → infer CODE or DESIGN
Input contains: incident timeline, metrics, logs → infer LIVE + POST-INCIDENT profile
Input contains: "quick", "fast", "just check" → infer FAST
Input contains: "thorough", "launch", "critical" → infer Standard
Input contains: "full audit", "compliance", "enterprise" → infer Full
No strong signal → default Standard

Context Mode inference:
  Has code/file structure → CODE
  Has only spec/requirements → DESIGN
  Has incidents/metrics → LIVE
  Has >2 team ownership → ENTERPRISE
  Has legacy system with debt → LEGACY
  Ambiguous → ask ONE clarifying question: "Is this pre-launch audit or investigation of existing system?"
```

**Never ask more than one clarifying question per ambiguity.**
If multiple ambiguities exist, pick the one that most changes the audit approach.

---

## Part 2 — Phase Skip Logic

Not every phase is required in every audit. Skip logic is NOT about cutting corners —
it's about not producing ceremonial output for phases with nothing to produce.

```
[P] Pre-conditions
  NEVER skip. Always run, even in FAST mode.
  FAST: compress to 5-line context declaration.

[V] Vision
  Skip if: DESIGN mode with no architecture diagram possible
           (no artifact to map from)
  In that case: note "V skipped — no architecture artifact available"
               and proceed with assumption-based hypotheses in [G].
  Skip [V.UT]: if scope is a single isolated function/module (no QA tradeoffs at stake)
  Skip [V.AS]: if FAST mode (run AS-01 + AS-03 inline in [G] instead)

[G] Generation
  NEVER skip. Core value of audit.
  Skip [G.T]: if no persistent state in scope (pure stateless functions)
  Skip [G.INC]: if FAST mode
  Skip [G.ORG]: if FAST mode or single-team scope
  Skip [G.FL] FMEA-lite: if not safety-critical, no state machines, blast_radius < 3
  Skip Language Profile: if LANGUAGE = N/A

[E] Experiment
  NEVER skip MANDATORY hypothesis verification.
  Skip [E.HV] Step 2 if: static confidence = HIGH (file was read, path is clear)
  Skip [E.MS] if: no test suite exists at all
  FAST: skip [E.HV] Steps 2+3, document "HV deferred — FAST mode"

[A] Architecture
  NEVER skip for MANDATORY findings.
  Skip Opposing ADR: if FAST mode and finding is unambiguous
  Skip full SNF: FAST uses Q1+Q3 only

[T] Transformation
  Skip if: audit-only run (no fixes being applied in this session)
  Document: "T skipped — audit only. Fixes deferred to next cycle."

[M] Closure
  NEVER skip [M.AP] adversarial pass — even FAST needs 1 lens.
  Skip [M.AM] Assurance Maturity Overlay: FAST and Standard modes
  Skip [M.AP] Full lenses: Standard mode gets 4-lens only (not Structured Lens Frame)

[KB] Knowledge Base
  Skip in FAST if session is ad-hoc and no persistent KB exists.
  Document: "KB update deferred — no persistent state in this session."
  In AI-ingestion Handoff YAML: flag kb_updated: false
```

---

## Part 3 — Context Budget Routing

### 🆕🆕 Reference File Overhead (v13.0)

Before routing by codebase budget, subtract reference file loading overhead.
Token estimates (approximate, based on file sizes):

| Ref # | File | Est. Tokens | Loaded when |
|---|---|---|---|
| 00 | context-modes.md | ~2.5k | Always at [P] |
| 01 | phase-guide.md | ~6k | Standard+Full; any detail needed |
| 02 | bias-probes.md | ~1.5k | [G.B] all 6 checks |
| 11 | cross-cutting-layer.md | ~2.5k | [G.H] L7 + AI-S1–S5 |
| 18 | ai-native-addenda.md | ~3k | [P] AI runner; [M.AP] |
| 19 | language-profiles.md | ~4k | [P] when LANGUAGE declared |
| 21 | runtime-controller.md | ~2.5k | Always for AI runner |
| 27 | l4-sublayers.md | ~2.5k | [G.H] L4 hypotheses |
| 28 | auditor-defense.md | ~3.5k | [G.AD] Standard+Full |
| 29 | execution-fidelity.md | ~3k | [M.EF] all modes |
| 30 | framework-lifecycle.md | ~3k | [P.FD]; [KB.EC]; [M.AM] |
| Others (avg) | varies | ~2k | On-demand trigger |

**Typical reference overhead by configuration:**

```
MINIMAL (FAST, no language, no AI):
  refs 00, 21, 29 → ~8k tokens overhead

STANDARD (CODE mode, LANGUAGE declared):
  refs 00, 01, 02, 11, 18, 19, 21, 27, 28, 29 → ~32k tokens overhead

MAXIMUM (Full, AI_INTEGRATED=YES, ENTERPRISE, LANGUAGE declared):
  refs 00, 01, 02, 11, 18, 19, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30 → ~45k tokens overhead
```

**Effective budget rule:**
```
effective_codebase_budget = declared_budget - reference_overhead

Example: CONTEXT_BUDGET = 80k tokens (MEDIUM), Full mode, AI_INTEGRATED=YES, ENTERPRISE:
  reference_overhead ≈ 45k
  effective_codebase_budget = 80k - 45k = 35k → route as SMALL for codebase reading

⚠️ MEDIUM budget + AI_INTEGRATED=YES + ENTERPRISE + LANGUAGE declared →
   effective codebase budget becomes SMALL. Declare SMALL routing proactively.
```

### Codebase Budget Routing (applied to effective_codebase_budget above)

```
CONTEXT_BUDGET states (applied AFTER subtracting reference overhead):
  LARGE   = > 100k tokens remaining for codebase (no budget constraint)
  MEDIUM  = 50k-100k remaining (some constraint — prioritize)
  SMALL   = < 50k remaining (significant constraint — compression required)
  UNKNOWN = treat as MEDIUM

LARGE → Standard routing. Read files on-demand.

MEDIUM → Priority reading:
  Phase 1: README + entry points + architecture overview ONLY
  Phase 2: Read files ONLY when specific hypothesis requires it
  Phase 3: For each unread file → mark hypothesis confidence = MEDIUM
  Document: "Medium budget: [N] files unread. Confidence calibrated."

SMALL → Compression routing:
  Read: entry points + top-level imports + error-handling paths ONLY
  All hypotheses about unread modules → confidence = LOW by default
  [E.HV] Step 2 REQUIRED for all MANDATORY findings (can't verify statically on unread)
  Consider: split audit into 2 sessions with explicit scope boundary
  Handoff: files_not_read list is MANDATORY (not optional)

CRITICALLY_SMALL (< 20k tokens for codebase):
  Declare at [P]: "CRITICALLY_CONSTRAINED"
  Audit limited to: architecture level only + entry point reads
  Recommend: human auditor supplement OR session splitting
  All QBR scores × 0.75 (uncertainty discount)
```

---

## Part 4 — Finding Confidence Gate

Before any finding exits [E] to [A]:

```
Confidence Gate:
  HIGH confidence:
    □ File was READ in this session (not reasoned about)
    □ Bug anchor is specific file:line
    □ Trigger path is deterministic (not conditional on unknown state)
    → Allow MANDATORY ADR

  MEDIUM confidence:
    □ File was read OR reasoning from architecture that is documented
    □ Bug is plausible but trigger path has conditional branches
    □ Pattern identified from similar code, not direct read
    → Allow MANDATORY ADR ONLY if [E.HV] Step 2 confirms
    → If [E.HV] deferred (FAST mode): issue as REQUIRED, not MANDATORY

  LOW confidence:
    □ File not read. Pattern inferred from architecture/documentation.
    □ Bug is theoretical, no direct evidence in code.
    → Maximum ADR priority: REQUIRED
    → Document: "Evidence: LOW confidence — [E.HV] required before implementation"
    → NEVER issue as MANDATORY without [E.HV] Step 2 confirmation

  NO confidence (Unknown anchor):
    □ AI has no codebase evidence. Pure architecture reasoning.
    → Maximum ADR priority: RECOMMENDED
    → Flag: HUMAN-REVIEW REQUIRED
```

---

## Part 5 — Output Volume Control

When context is constrained or output would exceed reasonable length:

```
Priority ordering for output:
  1. MANDATORY ADRs with evidence (always include)
  2. REQUIRED ADRs (include if budget allows)
  3. Executive summary (include in Standard+Full)
  4. Hypothesis table (include in Standard+Full; FAST: top 3 only)
  5. RECOMMENDED ADRs (include in Full only; defer to debt register otherwise)
  6. Bias probes verbose output (FAST: one-line; Standard: abbreviated)
  7. Phase ceremony text (compress to: "Phase X: N findings, [gate status]")

Compression trigger: if output draft > 2000 tokens for FAST, > 5000 for Standard,
  > 10000 for Full → trim ceremonial text first, then RECOMMENDED section,
  then verbose bias output. Never trim MANDATORY ADRs or evidence anchors.
```

---

## Part 6 — Stop Conditions

When to stop the audit and declare explicitly:

```
HALT conditions:
  □ Context budget < 10% remaining and critical findings still unverified →
    DECLARE "BUDGET HALT. Verified [N] of [M] critical hypotheses.
    Remaining deferred to next session. See handoff."

  □ Evidence quality is uniformly LOW (too many unread files) →
    DECLARE "COVERAGE INSUFFICIENT. Recommendation: scope audit to subsystem."

  □ Finding is outside AI auditor competence (e.g., hardware-software boundary,
    organizational incentives without org data, live production behavior) →
    DECLARE HUMAN-REVIEW REQUIRED for that finding.
    Do NOT substitute speculation for evidence.

  □ Handoff contains contradictory state (cycle N says X fixed, codebase still shows X) →
    DECLARE "STATE CONFLICT detected between handoff and codebase.
    Running SCR (Same-Cycle Re-Audit) before proceeding."
    Activate [G.SCR] with QBR × 1.20.
```

---

## Part 7 — Ritual Suppression Rule

**Every phase section in the output must produce exactly one of:**
```
  □ decision      — "Based on [evidence], the finding is [priority]"
  □ finding       — New H-ID with anchor
  □ downgrade     — "H-N downgraded to [lower priority] because [reason]"
  □ debt          — "Deferred to debt register: [description, age, owner]"
  □ next action   — "Required before next cycle: [specific condition]"
  □ skip reason   — "Phase X skipped: [explicit reason]. Implication: [what was not covered]"
```

**If a section produces only prose explanation with none of the above outputs → the section is ceremonial. Remove it.**

This rule applies to every phase in every mode. "Declaring context" is not an output.
The output of [P] is: context declaration + CLI + test coverage status = structured data.

---

## Part 8 — POST-INCIDENT Execution Profile

When the audit is triggered by a production incident:

```
POST-INCIDENT profile activates when:
  □ Input contains incident timeline, postmortem draft, or symptom description
  □ User says "what went wrong", "root cause", "why did X fail"

Execution changes:
  [P]:  Context Mode = LIVE. Goal = ROOT-CAUSE + BLAST-RADIUS.
        Add incident timeline to [P] declarations.
  [V]:  Focus on system state AT TIME OF INCIDENT, not current state.
        Delta map: "what changed in last N deploys before incident?"
  [G]:  Pre-mortem becomes POST-mortem: "Given the incident occurred, trace backward."
        Hypothesis generation STARTS from observed failure mode, not from code scan.
        L-T (temporal) and L5 (external dependency) are highest priority layers.
  [E]:  Evidence = incident timeline events, logs, metrics, not just code.
        Evidence Tier for incidents:
          T1: Confirmed metric drop / error spike at specific timestamp
          T2: Log evidence of failure path
          T3: Correlated deploy or config change
          T4: Inferred from code path (no log evidence)
  [A]:  ADRs split into:
          IMMEDIATE: stop-the-bleeding fixes (deploy now)
          SHORT-TERM: root cause remediation (this sprint)
          LONG-TERM: systemic prevention (debt register)
  [M]:  Mandatory: "How would monitoring have detected this earlier?"
        Add: Detection Gap ADR for any MANDATORY finding with no prior alerting.
  [M.AP]: Adversarial lens = "What OTHER failure modes did this incident expose?"

Handoff note: POST-INCIDENT audits should reference the incident ID.
  "Post-incident audit for INC-[ID]. Root cause: [H-ID]. Next cycle: verify fix + detection coverage."
```

---

*Reference 21 — VHEATM 12.0 | Runtime Controller — AI-native execution logic*
