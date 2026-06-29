# ADR-0002: Formal Resolver via Stack Graphs

- **Status**: Accepted
- **Date**: 2026-06-29
- **Context**: Phase 2 — Resolver Formal

## Decision

Use GitHub's `stack-graphs` crate (v0.14) with `tree-sitter-stack-graphs` (v0.10)
for formal name resolution in Tier-0 languages.

### Tier-0 (Stack Graphs — formal confidence)

- **Python**: `tree-sitter-stack-graphs-python` v0.3 (pre-built `.tsg` rules + builtins)
- **TypeScript / JavaScript / Java**: Future — requires `.tsg` rule authoring or
  upstream crate availability

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
