<!-- VHEATM 10.0: unchanged from 9.0 -->
# Pattern Globalization — [G.PG]

> **Tikai Evidence**: After 2 rounds of VHEATM 8.0 audit, 7 mutation endpoints still lacked rate limiting because the first found instance (recompute) was fixed in isolation. Same pattern bug class was repeated 8 times in the same codebase.

Run AFTER each confirmed bug in [G.H], BEFORE issuing ADR.

---

## The Core Question

**"Is this bug an instance of a pattern that exists elsewhere?"**

Every bug must be classified BEFORE fixing:
- **Singleton bug**: This specific bug exists in exactly this one place. Fixing the instance closes the class.
- **Pattern bug**: This bug class can exist anywhere meeting certain conditions. Fixing one instance does NOT close the class.

**Rule**: If you cannot prove it's a singleton, treat it as a pattern.

---

## Protocol — Required Steps

### Step 1: Extract the Pattern Signature

For the confirmed bug, write down:

```
Pattern signature:
  Trigger condition: [What state/code shape creates this bug?]
  Symptom: [How does it manifest?]
  Anti-pattern code: [The specific code pattern that creates the bug]
  Correct pattern: [The fixed shape]
```

**Example from Tikai F-2:**
```
Pattern signature:
  Trigger condition: Router that needs to enqueue ARQ jobs
  Symptom: Creates new Redis connection per request (resource leak)
  Anti-pattern: `await ArqRedis.from_url(get_settings().redis_url)` inline
  Correct: `from app.core.arq_pool import get_arq_pool; await get_arq_pool()`
```

### Step 2: Define Search Scope

```
Search scope:
  Layer: [Same layer? Cross-layer? Cross-file?]
  Search method: [grep regex / AST traversal / mental model / both]
  Search command: [Exact command you will run]
```

**Example:**
```
Layer: All routers + tasks
Search method: grep regex
Search command: grep -rn "ArqRedis.from_url\|RedisSettings.from_dsn" app/
```

### Step 3: Execute the Search

```
□ Run the search
□ Document ALL instances found (not just the first)
□ For each instance, classify:
    - SAME BUG: Apply same fix
    - VARIANT: Note the difference, may need different fix
    - FALSE POSITIVE: Code matches pattern but context is intentional, document why
```

### Step 4: Cascade Decision

For each non-false-positive instance:

```
Instance: [file:line]
Classification: SAME / VARIANT
Action:
  - If SAME → add to current ADR's Fix Cascade list
  - If VARIANT → create new hypothesis in [G.H] for next [E] verification
```

### Step 5: Document Pattern Closure

After fixing all instances:

```
Pattern closure verification:
  □ Re-run search command → confirm no instances remaining
  □ Add to Bug Class Catalog ([KB]):
      - Pattern signature
      - Search command (so future cycles can replay)
      - Date pattern was closed
      - Instances closed: [count]
```

---

## When [G.PG] is MANDATORY vs OPTIONAL

**MANDATORY (always run):**
- CODE/LIVE context mode
- Any bug touching: routing, auth, validation, error handling, resource lifecycle
- Self-audit mode (audit of own prior work)
- After a "compound feature" finding (paired with [G.CF])

**OPTIONAL (case-by-case):**
- DESIGN mode — no code to grep
- Genuinely unique bugs (e.g., single-line logic error in unique function)
- FAST mode — mental grep only acceptable

---

## Anti-Patterns

🚫 **"It looked unique so I didn't search"** — Default to searching. Cost is low, miss cost is high.

🚫 **"I searched but found nothing similar"** — Document the search command AND the empty result. "I checked" without proof is not verification.

🚫 **"Found 3 instances but only fixed the one mentioned in the audit"** — Either fix all or document why others are deferred (with explicit Defer-Debt-ID).

🚫 **"Pattern signature too narrow"** — If your search only matches the exact bug you found, your signature is too tight. Generalize the pattern.

---

## Pattern Globalization Output Template

```yaml
pattern_globalization:
  hypothesis_id: H-[ID]
  pattern_signature:
    trigger: "[...]"
    anti_pattern: "[code shape]"
    correct_pattern: "[code shape]"
  search:
    scope: "[layers/files searched]"
    command: "[exact grep/AST/manual command]"
    result_count: [N]
  instances_found:
    - location: "[file:line]"
      classification: SAME | VARIANT | FALSE_POSITIVE
      action: "[fix in this ADR / new hypothesis / documented exemption]"
  closure:
    all_instances_addressed: true | false
    deferred_items: ["Debt-ID-..." if any]
    catalog_entry_added: true
```

---

## FAST Mode Adaptation

In FAST mode, [G.PG] reduces to:

```
For each finding, mental check:
  □ "Does this code shape exist in 2+ other places I should check?"
  □ If YES → quick visual scan (no formal grep) of 2-3 likely sibling locations
  □ If found → note as VARIANT, defer to backlog (don't expand FAST scope)
```

Document: "PG-FAST: visual check only. Full grep deferred to next Standard/Full cycle."

---

## Calibration: When to escalate Pattern Globalization

If across 3 cycles you find that:
- Pattern siblings discovered ≥ 50% of confirmed bugs

Then auto-promote [G.PG] from Step in [G] to its own Hard Gate `HG-PG` (already done in 9.0).

If across 5 cycles:
- Pattern siblings discovered ≥ 70% of confirmed bugs

Then this codebase has structural homogeneity issues — escalate to architectural review at [V].

---

## Extension: Divergent Implementation Pattern

> **Rationale**: Standard [G.PG] is horizontal — "find other locations with the same bug."
> This extension is vertical — "find other locations doing the same *operation*, verify
> all locations implement the same correctness invariants."
> The distinction: horizontal search starts from a bug; vertical search starts from a
> *correct* implementation and verifies consistency across all callsites.

**Trigger**: When a confirmed implementation is CORRECT at one callsite, ask:
> "Are there other code paths that perform the same resource operation?"

If yes → apply Divergent Implementation Protocol:

```
Step 1: Name the operation and resource
  operation: e.g. "FeeConfig lookup", "DB session commit", "user write"
  resource:  e.g. "fee_configs table", "PostgreSQL write", "Redis key"

Step 2: Search ALL callsites for this operation
  Search method: grep regex or AST traversal (same as Step 2 in standard [G.PG])
  Search command: [exact grep for the operation pattern]

Step 3: For each callsite pair (correct_A, candidate_B):
  Verify candidate_B has the same correctness properties as correct_A:
    □ Same commit/transaction boundary?
    □ Same validation order (input → write vs write → validate)?
    □ Same error handling (rollback on failure, not partial commit)?
    □ Same scope (platform-aware, tenant-filtered, etc.)?

Step 4: Classify divergences
  CONSISTENT     — B implements same invariants as A (document, no action)
  VARIANT        — B intentionally differs for good reason (document justification)
  DIVERGENT-BUG  — B is missing an invariant A has → HIGH hypothesis (L2 or L3)

Output: add DIVERGENT-BUG findings to [G.H] hypothesis list.
```

**Examples that standard [G.PG] would miss**:

```
Example 1 — Session commit:
  Correct:  API endpoint uses get_db() with auto-commit via FastAPI dependency
  Divergent: ARQ worker creates session manually, no commit() call
  Operation: "PostgreSQL write"
  Bug: worker session silently rolls back on context exit (PY-07)
  Standard [G.PG] misses this: no "bug pattern" to grep for in the API path

Example 2 — FeeConfig lookup:
  Correct:  process_import() queries FeeConfig with platform filter (platform-aware)
  Divergent: recompute_insight() queries FeeConfig without platform filter
  Operation: "FeeConfig table read"
  Bug: different tenants receive same fee configuration on recompute
  Standard [G.PG] misses this: the "bug" is absence of a filter, not presence
    of an incorrect pattern
```

**Interaction with [G.CPT]**: Divergent Implementation Step 4 is a natural entry point
for [G.CPT] Code Path Trace (ref 31). When a DIVERGENT-BUG involves a commit boundary,
run [G.CPT] on the divergent path to confirm terminal state is UNCOMMITTED.

---

*Reference 08 — VHEATM 9.0 | Derived from Tikai F-2, F-6 failure modes*
