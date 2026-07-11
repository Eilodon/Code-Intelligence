# ADR-0002: Formal Resolver via Stack Graphs

- **Status**: Accepted
- **Date**: 2026-06-29
- **Context**: Phase 2 — Resolver Formal

## Decision

Use GitHub's `stack-graphs` crate (v0.14) with `tree-sitter-stack-graphs` (v0.10)
for formal name resolution in Tier-0 languages.

### Tier-0 (Stack Graphs — formal confidence)

- **Python**: `tree-sitter-stack-graphs-python` v0.3 (pre-built `.tsg` rules + builtins)
- **TypeScript**: Shipped (`FormalResolver::load_typescript`, `crates/calm-core/src/resolver/formal.rs:385`, 2026-07-03)
- **JavaScript**: Shipped (`load_javascript`, `formal.rs:425`, 2026-07-04)
- **Java**: Shipped (`load_java`, `formal.rs:451`, 2026-07-06)

### ConservativeResolver (retained — not replaced)

- **All languages**: ConservativeResolver remains the primary edge builder for
  alias tracking (`x = y` patterns) and tier-1 resolution (file symbols, imports).
- **Rust, Go, C, C++, Ruby, PHP**: ConservativeResolver is the only resolver
  (no Stack Graphs rules available).

### EdgeConfidence tiers

| Tier | Source | Rank |
|------|--------|------|
| `formal` | Stack Graphs complete path (reference → definition) | 3 |
| `resolved` | ConservativeResolver tier-1 (file symbol, import, alias) | 2 |
| `inferred` | Type-based inference (future) | 1 |
| `textual` | Name-only match | 0 |

## Update (2026-07-03)

TypeScript implemented (`FormalResolver::load_typescript`, `crates/ci-core/src/resolver/formal.rs`):
`tree-sitter-stack-graphs-typescript` 0.4.0, exact version match with the workspace's already-pinned
`tree-sitter-typescript = 0.23.2` — zero dependency conflict. Covers both `.ts` and `.tsx` (separate
`StackGraphLanguage`/builtins pair internally, dispatched by file extension — TSX is a distinct
upstream grammar, not a superset). Unlike Python, upstream's `builtins.ts` is non-empty (~10KB) and
resolves real ECMAScript globals (`Array`, `.isArray`, etc. — verified by test, not assumed) — no
DEBT-005-style synthetic stub needed.

JavaScript and Java were **both since implemented** (Update below) — at the time of this section
(2026-07-03) they were riskier than TypeScript for the reasons kept here for historical context:
- **JavaScript**: `builtins.js` ships empty upstream (same as Python originally), but the fix isn't a
  drop-in DEBT-005-style stub — `stack-graphs.tsg` wires primitive-prototype builtins
  (`builtins_number`, `builtins_string`, `builtins_Regex_prototype`, ...) as nodes generated per-file
  on `@prog`'s own scope, not through the `<builtins>` file/`push_symbol` fallback edge Python and
  TypeScript both use. Global *functions* (`parseInt`, `setTimeout`, ...) appeared to have no fallback
  path in the rules as written at the time — resolved during implementation, see Update below.
- **Java**: `builtins.java` also ships empty, and `stack-graphs.tsg` had **zero** references to
  "builtins" of any form — how `java.lang` auto-import (`String`, `Object`, `System`, ...) resolves
  through this mechanism needed the deepest investigation of the three — resolved during
  implementation, see Update below.

Dependency versions are otherwise ready whenever the above is resolved: `tree-sitter-stack-graphs-javascript`
0.3.0 (needs `tree-sitter-javascript` pinned to exactly `0.23.1` — already the workspace's resolved
version) and `tree-sitter-stack-graphs-java` 0.5.0 (needs `tree-sitter-java` pinned to exactly
`0.23.4` — one patch below the workspace's current `0.23.5`, a safe Cargo-resolvable downgrade).

## Consequences

- `stack-graphs` repo is archived by GitHub (Sept 2025) — crates work but receive
  no updates. If critical bugs surface, we fork.
- tree-sitter 0.24 version is pinned by stack-graphs compatibility.
- FormalResolver produces edges per-file; cross-file resolution requires building
  a shared StackGraph with all project files indexed.
- Python builtins are embedded in the crate; no runtime download needed.

## Alternatives Considered

- **rust-analyzer style**: Too tightly coupled to Rust; not multi-language.
- **LSP-based**: Requires running external language servers; high latency, hard to embed.
- **Scope analysis from scratch**: Reinventing what Stack Graphs already solves.

## Update (2026-07-06): JavaScript and Java shipped

Both resolved during implementation, closing the open questions from Update (2026-07-03) above:
`load_javascript` (`crates/calm-core/src/resolver/formal.rs:425`, 2026-07-04) and `load_java`
(`formal.rs:451`, 2026-07-06). See `test_resolve_file_resolves_java_builtins` and the JS/TS
equivalents in the same test module for what specifically got verified working, not just compiling.
