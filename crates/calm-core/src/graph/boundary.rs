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

    let mut stmt = conn.prepare(
        "SELECT qualified_name, path, line_start, line_end FROM symbols ORDER BY path, line_start",
    )?;
    let rows: Vec<(String, String, i64, i64)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
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
        assert_eq!(a, 0, "flag must clear once the boundary no longer overlaps");
    }
}
