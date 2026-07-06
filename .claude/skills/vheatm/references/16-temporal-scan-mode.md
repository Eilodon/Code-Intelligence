# Temporal Scan Mode — [G.T] (v11.0)

> **Research Basis:**
> [ProveNFix (Song et al., 2024, 10 citations)](https://consensus.app/papers/details/7b51c53ea5c85be38fe401eb337e6ee6/):
> first compositional static analyzer for temporal properties (typestate); detects
> 515 vulnerabilities across 1M+ LOC, finds 72.2% more true alarms than Infer.
> Supports: memory usage bugs, unchecked return values, resource leaks.
> [Chen et al. (2023, 9 citations)](https://consensus.app/papers/details/befcfb360aec578a8354a3506d40fdaa/):
> comprehensive empirical study of 150 Use-After-Free bugs — "the intricacy rooted
> in the **temporal nature** of UaF vulnerabilities makes it challenging." Categorizes
> 11 root-cause patterns; many require temporal state tracking, not snapshot analysis.
> [Lipp et al. 2022 (112 citations)]: static analyzers miss 47-80% of real vulnerabilities;
> temporal state bugs are among least-covered categories.
>
> **Core insight:** Every VHEATM layer (L1-L7) is a *snapshot* question —
> "what is wrong with the code RIGHT NOW?" Temporal bugs only manifest over time.
> [G.T] is not a new layer — it is a **temporal re-read** of existing layers,
> asking the time-dimension version of each layer's question.

---

## When to Activate

```
REQUIRED:
  □ Any persistent state exists (DB, AsyncStorage, SecureStore, file system, caches)
  □ CONTEXT_MODE = LIVE or ENTERPRISE
  □ Resource management code in scope (connections, file handles, locks, buffers)
  □ Date/time logic in scope (any timestamp, deadline, TTL, counter with reset)

OPTIONAL (recommended):
  □ CONTEXT_MODE = CODE with state management
  □ Quota or rate limiting logic in scope

NOT APPLICABLE:
  □ Pure stateless functions, no persistent side effects
  □ CONTEXT_MODE = DESIGN (conceptual only, no runtime)
```

FAST mode: ask L-T.1 and L-T.4 only (accumulation + quota). 2 minutes max.

---

## The 5 Temporal Questions

Run AFTER L1-L7 hypothesis generation. For each question, generate a hypothesis
if the answer reveals unbounded, drifting, or triggered state.

### L-T.1 — Accumulation

```
"What state grows unboundedly? Where is it drained or capped?"

Ask for every persistent store in scope:
  □ What writes to this store? (reads, creates, appends)
  □ What removes from it? (deletes, evictions, TTL expiry)
  □ Is the removal mechanism: (a) automatic, (b) triggered, (c) absent?
  □ If triggered — what triggers it? Can the trigger fail to fire?
  □ What is the worst-case growth rate? When does it become a problem?

Classic examples: log files without rotation, caches without eviction,
session tables without cleanup, event queues without consumers, audit trails
growing without archive/delete.

Temporal hypothesis if: removal mechanism absent or unreliable.
Layer assignment: L2 (state management) or L7.5 (resource lifecycle).
```

---

### L-T.2 — Date/Time Boundary

```
"What assumptions does the code make about time? What happens at
midnight, DST change, year rollover, leap second, or epoch boundary?"

Ask for every timestamp, deadline, counter-with-reset, or scheduled job:
  □ What timezone does the code assume? Is it consistent across all calls?
  □ Does any counter reset at a time boundary? How is the reset implemented?
    (cron job? DB trigger? Application logic on first read after midnight?)
  □ Does the reset logic handle the case where no read occurs exactly at midnight?
    (e.g., if reset fires only "on first read after midnight" and the app is idle,
    does the old counter persist?)
  □ What happens during DST transition (23h or 25h days)?
  □ What happens when the server clock drifts or is corrected?
  □ Any date arithmetic that assumes consistent day lengths?

Canonical failure: "daily limit" counter that checks `today's date` at read time,
but resets only when a user action fires. If no action fires at midnight,
the old date persists and the user gets blocked until their next action.
(cf. Aletheia BUG-01 — temporal bug class, caught by this probe, missed
by all 5 prior VHEATM snapshot cycles.)

Temporal hypothesis if: reset depends on user-triggered or externally-triggered
event that may not fire at the boundary.
Layer assignment: L2 (state management).
```

---

### L-T.3 — Session State Drift

```
"Does state carry over between sessions in ways that could corrupt future behavior?"

Ask:
  □ What state is read from persistent storage at session start?
  □ What assumptions does the code make about the freshness of that state?
  □ Can the state become stale between writes? (network partition, concurrent writers,
    app restart without flush)
  □ Is there a WAL / checkpoint lag? (SQLite WAL mode: reader may see stale data
    if checkpoint has not fired since last write)
  □ Is there any caching layer between persistent storage and application state?
    What invalidates the cache?

Temporal hypothesis if: state can be read that is stale by design (WAL lag,
cache without invalidation, no version/timestamp check on read).
Layer assignment: L2 (state consistency) or L3 (cross-layer integration).
```

**L-T.3 Extension — Async Session Lifecycle** *(activate when async ORM detected)*

```
"Does __aexit__ commit, or only close?"

This is application-level temporal state drift: the session *believes* it wrote data;
the DB never received the commit. Symptom: writes silently succeed (no exception),
data is absent on next read. Surfaces as "data disappears randomly."

For each async context manager wrapping a DB session:
  □ Pattern: async with session.begin(): → auto-commits on clean exit — SAFE
  □ Pattern: async with AsyncSession() as session: + await session.commit() before
    exit → explicit commit — SAFE
  □ Pattern: async with AsyncSession() as session: without session.begin() and without
    explicit commit() → __aexit__ calls session.close() ONLY → ALL writes ROLLED BACK

Ask:
  □ For every async with [Session | sessionmaker() | get_db()]: block:
    → Does __aexit__ commit (session.begin() context) or only close?
    → If only close: is there an explicit await session.commit() before exit?
    → If neither: every write in this block is silently rolled back.
  □ Are there multiple session factories (API + worker)? Each must be verified
    independently — correct commit semantics in API does NOT imply correct
    commit semantics in worker (different instantiation path, different lifecycle).
  □ Is autoflush=False declared? This means even flush() is suppressed between
    operations — verify flush() calls where FK references are needed.

Temporal hypothesis if: async context manager found without begin() or commit().
Layer: L2 (state consistency) — CRIT severity.
Assign PY-07 tag in hypothesis. Run [G.CPT] (ref 31) on this path.
```

---

### L-T.4 — Quota Exhaustion

```
"What counters or quotas exist? What resets them? What happens if reset never fires?"

Ask for every counter, rate limiter, or quota variable:
  □ What increments this counter? What decrements or resets it?
  □ Is the reset: time-based (cron), event-based (user action), threshold-based?
  □ What is the maximum value before quota breach? What happens at breach?
  □ Can the counter increment faster than the reset fires?
    (burst traffic, replay attack, race condition between increment and reset)
  □ What happens if the reset job fails silently?
    (process crash, network timeout, DB lock during reset transaction)

Temporal hypothesis if: reset can fail, be skipped, or fall behind increments.
Layer assignment: L7.5 (resource lifecycle) or L1 (contract: quota is a promise).
```

---

### L-T.5 — Storage / Log Growth

```
"What writes to persistent storage? Is there a rotation / eviction strategy?"

Ask:
  □ Every write path: log entries, event records, analytics, temp files, cached responses.
  □ Is there a retention policy? Is it enforced in code or only in documentation?
  □ Is there a maximum size check before write? What happens when max is reached?
  □ Are log rotation / storage cleanup jobs tested? Is their failure detectable?
  □ What is the estimated storage growth rate at 10× current load?

Temporal hypothesis if: unbounded write path without enforced retention.
Layer assignment: L7.5 (resource lifecycle).
```

---

## Temporal Hypothesis Format

```yaml
temporal_hypothesis:
  id: H-T-[N]
  layer: L2 | L7.5 | L3 | L1  (temporal bugs map to existing layers)
  temporal_sub_class: accumulation | date-boundary | session-drift | quota | storage
  description: "[What accumulates / drifts / triggers]"
  trigger_condition: "[What needs to happen for this to manifest]"
  trigger_probability: CERTAIN | HIGH | MEDIUM | LOW | RARE
    # CERTAIN: fires on every session/day
    # HIGH: fires under normal usage within weeks
    # MEDIUM: fires under specific conditions (high load, boundary crossing)
    # LOW: fires at edge case (leap second, exact timing race)
    # RARE: theoretical; requires specific attack or cosmic timing
  first_manifestation_estimate: "[when a user would first notice]"
  evidence_anchor: "[file:line of accumulation source, reset mechanism, or missing reset]"
  qbr_note: "Use trigger_probability to calibrate blast_radius input in QBR"
```

**QBR calibration for temporal bugs:**
- trigger_probability CERTAIN/HIGH → use actual blast_radius
- trigger_probability MEDIUM → blast_radius × 0.75 (still real, just slower)
- trigger_probability LOW/RARE → consider REQUIRED not MANDATORY unless data_integrity_risk = 3

---

## Typestate Analysis Lens (for resource-heavy code)

Inspired by ProveNFix's typestate approach: track the lifecycle states of resources.

For any resource (file handle, DB connection, lock, socket, auth token):
```
Valid states: [UNINITIALIZED → OPENED → IN_USE → CLOSED/RELEASED]
Questions:
  □ Can IN_USE happen before OPENED? (use-before-init)
  □ Can CLOSED happen twice? (double-free / double-close)
  □ Can the resource stay IN_USE indefinitely? (resource leak)
  □ Can OPENED happen but CLOSED never follow? (leak under exception path)
  □ Is there a path where CLOSED is assumed but the resource is still IN_USE?
    (use-after-free / use-after-close)
```

Each typestate violation → hypothesis with layer = L2 or L5 (external dep lifecycle).
Most resource leaks are temporal accumulation bugs (L-T.1 + typestate).

---

## Integration with Other Phases

**[G.PG] Pattern Globalization after temporal finding:**
Temporal bugs often appear in patterns. After confirming any temporal hypothesis:
- Code grep: search for same accumulation pattern, same reset mechanism, same counter type
- Structural: "Are there other modules with similar state machines?"

**[E.HV] interaction:**
Temporal bugs are notoriously hard to verify statically.
For L-T.1, L-T.4, L-T.5: static confidence typically MEDIUM → [E.HV] Step 2 recommended.
Design a test that runs N cycles or crosses a time boundary.

**[T.FV] Fix Verification for temporal bugs:**
Fix Anchor must include:
- The reset mechanism code change
- The test that crosses the temporal boundary (date mock, N-cycle loop)
"Fixed in code" without a temporal boundary test = FV incomplete.

---

## Anti-Patterns

🚫 **"L-T not applicable because we have tests"** — Most test suites run single-session,
single-day scenarios. Temporal bugs fail at second session, midnight, or after 1000 writes.

🚫 **"Reset is documented in runbook"** — "Documented ≠ Verified" (Tikai Principle 2).
If reset is not in code, it doesn't run automatically.

🚫 **"RARE trigger probability = skip"** — A RARE temporal bug in a security-critical
path (auth token reuse after "invalidation") is still MANDATORY. Use QBR, not probability
alone, to determine priority.

🚫 **"SQLite WAL is fine"** — WAL mode has checkpoint lag by design. If your code
reads immediately after write and expects strong consistency, this is L-T.3.

---

*Reference 16 — VHEATM 11.0 | ProveNFix Song 2024; Chen 2023 UaF temporal nature;
Lipp 2022 static analysis coverage gaps*
