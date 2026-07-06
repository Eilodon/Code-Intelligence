<!-- VHEATM 10.0: unchanged from 9.0 -->
# Compound Feature Decomposition — [G.CF]

> **Tikai Evidence**: PWA was declared "done" in v0.5.0 with manifest.json + icons + meta tags. Service Worker was missing. Result: app crashed when offline. The auditor saw "manifest = PWA" and stopped. Compound feature blindspot.

Run at [G.H] AFTER initial hypothesis listing, BEFORE [G.PG] Pattern Globalization.

---

## What is a Compound Feature?

A compound feature is one that REQUIRES multiple atomic components working together to deliver its promised behavior. Removing any one component degrades or breaks the feature.

Examples:
- PWA = Manifest + Service Worker + Icons + Offline page
- OAuth flow = Auth URL + Callback + Token exchange + Refresh + Revocation
- Retry logic = Try + Backoff + Max retries + Failure persistence + Dead-letter
- Pagination = Cursor/offset + Page size + Total count + Has-next signal
- File upload = Storage upload + Metadata save + Cleanup on failure + Dedup

A compound feature audit MUST decompose into atomic components and verify each.

---

## The Decomposition Protocol

For every feature that appears in scope:

### Step 1: Classify Atomic vs Compound

```
Question: "Can this feature deliver value with just ONE code path / file / component?"

YES → ATOMIC: standard audit applies, no decomposition needed
NO  → COMPOUND: proceed to Step 2
```

**Heuristic**: If the feature name appears in user-facing copy ("offline support", "OAuth login", "retry on failure"), it's almost certainly compound.

### Step 2: List Required Components

```
For the compound feature, list ALL components needed:
  [C1] Component name — purpose — exists in code? (yes/no/partial)
  [C2] ...
  [Cn] ...
```

**Important**: List what SHOULD exist for the feature to work as advertised, not just what IS in the code. The gap between expected components and actual components is where bugs live.

### Step 3: Verify Each Component

For each component [Ci]:

```
□ Does the code/file exist?
□ Is it wired up (referenced from where it should be)?
□ Does it handle its error cases?
□ Does it have its own evidence anchor?
```

Any component scoring NO → hypothesis with QBR ≥ matching the impact level.

### Step 4: Verify Interactions

Components in a compound feature don't just need to exist — they need to work together:

```
For each pair (Ci, Cj):
  □ Does Ci correctly invoke Cj at the right time?
  □ Does Ci handle Cj's failure modes?
  □ Is there a state/data flow contract between them?
```

---

## Decomposition Templates (Catalog)

### PWA (Progressive Web App)

```
Required components:
  C1: manifest.json — app metadata, icons, start_url
  C2: Service Worker — fetch interception, cache strategy
  C3: Icons — 192px, 512px, maskable, favicon
  C4: Offline fallback page — content when network unavailable
  C5: SW registration — invoked from app entry (e.g., layout component)
  C6: HTTPS — required by browsers for SW registration

Required interactions:
  manifest.json references icons → icons must exist
  Service Worker registration → must run on app mount
  Service Worker fetch strategy → must distinguish API (network-only)
    from static (cache-first) from HTML (network-first with offline fallback)
  Offline page → must be cached on SW install
```

**Common miss (Tikai)**: C2 + C4 + C5 all missing while C1 + C3 exist.

### OAuth / SSO Flow

```
Required components:
  C1: Authorize endpoint URL construction (with state param)
  C2: Callback handler (validates state, exchanges code for token)
  C3: Token storage (secure — HttpOnly cookie or server session)
  C4: Token refresh handler
  C5: Token revocation on logout
  C6: Scope validation server-side
  C7: PKCE for public clients (mobile/SPA)
  C8: State parameter CSRF protection
  C9: Redirect URI allowlist enforcement

Required interactions:
  state param generated → validated on callback
  PKCE verifier stored → matched on callback
  Token expiry → triggers refresh BEFORE expiry, not after
```

### Retry Logic

```
Required components:
  C1: Retry policy (max attempts, backoff strategy)
  C2: Idempotency key (so retries don't double-execute)
  C3: Retryable error classification (5xx, network errors retry; 4xx don't)
  C4: Backoff with jitter (not just exponential — needs random jitter)
  C5: Failure persistence (after max retries, where does the failure go?)
  C6: Dead-letter queue OR explicit failure state
  C7: Observability — log every retry attempt with attempt number

Required interactions:
  Idempotency key passed to receiver (server checks for duplicate)
  Failure after max retries → either alert or human-recoverable state
```

### Rate Limiting (counts as compound for full coverage)

```
Required components:
  C1: Limiter middleware/decorator on individual endpoints
  C2: Key strategy (per-IP, per-shop, per-user) — explicit choice
  C3: Storage backend (in-memory, Redis) — distributed if multi-instance
  C4: Response when exceeded (429 status + Retry-After header)
  C5: Whitelist/exempt mechanism (health checks, internal services)
  C6: Monitoring (rate limit hit counter as metric)

Required interactions:
  All mutation endpoints covered (see L7.1 scan)
  Key extracted AFTER auth (per-shop key requires shop_id)
  Storage backend shared across instances
```

### File Upload

```
Required components:
  C1: File received endpoint
  C2: Validation (size, type, content — not just extension)
  C3: Storage upload (cloud or disk)
  C4: Metadata DB record
  C5: Cleanup on partial failure (orphan storage)
  C6: Deduplication (hash check)
  C7: Access control (uploaded file readable by owner only)
  C8: Virus scan (for user-generated content sites)

Required interactions:
  Storage upload SUCCEEDS → DB insert
  DB insert FAILS → storage delete (compensating action)
  Dedup check happens BEFORE storage upload (save bandwidth)
```

### Pagination

```
Required components:
  C1: Page size parameter (with max cap)
  C2: Cursor or offset
  C3: Has-next signal (more pages exist?)
  C4: Total count (optional but commonly expected)
  C5: Stable ordering (must order by tie-breaking key, not just timestamp)
  C6: Cursor encoding (opaque to client to prevent tampering)

Required interactions:
  Page size capped server-side (client can't request 10000)
  Cursor stable across writes (new inserts don't shift pages already shown)
```

---

### DB Write Pipeline

```
[G.CF] Template: DB Write Pipeline
────────────────────────────────────────────────────────────────────────
Use when: any feature writes to a database (ORM or raw query).
Feeds [G.CPT]: after decomposing, run Code Path Trace (ref 31) on the
  commit boundary (C5) to verify it is actually reached.

Components:
  [C1] Transaction boundary open   — async with session.begin() [auto-commit]
                                      OR explicit session = AsyncSession() [manual]
  [C2] Input validation            — BEFORE write; not after (C3 cannot undo)
  [C3] Write operation(s)          — session.add() / session.execute() / db.add()
  [C4] flush() if needed           — makes generated IDs available for FK references
                                      only needed when C3 result is used in same txn
  [C5] COMMIT                      — await session.commit() [EXPLICIT]
                                      OR async with session.begin(): exit [CONTEXT]
                                      ABSENT → all writes ROLLED BACK silently (PY-07)
  [C6] Post-commit side effects    — notifications, cache invalidation, queue enqueue
                                      MUST execute AFTER C5, never before

Decomposition audit questions:
  "Where does the transaction open? (C1) — explicit or context manager?"
  "Where does commit happen? (C5) — if not found → CRIT (PY-07 class)"
  "Are there side effects (C6) — email, queue, cache — running BEFORE commit?
    → Risk: side effect fires, then C5 fails → external action with no DB record."
  "If C5 raises an exception, what is the state of any C6 already executed?"
  "Is there a commit inside a loop body? → Only last iteration committed, or
    separate transaction per iteration? Verify intent matches implementation."

Common failure modes → CRIT:
  F-1: C5 entirely absent — async session context closes without commit
  F-2: C6 executes before C5 — email sent before DB committed
  F-3: C5 inside loop but not after loop — partial commit on last iteration only
  F-4: C4 (flush) missing when FK reference needed in same transaction
```

---

### Background Job Pipeline

```
[G.CF] Template: Background Job Pipeline
────────────────────────────────────────────────────────────────────────
Use when: feature involves async workers (ARQ, Celery, Dramatiq, RQ).
ASYNC_WORKER profile: auto-activates this template when worker deps detected.
Feeds [G.CPT]: run Code Path Trace on J4 commit boundary — worker session
  is INDEPENDENT from API session; verify commit semantics separately.

Components:
  [J1] Enqueue                     — task dispatch with idempotency key
                                      absence of idempotency key → at-least-once
                                      causes duplicate processing
  [J2] Worker dequeue              — at-least-once delivery semantics (assume)
  [J3] Business logic execution    — core processing; may be CPU or IO bound
  [J4] DB persist                  — REQUIRES explicit commit (PY-07 mandatory here)
                                      worker session ≠ API session; verify independently
  [J5] Status update               — mark job terminal: completed / failed / partial
                                      MUST happen AFTER J4, never before
  [J6] Cleanup                     — queue ack, temp files, orphan resources
                                      MUST happen AFTER J4 commit

Divergence check — feeds [G.PG]:
  "Does this job use SAME session factory as API handlers?
    If not (worker has own sessionmaker): verify commit semantics independently.
    A correct API session does NOT imply a correct worker session."

Decomposition audit questions:
  "Is J4 present? If absent → CRIT (data processed, never persisted)"
  "Is J5 before J4? → Job marked complete but data not committed"
  "Does J3 exception handling suppress J4? → status=failed is correct,
    but partial write may be uncommitted (verify rollback vs commit on error)"
  "Does J6 cleanup run before J4 commit? → orphan resource deleted,
    order record never saved → orphaned external resource"
  "Is there an idempotency key on J1? → duplicate jobs = duplicate writes"

Common failure modes → CRIT:
  F-1: J4 absent — worker runs successfully, DB never updated (PY-07 class)
  F-2: J5 before J4 — job marked complete, commit fails → ghost completion
  F-3: J6 before J4 — resource cleaned up before data confirmed durable
  F-4: No idempotency key on J1 — retry storm creates duplicate records
```

---

## Adding to the Catalog

When you encounter a compound feature not in this catalog:

```
1. List its components in the audit output
2. Verify each
3. Add the template to [KB] Bug Class Catalog → "Compound Feature Templates"
4. Future cycles can replay the template
```

This is how the catalog grows organically without becoming exhaustive ahead of time.

---

## Output Template

```yaml
compound_feature_decomposition:
  feature: "[Feature name]"
  is_compound: true | false
  components:
    - id: C1
      name: "[Component name]"
      purpose: "[Why this component]"
      expected_location: "[Where it should be]"
      exists: true | false | partial
      evidence_anchor: "[file:line or N/A]"
      hypothesis_if_missing: "H-[ID]"
  interactions:
    - between: [C1, C2]
      contract: "[What flows between them]"
      verified: true | false
  completeness_verdict: COMPLETE | PARTIAL | INCOMPLETE
  missing_components_count: [N]
  hypotheses_generated: ["H-[ID]", ...]
```

---

## FAST Mode

```
[G.CF]-FAST: only run if a compound feature is explicitly mentioned in scope.
Use catalog template if available; otherwise list 3 most likely missing components.
Document: "Compound feature audit limited to top-3 components."
```

---

## When [G.CF] is MANDATORY

```
MANDATORY:
  □ Feature name appears in user-facing copy as a capability ("offline support")
  □ Feature involves multiple files / layers / endpoints
  □ Feature has a known catalog template
  □ Feature was recently added (this cycle or last)
  □ Self-audit mode (auditor's own compound features under +20% scrutiny)

OPTIONAL:
  □ Single-file utility additions
  □ Pure refactors (no new feature surface)
```

---

*Reference 12 — VHEATM 9.0 | Derived from Tikai F-5 PWA-incomplete failure*
