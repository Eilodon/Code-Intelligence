//! Upgrade existing Rust call edges to `formal` confidence using SCIP evidence.
//! ADDITIVE ONLY: never inserts, deletes, or downgrades an edge (ADR-0004 §3).

use std::collections::HashMap;

use rusqlite::Connection;

use super::parse::ScipOccurrence;

/// Match SCIP occurrences against existing call edges and upgrade the confidence
/// of each corroborated edge to `formal`. Returns the number of edges upgraded.
///
/// Matching (conservative): a call edge `(from_path, call_site_line) -> to_symbol`
/// is corroborated when there is a non-local SCIP reference at
/// `(from_path, call_site_line)` whose definition occurrence lands on the same
/// file+line as `to_symbol`'s declaration.
pub fn ingest_occurrences(conn: &Connection, occ: &[ScipOccurrence]) -> rusqlite::Result<usize> {
    // moniker -> (def_file, def_line)
    let mut def_of: HashMap<&str, (&str, usize)> = HashMap::new();
    for o in occ {
        if o.is_def && !o.is_local {
            def_of.insert(o.symbol.as_str(), (o.file.as_str(), o.line));
        }
    }
    // (ref_file, ref_line) -> set of def sites it points to
    let mut ref_targets: HashMap<(&str, usize), Vec<(&str, usize)>> = HashMap::new();
    for o in occ {
        if !o.is_def
            && !o.is_local
            && let Some(&def) = def_of.get(o.symbol.as_str())
        {
            ref_targets
                .entry((o.file.as_str(), o.line))
                .or_default()
                .push(def);
        }
    }

    // Load candidate edges (rank below formal) joined to their target's decl site.
    let rows: Vec<(i64, String, i64, String, i64)> = {
        let mut stmt = conn.prepare(
            "SELECT ce.id, ce.from_path, ce.call_site_line, s.path, s.line_start \
             FROM call_edges ce \
             JOIN symbols s ON s.qualified_name = ce.to_symbol \
             WHERE ce.edge_confidence != 'formal' \
               AND ce.call_site_line IS NOT NULL \
               AND ce.from_path IS NOT NULL",
        )?;
        stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
    };

    let mut to_upgrade: Vec<i64> = Vec::new();
    for (id, from_path, call_line, def_path, def_line) in &rows {
        let key = (from_path.as_str(), *call_line as usize);
        if let Some(targets) = ref_targets.get(&key)
            && targets
                .iter()
                .any(|(f, l)| *f == def_path.as_str() && *l == *def_line as usize)
        {
            to_upgrade.push(*id);
        }
    }

    let mut stmt =
        conn.prepare("UPDATE call_edges SET edge_confidence = 'formal' WHERE id = ?1")?;
    for id in &to_upgrade {
        stmt.execute([id])?;
    }
    Ok(to_upgrade.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn db_with_one_textual_edge() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::schema::init_db(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO symbols (qualified_name, name, kind, language, path, line_start, line_end)
             VALUES ('core/src/engine.rs::Engine::start','start','method','rust','core/src/engine.rs',6,8);
             INSERT INTO call_edges (from_symbol, to_symbol, call_site_line, edge_confidence, from_path, to_path)
             VALUES ('app/src/main.rs::main','core/src/engine.rs::Engine::start',5,'textual','app/src/main.rs','core/src/engine.rs');",
        )
        .unwrap();
        conn
    }

    #[test]
    fn upgrades_matching_edge_to_formal() {
        let conn = db_with_one_textual_edge();
        let occ = vec![
            // def of start() at engine.rs line 6
            ScipOccurrence {
                file: "core/src/engine.rs".into(),
                line: 6,
                symbol: "M".into(),
                is_def: true,
                is_local: false,
            },
            // ref at the call site (main.rs line 5) pointing to the same moniker
            ScipOccurrence {
                file: "app/src/main.rs".into(),
                line: 5,
                symbol: "M".into(),
                is_def: false,
                is_local: false,
            },
        ];
        let n = ingest_occurrences(&conn, &occ).unwrap();
        assert_eq!(n, 1);
        let conf: String = conn
            .query_row("SELECT edge_confidence FROM call_edges", [], |r| r.get(0))
            .unwrap();
        assert_eq!(conf, "formal");
    }

    #[test]
    fn never_downgrades_or_inserts() {
        let conn = db_with_one_textual_edge();
        conn.execute("UPDATE call_edges SET edge_confidence = 'resolved'", [])
            .unwrap();
        // Occurrences that match nothing must leave the edge and count untouched.
        let occ = vec![ScipOccurrence {
            file: "zzz.rs".into(),
            line: 99,
            symbol: "X".into(),
            is_def: false,
            is_local: false,
        }];
        let n = ingest_occurrences(&conn, &occ).unwrap();
        assert_eq!(n, 0);
        let (conf, cnt): (String, i64) = conn
            .query_row(
                "SELECT edge_confidence, (SELECT COUNT(*) FROM call_edges) FROM call_edges",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(conf, "resolved");
        assert_eq!(cnt, 1);
    }
}
