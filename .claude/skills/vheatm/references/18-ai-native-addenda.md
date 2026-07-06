# AI-Native Addenda (v11.0)

> This reference file covers three areas introduced in Track B:
> (1) Context Budget management at [P],
> (2) Structured Lens Frame for [M.AP] adversarial pass,
> (3) AI False Positive Catalog for [KB].
>
> **Research basis:**
> [MemGPT (Packer et al., 2023, 323 citations)](https://consensus.app/papers/details/d13784ef80605de6a9306c7dcae89ebd/):
> virtual context management for LLMs — context window as OS memory resource requiring
> active management. Key finding: intelligent memory tier management enables tasks
> far exceeding raw context window limits.
>
> [SPP (Wang et al., 2023, 238 + 100 citations)](https://consensus.app/papers/details/8345b740e11555b29ea628608ed80767/):
> Solo Performance Prompting — single LLM into cognitive synergist via multi-persona
> self-collaboration. **Key finding: assigning multiple fine-grained personas with
> specific context constraints significantly outperforms single or generic personas.**
> "Dynamic identifying and simulating different personas based on task inputs unleashes
> cognitive synergy." Cognitive synergy only emerges with fine-grained context grants,
> not role labels alone.

---

## Part 1 — Context Budget Management ([P])

### Why time-boxing doesn't work for AI auditors

VHEATM 9.0 and 10.0 include time-boxes: "spend 3 minutes on [G.B]", "2 minutes max for FAST."
For a human auditor these are meaningful constraints. For an AI auditor:
- Time is not the binding constraint. Context window is.
- "3 minutes" maps to an arbitrary number of tokens depending on codebase complexity.
- The real question is: "Can I read all relevant files within my context budget?"

### Context Budget Declaration at [P]

```
CONTEXT_BUDGET: [tokens available | "unknown"]
```

When declared, route as follows:

```
CONTEXT_BUDGET routing:

UNKNOWN or LARGE (> 100k tokens for typical codebase):
  → Standard routing. Read files as needed.

CONSTRAINED (context budget < estimated full-codebase read):
  Phase 1 — Architecture First:
    [V] Read ONLY: README, top-level directory, main entry points, dependency manifest.
    Build C4 sketch from this, not from full file reads.

  Phase 2 — Hypothesis-Driven Reading:
    Do NOT read all files. Read ONLY files relevant to hypotheses in [G.H].
    Defer reads until a specific hypothesis requires them.
    Document: "Context-constrained: [file X] not read. Hypothesis H-N confidence = MEDIUM."

  Phase 3 — Priority Triage:
    If context nearly exhausted: suspend RECOMMENDED/OPTIONAL hypothesis pursuit.
    Complete MANDATORY hypotheses only.
    Handoff: explicitly list "files not read this session" for next instance.

CRITICALLY CONSTRAINED (context budget < architecture-only read):
  → Declare at [P]: "CRITICALLY_CONSTRAINED. Audit limited to partial scope."
  → Read: entry point + top-level architecture only.
  → All hypotheses default to MEDIUM confidence (Verify-Before-Claim applies).
  → Recommend: split audit into multiple sessions with explicit scope boundaries.
```

### AI-Ingestion Handoff YAML Schema

Replace (or supplement) prose Compressed Handoff with this machine-parseable schema
when the next instance is also an AI:

```yaml
handoff:
  cycle: [N]
  version: "13.0"                    # 🆕🆕 updated from 11.0
  # 🆕🆕 v13 cycle status fields (from ref 29 Part 2):
  cycle_status: COMPLETE | PARTIAL | HALTED | BUDGET_HALT
  interrupted_at_phase: "P | V | G | E | A | T | M | KB"   # if cycle_status ≠ COMPLETE
  phases_not_completed: ["[phase]", ...]                     # if cycle_status ≠ COMPLETE
  critical_hypotheses_unverified: ["H-[ID]", ...]           # MANDATORY H-IDs not yet in [E]
  minimum_resume_action: "[what next instance MUST do before continuing]"
  context:
    mode: "[DESIGN|CODE|LIVE|LEGACY|ENTERPRISE]"
    language: "[rust|typescript-react|python|other|N/A]"
    ai_integrated: true | false
  codebase_state:
    git_sha: "[SHA or 'unknown']"
    context_budget_used: "[tokens or 'unknown']"
    files_read_this_session:
      - "[path/to/file.rs]"
      - "[path/to/other.ts]"
    files_not_read:
      - "[path/to/large_module.rs — deferred: context budget]"
  open_hypotheses:
    - id: "H-001"
      description: "[brief]"
      layer: "L2"
      status: "UNVERIFIED"
      evidence_anchor: "[file:line or PENDING]"
      qbr: [N]
  confirmed_bugs:
    - id: "H-003"
      adr_id: "ADR-1"
      fix_status: "PENDING | APPLIED | VERIFIED"
      fix_anchor: "[file:line or PENDING]"
  adrs_issued:
    - id: "ADR-1"
      priority: "MANDATORY"
      owner: "[team]"
      boundary: "YES | NO"
      fix_status: "PENDING | APPLIED | VERIFIED"
  bug_class_catalog_hot:
    - class_id: "BC-001"
      last_seen: "[H-003]"
      search_command: "[grep command]"
  ai_fp_catalog_recent:
    - type: "hallucinated_path"
      instance: "[H-005 cited path/file.rs:42 which does not exist]"
  next_instance_read_first:
    - "[path/to/critical_module.rs]"
    - "[path/to/state_manager.ts]"
  debt_register:
    - id: "Debt-001"
      description: "[brief]"
      age_cycles: [N]
      priority: "HIGH | MEDIUM | LOW"
  next_cycle_trigger: "[specific condition]"
  next_cycle_focus: "[specific questions]"
  cli_last: [N]
  cli_ema3: [N]
```

**Key fields for next AI instance:**
- `files_read_this_session` → avoid re-reading, use context efficiently
- `files_not_read` → explicit gaps; hypotheses about these files default to MEDIUM
- `next_instance_read_first` → auditor judgment on what matters most
- `ai_fp_catalog_recent` → pattern of AI errors in this codebase to watch for

---

## Part 2 — Structured Lens Frame ([M.AP])

### Why narrative personas don't work for AI adversarial passes

VHEATM 10.0 Full mode adversarial pass:
> "Lens 1 — Pattern Lens (SRE Perspective). You are an SRE who gets paged at 3am..."

The problem: a role label ("You are an SRE") does not change the underlying reasoning.
The same analysis happens with a different header. SPP (Wang 2023) shows why: cognitive
synergy in LLMs requires **fine-grained context constraints**, not role labels.

**v11.0 mechanism**: `CONTEXT_DENIED` — explicitly forbidding the AI from using knowledge
from previous lenses.

**v13.0 correction** (→ ref 28 Part 2): `CONTEXT_DENIED` replaced by **Independent Generation
Protocol (IGP)**. Transformer models cannot truly quarantine context — CONTEXT_DENIED was
aspirational, not mechanically enforceable. IGP uses YAML-only cross-lens visibility
(bounded contamination, ≤500 tokens per prior lens YAML) instead.

### v13.0 Structured Lens Frame Format (IGP)

For each lens in [M.AP], declare explicitly:

```
LENS: [name]
PERSPECTIVE: [stakeholder role]
CONTEXT_GRANTED: [what knowledge this lens has access to]
CONSTRAINT: "Generate findings ONLY from your perspective context above.
             Do not reference or build on findings from prior lenses.
             Output: YAML only, no prose."
QUESTION_FRAME: [the single forcing question for this lens]
N: [number of bugs to find]
```

After lens generation: load ONLY the YAML summary of this lens (≤500 tokens).
Before next lens: provide prior lens YAML summaries only (not prose reasoning).
After all lenses complete: reconcile via post-lens reconciliation format (→ ref 28 Part 2 Step 3).

### Full Mode — 4+1 Structured Lens Frames

**Lens 1 — SRE / Operations**
```
LENS: Pattern
PERSPECTIVE: SRE who owns incident response for this system
CONTEXT_GRANTED: System architecture, deployment topology, past incidents, SLA commitments.
CONSTRAINT: "Generate findings ONLY from SRE perspective above. Do not reference prior lens
  findings. Output: YAML only, no prose."
  (Rationale: SRE sees failure patterns, not the security surface or compliance requirements)
QUESTION_FRAME: "Assume this system pages me at 3am with a vague alert. What failure mode
  caused it, and why would it be non-obvious from the alert alone?"
N: [from calibration table]
```

**Lens 2 — Security Engineer**
```
LENS: Self
PERSPECTIVE: Security engineer performing threat model review
CONTEXT_GRANTED: Attack taxonomy (OWASP, STRIDE), system interfaces, data flows, auth model.
CONSTRAINT: "Generate findings ONLY from security perspective above. Do not reference prior
  lens findings. Output: YAML only, no prose."
  (Rationale: security engineer thinks in threat actors, not ops or compliance framing)
QUESTION_FRAME: "Assume a motivated attacker with read access to this codebase.
  What attack surface did this code create or expand that wasn't there before?"
N: [from calibration table]
```

**Lens 3 — Compliance / Audit**
```
LENS: Cross-Cutting
PERSPECTIVE: External compliance officer preparing for SOC2 audit
CONTEXT_GRANTED: Applicable regulatory frameworks (from [P] declarations), L7.11 scan results,
  data flows touching PII/financial/health data.
CONSTRAINT: "Generate findings ONLY from compliance perspective above. Do not reference prior
  lens findings or technical implementation details not visible to a compliance reviewer.
  Output: YAML only, no prose."
  (Rationale: compliance officer sees obligations and audit trails, not code paths)
QUESTION_FRAME: "Assume a regulator asks me to demonstrate that this code satisfies
  [applicable framework]. What evidence would I be unable to produce?"
N: [from calibration table]
```

**Lens 4 — Product Manager**
```
LENS: Compound
PERSPECTIVE: Product manager demoing this feature to a key customer at a conference
CONTEXT_GRANTED: User journeys, product requirements, customer commitments, UI behavior.
CONSTRAINT: "Generate findings ONLY from product perspective above. Do not reference
  implementation details, security findings, compliance obligations, or ops runbooks.
  Output: YAML only, no prose."
  (Rationale: PM sees user outcomes and product promises, not internals)
QUESTION_FRAME: "Assume I'm showing this feature to 100 customers. What failure mode
  would cause visible, embarrassing, or trust-destroying behavior in that demo?"
N: [from calibration table]
```

**Lens 5 — Team B / Cross-team (ENTERPRISE mode only)**
```
LENS: ORG
PERSPECTIVE: Engineer on Team B who receives the output of this system
CONTEXT_GRANTED: Team B's documented expectations of this service, SLA commitments,
  API contracts, integration tests.
CONSTRAINT: "Generate findings ONLY from Team B's external perspective above. Do not
  reference Team A's internal implementation, prior audit findings, or any internal knowledge.
  Output: YAML only, no prose."
  (Rationale: Team B only knows what's in the contract, not what's in the code)
QUESTION_FRAME: "Assume Team A ships this without telling me anything changed.
  What assumption I made about their service would silently break my integration?"
N: [from calibration table]
```

### Standard Mode — Abbreviated Lens Frame

Standard mode applies IGP core mechanism: YAML-only output, independent generation.

```
Standard [M.AP] lens format:
  LENS: [name]
  CONSTRAINT: "Generate independently — do not reference prior lens findings. YAML output only."
  QUESTION: [one forcing question]
  RESULT_YAML:
    lens: [name]
    findings: [...H-IDs or descriptions...]
    candidates_found: [N]
```

### FAST Mode

```
FAST [M.AP]: 1 lens, 5 minutes.
Select lens based on audit profile:
  Security-heavy → Lens 2
  Ops/reliability → Lens 1
  Compliance → Lens 3
  Feature work → Lens 4
  ENTERPRISE → Lens 5

Apply context_denied: "I must not use findings from [G.H] in this pass."
QUESTION_FRAME: [lens-specific forcing question]
```

---

## Part 3 — AI False Positive Catalog ([KB])

### Five AI-specific FP types

Standard Bug Class Catalog tracks code bugs. AI FP Catalog tracks the specific
ways an AI auditor generates wrong findings — for meta-improvement of the audit process.

```
FP-AI-01: Hallucinated Path
  Description: AI cites file:line that does not exist in the actual codebase.
  Trigger: AI reasoning from architecture knowledge or training data, not actual reads.
  Detection: Fix Verification finds "file not found" or wrong line content.
  Prevention: Verify-Before-Claim gate in ABG Guard 3.
  Example: "src/auth/token.rs:142 — unwrap() on None path" where token.rs has only 80 lines.

FP-AI-02: Misread Variable Name
  Description: AI reads a similar variable name as the target, cites wrong identifier.
  Trigger: Similar identifiers in scope (user_id vs user_uuid, token vs refresh_token).
  Detection: Fix Verification re-reads — actual variable at cited line is different.
  Prevention: Read file → quote exact identifier before citing it.
  Example: Citing "session_token" but actual variable is "csrf_token" at that line.

FP-AI-03: Wrong Location (Correct Pattern)
  Description: Pattern is genuine bug, but cited at wrong file/line (sibling instance).
  Trigger: Pattern Globalization found multiple instances; AI cites wrong sibling.
  Detection: Fix Verification: fix applied, but test still fails (wrong location fixed).
  Prevention: [G.PG] must list ALL sibling locations; ADR cites the PRIMARY instance.
  Reclassification: this may be TRUE_POSITIVE at different location — re-anchor.
  Example: "bc-retry-loop found at api_client.rs:88" but actual instance is api_client.rs:201.

FP-AI-04: Training Data Contamination
  Description: AI claims finding based on "similar projects" or common patterns from
  training data, not from reading the actual codebase.
  Trigger: AI fills reasoning gaps with priors from training data.
  Detection: ABG Guard 4 check; or reviewer notes "this is a different architecture."
  Prevention: Every claim requires actual codebase evidence. "Common pattern" is T5 (intuition).
  Example: "This codebase likely uses synchronous SQLite calls which block the main thread"
  when the codebase actually uses async SQLite wrappers throughout.

FP-AI-05: Stale Context Confusion
  Description: AI uses information from earlier in context (prior file reads, prior
  cycle handoff) that is no longer accurate in the current state of the codebase.
  Trigger: Long sessions, multi-cycle context, or files that changed since last read.
  Detection: Fix Verification finds the cited code no longer matches the description.
  Prevention: For multi-cycle audits — check git_sha from handoff against current.
  Flag findings with "based on read from prior context" as MEDIUM confidence.
  Example: Citing a pattern that existed in the prior cycle's version but was already
  fixed in a commit between cycles.
```

### AI FP Catalog Template at [KB]

```yaml
ai_fp_catalog:
  - cycle: [N]
    hypothesis_id: "[H-ID]"
    fp_type: "hallucinated_path | misread_variable | wrong_location | training_contamination | stale_context"
    description: "[what was wrong]"
    detection_method: "[how it was found: Fix Verification / Stranger Review / etc.]"
    root_cause: "[why the AI made this error]"
    prevention_applied_next: "[what protocol change prevents recurrence]"
```

**Pattern analysis:** After 3+ cycles, review AI FP Catalog for recurring FP types.
If FP-AI-01 (hallucinated path) appears > 2 times → strengthen Verify-Before-Claim
gate in [P] instructions for this codebase. If FP-AI-04 appears → add codebase-specific
context declaration at [P]: "This codebase uses [X] pattern, not the common [Y] pattern."

---

## Integration Points

| Protocol | Where AI-native addenda applies |
|---|---|
| [P] | CONTEXT_BUDGET + LANGUAGE + AI_INTEGRATED declarations |
| [E] | Verify-Before-Claim before every Evidence Anchor |
| [E.HV] | MEDIUM confidence from unread files → dynamic confirmation |
| [M.AP] | Independent Generation Protocol (IGP) — YAML-only cross-lens visibility (→ ref 28 Part 2) |
| [KB] | AI FP Catalog update after every false positive discovered |
| Handoff | AI-ingestion YAML schema instead of (or alongside) prose |

---

*Reference 18 — VHEATM 11.0 | MemGPT Packer 2023 (323 cit.); SPP Wang 2023 (238+100 cit.)*
