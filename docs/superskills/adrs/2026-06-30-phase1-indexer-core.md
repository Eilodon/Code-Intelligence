# ADR: Phase 1 Indexer Core (Tree-sitter & SQLite WAL)

## 1. Title
Implement Greenfield Rust Indexer using Tree-sitter Cursor API and SQLite WAL transactions.

## 2. Context
The system required migrating from the legacy Python indexer to a native Rust implementation to achieve zero-IPC overhead and eliminate the Python runtime dependency.

## 3. Decision
We built a pure-Rust pipeline (`ci-core/src/indexer`) consisting of a `tree-sitter` extraction layer, a transactional edge-building module (`insert_symbols_batch`), and an `IndexingPhase` state machine. The background indexer runs in a `rusqlite::Transaction` with WAL enabled to guarantee crash integrity without blocking concurrent user queries.

## 4. Status
ACCEPTED

## 5. Consequences
- **Improved**: System now runs 100% in Rust; no Python dependency.
- **Improved**: Crash-safe edge building.
- **Debt Created**: Only Python is currently wired in `parser.rs`; the remaining 5 tier-0 languages must be dynamically configured (PATTERN-DEBT).

## 6. Alternatives Considered
- *Using Tree-sitter Visitor Wrapper*: Rejected in favor of the direct Cursor API for maximum performance and explicit traversal control.

## 7. Evidence
Unit tests for `test_python_symbol_extraction`, `test_insert_symbols_transaction`, and `test_run_indexing_pipeline_transaction` all pass. SQLite is configured with `PRAGMA journal_mode=WAL` (verified in `schema.rs`).

## 8. Owner
Eilodon

## 8b. Known Debts (PATTERN-DEBT)
PATTERN-DEBT entries introduced or affected by this change:
  - PATTERN-DEBT-005 (Incomplete Language Parsers): OPEN — 5 languages remaining

## 9. Next Cycle Trigger
When support for JavaScript/TypeScript extraction is requested by the user.

## 10. Cycle Retrospective
- What assumption proved wrong during this implementation? We assumed `tree_sitter_python::language` was a function, but it is a constant `LANGUAGE` yielding a `LanguageFn`.
- What surprised us about the codebase / domain / dependencies? `rusqlite` transactions cleanly handle the transition atomicity without adding boilerplate.
- What would we design differently if starting over? Inject the parser language dynamically from the start instead of hardcoding the Python block.
- What debt was knowingly created and why? We only stubbed Python parsing inside `extract_symbols` to meet the test boundary and pass the task; full dynamic loading is deferred.
- What signal should the next cycle watch for? The lack of other tier-0 languages (Go, TS, JS, Java, Rust) in `parser.rs`.
