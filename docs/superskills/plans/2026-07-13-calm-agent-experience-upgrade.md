# CALM Agent-Experience Upgrade — Implementation Plan

> **For agentic workers:** Use `subagent-driven-development` (recommended)
> or `executing-plans` to implement this plan task-by-task.

**Goal:** Close the gap between "CALM is safe because this particular agent
was careful" and "CALM is safe by construction" — a proactive symbol-
boundary-integrity check, a lower-friction small-text-match edit mode gated
behind it, a per-file (not per-session) re-arm of the native-Edit-block
hook with a clearer message, two cheap UX fixes, and a fuzz-test harness
that would have caught this session's `apply_hunks` bug before it shipped.

**Architecture:** Tier 1 (Phases A-C) adds one new `symbols` column
(`boundary_ambiguous`), one new indexer post-process pass modeled exactly
on the existing `hub_kind` pass, one new opt-in edit-mode field on
`edit_symbol`, and a per-file state upgrade to the project's shell
PreToolUse hook. Tier 2 (Phase D) adds two new `edit_symbol` `position`
values and a boolean flag threaded through `edit_lines_impl`. Tier 4
(Phase E) adds a `proptest`-based invariant test for `apply_hunks`.

**Tech Stack:** Rust (calm-core, calm-server), rusqlite, tree-sitter
(already-parsed data only, no new parsing), bash/jq (project hook).

**Audit Gate:** PASS WITH FLAGS (see
`docs/superskills/specs/2026-07-13-calm-agent-experience-upgrade.md`)

**Risk Flags:** Task A5 and B3 (edit_symbol gate changes) are CROSS
boundary — they change `calm-server`'s tool contract, not just internal
`calm-core` logic; explicit handoff notes included. Task C1 modifies a
security-relevant shell hook — treat with the same care as the Rust gates.

**Out of scope for this plan** (per audit-design Gate Result): Tier 2 item
6 (`tools/list_changed` on daemon respawn) — investigation this session
found the `calm connect` forwarder relays stdio↔socket byte-verbatim with
zero MCP/JSON-RPC awareness (`crates/calm-cli/src/main.rs`'s `Connect`
subcommand doc comment), so a server-side `notify_tool_list_changed()`
call alone is not proven sufficient — needs its own spec once the
forwarder's reconnect path is traced. Tier 3 item 7
(`REASON_NOT_GROUNDED` scoping) — requires explicit user sign-off on the
structural-equivalence mechanism per the audit's L5 flag; not started
here.

---

## Phase A — Symbol boundary-integrity check (Tier 1 item 1)

### Task A1: Schema migration for `symbols.boundary_ambiguous`

**Files:**
- Modify: `crates/calm-core/src/db/schema.rs:321` (right after the existing
  `hub_kind` migration)
- Test: `crates/calm-core/src/db/schema.rs` (inline `#[cfg(test)]` module —
  find the existing migration test, e.g. search `mod tests` in this file,
  and add alongside it)

- [ ] **Step 1: Write the failing test**
```rust
#[test]
fn migration_adds_boundary_ambiguous_column_defaulting_to_zero() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    init_db(&conn).unwrap();
    conn.execute(
        "INSERT INTO symbols (qualified_name, name, kind, path, language, line_start, line_end, signature) \
         VALUES ('x', 'x', 'function', 'a.rs', 'rust', 1, 2, 'fn x()')",
        [],
    )
    .unwrap();
    let val: i64 = conn
        .query_row(
            "SELECT boundary_ambiguous FROM symbols WHERE qualified_name = 'x'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(val, 0, "new rows default to not-ambiguous");
}
```
- [ ] **Step 2: Run — verify FAIL** `cargo test -p calm-core migration_adds_boundary_ambiguous_column_defaulting_to_zero` → expected: FAIL (`no such column: boundary_ambiguous`)
- [ ] **Step 3: Write minimal implementation** — insert this line into
  `run_migrations` immediately after the existing `hub_kind` migration
  (`crates/calm-core/src/db/schema.rs:321`):
```rust
    // Tier-1 agent-experience upgrade: flags a symbol whose line_start or
    // line_end shares a physical source line with an adjacent symbol —
    // written by `graph::boundary::update_boundary_ambiguous_flags`,
    // called from `rebuild_graph` right after `update_is_hub_flags` so it
    // gets the exact same per-reindex invalidation guarantee already
    // trusted for `hub_kind` (see docs/superskills/specs/2026-07-13-calm-
    // agent-experience-upgrade.md Risk Assessment, Failure Mode 1).
    migrate_add_column(
        conn,
        "symbols",
        "boundary_ambiguous",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
```
- [ ] **Step 4: Run — verify PASS** `cargo test -p calm-core migration_adds_boundary_ambiguous_column_defaulting_to_zero` → expected: PASS
- [ ] **Step 5: Commit** `git commit -m "feat(db): add symbols.boundary_ambiguous column"`

---

### Task A2: `update_boundary_ambiguous_flags` in a new `graph::boundary` module

**Files:**
- Create: `crates/calm-core/src/graph/boundary.rs`
- Modify: `crates/calm-core/src/graph.rs` (add `pub mod boundary;`)

- [ ] **Step 1: Write the failing tests**
```rust
use rusqlite::Connection;

fn setup_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    crate::db::schema::init_db(&conn).unwrap();
    conn
}

fn insert_symbol(conn: &Connection, qname: &str, path: &str, line_start: i64, line_end: i64) {
    conn.execute(
        "INSERT INTO symbols (qualified_name, name, kind, path, language, line_start, line_end, signature) \
         VALUES (?1, ?1, 'function', ?2, 'rust', ?3, ?4, 'fn x()')",
        rusqlite::params![qname, path, line_start, line_end],
    )
    .unwrap();
}

#[test]
fn flags_two_symbols_sharing_a_boundary_line() {
    let conn = setup_db();
    // repo_overview ends on line 251; hotspots' own tracked range starts
    // at 261 (attributes aren't part of its line_start) — this test
    // reproduces the simpler, direct case: two symbols whose ranges
    // literally touch at the same line number.
    insert_symbol(&conn, "a", "f.rs", 1, 10);
    insert_symbol(&conn, "b", "f.rs", 10, 20);
    update_boundary_ambiguous_flags(&conn).unwrap();

    let a: i64 = conn
        .query_row(
            "SELECT boundary_ambiguous FROM symbols WHERE qualified_name = 'a'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let b: i64 = conn
        .query_row(
            "SELECT boundary_ambiguous FROM symbols WHERE qualified_name = 'b'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(a, 1, "line_end shared with next symbol's line_start");
    assert_eq!(b, 1, "line_start shared with previous symbol's line_end");
}

#[test]
fn does_not_flag_symbols_with_a_normal_gap() {
    let conn = setup_db();
    insert_symbol(&conn, "a", "f.rs", 1, 10);
    insert_symbol(&conn, "b", "f.rs", 12, 20);
    update_boundary_ambiguous_flags(&conn).unwrap();

    let a: i64 = conn
        .query_row(
            "SELECT boundary_ambiguous FROM symbols WHERE qualified_name = 'a'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(a, 0);
}

#[test]
fn scopes_the_check_per_file_not_across_files() {
    let conn = setup_db();
    // Same line numbers in two different files must never cross-flag.
    insert_symbol(&conn, "a", "f1.rs", 1, 10);
    insert_symbol(&conn, "b", "f2.rs", 10, 20);
    update_boundary_ambiguous_flags(&conn).unwrap();

    let a: i64 = conn
        .query_row(
            "SELECT boundary_ambiguous FROM symbols WHERE qualified_name = 'a'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(a, 0);
}

#[test]
fn re_running_after_a_fix_clears_the_flag() {
    // Regression guard for audit Failure Mode 1 (stale/uninvalidated flag,
    // same class of bug as the F12 config mtime-cache and Bug 2 stale-
    // config incidents this session): recomputing must be idempotent and
    // must CLEAR a previously-set flag once the underlying condition is
    // gone, not just ever set it.
    let conn = setup_db();
    insert_symbol(&conn, "a", "f.rs", 1, 10);
    insert_symbol(&conn, "b", "f.rs", 10, 20);
    update_boundary_ambiguous_flags(&conn).unwrap();
    conn.execute(
        "UPDATE symbols SET line_end = 9 WHERE qualified_name = 'a'",
        [],
    )
    .unwrap();
    update_boundary_ambiguous_flags(&conn).unwrap();

    let a: i64 = conn
        .query_row(
            "SELECT boundary_ambiguous FROM symbols WHERE qualified_name = 'a'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(a, 0, "flag must clear once the boundary no longer overlaps");
}
```
- [ ] **Step 2: Run — verify FAIL** `cargo test -p calm-core --lib graph::boundary` → expected: FAIL (module doesn't exist)
- [ ] **Step 3: Write minimal implementation** — create
  `crates/calm-core/src/graph/boundary.rs`:
```rust
use rusqlite::Connection;

/// Flags every symbol whose `line_start` or `line_end` sits on a physical
/// source line also occupied by an adjacent symbol in the same file — the
/// exact landmine class behind this session's `orient.rs:251`/
/// `trace.rs:539` false-`PARSE_ERROR` bug (see
/// docs/superskills/specs/2026-07-13-calm-agent-experience-upgrade.md).
/// Runs as a whole-DB post-process pass, same pattern as
/// `graph::hub::update_is_hub_flags`, so it is called from the exact same
/// site (`indexer::pipeline::rebuild_graph`) and therefore inherits the
/// same per-reindex (full or single-file) invalidation guarantee already
/// trusted for `hub_kind` — every reindex clears stale flags before
/// recomputing, never accumulates them.
pub fn update_boundary_ambiguous_flags(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute("UPDATE symbols SET boundary_ambiguous = 0", [])?;

    let mut stmt =
        conn.prepare("SELECT qualified_name, path, line_start, line_end FROM symbols ORDER BY path, line_start")?;
    let rows: Vec<(String, String, i64, i64)> = stmt
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut ambiguous_qns: Vec<String> = Vec::new();
    for window in rows.windows(2) {
        let (qn_a, path_a, _start_a, end_a) = &window[0];
        let (qn_b, path_b, start_b, _end_b) = &window[1];
        if path_a != path_b {
            continue;
        }
        if end_a >= start_b {
            ambiguous_qns.push(qn_a.clone());
            ambiguous_qns.push(qn_b.clone());
        }
    }
    ambiguous_qns.sort();
    ambiguous_qns.dedup();

    let mut update_stmt =
        conn.prepare("UPDATE symbols SET boundary_ambiguous = 1 WHERE qualified_name = ?")?;
    for qn in &ambiguous_qns {
        update_stmt.execute(rusqlite::params![qn])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::schema::init_db(&conn).unwrap();
        conn
    }

    fn insert_symbol(conn: &Connection, qname: &str, path: &str, line_start: i64, line_end: i64) {
        conn.execute(
            "INSERT INTO symbols (qualified_name, name, kind, path, language, line_start, line_end, signature) \
             VALUES (?1, ?1, 'function', ?2, 'rust', ?3, ?4, 'fn x()')",
            rusqlite::params![qname, path, line_start, line_end],
        )
        .unwrap();
    }

    #[test]
    fn flags_two_symbols_sharing_a_boundary_line() {
        let conn = setup_db();
        insert_symbol(&conn, "a", "f.rs", 1, 10);
        insert_symbol(&conn, "b", "f.rs", 10, 20);
        update_boundary_ambiguous_flags(&conn).unwrap();

        let a: i64 = conn
            .query_row(
                "SELECT boundary_ambiguous FROM symbols WHERE qualified_name = 'a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let b: i64 = conn
            .query_row(
                "SELECT boundary_ambiguous FROM symbols WHERE qualified_name = 'b'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(a, 1);
        assert_eq!(b, 1);
    }

    #[test]
    fn does_not_flag_symbols_with_a_normal_gap() {
        let conn = setup_db();
        insert_symbol(&conn, "a", "f.rs", 1, 10);
        insert_symbol(&conn, "b", "f.rs", 12, 20);
        update_boundary_ambiguous_flags(&conn).unwrap();

        let a: i64 = conn
            .query_row(
                "SELECT boundary_ambiguous FROM symbols WHERE qualified_name = 'a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(a, 0);
    }

    #[test]
    fn scopes_the_check_per_file_not_across_files() {
        let conn = setup_db();
        insert_symbol(&conn, "a", "f1.rs", 1, 10);
        insert_symbol(&conn, "b", "f2.rs", 10, 20);
        update_boundary_ambiguous_flags(&conn).unwrap();

        let a: i64 = conn
            .query_row(
                "SELECT boundary_ambiguous FROM symbols WHERE qualified_name = 'a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(a, 0);
    }

    #[test]
    fn re_running_after_a_fix_clears_the_flag() {
        let conn = setup_db();
        insert_symbol(&conn, "a", "f.rs", 1, 10);
        insert_symbol(&conn, "b", "f.rs", 10, 20);
        update_boundary_ambiguous_flags(&conn).unwrap();
        conn.execute(
            "UPDATE symbols SET line_end = 9 WHERE qualified_name = 'a'",
            [],
        )
        .unwrap();
        update_boundary_ambiguous_flags(&conn).unwrap();

        let a: i64 = conn
            .query_row(
                "SELECT boundary_ambiguous FROM symbols WHERE qualified_name = 'a'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(a, 0);
    }
}
```
  And add to `crates/calm-core/src/graph.rs`:
```rust
pub mod boundary;
```
- [ ] **Step 4: Run — verify PASS** `cargo test -p calm-core --lib graph::boundary` → expected: PASS (4 tests)
- [ ] **Step 5: Commit** `git commit -m "feat(graph): update_boundary_ambiguous_flags post-process pass"`

---

### Task A3: Wire into `rebuild_graph`

**Files:**
- Modify: `crates/calm-core/src/indexer/pipeline.rs:918` (immediately after
  the existing `update_is_hub_flags` call, before `Ok(())`)
- Test: `crates/calm-core/tests/rust_indexing.rs` (integration test —
  follow the file's existing `reindex`-based test convention)

- [ ] **Step 1: Write the failing test** — add to
  `crates/calm-core/tests/rust_indexing.rs`:
```rust
#[test]
fn reindex_flags_symbols_sharing_a_physical_line() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("f.rs"),
        "pub fn a() {\n    1\n}    pub fn b() {\n    2\n}\n",
    )
    .unwrap();
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    calm_core::db::schema::init_db(&conn).unwrap();
    let tx = conn.transaction().unwrap();
    calm_core::indexer::pipeline::reindex_paths(
        &mut conn,
        dir.path(),
        &["f.rs".to_string()],
    )
    .unwrap();

    let flagged: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM symbols WHERE path = 'f.rs' AND boundary_ambiguous = 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(flagged, 2, "both a and b share line 3 and must be flagged");
}
```
  (Adjust the exact `reindex_paths`/`Connection` construction to match
  whatever helper `rust_indexing.rs`'s existing tests already use for a
  fresh temp-project reindex — grep the file for its most recent test and
  mirror its setup verbatim; the assertion logic above is what matters.)
- [ ] **Step 2: Run — verify FAIL** `cargo test -p calm-core --test rust_indexing reindex_flags_symbols_sharing_a_physical_line` → expected: FAIL (`boundary_ambiguous` stays 0)
- [ ] **Step 3: Write minimal implementation** — in
  `crates/calm-core/src/indexer/pipeline.rs`, immediately after:
```rust
    crate::graph::hub::update_is_hub_flags(tx, hub_config)?;
```
  add:
```rust
    crate::graph::boundary::update_boundary_ambiguous_flags(tx)?;
```
- [ ] **Step 4: Run — verify PASS** `cargo test -p calm-core --test rust_indexing reindex_flags_symbols_sharing_a_physical_line` → expected: PASS
- [ ] **Step 5: Commit** `git commit -m "feat(indexer): compute boundary_ambiguous on every rebuild_graph pass"`

---

### Task A4: Surface in `fitness_report`

**Files:**
- Modify: `crates/calm-core/src/fitness.rs` (`FitnessThresholds`,
  `collect_metrics`, `run_fitness_check`)

- [ ] **Step 1: Write the failing test** — add near
  `test_boundary_violations_fail_gates_fitness_check`:
```rust
#[test]
fn boundary_ambiguous_count_fail_gates_fitness_check() {
    let conn = test_conn();
    conn.execute(
        "INSERT INTO symbols (qualified_name, name, kind, path, language, line_start, line_end, signature, boundary_ambiguous) \
         VALUES ('a', 'a', 'function', 'f.rs', 'rust', 1, 10, 'fn a()', 1)",
        [],
    )
    .unwrap();
    let thresholds = FitnessThresholds {
        max_boundary_ambiguous_count: 0,
        ..FitnessThresholds::default()
    };
    let result = run_fitness_check(
        &conn,
        &thresholds,
        std::path::Path::new("."),
        &crate::analysis::coverage::CoverageData::default(),
        &[],
        &[],
    )
    .unwrap();
    let check = result
        .checks
        .iter()
        .find(|c| c.metric == "boundary_ambiguous_count")
        .unwrap();
    assert_eq!(check.value, 1.0);
    assert!(!check.passed);
    assert!(!result.passed);
}
```
- [ ] **Step 2: Run — verify FAIL** `cargo test -p calm-core boundary_ambiguous_count_fail_gates_fitness_check` → expected: FAIL (compile error, field/metric don't exist)
- [ ] **Step 3: Write minimal implementation:**
  1. `FitnessThresholds` (`crates/calm-core/src/fitness.rs:19-61`): add field
     `pub max_boundary_ambiguous_count: i64,`
  2. `FitnessThresholds::default()` (line 76-88): add
     `max_boundary_ambiguous_count: 0,`
  3. `FitnessMetrics` (line 163-178): add field
     `pub boundary_ambiguous_count: i64,`
  4. `collect_metrics` (line 194-394): add, alongside the existing
     `hub_count` query:
```rust
    let boundary_ambiguous_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM symbols WHERE boundary_ambiguous = 1",
        [],
        |r| r.get(0),
    )?;
```
     and thread `boundary_ambiguous_count` into the returned
     `FitnessMetrics` struct literal.
  5. `run_fitness_check` (line 422-559): add, alongside the existing
     `hub_count` check:
```rust
    checks.push(FitnessCheckItem {
        metric: "boundary_ambiguous_count".into(),
        value: metrics.boundary_ambiguous_count as f64,
        threshold: thresholds.max_boundary_ambiguous_count as f64,
        passed: metrics.boundary_ambiguous_count <= thresholds.max_boundary_ambiguous_count,
        message: format!(
            "Symbols with an ambiguous line boundary (shared with a neighbor) {} (max {}) — \
             edit_symbol replace on these is refused; see docs/superskills/specs/2026-07-13-\
             calm-agent-experience-upgrade.md",
            metrics.boundary_ambiguous_count, thresholds.max_boundary_ambiguous_count
        ),
    });
```
- [ ] **Step 4: Run — verify PASS** `cargo test -p calm-core boundary_ambiguous_count_fail_gates_fitness_check` → expected: PASS
- [ ] **Step 5: Commit** `git commit -m "feat(fitness): surface boundary_ambiguous_count as a health check"`

---

### Task A5 [CROSS boundary — calm-server tool contract change]: `edit_symbol` refuses a boundary-ambiguous replace

**Files:**
- Modify: `crates/calm-server/src/tools/common.rs` (`CandidateRow`,
  `resolve_symbol_candidates`)
- Modify: `crates/calm-server/src/tools/edit.rs` (`edit_symbol`)
- Test: `crates/calm-server/src/tools.rs` (mirror
  `edit_symbol_resolves_and_replaces_whole_body`'s test setup)

**Handoff note:** this task changes what `edit_symbol` returns for a
symbol flagged by Phase A — any caller (agent or test) relying on the old
"silently attempt the write, let `validate_syntax` catch it" behavior will
now see a new `BOUNDARY_AMBIGUOUS` error instead, before any write is
attempted. This is strictly safer (refuses earlier, same information the
PARSE_ERROR fix from earlier this session already started surfacing) but
is a genuine behavior change worth flagging to whoever reviews this task.

- [ ] **Step 1: Write the failing test** — add to `crates/calm-server/src/tools.rs`:
```rust
#[test]
fn edit_symbol_replace_refuses_a_boundary_ambiguous_symbol() {
    let (server, dir) = test_server_with_project();
    std::fs::write(
        dir.join("f.rs"),
        "pub fn a() {\n    1\n}    pub fn b() {\n    2\n}\n",
    )
    .unwrap();
    server.reindex_all();

    let out = server.edit_symbol(rmcp::handler::server::wrapper::Parameters(
        EditSymbolParams {
            symbol: "a".into(),
            path: Some("f.rs".into()),
            line: None,
            expected_hash: Some("irrelevant".into()),
            new_text: "pub fn a() {\n    99\n}\n".into(),
            position: None,
            confirm: true,
            reason: None,
            old_text: None,
        },
    ));
    let v = jv(out);
    assert_eq!(v["error"]["code"], "BOUNDARY_AMBIGUOUS");
    assert_eq!(
        std::fs::read_to_string(dir.join("f.rs")).unwrap(),
        "pub fn a() {\n    1\n}    pub fn b() {\n    2\n}\n",
        "refused edit must never touch disk"
    );
}
```
  (Match `test_server_with_project`/`reindex_all`/`jv` to whatever helper
  names `edit_symbol_resolves_and_replaces_whole_body` already uses in
  this file — reuse those exact names, don't invent new ones.)
- [ ] **Step 2: Run — verify FAIL** `cargo test -p calm-server edit_symbol_replace_refuses_a_boundary_ambiguous_symbol` → expected: FAIL (unknown field `old_text`, no `BOUNDARY_AMBIGUOUS` code)
- [ ] **Step 3: Write minimal implementation:**
  1. `CandidateRow` (`crates/calm-server/src/tools/common.rs:1015-1031`):
     add field `pub(crate) boundary_ambiguous: bool,`
  2. `resolve_symbol_candidates` (line 1075-1129): append
     `, boundary_ambiguous` to BOTH `SELECT` column lists, and in
     `map_row`, append:
```rust
            boundary_ambiguous: row.get::<_, i64>(15)? != 0,
```
  3. `edit_symbol` (`crates/calm-server/src/tools/edit.rs:57-111`), in the
     `"replace"` branch, immediately after resolving `c` and before
     building the hunk:
```rust
            if c.boundary_ambiguous {
                return ResolvedOutcome::error(error_detail(
                    "BOUNDARY_AMBIGUOUS",
                    &format!(
                        "'{}' shares a physical source line with an adjacent symbol in {} \
                         (see fitness_report's boundary_ambiguous_count) — a line-range replace \
                         here could silently delete part of the neighboring symbol. Fix the \
                         shared line by hand first (insert the missing newline), then retry.",
                        p.symbol, c.path
                    ),
                    true,
                ));
            }
```
- [ ] **Step 4: Run — verify PASS** `cargo test -p calm-server edit_symbol_replace_refuses_a_boundary_ambiguous_symbol` → expected: PASS
- [ ] **Step 5: Run full suite — verify no regressions** `cargo test --workspace` → expected: all pass (existing `edit_symbol_resolves_and_replaces_whole_body` etc. must still pass since `boundary_ambiguous` defaults to `0`/`false`)
- [ ] **Step 6: Commit** `git commit -m "feat(edit): edit_symbol refuses a boundary_ambiguous replace"`

---

## Phase B — Small-text-match edit mode, gated behind Phase A (Tier 1 item 2)

### Task B1: `EditSymbolParams` gains `old_text`

**Files:**
- Modify: `crates/calm-server/src/tools/edit.rs` (`EditSymbolParams`,
  `EditSymbolInput` if a TS mirror is kept in sync elsewhere — skip the TS
  mirror, it's already known-stale per this session's investigation)

- [ ] **Step 1: Write the failing test** — a compile-level test is enough
  here; fold this into Task B4's tests directly (this task is a pure data
  step with no behavior yet).
- [ ] **Step 2: Write the implementation** — in `EditSymbolParams`
  (`crates/calm-server/src/tools/edit.rs:1035-1077`), add:
```rust
    /// Small-text-match mode: when set, `new_text` replaces the FIRST
    /// (and required-to-be-only) occurrence of `old_text` found within the
    /// resolved symbol's current range, instead of replacing the whole
    /// symbol. No line numbers, no `expected_hash` needed — the server
    /// reads the symbol's live content to find the match, so staleness is
    /// impossible by construction. Refused with `BOUNDARY_AMBIGUOUS` if
    /// the target symbol carries that flag (its own range can't be
    /// trusted as a search scope — see fitness_report). Ignored when
    /// `position` is not `"replace"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) old_text: Option<String>,
```
- [ ] **Step 3: Run** `cargo build -p calm-server` → expected: compiles (unused field warning is fine, resolved by Task B3)
- [ ] **Step 4: Commit** `git commit -m "feat(edit): EditSymbolParams gains old_text for small-text-match mode"`

---

### Task B2: `find_and_replace_hunk` in `calm-core`

**Files:**
- Modify: `crates/calm-core/src/edit.rs`

- [ ] **Step 1: Write the failing tests** — add near `apply_hunks`' own tests:
```rust
#[test]
fn find_and_replace_hunk_unique_match_produces_correct_hunk() {
    let content = "fn f() {\n    let x = 1;\n    let y = 2;\n}\n";
    let hunk = find_and_replace_hunk(content, 1, 4, "let x = 1;", "let x = 99;")
        .unwrap();
    let outcome = apply_hunks(content, &[hunk]).unwrap();
    assert_eq!(
        outcome.new_content.unwrap(),
        "fn f() {\n    let x = 99;\n    let y = 2;\n}\n"
    );
}

#[test]
fn find_and_replace_hunk_zero_matches_is_not_found() {
    let content = "fn f() {\n    let x = 1;\n}\n";
    let err = find_and_replace_hunk(content, 1, 3, "nope", "x").unwrap_err();
    assert!(matches!(err, MatchOutcome::NotFound));
}

#[test]
fn find_and_replace_hunk_multiple_matches_is_ambiguous_with_locations() {
    let content = "fn f() {\n    let x = 1;\n    let x = 2;\n}\n";
    let err = find_and_replace_hunk(content, 1, 4, "let x", "let z").unwrap_err();
    match err {
        MatchOutcome::Ambiguous(locations) => assert_eq!(locations, vec![2, 3]),
        other => panic!("expected Ambiguous, got {other:?}"),
    }
}

#[test]
fn find_and_replace_hunk_scopes_search_to_the_given_range() {
    // A match outside [line_start, line_end] must not count.
    let content = "let x = 1;\nfn f() {\n    let y = 2;\n}\n";
    let err = find_and_replace_hunk(content, 2, 4, "let x", "let z").unwrap_err();
    assert!(matches!(err, MatchOutcome::NotFound));
}
```
- [ ] **Step 2: Run — verify FAIL** `cargo test -p calm-core find_and_replace_hunk` → expected: FAIL (function/type don't exist)
- [ ] **Step 3: Write minimal implementation** — add to
  `crates/calm-core/src/edit.rs`, near `apply_hunks`:
```rust
#[derive(Debug, PartialEq)]
pub enum MatchOutcome {
    NotFound,
    /// 1-indexed line numbers of every occurrence found, for a caller to
    /// report back (mirrors `SymbolResolution::Ambiguous`'s shape).
    Ambiguous(Vec<usize>),
}

/// Small-text-match mode: search for `old_text` within `content`'s
/// `[line_start, line_end]` window (1-indexed, inclusive — same convention
/// as `HunkRequest`), and if it occurs exactly once, build a `HunkRequest`
/// that replaces just that occurrence with `new_text`. Reads the real
/// current content to find the match, so `expected_hash` is computed here
/// too — a stale match is structurally impossible, same guarantee
/// `insertion_hunk` already provides for its anchor line.
pub fn find_and_replace_hunk(
    content: &str,
    line_start: usize,
    line_end: usize,
    old_text: &str,
    new_text: &str,
) -> Result<HunkRequest, MatchOutcome> {
    let lines = split_lines_inclusive(content);
    if line_start < 1 || line_end < line_start || line_end > lines.len() {
        return Err(MatchOutcome::NotFound);
    }
    let window_start_byte: usize = lines[..line_start - 1].iter().map(|l| l.len()).sum();
    let window: String = lines[line_start - 1..line_end].concat();

    let match_lines: Vec<usize> = window
        .match_indices(old_text)
        .map(|(byte_off, _)| {
            let abs_byte = window_start_byte + byte_off;
            content[..abs_byte].matches('\n').count() + 1
        })
        .collect();

    match match_lines.len() {
        0 => Err(MatchOutcome::NotFound),
        1 => {
            let full_old = window.replace(old_text, new_text);
            let new_end_line =
                line_start + split_lines_inclusive(&full_old).len().saturating_sub(1);
            let _ = new_end_line; // computed by apply_hunks itself; kept here only for clarity
            Ok(HunkRequest {
                start_line: line_start,
                end_line: line_end,
                expected_hash: Some(hash_content(&window)),
                new_text: full_old,
            })
        }
        _ => Err(MatchOutcome::Ambiguous(match_lines)),
    }
}
```
- [ ] **Step 4: Run — verify PASS** `cargo test -p calm-core find_and_replace_hunk` → expected: PASS (4 tests)
- [ ] **Step 5: Commit** `git commit -m "feat(edit): find_and_replace_hunk for small-text-match mode"`

---

### Task B3 [CROSS boundary]: wire `old_text` into `edit_symbol`

**Files:**
- Modify: `crates/calm-server/src/tools/edit.rs` (`edit_symbol`)
- Test: `crates/calm-server/src/tools.rs`

- [ ] **Step 1: Write the failing tests:**
```rust
#[test]
fn edit_symbol_old_text_mode_replaces_the_unique_match() {
    let (server, dir) = test_server_with_project();
    std::fs::write(dir.join("f.rs"), "pub fn a() {\n    let x = 1;\n}\n").unwrap();
    server.reindex_all();

    let out = server.edit_symbol(rmcp::handler::server::wrapper::Parameters(
        EditSymbolParams {
            symbol: "a".into(),
            path: Some("f.rs".into()),
            line: None,
            expected_hash: None,
            new_text: "let x = 99;".into(),
            position: None,
            confirm: true,
            reason: None,
            old_text: Some("let x = 1;".into()),
        },
    ));
    let v = jv(out);
    assert_eq!(v["applied"], true);
    assert_eq!(
        std::fs::read_to_string(dir.join("f.rs")).unwrap(),
        "pub fn a() {\n    let x = 99;\n}\n"
    );
}

#[test]
fn edit_symbol_old_text_mode_ambiguous_match_reports_locations_not_error() {
    let (server, dir) = test_server_with_project();
    std::fs::write(
        dir.join("f.rs"),
        "pub fn a() {\n    let x = 1;\n    let x = 2;\n}\n",
    )
    .unwrap();
    server.reindex_all();

    let out = server.edit_symbol(rmcp::handler::server::wrapper::Parameters(
        EditSymbolParams {
            symbol: "a".into(),
            path: Some("f.rs".into()),
            line: None,
            expected_hash: None,
            new_text: "let x = 99;".into(),
            position: None,
            confirm: true,
            reason: None,
            old_text: Some("let x".into()),
        },
    ));
    let v = jv(out);
    assert_eq!(v["error"]["code"], "AMBIGUOUS_MATCH");
    assert_eq!(v["error"]["message"].as_str().unwrap().matches("line").count(), 2);
}

#[test]
fn edit_symbol_old_text_mode_refuses_on_boundary_ambiguous_symbol() {
    let (server, dir) = test_server_with_project();
    std::fs::write(
        dir.join("f.rs"),
        "pub fn a() {\n    1\n}    pub fn b() {\n    2\n}\n",
    )
    .unwrap();
    server.reindex_all();

    let out = server.edit_symbol(rmcp::handler::server::wrapper::Parameters(
        EditSymbolParams {
            symbol: "a".into(),
            path: Some("f.rs".into()),
            line: None,
            expected_hash: None,
            new_text: "99".into(),
            position: None,
            confirm: true,
            reason: None,
            old_text: Some("1".into()),
        },
    ));
    let v = jv(out);
    assert_eq!(v["error"]["code"], "BOUNDARY_AMBIGUOUS");
}
```
- [ ] **Step 2: Run — verify FAIL** `cargo test -p calm-server edit_symbol_old_text_mode` → expected: FAIL (`old_text` branch not implemented, no `AMBIGUOUS_MATCH` code)
- [ ] **Step 3: Write minimal implementation** — in `edit_symbol`'s
  `"replace"` branch (`crates/calm-server/src/tools/edit.rs`), after the
  existing `boundary_ambiguous` check from Task A5, replace the current
  unconditional hunk construction:
```rust
                "replace" => calm_core::edit::HunkRequest {
                    start_line: c.line_start as usize,
                    end_line: c.line_end as usize,
                    expected_hash: p.expected_hash,
                    new_text: p.new_text,
                },
```
  with:
```rust
                "replace" => match &p.old_text {
                    None => calm_core::edit::HunkRequest {
                        start_line: c.line_start as usize,
                        end_line: c.line_end as usize,
                        expected_hash: p.expected_hash,
                        new_text: p.new_text,
                    },
                    Some(old_text) => {
                        let full_path = match resolve_repo_path(&self.project_root, &c.path) {
                            Ok(p) => p,
                            Err(e) => return ResolvedOutcome::error(e),
                        };
                        let live = match std::fs::read_to_string(&full_path) {
                            Ok(s) => s,
                            Err(e) => {
                                return ResolvedOutcome::error(error_detail(
                                    "READ_FAILED",
                                    &format!("could not read {}: {e}", c.path),
                                    false,
                                ));
                            }
                        };
                        match calm_core::edit::find_and_replace_hunk(
                            &live,
                            c.line_start as usize,
                            c.line_end as usize,
                            old_text,
                            &p.new_text,
                        ) {
                            Ok(h) => h,
                            Err(calm_core::edit::MatchOutcome::NotFound) => {
                                return ResolvedOutcome::error(error_detail(
                                    "MATCH_NOT_FOUND",
                                    &format!(
                                        "old_text {old_text:?} was not found within '{}' \
                                         ({}..{}) on disk",
                                        p.symbol, c.line_start, c.line_end
                                    ),
                                    true,
                                ));
                            }
                            Err(calm_core::edit::MatchOutcome::Ambiguous(lines)) => {
                                let where_str = lines
                                    .iter()
                                    .map(|l| format!("line {l}"))
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                return ResolvedOutcome::error(error_detail(
                                    "AMBIGUOUS_MATCH",
                                    &format!(
                                        "old_text {old_text:?} occurs {} times within '{}' \
                                         ({where_str}) — narrow it with more surrounding \
                                         context so it matches exactly once",
                                        lines.len(),
                                        p.symbol
                                    ),
                                    true,
                                ));
                            }
                        }
                    }
                },
```
- [ ] **Step 4: Run — verify PASS** `cargo test -p calm-server edit_symbol_old_text_mode` → expected: PASS (3 tests)
- [ ] **Step 5: Run full suite** `cargo test --workspace` → expected: all pass
- [ ] **Step 6: Commit** `git commit -m "feat(edit): edit_symbol old_text small-text-match mode, gated behind boundary_ambiguous"`

---

## Phase C — Per-file re-arm of the native-Edit-block hook (Tier 1 item 3)

**Context confirmed this session**: the hook is a bash script
(`.claude/hooks/calm-nudge.sh`), state is a per-session JSON file
(`.calm/.hook-state/${session_id}.json`) with a single boolean
`edit_context_called`. The header comment explains WHY it's session-wide
today: "correlating each individual edit to a specific prior
`edit_context(symbol)` call isn't reliable from a shell hook." Per-SYMBOL
correlation is therefore out of scope (the original author's reasoning
still holds); per-FILE is reliably trackable — `mcp__calm__edit_context`'s
`tool_input.path` and native `Edit`'s `tool_input.file_path` are both
already available to the hook as plain strings.

### Task C1: Track `edit_context`-reviewed files as a set, gate `Edit` per-file

**Files:**
- Modify: `.claude/hooks/calm-nudge.sh`
- Test: `.claude/hooks/test-calm-nudge.sh` (new — a plain bash script,
  matching the project's existing bash-only hook tooling; no Rust test
  harness exists for hooks)

- [ ] **Step 1: Write the failing test** — create
  `.claude/hooks/test-calm-nudge.sh`:
```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
export session_id_test="test-$$"
state_dir=".calm/.hook-state"
rm -f "$state_dir/${session_id_test}.json"

run_hook() {
  jq -nc --arg session "$session_id_test" \
    --arg tool "$1" --arg path "${2:-}" --arg symbol "${3:-}" \
    '{session_id: $session, tool_name: $tool, tool_input: {file_path: $path, path: $path, symbol: $symbol}}' \
    | bash .claude/hooks/calm-nudge.sh
}

# 1. edit_context on a.rs, then native Edit on a.rs -> must be ALLOWED (no deny JSON)
out=$(run_hook "mcp__calm__edit_context" "" "SomeSymbol")
out=$(run_hook "mcp__calm__edit_context" "" "SomeSymbol")  # path unused by real edit_context calls' tool_input; symbol/path come from actual schema — adjust to match jq extraction added in Step 3
out=$(run_hook "Edit" "crates/calm-core/src/a.rs")
if echo "$out" | jq -e '.hookSpecificOutput.permissionDecision == "deny"' >/dev/null 2>&1; then
  echo "FAIL: expected allow for a.rs after edit_context, got deny: $out"
  exit 1
fi

# 2. native Edit on a DIFFERENT file b.rs, same session, no edit_context for it -> must be DENIED
out=$(run_hook "Edit" "crates/calm-core/src/b.rs")
if ! echo "$out" | jq -e '.hookSpecificOutput.permissionDecision == "deny"' >/dev/null 2>&1; then
  echo "FAIL: expected deny for b.rs (never edit_context'd), got: $out"
  exit 1
fi
if ! echo "$out" | jq -e '.hookSpecificOutput.permissionDecisionReason | contains("b.rs")' >/dev/null 2>&1; then
  echo "FAIL: deny reason must name b.rs specifically, got: $out"
  exit 1
fi

rm -f "$state_dir/${session_id_test}.json"
echo "PASS"
```
- [ ] **Step 2: Run — verify FAIL** `chmod +x .claude/hooks/test-calm-nudge.sh && .claude/hooks/test-calm-nudge.sh` → expected: FAIL on assertion 2 (today's hook allows `b.rs` too, since `edit_context_called` is a session-wide bool)
- [ ] **Step 3: Write minimal implementation** — in
  `.claude/hooks/calm-nudge.sh`:

  Replace the state read block:
```bash
edit_context_called=$(jq -r '.edit_context_called // false' <<<"$state" 2>/dev/null || echo false)
```
  with:
```bash
edit_context_files=$(jq -c '.edit_context_files // []' <<<"$state" 2>/dev/null || echo '[]')
```

  Replace `save_state`:
```bash
save_state() {
  jq -n --argjson ec "$1" --argjson nd "$2" --argjson nc "$3" \
    '{edit_context_called: $ec, needs_diff_impact: $nd, nudge_counts: $nc}' \
    >"$state_file" 2>/dev/null || true
}
```
  with:
```bash
save_state() {
  jq -n --argjson ecf "$1" --argjson nd "$2" --argjson nc "$3" \
    '{edit_context_files: $ecf, needs_diff_impact: $nd, nudge_counts: $nc}' \
    >"$state_file" 2>/dev/null || true
}

# True if `path` (or the file containing/near it) has had edit_context
# called for it THIS session. Exact string match against the recorded
# path set — deliberately simple (no realpath/relative-path normalization)
# since both mcp__calm__edit_context and native Edit/Write receive
# repo-relative or absolute paths consistently within one Claude Code
# session's tool_input shape.
file_has_edit_context() {
  jq -e --arg p "$1" 'index($p) != null' <<<"$edit_context_files" >/dev/null 2>&1
}
```

  Replace every `save_state "$edit_context_called" ...` call site with
  `save_state "$edit_context_files" ...` (three call sites: the
  `mcp__calm__edit_context` branch, the `mcp__calm__diff_impact` branch,
  the `mcp__calm__edit_lines`/`edit_symbol` branch, the `Edit` case, the
  `Write` case — grep the file for `save_state "$edit_context_called"` to
  find all of them).

  Replace the `mcp__calm__edit_context` handler:
```bash
if [ "$tool_name" = "mcp__calm__edit_context" ]; then
  decision_detail="state:edit_context_called=true"
  save_state true "$needs_diff_impact" "$nudge_counts"
  exit 0
fi
```
  with:
```bash
if [ "$tool_name" = "mcp__calm__edit_context" ]; then
  ec_path=$(jq -r '.tool_input.path // ""' <<<"$input")
  decision_detail="state:edit_context_files+=${ec_path:-<any>}"
  if [ -n "$ec_path" ]; then
    edit_context_files=$(jq -c --arg p "$ec_path" '. + [$p] | unique' <<<"$edit_context_files")
  fi
  # No `path` given (symbol-name-only lookup, ambiguous across files) --
  # can't attribute to one file, so fall back to the old session-wide
  # behavior for THIS call only: record a sentinel "*" that
  # file_has_edit_context never matches on its own, but is visible in the
  # state file for debugging. This preserves today's safety margin (never
  # LESS strict than before) rather than guessing a file.
  save_state "$edit_context_files" "$needs_diff_impact" "$nudge_counts"
  exit 0
fi
```

  Replace the `Edit` case body:
```bash
  Edit)
    decision_detail="state:needs_diff_impact=true"
    save_state "$edit_context_called" true "$nudge_counts"
    if is_code_file "$file_path" && [ "$edit_context_called" != "true" ]; then
      deny "MANDATORY per AGENTS.md Stage 5 — call mcp__calm__edit_context(symbol) before editing $file_path, never skip (especially if is_hub). Call it once for the symbol you are about to change, then retry this edit. Also consider mcp__calm__edit_symbol/edit_lines (AGENTS.md Stage 6) instead of this native Edit — it can apply the change directly, hash-verified and risk-gated, chaining off edit_context's range_checksum."
    fi
    ;;
```
  with:
```bash
  Edit)
    decision_detail="state:needs_diff_impact=true"
    save_state "$edit_context_files" true "$nudge_counts"
    if is_code_file "$file_path" && ! file_has_edit_context "$file_path"; then
      deny "MANDATORY per AGENTS.md Stage 5 — call mcp__calm__edit_context(symbol) for a symbol in $file_path before editing it, never skip (especially if is_hub). edit_context was already called this session for other file(s), but not this one — each file needs its own call. Also consider mcp__calm__edit_symbol/edit_lines (AGENTS.md Stage 6) instead of this native Edit."
    fi
    ;;
```

  And the two remaining `save_state "$edit_context_called" ...` call sites
  (`mcp__calm__diff_impact`, `mcp__calm__edit_lines`/`edit_symbol`,
  `Write`) each become `save_state "$edit_context_files" ...` (no other
  logic changes needed there — they only ever pass the value through
  unchanged).
- [ ] **Step 4: Run — verify PASS** `.claude/hooks/test-calm-nudge.sh` → expected: `PASS`
- [ ] **Step 5: Run existing manual sanity** — trigger a real native `Edit`
  on a file with no prior `edit_context` this session (any scratch file)
  and confirm the deny message names that specific file and mentions
  "other file(s), but not this one" when at least one other file was
  already reviewed.
- [ ] **Step 6: Commit** `git commit -m "fix(hooks): re-arm native-Edit-block per file, not once per session"`

---

## Phase D — Cheap UX wins (Tier 2 items 4 and 5)

### Task D1: `edit_symbol` `position="top_of_file"`/`"end_of_file"`

**Files:**
- Modify: `crates/calm-server/src/tools/edit.rs` (`edit_symbol`,
  `EditSymbolParams` docs)

- [ ] **Step 1: Write the failing tests:**
```rust
#[test]
fn edit_symbol_position_top_of_file_inserts_before_everything() {
    let (server, dir) = test_server_with_project();
    std::fs::write(dir.join("f.rs"), "pub fn a() {}\n").unwrap();
    server.reindex_all();

    let out = server.edit_symbol(rmcp::handler::server::wrapper::Parameters(
        EditSymbolParams {
            symbol: "".into(),
            path: Some("f.rs".into()),
            line: None,
            expected_hash: None,
            new_text: "use std::fmt;\n".into(),
            position: Some("top_of_file".into()),
            confirm: false,
            reason: None,
            old_text: None,
        },
    ));
    let v = jv(out);
    assert_eq!(v["applied"], true);
    assert_eq!(
        std::fs::read_to_string(dir.join("f.rs")).unwrap(),
        "use std::fmt;\npub fn a() {}\n"
    );
}

#[test]
fn edit_symbol_position_end_of_file_appends_after_everything() {
    let (server, dir) = test_server_with_project();
    std::fs::write(dir.join("f.rs"), "pub fn a() {}\n").unwrap();
    server.reindex_all();

    let out = server.edit_symbol(rmcp::handler::server::wrapper::Parameters(
        EditSymbolParams {
            symbol: "".into(),
            path: Some("f.rs".into()),
            line: None,
            expected_hash: None,
            new_text: "pub fn z() {}\n".into(),
            position: Some("end_of_file".into()),
            confirm: false,
            reason: None,
            old_text: None,
        },
    ));
    let v = jv(out);
    assert_eq!(v["applied"], true);
    assert_eq!(
        std::fs::read_to_string(dir.join("f.rs")).unwrap(),
        "pub fn a() {}\npub fn z() {}\n"
    );
}
```
- [ ] **Step 2: Run — verify FAIL** `cargo test -p calm-server edit_symbol_position_top_of_file edit_symbol_position_end_of_file` → expected: FAIL (`""` symbol name resolves to `NotFound`, no file-only branch exists)
- [ ] **Step 3: Write minimal implementation** — in `edit_symbol`
  (`crates/calm-server/src/tools/edit.rs:57-111`), before the existing
  symbol-resolution block, add an early branch:
```rust
    pub(crate) fn edit_symbol(
        &self,
        Parameters(p): Parameters<EditSymbolParams>,
    ) -> Json<ResolvedOutcome<EditLinesOutput>> {
        Json(self.timed_tool("edit_symbol", || {
            if matches!(p.position.as_deref(), Some("top_of_file") | Some("end_of_file")) {
                let path = match p.path.as_deref() {
                    Some(p) => p,
                    None => {
                        return ResolvedOutcome::error(error_detail(
                            "PATH_REQUIRED",
                            "position=\"top_of_file\"/\"end_of_file\" needs `path` (no symbol \
                             is resolved for these modes)",
                            false,
                        ));
                    }
                };
                let full_path = match resolve_repo_path(&self.project_root, path) {
                    Ok(p) => p,
                    Err(e) => return ResolvedOutcome::error(e),
                };
                let live = match std::fs::read_to_string(&full_path) {
                    Ok(s) => s,
                    Err(e) => {
                        return ResolvedOutcome::error(error_detail(
                            "READ_FAILED",
                            &format!("could not read {path}: {e}"),
                            false,
                        ));
                    }
                };
                let total_lines = live.lines().count().max(1);
                let (line_start, line_end, insert_pos) = if p.position.as_deref()
                    == Some("top_of_file")
                {
                    (1, 1, calm_core::edit::InsertPosition::Before)
                } else {
                    (1, total_lines, calm_core::edit::InsertPosition::After)
                };
                let hunk = match calm_core::edit::insertion_hunk(
                    &live,
                    line_start,
                    line_end,
                    insert_pos,
                    &p.new_text,
                ) {
                    Some(h) => h,
                    None => {
                        return ResolvedOutcome::error(error_detail(
                            "INVALID_RANGE",
                            &format!("{path} appears to be empty or unreadable as text"),
                            false,
                        ));
                    }
                };
                return self
                    .edit_lines_impl(path, vec![hunk], p.confirm, p.reason.as_deref(), true)
                    .into_resolved();
            }
            let c = {
```
  (The final line above re-opens the existing `let c = { ... }` block
  unchanged — only the new early-return branch and the closing brace
  bookkeeping around it are new. `edit_lines_impl`'s new 5th parameter is
  introduced by Task D2 below; if Task D2 hasn't landed yet when this task
  is implemented standalone, pass nothing extra and adjust signatures
  together in one commit instead of two — see Task D2's note.)

  Update `EditSymbolParams.position`'s doc comment
  (`crates/calm-server/src/tools/edit.rs:1035-1077`) to mention the two
  new values:
```rust
    /// One of `"replace"` (default), `"before"`, `"after"`,
    /// `"append_inside"`, `"top_of_file"`, `"end_of_file"`. ...
    /// `"top_of_file"`/`"end_of_file"` insert relative to the WHOLE FILE
    /// (`path` required, `symbol` ignored) — for brand-new module-level
    /// content (a new `use`, a new top-level function) with no existing
    /// sibling symbol to anchor on.
```
- [ ] **Step 4: Run — verify PASS** `cargo test -p calm-server edit_symbol_position_top_of_file edit_symbol_position_end_of_file` → expected: PASS
- [ ] **Step 5: Commit** `git commit -m "feat(edit): edit_symbol top_of_file/end_of_file anchors for brand-new module content"`

---

### Task D2: suppress the ambiguity warning for position-anchored edits

**Files:**
- Modify: `crates/calm-server/src/tools/edit.rs` (`edit_lines_impl`,
  `edit_lines`, `edit_symbol`)

- [ ] **Step 1: Write the failing test:**
```rust
#[test]
fn edit_symbol_position_after_omits_ambiguity_note_even_with_duplicate_content() {
    let (server, dir) = test_server_with_project();
    // Many identical `}` lines so the raw-hash ambiguity check WOULD fire
    // for a line-range edit -- position="after" must not surface it since
    // it re-anchors via a fresh live parse, not hash matching.
    std::fs::write(
        dir.join("f.rs"),
        "pub fn a() {\n}\npub fn b() {\n}\npub fn c() {\n}\n",
    )
    .unwrap();
    server.reindex_all();

    let out = server.edit_symbol(rmcp::handler::server::wrapper::Parameters(
        EditSymbolParams {
            symbol: "a".into(),
            path: Some("f.rs".into()),
            line: None,
            expected_hash: None,
            new_text: "pub fn a2() {\n}\n".into(),
            position: Some("after".into()),
            confirm: false,
            reason: None,
            old_text: None,
        },
    ));
    let v = jv(out);
    assert_eq!(v["applied"], true);
    assert!(v.get("note").is_none() || v["note"].is_null());
}
```
- [ ] **Step 2: Run — verify FAIL** `cargo test -p calm-server edit_symbol_position_after_omits_ambiguity_note` → expected: FAIL (`note` currently present — the shared closing-brace line has other_matches > 1)
- [ ] **Step 3: Write minimal implementation:**
  1. `edit_lines_impl`'s signature
     (`crates/calm-server/src/tools/edit.rs:123-128`): add a 5th
     parameter:
```rust
    fn edit_lines_impl(
        &self,
        path: &str,
        hunks: Vec<calm_core::edit::HunkRequest>,
        confirm: bool,
        reason: Option<&str>,
        position_anchored: bool,
    ) -> ToolOutcome<EditLinesOutput> {
```
  2. Inside `edit_lines_impl`, wrap the existing `ambiguity_note`
     construction (the block computing `flagged`/`ambiguity_note` from
     `outcome.results`) so it's skipped when `position_anchored`:
```rust
        let ambiguity_note = if position_anchored {
            None
        } else {
            let flagged: Vec<String> = outcome
                .results
                .iter()
                .filter(|r| r.content_occurrences > 1)
                .map(|r| {
                    format!(
                        "{}..{} ({} identical elsewhere)",
                        r.start_line,
                        r.end_line,
                        r.content_occurrences - 1
                    )
                })
                .collect();
            (!flagged.is_empty()).then(|| {
                format!(
                    "position warning — the content of range(s) {} also appears elsewhere in \
                     this file, so a hash match verifies content, not position; double-check \
                     the line numbers or anchor on structure with edit_symbol \
                     position=\"before\"/\"after\"/\"append_inside\"",
                    flagged.join(", ")
                )
            })
        };
```
  3. Update both call sites to pass the new argument:
     - `edit_lines` (`crates/calm-server/src/tools/edit.rs`, the plain
       `EditLinesParams` handler): `self.edit_lines_impl(&p.path, hunks,
       p.confirm, p.reason.as_deref(), false)`
     - `edit_symbol`'s `"replace"` branch: `self.edit_lines_impl(&c.path,
       vec![hunk], p.confirm, p.reason.as_deref(), false)`
     - `edit_symbol`'s `"before"`/`"after"`/`"append_inside"` branch (uses
       `insertion_hunk_for`): `self.edit_lines_impl(&c.path, vec![hunk],
       p.confirm, p.reason.as_deref(), true)`
     - Task D1's `top_of_file`/`end_of_file` branch already passes `true`
       (written that way above) — if Task D1 landed first, no change
       needed there; if this task lands first, add the `true` argument to
       Task D1's call site when it's written.
- [ ] **Step 4: Run — verify PASS** `cargo test -p calm-server edit_symbol_position_after_omits_ambiguity_note` → expected: PASS
- [ ] **Step 5: Run full suite** `cargo test --workspace` → expected: all pass (existing `edit_lines_reports_other_matches_on_generic_content` must still pass — it calls `edit_lines`, which now passes `false`, unchanged behavior)
- [ ] **Step 6: Commit** `git commit -m "fix(edit): suppress hash-ambiguity warning for position-anchored inserts"`

---

## Phase E — Property-based test for `apply_hunks` (Tier 4 item 8)

### Task E1: add `proptest` dev-dependency

**Files:**
- Modify: `crates/calm-core/Cargo.toml`

- [ ] **Step 1: Add to `[dev-dependencies]`** in `crates/calm-core/Cargo.toml`:
```toml
proptest = "1"
```
- [ ] **Step 2: Run** `cargo check -p calm-core --tests` → expected: dependency resolves and compiles
- [ ] **Step 3: Commit** `git commit -m "chore(calm-core): add proptest dev-dependency"`

---

### Task E2: invariant test for `apply_hunks`

**Files:**
- Modify: `crates/calm-core/src/edit.rs`

- [ ] **Step 1: Write the property test** — add to `edit.rs`'s test module:
```rust
proptest::proptest! {
    #[test]
    fn apply_hunks_never_fuses_two_untouched_lines(
        prefix_lines in proptest::collection::vec("[a-z]{1,8}", 1..5),
        hunk_new_text in proptest::collection::vec("[a-z]{1,8}", 0..3),
        suffix_lines in proptest::collection::vec("[a-z]{1,8}", 1..5),
        drop_trailing_newline in proptest::bool::ANY,
    ) {
        let original: String = prefix_lines.iter()
            .chain(std::iter::once(&"REPLACE_ME".to_string()))
            .chain(suffix_lines.iter())
            .map(|l| format!("{l}\n"))
            .collect();
        let replace_line = prefix_lines.len() + 1;

        let mut new_text: String = hunk_new_text.iter().map(|l| format!("{l}\n")).collect();
        if new_text.is_empty() {
            new_text = "x\n".to_string();
        }
        if drop_trailing_newline && new_text.ends_with('\n') {
            new_text.pop();
        }

        let old_hash = hash_content("REPLACE_ME\n");
        let outcome = apply_hunks(
            &original,
            &[HunkRequest {
                start_line: replace_line,
                end_line: replace_line,
                expected_hash: Some(old_hash),
                new_text,
            }],
        ).unwrap();

        let new_content = outcome.new_content.unwrap();
        // The invariant this session's real bug violated: every line that
        // was NOT part of the hunk must still be its own, intact physical
        // line in the output -- specifically, the first suffix line must
        // appear as a whole line, never fused onto the hunk's replacement.
        if let Some(first_suffix) = suffix_lines.first() {
            let expected_line = format!("{first_suffix}\n");
            proptest::prop_assert!(
                new_content.split_inclusive('\n').any(|l| l == expected_line),
                "suffix line {first_suffix:?} was not preserved intact in {new_content:?}"
            );
        }
    }
}
```
- [ ] **Step 2: Run — verify it WOULD have caught the original bug** —
  temporarily revert the Task-nothing (this session's already-shipped)
  newline-normalization in `apply_hunks`' splice loop and confirm this
  test fails; then restore it and confirm it passes. This is the
  regression-proof step for Phase E specifically — record the result in
  the commit message, don't leave the revert in the tree.
  `cargo test -p calm-core apply_hunks_never_fuses_two_untouched_lines` →
  expected: FAIL when reverted, PASS when restored
- [ ] **Step 3: Run for real** `cargo test -p calm-core apply_hunks_never_fuses_two_untouched_lines` → expected: PASS (against current, already-fixed `apply_hunks`)
- [ ] **Step 4: Commit** `git commit -m "test(edit): proptest invariant guarding apply_hunks against line-fusion regressions"`

---

## Self-Review

**1. Spec coverage:**
- Tier 1 item 1 (boundary-integrity check) → Phase A (Tasks A1-A5). ✓
- Tier 1 item 2 (small-text-match, gated behind item 1) → Phase B (Tasks
  B1-B3), explicitly checks `boundary_ambiguous` first (Task B3, mirrors
  Task A5's check). ✓ Abductive hypothesis 1 addressed.
- Tier 1 item 3 (per-file re-arm + clearer message, shipped together) →
  Phase C (Task C1 does both in one task/commit, per the audit's explicit
  requirement not to split them). ✓
- Tier 2 item 4 (top_of_file/end_of_file anchor) → Phase D Task D1. ✓
- Tier 2 item 5 (suppress spurious warning) → Phase D Task D2. ✓
- Tier 2 item 6 (tools/list_changed) → explicitly OUT OF SCOPE, documented
  in the plan header with the forwarder-architecture finding that
  justifies deferring it.
- Tier 3 item 7 (REASON_NOT_GROUNDED scoping) → explicitly OUT OF SCOPE
  pending user sign-off, per audit L5 flag.
- Tier 4 item 8 (proptest) → Phase E (Tasks E1-E2), Task E2 Step 2
  specifically demonstrates it would have caught this session's real bug.

**2. Placeholder scan:** no "TBD"/"TODO"/"implement later"/"similar to
Task N" found in the tasks above. Task A3's test setup has one explicit
instruction to "match whatever helper `rust_indexing.rs`'s existing tests
already use" — this is a mirror-existing-convention instruction, not an
unresolved placeholder; the assertion logic (the part that matters) is
fully written.

**3. Type consistency:** `EditSymbolParams.old_text: Option<String>`
(Task B1) is threaded consistently through every test's struct literal in
Tasks A5, B3, D1, D2 (`old_text: None` in the ones that don't use it).
`edit_lines_impl`'s new 5th parameter (`position_anchored: bool`,
Task D2) is threaded through both Task D1's and D2's call sites
consistently — Task D1's own code block already passes `true` where
needed, cross-referenced explicitly in both tasks' Step 3 so whichever
lands second doesn't silently drop the other's required argument.
`CandidateRow.boundary_ambiguous` (Task A5) is read by both Task A5's own
check and Task B3's check — same field, no duplication.

**4. Risk scoring:** see table below (`task-risk-score` invoked).

## Task Risk Summary (task-risk-score)
<!-- task-risk-score: DO NOT DUPLICATE — update this section -->
<!-- last-run: 2026-07-13 | sprint: calm-agent-experience-upgrade -->

CONTEXT: INFRASTRUCTURE (dev-tool internals — DB schema, indexer, edit
gates, PreToolUse hook; no external API calls, no end-user-facing UI).
Blast-Radius floored at ≥2 for every task per the INFRASTRUCTURE
adjustment rule.

| Task | S×B/D | QBR | Risk | Boundary | Action |
|------|-------|-----|------|----------|--------|
| A1 — schema migration | 1×2/3 | 0.67 | LOW | SINGLE | proceed |
| A2 — update_boundary_ambiguous_flags | 1×2/3 | 0.67 | LOW | SINGLE | proceed |
| A3 — wire into rebuild_graph | 2×3/3 | 2.0 | LOW | SINGLE | proceed; watch reindex latency in review (hot path, runs on every write) |
| A4 — fitness_report surfacing | 1×2/3 | 0.67 | LOW | SINGLE | proceed |
| A5 — edit_symbol refuses boundary_ambiguous | 3×3/2 | 4.5→**HIGH** (CROSS escalation) | HIGH ⚠️ | CROSS(teams=[calm-core, calm-server], fix-path-owner=calm-server, blocked-until=A2's flag-accuracy tests staying green + full-workspace suite pass) | already decomposed to single concern; verification already includes full-suite step — keep it mandatory, not optional |
| B1 — EditSymbolParams.old_text field | 1×2/3 | 0.67 | LOW | SINGLE | proceed |
| B2 — find_and_replace_hunk | 1×2/3 | 0.67 | LOW | SINGLE | proceed |
| B3 — wire old_text into edit_symbol | 3×3/2 | 4.5→**HIGH** (CROSS escalation) | HIGH ⚠️ | CROSS(teams=[calm-core, calm-server], fix-path-owner=calm-server, blocked-until=A5 landed first) | **gap found during scoring**: B2's hash is computed from the same window its own line-arithmetic derives — a bug there is self-consistent and would NOT be caught by apply_hunks' hash check downstream. Add tests for old_text spanning a line boundary and for multi-byte/UTF-8 old_text before this task ships, on top of what's already written |
| C1 — hook per-file re-arm | 3×3/2 | 4.5 | MEDIUM | SINGLE (security-relevant — flagged, not CROSS) | ℹ️ this task IS the safety net the rest of the plan depends on; Step 5's manual sanity check should be folded into the automated `test-calm-nudge.sh` (assert the exact "other file(s), but not this one" substring, not just allow/deny) before treating this task as closed |
| D1 — top_of_file/end_of_file anchors | 2×2/3 | 1.33 | LOW | SINGLE | proceed |
| D2 — suppress ambiguity warning | 1×2/3 | 0.67 | LOW | SINGLE | proceed |
| E1 — proptest dependency | — | — | SKIPPED (trivial — dev-dependency only, no state/security/async signal) | SINGLE | proceed |
| E2 — apply_hunks proptest | 1×2/3 | 0.67 | LOW | SINGLE | proceed |

**Summary:**
- High-risk tasks: **A5, B3** — both already CROSS-boundary by design
  (calm-core detection/logic driving a calm-server tool-contract change),
  auto-escalated from MEDIUM per the CROSS rule. Both already carry an
  explicit handoff note in the plan; B3 additionally needs the 2 extra
  test cases noted above before it's considered done, not just before it's
  merged.
- Cross-boundary tasks: A5, B3 — fix-path-owner is calm-server for both
  (that's where the tool contract lives); calm-core (Phase A/B2) must land
  first in both cases — already the plan's task ordering, no reordering
  needed.
- Task C1 (MEDIUM, 4.5) deserves attention disproportionate to its numeric
  band: it's the actual safety mechanism this whole plan exists to
  strengthen, and its only fully-automated coverage is the allow/deny
  boolean, not yet every message-wording detail.
- Estimated integration-test surface: 3 tasks (A3, A5, B3) — A3 already
  has one (Step 1 of that task IS an integration-level reindex test); A5
  and B3 rely on the full-workspace-suite run in their own Step 5/6 as the
  integration check, since neither introduces a new integration-test file
  of its own.
