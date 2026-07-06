# L4 Concurrency Sublayers — Canonical Definitions (v13.0)

> **Gap closed**: VHEATM v11-v12 expanded L4 to 6 sublayers but never defined them
> formally in one place. They appeared in Language Profiles (ref 19) by implication,
> but the canonical source was missing. This file IS that source.
>
> **Evidence basis:**
> ProveNFix (Song et al., 2024): typestate analysis catches 515 vulnerabilities across
> 1M+ LOC — temporal/concurrency bugs are the least-covered by standard analysis.
> Chen et al. (2023): 150 Use-After-Free bugs — root cause in temporal state tracking.
> NIST NVD data: L4.1 (data race) + L4.5 (event-race) account for ~23% of CVEs in
> concurrent software systems (based on race condition + TOCTOU CVE categories).

Load this file at [G.H] before generating L4 hypotheses.

---

## L4 Sublayer Table

| Sublayer | Name | Core Question |
|---|---|---|
| **L4.1** | Data Race | "Can two threads/coroutines read/write shared mutable state without synchronization?" |
| **L4.2** | TOCTOU (Time-of-Check-to-Time-of-Use) | "Is there a window between checking a condition and using its result where the condition can change?" |
| **L4.3** | Initialization Race | "Can initialization of shared state be observed in a partially-complete form?" |
| **L4.4** | Order Violation | "Does the code assume a particular ordering of operations that is not guaranteed?" |
| **L4.5** | Event-Race (Async Lifecycle) | "Can an async callback or event handler fire after the resource it references has been released?" |
| **L4.6** | Livelock / Deadlock | "Can two or more operations block indefinitely waiting for each other?" |

---

## L4.1 — Data Race

**Definition:** Two concurrent threads/coroutines access shared mutable state where
at least one access is a write, and there is no synchronization mechanism.

```
Hypothesis trigger questions:
  □ Is there any global or module-level mutable variable that can be written by
    more than one coroutine/thread?
  □ Are there any reference-counted objects modified without atomic operations?
  □ Does any singleton initialization happen without a lock?
  □ In Python: does the GIL protect this? (Only single bytecodes — not multi-step ops)
  □ In Rust: is there any manual Send/Sync impl that bypasses ownership checks?

Classic examples:
  - Global counter incremented without atomic compare-and-swap
  - Singleton: `if instance is None: instance = Class()` (two threads → two instances)
  - Module-scope list.append() in SSR context (shared across requests)

Layer assignment: L4.1
QBR typical: security_risk depends on whether race enables auth bypass or data corruption
```

---

## L4.2 — TOCTOU (Time-of-Check to Time-of-Use)

**Definition:** A condition is checked (CHECK), then some action is taken based on
that check (USE), but the state could change between CHECK and USE.

```
Hypothesis trigger questions:
  □ Is there any `if exists: use` pattern where another actor could delete/modify
    between the check and the use?
  □ File operations: `if os.path.exists(f): open(f)` — file could be deleted
  □ Auth checks: `if user.has_permission('X'): perform_action('X')` — permission
    could be revoked between check and use in long-running operations
  □ Database: `if balance >= amount: deduct(amount)` without transaction isolation
  □ API: `if resource.state == 'available': allocate(resource)` without optimistic locking

Classic examples:
  - Race condition in inventory: `if stock > 0: decrement()` with concurrent requests
  - Security bypass: check authentication, network latency, perform action
    (auth can expire between check and action)

Layer assignment: L4.2
QBR typical: security_risk=2-3 (TOCTOU in auth context), data_integrity_risk=2-3
```

---

## L4.3 — Initialization Race

**Definition:** Shared state (singleton, static, global) is observed in a partially
initialized form because initialization is not atomic.

```
Hypothesis trigger questions:
  □ Is there any singleton pattern that uses double-checked locking without
    memory fence/barrier?
  □ Is there any static or global initialized lazily that other code can reference
    before initialization completes?
  □ OnceLock / Lazy patterns in Rust: can the initializer panic? (poisons the lock forever)
  □ Module-level initialization with side effects that other modules can import before
    the initialization completes?

Classic examples (from Language Profile R-03):
  - `OnceLock::get_or_init(|| { expensive_init() })` where `expensive_init` can panic
  - C++ static initialization order fiasco (A depends on B's static, B initializes after A)
  - Python module with mutable globals set conditionally during import

Layer assignment: L4.3
QBR typical: data_integrity_risk=2-3 (corrupted singleton state propagates everywhere)
```

---

## L4.4 — Order Violation

**Definition:** Code assumes a particular ordering of operations or data that is
not guaranteed by the runtime.

```
Hypothesis trigger questions:
  □ Does any code assume HashMap/dict iteration order? (Not guaranteed in Rust/Python 3.7<)
  □ Does any code assume arrival order of messages from a message queue?
  □ Does any code assume two threads complete in a particular order without synchronization?
  □ Does any code depend on object finalization/destruction order?
  □ Closures in loops: does the closure capture the loop variable by reference?
    (Classic Python/JS late-binding closure — captured by ref, not value)

Classic examples (from Language Profiles R-09, PY-03):
  - Rust: HashMap iteration used in output, assumed stable across runs
  - Python: `[lambda: i for i in range(5)]` — all lambdas capture last `i`
  - JavaScript: Promise resolution order assumed to match submission order
  - DB: query results without ORDER BY assumed to return in insertion order

Layer assignment: L4.4
QBR typical: data_integrity_risk=1-2 (non-determinism produces inconsistent results)
```

---

## L4.5 — Event-Race (Async Lifecycle)

**Definition:** An async callback, event handler, or continuation fires after
the resource, component, or state it references has already been released/unmounted/invalidated.

```
Hypothesis trigger questions:
  □ React: does any async operation in a component call setState after unmount?
    (Unmounted component → memory leak + potential crash)
  □ Is there any async callback that captures a reference to an object that could
    be freed before the callback fires?
  □ useEffect: does the effect start an async operation without a cleanup function?
  □ Rust: does any tokio task capture references with shorter lifetimes than the task?
  □ Python async: does any coroutine reference a variable that could be garbage collected
    while the coroutine is suspended?

Classic examples (from Language Profiles TS-01, TS-02):
  - useEffect fetch that calls setSatate after component unmounts (React)
  - asyncio task capturing a reference to a SocketTransport that closes
  - Promise chain that references `this` after the originating object is destroyed

Layer assignment: L4.5
QBR typical: user_facing_impact=1-2 (UX glitches, memory leaks); data_integrity_risk=1-2
```

---

## L4.6 — Livelock / Deadlock

**Definition:** Two or more operations block indefinitely — deadlock (each waits for
a resource held by the other) or livelock (each responds to the other but no progress
is made).

```
Hypothesis trigger questions:
  □ Are there nested locks? (Lock A inside Lock B; elsewhere Lock B inside Lock A = deadlock)
  □ Are there any asyncio locks inside sync contexts that await inside a sync function?
  □ Are there database transactions that update rows in different orders?
    (Transaction A: row 1 then row 2; Transaction B: row 2 then row 1 = deadlock)
  □ Are there message queue consumers that wait for another consumer's output?
    (Consumer A waits for queue B; Consumer B waits for queue A = deadlock)
  □ Are there retry loops that can interact with rate limiters to produce livelock?
    (Retry triggers rate limit; rate limit triggers retry)

Classic examples:
  - Two goroutines each holding a mutex and waiting for the other's mutex
  - SQLite WAL + concurrent writers with row-level locking
  - Redis EVAL (Lua scripts) + BLPOP can livelock under specific conditions

Layer assignment: L4.6
QBR typical: user_facing_impact=2-3 (system appears frozen), blast_radius=2-3
             Detectability often HIGH (D=6-8) — hangs are visible but root cause unclear
```

---

## L4 Quick Reference for FAST Mode

```
FAST mode L4 scan (mental check — 3 minutes):
  L4.1: "Any shared mutable global without a lock?"
  L4.2: "Any if-then-use pattern on state that could change between check and use?"
  L4.3: "Any lazy singleton that could panic during init?"
  L4.4: "Any HashMap/iteration order assumed stable?"
  L4.5: "Any async callback referencing state that could be freed?"
  L4.6: "Any nested locks or cross-ordering transactions?"

If YES to any → hypothesis with minimum QBR = REQUIRED.
If YES + security-sensitive component → immediately MANDATORY.
```

---

## Integration with Language Profiles (ref 19)

Language profiles pre-seed L4 hypotheses. The canonical layer mapping:

```
L4.1 (data race):
  Rust:    R-04 (manual Send/Sync bypass)
  TS/React: TS-06 (module-scope global in SSR)
  Python:   PY-04 (singleton without lock)

L4.3 (initialization):
  Rust:    R-03 (OnceLock panic)

L4.4 (order violation):
  Rust:    R-09 (HashMap iteration assumed stable)
  Python:  PY-03 (late binding closure in loop)

L4.5 (event-race):
  TS/React: TS-01 (useEffect stale deps), TS-02 (setState after unmount),
             TS-07 (async in useLayoutEffect)
  Python:   PY-05 (blocking I/O in async)
```

L4.2 and L4.6 have no language-specific profiles yet — use the generic questions above.
When confirmed in a new language: add to ref 19 and log to [KB] Bug Class Catalog.

---

## L4 in [G.PG] Pattern Globalization

L4 bugs are excellent Pattern Globalization candidates because they often appear in
multiple places with the same underlying pattern:

```
L4.1 grep pattern: `grep -rn "_instance = \|_pool = \|_cache = " app/ | grep -v "Lock\|lock\|atomic"`
L4.2 grep pattern: `grep -rn "if.*exists.*:\|if.*has_.*:\|check.*then.*use" app/ | grep -v "transaction"`
L4.5 grep pattern: `grep -rn "setState\|setData\|setLoading" src/ | grep -v "useEffect cleanup\|isMounted"`
L4.6 grep pattern: `grep -rn "acquire.*lock.*acquire\|LOCK.*await.*LOCK\|SELECT.*FOR UPDATE" app/`
```

When any L4 hypothesis is confirmed → [G.PG] must search all 6 sublayers, not just the confirmed one.
A codebase with one L4.1 bug often has siblings across multiple sublayers.

---

*Reference 27 — VHEATM 13.0 | L4 Sublayer canonical definitions
Research: ProveNFix Song 2024; Chen 2023 UaF temporal; NIST NVD race condition CVEs*
