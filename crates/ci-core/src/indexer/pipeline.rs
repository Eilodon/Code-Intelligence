use crate::types::IndexingPhase;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::indexer::edges::{CallEdge, insert_call_edges_batch, insert_symbols_batch};
use crate::indexer::lang_constants::language_for_extension;
use crate::indexer::parser::{ParsedSymbol, extract_calls, extract_symbols};

/// Directories never descended into during a project scan.
const IGNORE_DIRS: &[&str] = &[
    "target",
    "node_modules",
    "dist",
    "build",
    "__pycache__",
    "venv",
    "legacy",
];

/// Recursively collect tier-0 source files under `root`, skipping ignored and
/// dot-prefixed directories. Deterministic order is imposed by the caller.
fn collect_source_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && (name.starts_with('.') || IGNORE_DIRS.contains(&name))
            {
                continue;
            }
            collect_source_files(&path, out);
        } else if ft.is_file()
            && let Some(ext) = path.extension().and_then(|e| e.to_str())
            && language_for_extension(ext).is_some()
        {
            out.push(path);
        }
    }
}

fn hash_content(s: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    format!("{:016x}", h.finish())
}

fn mtime_secs(path: &Path) -> f64 {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

fn now_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

struct FileRow {
    rel: String,
    lang: &'static str,
    hash: String,
    mtime: f64,
    symbol_count: usize,
}

/// Full (re)index of a project tree into `conn`.
///
/// Pipeline: scan files → extract symbols (tree-sitter) → resolve call edges
/// (conservative, name-based) → persist atomically → compute caller_count,
/// coreness, and is_hub flags. Everything happens in one transaction so the
/// graph is never observed in a half-built state.
///
/// Phase I does a full reindex each run; incremental updates are Phase II.
pub fn run_indexing_pipeline(conn: &mut Connection, project_root: &Path) -> rusqlite::Result<()> {
    let mut files = Vec::new();
    collect_source_files(project_root, &mut files);
    files.sort();

    let mut file_rows: Vec<FileRow> = Vec::new();
    let mut all_symbols: Vec<ParsedSymbol> = Vec::new();
    // (relative file path, raw call site)
    let mut all_calls: Vec<(String, crate::indexer::parser::RawCall)> = Vec::new();

    for file in &files {
        let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
        let Some(lang) = language_for_extension(ext) else {
            continue;
        };
        let Ok(source) = std::fs::read_to_string(file) else {
            continue;
        };

        let rel = file
            .strip_prefix(project_root)
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");

        let mut syms = extract_symbols(&source, lang, &rel).unwrap_or_default();
        let symbol_count = syms.len();
        for s in &mut syms {
            s.path = rel.clone();
            s.qualified_name = format!("{}::{}", rel, s.name);
        }
        all_symbols.append(&mut syms);

        if let Ok(calls) = extract_calls(&source, lang, &rel) {
            for c in calls {
                all_calls.push((rel.clone(), c));
            }
        }

        file_rows.push(FileRow {
            rel,
            lang,
            hash: hash_content(&source),
            mtime: mtime_secs(file),
            symbol_count,
        });
    }

    // Disambiguate qualified_name collisions (same bare name within one file) so the
    // UNIQUE(qualified_name) index never rejects a real symbol.
    let mut seen: HashSet<String> = HashSet::new();
    for s in &mut all_symbols {
        if !seen.insert(s.qualified_name.clone()) {
            s.qualified_name = format!("{}#{}", s.qualified_name, s.line_start);
            seen.insert(s.qualified_name.clone());
        }
    }

    // Lookup maps for edge resolution.
    let mut qn_by_loc: HashMap<(String, String, usize), String> = HashMap::new();
    let mut qns_by_name: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for s in &all_symbols {
        qn_by_loc.insert(
            (s.path.clone(), s.name.clone(), s.line_start),
            s.qualified_name.clone(),
        );
        qns_by_name
            .entry(s.name.clone())
            .or_default()
            .push((s.qualified_name.clone(), s.path.clone()));
    }

    // Resolve call edges conservatively: bare-name match. Same-file target →
    // "resolved", otherwise "textual". Skip wildly ambiguous names (>20 matches).
    let mut call_edges: Vec<CallEdge> = Vec::new();
    for (path, c) in &all_calls {
        let Some(from_qn) =
            qn_by_loc.get(&(path.clone(), c.enclosing_name.clone(), c.enclosing_line))
        else {
            continue;
        };
        let Some(targets) = qns_by_name.get(&c.callee) else {
            continue;
        };
        if targets.len() > 20 {
            continue;
        }
        for (to_qn, to_path) in targets {
            let confidence = if to_path == path {
                "resolved"
            } else {
                "textual"
            };
            call_edges.push(CallEdge {
                from_symbol: from_qn.clone(),
                to_symbol: to_qn.clone(),
                call_site_line: Some(c.line as i32),
                edge_confidence: confidence.to_string(),
                from_path: Some(path.clone()),
                to_path: Some(to_path.clone()),
            });
        }
    }

    let now = now_secs();
    let tx = conn.transaction()?;

    // Full reindex: clear prior state. (Triggers keep the FTS tables in sync.)
    tx.execute("DELETE FROM call_edges", [])?;
    tx.execute("DELETE FROM import_edges", [])?;
    tx.execute("DELETE FROM symbols", [])?;
    tx.execute("DELETE FROM file_index", [])?;

    insert_symbols_batch(&tx, &all_symbols)?;
    insert_call_edges_batch(&tx, &call_edges)?;

    {
        let mut stmt = tx.prepare(
            "INSERT OR REPLACE INTO file_index (path, hash, language, symbol_count, last_indexed, mtime) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for f in &file_rows {
            stmt.execute(rusqlite::params![
                f.rel,
                f.hash,
                f.lang,
                f.symbol_count as i64,
                now,
                f.mtime
            ])?;
        }
    }

    // caller_count = number of distinct callers (must precede hub flagging).
    tx.execute(
        "UPDATE symbols SET caller_count = \
            (SELECT COUNT(DISTINCT from_symbol) FROM call_edges WHERE to_symbol = symbols.qualified_name)",
        [],
    )?;

    // Graph metrics on the freshly-built edges. compute_coreness persists
    // symbols.coreness; update_is_hub_flags reads caller_count + coreness.
    crate::graph::coreness::compute_coreness(&tx)?;
    let hub_config = crate::config::HubThresholdConfig::default();
    crate::graph::hub::update_is_hub_flags(&tx, &hub_config)?;

    tx.commit()?;
    Ok(())
}

pub struct IndexStateMachine {
    phase: IndexingPhase,
}

impl Default for IndexStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl IndexStateMachine {
    pub fn new() -> Self {
        Self {
            phase: IndexingPhase::Scanning,
        }
    }
    pub fn current(&self) -> IndexingPhase {
        self.phase
    }
    pub fn advance(&mut self) {
        self.phase = match self.phase {
            IndexingPhase::Scanning => IndexingPhase::Parsing,
            IndexingPhase::Parsing => IndexingPhase::BuildingEdges,
            IndexingPhase::BuildingEdges => IndexingPhase::Ready,
            IndexingPhase::Ready => IndexingPhase::Ready,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::init_db;

    #[test]
    fn test_phase_transition() {
        let mut sm = IndexStateMachine::new();
        assert_eq!(sm.current(), IndexingPhase::Scanning);
        sm.advance();
        assert_eq!(sm.current(), IndexingPhase::Parsing);
    }

    #[test]
    fn test_run_indexing_pipeline_empty_dir() {
        let dir = std::env::temp_dir().join(format!("ci_idx_empty_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        assert!(run_indexing_pipeline(&mut conn, &dir).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_run_indexing_pipeline_real_extraction() {
        let dir = std::env::temp_dir().join(format!("ci_idx_real_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("a.py"),
            "def helper():\n    pass\n\ndef main():\n    helper()\n    helper()\n",
        )
        .unwrap();

        let mut conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        run_indexing_pipeline(&mut conn, &dir).unwrap();

        // Symbols extracted and inserted (not a mock).
        let sym_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM symbols", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sym_count, 2, "expected helper + main");

        // file_index populated.
        let file_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM file_index", [], |r| r.get(0))
            .unwrap();
        assert_eq!(file_count, 1);

        // main → helper edge resolved within the same file.
        let edge_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM call_edges WHERE from_symbol = 'a.py::main' AND to_symbol = 'a.py::helper'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(edge_count >= 1, "expected main→helper call edge");

        // helper has exactly one distinct caller (main).
        let helper_callers: i64 = conn
            .query_row(
                "SELECT caller_count FROM symbols WHERE qualified_name = 'a.py::helper'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(helper_callers, 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
