# Architecture Smell Scan — [V.AS] (v11.0)

> **Research Basis:**
> [Gnoyke et al. (2024, 8 citations, JSS)](https://consensus.app/papers/details/9a5ce571723d5156825b1b189e9d49d2/):
> study of 485 releases across 14 OSS systems. "Cyclic dependencies on the class-level
> are prone to becoming **highly complex over time**, with one reason being continued
> merging of smells, resulting in tangled multi-hubs." Unstable dependencies grow slowly
> but persistently.
>
> [Sas et al. (2022, 20 citations, Empirical SE)](https://consensus.app/papers/details/c70338509c745566b850fca0d7609d87/):
> industrial case study (9 C/C++ projects, 30+ releases, 20M+ LOC, 12 developer interviews).
> Qualitative finding: practitioners confirm architectural smells directly affect
> "long-term maintainability and evolvability." Quantitative: smell instances tracked
> across releases — individual smells merge into tangled multi-hubs.
>
> [Palomba et al. (2019, 109 citations, IEEE Trans. SE)](https://consensus.app/papers/details/314f7859ad355b26a50349075a61081c/):
> code smell intensity added to bug prediction models improves F-measure by **≥13%**
> vs models without smell data. Smell-affected components are measurably more bug-prone.
>
> [Esposito et al. (2024, 3 citations, Software Quality Journal)](https://consensus.app/papers/details/54006d5daf8a595584daffc3c849b9b3/):
> 103 Java projects, 72M LOC: "moderate correlation between static analysis warnings and
> architectural smells." Warning-based prioritization provides pragmatic bridge to
> architectural concerns.
>
> **Core design decision:** [V.AS] runs at end of [V] Vision, not in [G] Generation.
> Reason: AS scan requires the architecture map built in [V] (C4 diagram / Org Ownership Map).
> AS findings are pre-seeded into [G.H] as hypotheses with their own QBR scores —
> they integrate into the standard hypothesis lifecycle, not a separate track.

---

## When to Run

```
REQUIRED: Standard + Full modes (all context modes except DESIGN-without-code)
RECOMMENDED: CODE and LEGACY modes
FAST: AS-01 (cyclic deps) + AS-03 (God component) only — 3 minutes
NOT APPLICABLE: Pure DESIGN mode (no dependency graph exists yet)
```

---

## 5 Architecture Smell Scans

Run AFTER [V] C4/architecture map is complete. Use the dependency graph from [V].

### AS-01 — Cyclic Dependencies

```
Detection question:
  "Does the dependency graph have any cycle? A → B → C → A is a cycle."

Why it matters (Gnoyke 2024): cyclic dependencies grow in complexity over time —
they merge into tangled multi-hubs with each release. A cycle today becomes a
multi-hub in 6 months. Blast radius of changes compounds.

Scan method (by language):
  Rust:       cargo-udeps + manual review of Cargo.toml workspace deps
  JS/TS:      dependency-cruiser (npx depcruiser --validate)
  Java:       ARCAN, JDepend, or IntelliJ dependency matrix
  Python:     pydeps, or manual review of import chains
  General:    build dependency graph from import statements; look for back-edges

VHEATM questions:
  □ Is there any import/use chain where module A depends on module B which depends (directly
    or transitively) on module A?
  □ If yes: which modules are in the cycle? How many hops?
  □ What changes to module A would cascade back to A through the cycle?

QBR calibration:
  Cycle with 2 nodes: blast_radius = 1
  Cycle with 3-5 nodes: blast_radius = 2
  Cycle with 6+ nodes or cross-layer cycle (e.g., UI imports from DB layer): blast_radius = 3
  data_integrity_risk = 1 (latent risk), user_facing = 0 unless cycle causes build failure

Typical QBR: 4-10 → REQUIRED or RECOMMENDED
Exception: cycle involving core module with fan-in > 10 → escalate blast_radius → may reach MANDATORY
```

---

### AS-02 — Unstable Dependencies

```
Detection question:
  "Do stable, rarely-changing modules depend on unstable, frequently-changing ones?"

Why it matters: stable modules (relied upon by many) become unpredictable when
they depend on volatile modules. Every change to the volatile module ripples
into the stable one.

Instability metric: instability(M) = fan_out(M) / (fan_in(M) + fan_out(M))
  0 = maximally stable (nothing depends on others)
  1 = maximally unstable (no one depends on it)

Smell: stable module (low instability) depends on unstable module (high instability)

VHEATM questions:
  □ Identify the 3-5 most-depended-upon modules (highest fan-in) — these are "stable."
  □ Do any of these depend on modules that change frequently or have high fan-out?
  □ Is the dependency direction aligned with stability? (stable → should depend on stable)

QBR calibration:
  stable module with 1-2 unstable deps: blast_radius = 1
  stable module with 3+ unstable deps: blast_radius = 2
  core business logic module with unstable deps: blast_radius = 3
  Typical QBR: 6-14 → REQUIRED
```

---

### AS-03 — God Component

```
Detection question:
  "Is there a single module/class with > N responsibilities, high fan-in, and high LOC?"

Why it matters: God components concentrate blast radius. Every bug in them affects
all callers. Every change requires understanding all N responsibilities.
Palomba 2019: smell-affected components have measurably higher bug density.

Detection heuristics:
  □ LOC > 500 for a single module (language-adjusted: Rust 300, Go 400, Python 300)
  □ Fan-in > 10 (more than 10 other modules import this one)
  □ Distinct concern count > 3 (handles auth AND business logic AND data access AND formatting)
  □ Cyclomatic complexity > 20 for any single function within it

VHEATM questions:
  □ What are the top 3 modules by LOC? By fan-in?
  □ For each: how many distinct concerns does it handle?
  □ If a bug is found in this module, how many callers are affected?

QBR calibration: usually REQUIRED (blast_radius = 2-3 by definition)
Pattern Globalization: God components often have scattered concern as a sibling smell (AS-04)
```

---

### AS-04 — Scattered Concern

```
Detection question:
  "Is a single concern (validation, error handling, date parsing, auth check)
  scattered across > 3 modules?"

Why it matters: scattered concerns create Pattern Globalization failures.
When a scattered concern has a bug, fixing one instance misses all others.
The [G.PG] grep will find the other instances — but only if the pattern is identified here.

VHEATM questions:
  □ Where is input validation happening? One place or many?
  □ Where is error handling / logging? Centralized or repeated?
  □ Where is date/time parsing? Consistent or duplicated with variations?
  □ Where is auth checking? Middleware only or also scattered in handlers?

Implication: if scattered concern confirmed → Pattern Globalization (Track B: structural)
is MANDATORY for the affected concern. All instances must be fixed together.

QBR calibration: RECOMMENDED typically; escalates to REQUIRED if security-critical concern
(auth checks) is scattered — one missed instance = bypass.
Special: scattered auth check with any instance missing → blast_radius = 3 → MANDATORY.
```

---

### AS-05 — Interface Segregation Violation

```
Detection question:
  "Do callers depend on interfaces with methods they don't use?"

Why it matters: callers are forced to recompile/retest when unused methods change.
In dynamic languages, fat interfaces create hidden coupling. In typed languages,
they force unnecessary mock complexity in tests.

VHEATM questions:
  □ Are there traits/interfaces/protocols with > 5 methods?
  □ For each: what percentage of callers use all methods?
  □ If < 50% of callers use all methods → the interface can be split.

QBR calibration: usually RECOMMENDED. Escalates to REQUIRED if:
  - The interface is in a public API (external callers)
  - Mock complexity is causing test debt (tests are hard to write → test gap)
```

---

## AS Output Format

Architecture smell findings are pre-seeded into [G.H] with:

```yaml
architecture_smell_hypothesis:
  id: H-AS-[N]
  smell_type: AS-01 | AS-02 | AS-03 | AS-04 | AS-05
  layer: L3  # all AS findings map to L3 (cross-layer integration) by default
  description: "[specific smell instance]"
  modules_affected: ["[module A]", "[module B]", ...]
  instability_score: [0.0-1.0 | N/A]  # AS-02 only
  fan_in: [N]   # AS-03
  loc: [N]      # AS-03
  concern_count: [N]  # AS-03, AS-04
  qbr_score: [N]
  adr_note: "Architecture smell ADRs use Pattern Globalization Track B (structural)"
  structural_siblings_check_required: true
```

---

## AS in [G.PG] Pattern Globalization

When an AS finding is confirmed, Pattern Globalization runs with **Track B (structural)**:

```
Track B for AS:
  AS-01 Cyclic: "Are there other cycles in the dependency graph?"
    → List all cycles, not just the one found first.
  AS-03 God: "Are there other God components by the same heuristics?"
    → Check all modules above LOC/fan-in thresholds.
  AS-04 Scattered: "Where else is this concern scattered?"
    → Find ALL instances of the scattered pattern before issuing ADR.
    (Fix one without finding others = Pattern Globalization failure.)
```

---

## Calibration Note

Architecture smell detection thresholds (LOC > 500, fan-in > 10, etc.) are heuristics,
not hard rules. They should be calibrated to the language and team context:

- Rust modules tend to be smaller → use LOC > 300
- Monolithic legacy systems → God component threshold may need to be higher to be actionable
- Microservices → fan-in thresholds per-service, not globally

Log calibration adjustments in [KB] when thresholds are changed for a project.

---

*Reference 20 — VHEATM 11.0 | Gnoyke 2024 (JSS); Sas 2022 industrial (Empirical SE);
Palomba 2019 (109 cit., IEEE Trans. SE); Esposito 2024 (Software Quality Journal)*
