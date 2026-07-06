# Language Profiles (v11.0)

> Load this file at [P] when LANGUAGE is declared.
> Each profile maps language-idiomatic bug patterns to existing VHEATM layers (L1-L7).
> These are ADDITIONAL hypothesis triggers — they run BEFORE the standard L1-L7 scan
> to seed language-specific candidates.
>
> **Why language profiles:** VHEATM is language-agnostic by design. But language-idiomatic
> patterns create bugs that generic L1-L7 questions may never surface. Rust's ownership
> model creates unique L2/L4 patterns. TypeScript/React's async component lifecycle
> creates unique L4.5 patterns. Language profiles make these explicit.

---

## How to Use

At [P], when LANGUAGE is declared:
1. Load this file
2. Scan the relevant profile (takes 2-5 minutes)
3. For each trigger that APPLIES to this codebase: add to [G.H] as a pre-seeded hypothesis
4. Continue to standard L1-L7 scan — language profile is additive, not a replacement

Marking: each pre-seeded hypothesis carries `source: language-profile` tag.

---

## Profile: Rust (from Clippy correctness lint category)

> Source: Clippy correctness group — highest-confidence lints, near-certain bugs.
> These are patterns where the Rust compiler or Clippy can prove a problem exists.
> T3 evidence (documented best practices with tooling validation).

```
R-01 — Unwrap on None-guaranteed path
  Layer: L1 (contract violation: code claims to handle error but panics)
  Trigger: .unwrap() or .expect() called on Option/Result where None/Err is provably reachable
  Clippy lint: clippy::unwrap_used (pedantic), manual review for guaranteed-None paths
  VHEATM question: "Is there any .unwrap() / .expect() on a value that can be None/Err
    in a reachable path? What happens to the user if this panics?"

R-02 — Panic inside fn returning Result/Option
  Layer: L1 (contract violation: function signature promises Result, panics instead)
  Trigger: panic!(), unwrap(), expect(), todo!() inside fn → Result<_, _>
  Clippy lint: clippy::panics_in_result_fn
  VHEATM question: "Does any function returning Result/Option contain a panic path?
    The caller cannot catch a panic — it propagates to thread boundary."

R-03 — Panic in static initializer (OnceLock/Lazy)
  Layer: L4.3 (atomicity: initialization must be atomic and successful)
  Trigger: .expect() / .unwrap() inside OnceLock::get_or_init() or Lazy::new()
  VHEATM question: "If any OnceLock/Lazy initializer can panic, the static is
    poisoned permanently for the process lifetime."

R-04 — Non-Send/Sync type shared across threads
  Layer: L4.1 (data race: Rust type system usually catches this, but manual Send impl can bypass)
  Trigger: Manual impl Send for [Type] or unsafe cell sharing
  VHEATM question: "Is there any manual Send/Sync impl? What guarantees thread safety
    for the types wrapped in unsafe?"

R-05 — FFI null pointer where non-null expected
  Layer: L3 (cross-layer integration: FFI boundary violation)
  Trigger: Passing potentially-null pointer to C function that requires non-null
  VHEATM question: "For every FFI call: are pointer arguments guaranteed non-null
    at the call site? Are return pointers checked before dereference?"

R-06 — Integer overflow in release builds
  Layer: L2 (data integrity: debug builds panic on overflow, release builds wrap silently)
  Trigger: Arithmetic on untrusted numeric inputs without checked_add/checked_mul
  VHEATM question: "For user-supplied numeric inputs: is there any arithmetic that
    could overflow? Rust wraps silently in release — use checked_* or saturating_* ops."

R-07 — Regex::new() with invalid pattern (runtime panic)
  Layer: L1 (input validation: compile-time-detectable panic from static input)
  Trigger: Regex::new() called with a hardcoded string that can fail at runtime
  Clippy lint: clippy::invalid_regex
  Mitigation: use Regex::new().expect() only for patterns that can be proven valid,
    or use once_cell::sync::Lazy to fail at startup, not mid-request.

R-08 — Lifetime annotation masking use-after-free
  Layer: L2 (state consistency)
  Trigger: 'static lifetime cast via unsafe transmute, or lifetime elision hiding
    a borrow that outlives its source
  VHEATM question: "Is there any unsafe transmute on lifetimes? Any 'static cast
    on a non-'static value?"

R-09 — Non-deterministic HashMap ordering assumed
  Layer: L4.4 (order violation: HashMap iteration order is random by design in Rust)
  Trigger: Code assumes iteration order of HashMap/HashSet is stable across runs
  VHEATM question: "Is HashMap iteration order used in any user-visible output,
    snapshot comparison, or hash-based equality check?"
```

---

## Profile: TypeScript / React

> Source: canonical TypeScript/React anti-pattern taxonomy from community documentation,
> React team advisories, and TypeScript compiler strict mode findings.
> T3 evidence (documented best practices with tooling validation via TypeScript strict mode).

```
TS-01 — useEffect dependency array incorrect or missing
  Layer: L4.5 (event-race: React lifecycle + background state mutation)
  Trigger: useEffect(() => {...}, []) or missing dependency causes stale closure
  VHEATM question: "For every useEffect: do the dependencies array match all
    external values referenced inside the effect? Missing deps = stale data.
    Extra deps = infinite loop."

TS-02 — setState called after component unmount
  Layer: L4.5 (event-race: async callback fires after component lifecycle ends)
  Trigger: async operation completes and calls setState after component has unmounted
  VHEATM question: "For every async operation triggered from a component: is there
    a cleanup / isMounted guard to prevent setState after unmount?"

TS-03 — Unhandled Promise rejection in async event handler
  Layer: L5 (external dependency failure: unhandled rejection swallows errors)
  Trigger: async onClick / useEffect without try/catch or .catch()
  VHEATM question: "For every async function in event handlers: is there a catch path?
    Unhandled rejections in React 18+ cause error boundaries to fire."

TS-04 — Non-null assertion on potentially null value
  Layer: L1 (contract violation: ! operator bypasses null check)
  Trigger: value! where value can be undefined/null at runtime
  VHEATM question: "For every ! assertion: is there a guarantee the value is non-null
    at that point? TypeScript trusts !, the runtime does not."

TS-05 — Type coercion masking undefined
  Layer: L2 (state consistency: falsy check vs nullish check)
  Trigger: if (value) {...} when value = 0 or "" is valid (falsy but defined)
  VHEATM question: "For every truthy check on a value that has a valid falsy state:
    should this be value !== undefined or value != null instead?"

TS-06 — Global mutable state in module scope
  Layer: L4.1 (data race equivalent: in SSR / Next.js, module-scope vars are shared
    across requests)
  Trigger: let/var at module top level mutated by request-handling code
  VHEATM question: "In SSR context: is any module-level variable mutated per-request?
    SSR shares module instances across concurrent requests."

TS-07 — Async/await in useLayoutEffect
  Layer: L4.5 (event-race: useLayoutEffect runs synchronously — async is ignored)
  Trigger: useLayoutEffect(async () => {...})
  VHEATM question: "Is useLayoutEffect used with async? The async return (a Promise)
    is ignored by React. Side effects appear to succeed but haven't."

TS-08 — Missing error boundary for async data loads
  Layer: L5 (external dependency: thrown errors during render crash the tree)
  Trigger: Data-loading component has no error boundary or Suspense fallback
  VHEATM question: "For components that load async data: is there an error boundary
    or loading fallback? An unhandled throw during render unmounts the whole tree."
```

---

## Profile: Python

> Source: Python typing system (mypy strict), common async pitfalls, Django/Flask patterns,
> SQLAlchemy async (v2.0+), ARQ/Celery/Dramatiq worker patterns.
> T3 evidence (PY-01..PY-06); PY-07..PY-09 derived from Tikai field evidence + SQLAlchemy
> async changelog — T4 evidence, upgrade path: 3 confirmed instances → T2.

```
PY-01 — Mutable default argument
  Layer: L2 (state consistency: mutable default shared across ALL calls)
  Trigger: def fn(data=[]) or def fn(config={}) — the default object is created ONCE
  VHEATM question: "Is there any function with a mutable default argument (list, dict, set)?
    The same object is reused across calls — mutations persist."

PY-02 — Bare except clause
  Layer: L1 (contract violation: catches SystemExit, KeyboardInterrupt, MemoryError)
  Trigger: except: without specifying exception type
  VHEATM question: "Is there any bare except? This silences signals and OOM errors."

PY-03 — Late binding closure in loop
  Layer: L4.4 (order violation: loop variable captured by reference, not by value)
  Trigger: [lambda: fn(x) for x in items] — all lambdas close over the LAST x
  VHEATM question: "Are there lambdas or closures created inside a for loop that
    reference the loop variable? Use default argument binding: lambda x=x: fn(x)"

PY-04 — Thread-unsafe singleton with race condition
  Layer: L4.1 (data race: Python GIL does not protect multi-step operations)
  Trigger: if _instance is None: _instance = Class() — check-then-act race
  VHEATM question: "Is there any singleton pattern not protected by threading.Lock?
    The GIL does not make multi-step operations atomic."

PY-05 — Blocking I/O in async context (generic)
  Layer: L4.5 (event-race: blocks the event loop)
  Trigger: requests.get(), open(), time.sleep() inside async def without await
  VHEATM question: "In async functions: is there any synchronous I/O call?
    This blocks the entire event loop for all coroutines."
  Note: PY-05 covers obvious blockers. PY-08 covers non-obvious library blockers
    (pandas, openpyxl, chardet) that pass PY-05 review because they look like
    data processing, not I/O.

PY-06 — Pickle deserialization of untrusted data
  Layer: L6 (security: arbitrary code execution)
  Trigger: pickle.loads() on user-supplied or network-received data
  VHEATM question: "Is pickle.loads() called on any data that could be user-controlled?
    Pickle is not safe against maliciously crafted data."

PY-07 — AsyncSession commit lifecycle (SQLAlchemy async)
  Layer: L2 (state consistency: writes silently discarded on session close)
  Trigger: async with AsyncSession() as session: — context exit CLOSES, does NOT commit.
           async with session: is the same trap. __aexit__ calls session.close() only.
  Severity: CRIT — uncommitted data is ROLLED BACK silently, no exception raised.
  VHEATM question: "Enumerate all async_sessionmaker / AsyncSession usages.
    For each: is await session.commit() called explicitly before context exit?
    OR is async with session.begin(): used (auto-commits on clean exit)?
    If neither pattern is present → data loss on every write."
  Correct patterns:
    PATTERN A: async with session.begin():
                   session.add(obj)       # commits automatically on exit
    PATTERN B: async with AsyncSession() as session:
                   session.add(obj)
                   await session.commit() # explicit
  Anti-pattern: async with AsyncSession() as session:
                    session.add(obj)      # ROLLS BACK on exit
  Audit command: grep -rn "async with.*[Ss]ession" app/ — inspect each hit.
  Note: This bug is silent — no exception, no warning, write appears to succeed,
    data is absent on next read. Typically surfaces as "data disappears randomly."
  ASYNC_WORKER profile: MANDATORY scan when arq/celery/dramatiq detected in deps.

PY-08 — Pandas / heavy-library blocking in async context
  Layer: L4.5 (event-race: non-obvious library calls block event loop 100ms-2s+)
  Trigger: pd.read_csv(), pd.read_excel(), openpyxl workbook load, chardet.detect(),
           hashlib on large bytes — inside async def without run_in_executor.
  Severity: HIGH — event loop stall; all concurrent coroutines blocked during parse.
  VHEATM question: "Search for pandas import in async def functions.
    Any pd.read_* / pd.DataFrame() construction on untransformed bytes without
    run_in_executor is a thread-blocking violation.
    Search: grep -rn 'pd\.\|pandas' app/ — flag any hit inside async def."
  Why PY-05 misses this: a developer reading PY-05 ('no requests.get in async')
    passes mental review while pd.read_excel(io.BytesIO(file_bytes)) is blocking
    500ms below. pandas reads are not perceived as I/O.
  Required fix: asyncio.get_event_loop().run_in_executor(None, pd.read_csv, path)
                OR execute inside ARQ/Celery worker (separate thread pool — OK).
  Note: FastAPI async endpoint = NOT OK. ARQ worker = OK (separate process).

PY-09 — dataclasses.replace() result not captured (silent no-op)
  Layer: L2 (state consistency: mutation appears to succeed, original unchanged)
  Trigger: dataclasses.replace(obj, field=value) — creates NEW object, original UNCHANGED.
  Severity: HIGH — if result is not reassigned or collected, the replacement is lost.
  VHEATM question: "Search for dataclasses.replace() usages.
    For each call: is the return value assigned or collected?
    Uncaptured result → the field update never happens, no error raised."
  Anti-pattern: dataclasses.replace(row, fee=new_fee)  # return value discarded
  Correct: row = dataclasses.replace(row, fee=new_fee)
           OR: updated.append(dataclasses.replace(row, fee=new_fee))
  Note: This bug class surfaces at the CALLER of a function that uses replace()
    internally. Verify both implementation AND callsite.
```

---

## Adding New Language Profiles

When auditing a language not listed above, create a minimal profile:

```
New language profile template:
  Language: [name]
  Source: [compiler warnings / linter / community documentation]
  Evidence tier: T3 (documented best practices) by default

  For each pattern:
    [CODE]-[N] — [Pattern name]
    Layer: [VHEATM L1-L7 or L4.1-L4.6]
    Trigger: [code construct that produces this bug]
    VHEATM question: [the specific question to ask]
```

Log new profiles in [KB] for persistence across cycles. If a language profile
generates confirmed bugs in 2+ cycles → promote to REQUIRED scan status.

---

*Reference 19 — VHEATM 11.0 | Clippy correctness lint category (T3);
TypeScript/React canonical anti-patterns (T3); Python community docs (T3)*
