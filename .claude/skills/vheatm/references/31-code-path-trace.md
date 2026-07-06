# Code Path Trace — [G.CPT]

> **Gap closed**: VHEATM lens-based audit (L1-L7 per component) has a structural blind spot:
> failure modes that live at *interface boundaries between components* rather than inside
> any single component. [G.CF] answers "are the right components present?" — [G.CPT]
> answers "does data flow correctly *through* those components?" These are orthogonal
> questions; neither subsumes the other.
>
> **Evidence basis**: Tikai field evidence — async session commit failures in worker
> architecture caught by baseline audit ("follow the data") but missed by VHEATM audit
> (lens-based, stopped at "component exists and is correct"). T4 field evidence; upgrade
> path: 3 codebases confirmed → T2.
>
> **Design principle**: [G.CPT] is a PATH lens, not a CATEGORY lens. It does not ask
> "what class of bug is this?" — it asks "can a unit of data reach its intended terminal
> state from its entry point, given this specific execution path?"

---

## Relationship to Other [G] Protocols

| Protocol | Unit | Asks | Finds |
|---|---|---|---|
| [G.H] | Layer (L1-L7) | "What bugs exist in this component?" | Intra-component defects |
| [G.CF] | Feature component | "Are all required components present?" | Missing components |
| **[G.CPT]** | **Interface boundary** | **"Does data flow correctly through components?"** | **Broken interface contracts, missing commit** |
| [G.PG] | Bug pattern | "Does this bug exist elsewhere?" | Pattern siblings |
| [G.T] L-T.3 | Session lifecycle | "Does commit happen?" | Session drift (async) |

**Sequential relationship**: [G.CPT] typically runs AFTER [G.CF] (decomposition reveals
the path) and feeds [G.PG] Divergent Implementation (Step 4 of path trace surfaces
paths doing same operation with different commit behavior).

---

## Trigger Conditions

Run [G.CPT] when ANY of the following:

```
MANDATORY:
  □ [G.CF] decomposed a write chain with ≥3 components and ≥1 persistence component
    (DB write, queue enqueue, file write, cache set)
  □ Financial / billing / fee calculation path detected in scope
  □ ASYNC_WORKER profile is active (arq/celery/dramatiq in deps — see ref 22)
  □ L-T.3 Extension fired (async session context manager found)

RECOMMENDED:
  □ Cross-boundary data transformation chain (data changes shape ≥2 times before
    reaching storage)
  □ Multiple code paths write to the same resource (API path + worker path)
  □ Feature involves compensating actions (write A, then write B, rollback A if B fails)
```

---

## Interface Boundary Taxonomy

The core vocabulary of [G.CPT]. Every boundary in a traced path gets ONE label.

```
COMMIT_CONTEXT    — async with session.begin(): → auto-commits on clean __aexit__
                    SAFE. Note: exception in context → rollback, not commit.

COMMIT_EXPLICIT   — await session.commit() called directly before context exit
                    SAFE. Requires verification that execution actually reaches this line
                    (not inside try/except that swallows exceptions before commit).

COMMIT_IMPLICIT   — framework handles commit transparently (e.g., Django ORM with
                    ATOMIC_REQUESTS, some FastAPI dependency patterns with commit in
                    finally block). CONDITIONALLY SAFE — must verify: which framework,
                    which version, which configuration. Never assume implicit.

COMMIT_ABSENT     — context manager exits without commit() and without begin()
                    → __aexit__ calls session.close() ONLY → ALL writes ROLLED BACK.
                    CRIT finding. No exception raised. Symptom: writes silently succeed,
                    data absent on next read.

COMMIT_CONDITIONAL — commit() inside an if/try branch without guaranteed execution
                    → if condition is false OR exception raised before commit: data lost.
                    HIGH finding if the non-commit path is reachable in normal operation.

PASS_THROUGH      — data is passed unchanged from one function to the next
                    SAFE (annotate: confirms data identity is preserved across boundary).

DROP              — a field is hardcoded, defaulted, or overridden at this boundary
                    → the original value is not forwarded.
                    HIGH finding if the dropped field carries semantically meaningful data.
                    Tag PY-09 if this is a dataclasses.replace() result not captured.

TRANSFORM         — data shape changes at this boundary (rename, restructure, calculate)
                    ANNOTATE: verify the transformation is lossless for all fields in scope.
                    Not a finding by itself — requires verification that no field is dropped.

ENQUEUE           — data is sent to a queue or background worker at this boundary
                    ANNOTATE: at-least-once delivery semantics (assume unless proven exactly-once).
                    Check: idempotency key present? Duplicate processing handled?

ENQUEUE_THEN_COMMIT — enqueue happens BEFORE DB commit
                    HIGH finding: if commit fails after enqueue, external action is already
                    dispatched with no DB record. Order should be COMMIT_EXPLICIT then ENQUEUE.

EXTERNAL_CALL     — data leaves the system boundary (HTTP, email, SMS, webhook)
                    ANNOTATE: once sent, cannot be recalled. Verify ordering with commit.
```

---

## Protocol — 5 Steps

### Step 1: SELECT PATH

Name the path. One trace per path — multiple paths require multiple traces.

```
path_name:  "user does [action] → [data] committed to [storage]"
entry:      [function signature + input type]
             e.g., "process_import(platform, file) → ImportRecord"
terminal:   [expected committed state]
             e.g., "ImportRecord row with all fields present in PostgreSQL"
trigger:    [what caused [G.CPT] to fire for this path]
```

**Choosing paths**: Start with the highest-risk path. Prioritizing criteria:
- Financial data writes (fee, price, billing) — always first
- Data that is read by another system or user facing — second
- Data that triggers irreversible external actions (email, payment) — third

---

### Step 2: TRACE BOUNDARIES

Walk the execution path from entry to terminal state. For each function call,
context manager entry/exit, and async await:

```
For each boundary B_n:
  boundary_type: [taxonomy label from above]
  location: [file:line]
  annotation: [brief note on what happens here]
  finding: [if COMMIT_ABSENT | DROP | ENQUEUE_THEN_COMMIT → immediate hypothesis]
```

**STOP RULE**: If COMMIT_ABSENT is found at any boundary, generate CRIT hypothesis
immediately. Continue tracing to check for additional COMMIT_ABSENT or DROP instances —
but the path already fails; do not mark it COMMITTED.

**FORK RULE**: If the path forks (if/else, try/except), trace the PESSIMISTIC branch
(the one less likely to commit). If pessimistic branch reaches COMMITTED, optimistic
also does. If pessimistic is UNCOMMITTED, flag regardless of optimistic.

---

### Step 3: TERMINAL STATE CHECK

After tracing all boundaries:

```
terminal_state: COMMITTED | UNCOMMITTED | UNKNOWN

COMMITTED:   path reaches terminal with a COMMIT_CONTEXT, COMMIT_EXPLICIT, or
             verified COMMIT_IMPLICIT boundary. No COMMIT_ABSENT on any fork.

UNCOMMITTED: any reachable execution fork reaches terminal with COMMIT_ABSENT or
             COMMIT_CONDITIONAL (unguaranteed). → CRIT finding.

UNKNOWN:     commit boundary is COMMIT_IMPLICIT but not verified (framework/version
             not confirmed). → HIGH finding: "commit semantics unverified — treat as
             UNCOMMITTED until confirmed."
```

If UNCOMMITTED or UNKNOWN → generate hypothesis before Step 4.

---

### Step 4: DIVERGENCE SWEEP

After terminal state check, even if COMMITTED:

```
"Are there other code paths that perform the same write operation on the same resource?"

Search:
  1. Name the resource: "fee_configs table write", "ImportRecord INSERT", etc.
  2. Search for all callsites: grep -rn "session.add\|INSERT INTO fee" app/
  3. For each alternate path:
     □ Apply same Step 2 boundary trace (abbreviated — focus on commit boundary only)
     □ Compare terminal state with primary path
     □ Divergence: primary=COMMITTED, alternate=UNCOMMITTED → HIGH: "inconsistent
       commit semantics across paths for same resource"

Feed divergences to [G.PG] Divergent Implementation (ref 08) as DIVERGENT-BUG.
```

This step is what connects [G.CPT] to [G.PG]. Standard [G.PG] starts from a confirmed
bug. [G.CPT] Step 4 starts from a confirmed-correct path and searches for divergent
paths — finding bugs by contrast rather than by pattern match.

---

### Step 5: OUTPUT

```yaml
code_path_trace:
  path_name: "[string]"
  entry: "[function:line]"
  terminal: "[expected committed state]"
  trigger: "[which trigger condition fired]"

  boundaries:
    - id: B1
      location: "[file:line]"
      type: "PASS_THROUGH | DROP | COMMIT_EXPLICIT | ..."  # taxonomy label
      annotation: "[what happens]"
      finding: null | "[CRIT/HIGH: description]"

  terminal_state: "COMMITTED | UNCOMMITTED | UNKNOWN"

  findings:
    - id: CPT-[N]
      severity: CRIT | HIGH | MEDIUM
      boundary: B[N]
      description: "[what is wrong]"
      hypothesis_id: H-[N]  # feeds [G.H]

  divergence_sweep:
    alternate_paths_checked: N
    divergences_found: N
    gpg_feeds:
      - resource: "[resource name]"
        divergent_location: "[file:line]"
        primary_terminal: "COMMITTED"
        divergent_terminal: "UNCOMMITTED"
        hypothesis_id: H-[N]
```

---

## Hard Gate: HG-CPT

```
HG-CPT: All triggered Code Path Traces completed; every traced path has explicit
        terminal_state declaration; UNCOMMITTED or UNKNOWN paths have corresponding
        CRIT/HIGH hypotheses in [G.H].

PASS: terminal_state = COMMITTED for all traced paths
      OR: terminal_state = UNCOMMITTED/UNKNOWN AND hypothesis generated with id
FAIL: any path traced without terminal_state declaration
      OR: UNCOMMITTED path without a corresponding hypothesis (silent miss)

Layer: Meta-Defense (audits the audit's own path coverage)
Mode: All modes when trigger conditions met; skip only if NO trigger condition fires
```

---

## Worked Example — Async Worker Write Path

```
path_name: "process_import() → ImportRecord committed to PostgreSQL"
entry:     "process_import(platform, file_bytes) in app/tasks/worker.py"
terminal:  "ImportRecord row with status=completed in import_records table"

Trace:

B1: async with session: (worker.py:47)
    type: ??? — must check: is this session.begin() or plain AsyncSession()?
    grep result: "async with async_session() as session:" — NOT begin()
    type: COMMIT_ABSENT (pending — check if commit() called before exit)

B2: record = ImportRecord(...)  (worker.py:52)
    type: PASS_THROUGH
    annotation: object created, not yet persisted

B3: session.add(record)  (worker.py:53)
    type: PASS_THROUGH
    annotation: object staged in session, not yet flushed or committed

B4: result = apply_fee_config(record)  (worker.py:55)
    type: TRANSFORM
    annotation: check for DROP — does apply_fee_config return new object?
    grep: "return dataclasses.replace(row, ...)" — YES, new object returned
    annotation: confirm result is captured at callsite
    grep: "record = apply_fee_config(record)" — CAPTURED. PASS_THROUGH confirmed.

B5: async with session: __aexit__ (worker.py: implicit at block end)
    type: COMMIT_ABSENT — confirmed: no session.commit() found before exit,
          no session.begin() context used
    finding: CRIT — "all writes in async with session: block rolled back on exit"

terminal_state: UNCOMMITTED
finding CPT-1: CRIT — "worker.py process_import() has no commit boundary.
  All writes (ImportRecord, fee calculation) silently rolled back on session close.
  async with session: without begin() or explicit commit() → PY-07 class."

Divergence sweep:
  Resource: "import_records table write"
  Search: grep -rn "session.add.*Import\|INSERT.*import" app/
  Found: app/api/routes/imports.py:88 — uses get_db() FastAPI dependency
  Trace B1 for API path: get_db() uses "async with session.begin(): yield session"
  API terminal_state: COMMITTED (COMMIT_CONTEXT via begin())
  Divergence: worker=UNCOMMITTED, API=COMMITTED → same resource, different semantics
  feeds [G.PG] Divergent Implementation
```

---

## FAST Mode

```
[G.CPT]-FAST: abbreviated trace — commit boundary check ONLY.

For each write chain detected:
  1. Locate the DB session context manager
  2. Check: session.begin()? OR explicit commit()? OR neither?
  3. If neither → CRIT hypothesis immediately (skip full boundary trace)
  4. Skip Steps 4 (Divergence Sweep) — defer to next Standard/Full cycle

Document: "CPT-FAST: commit boundary check only. Full path trace deferred."
```

---

## Integration with ASYNC_WORKER Profile

When ASYNC_WORKER=YES (ref 22):
- [G.CPT] is MANDATORY (not triggered by threshold)
- Apply to ALL detected write chains — not just those with ≥3 components
- Divergence sweep (Step 4) is MANDATORY — the API/worker split guarantees
  divergent session factory; verify both sides explicitly
- [E.IJ] mandatory for all CPT-1 class findings

---

*Reference 31 — VHEATM 16.1 | [G.CPT] Code Path Trace*
*Derived from: Tikai field evidence (async session commit failure, worker vs API session*
*divergence); Doc 1/Doc 2 analysis — lens-based vs path-based audit failure mode study.*
*Design: [G.CPT] closes the path-tracing gap while [G.CF] closes the component-presence gap.*
