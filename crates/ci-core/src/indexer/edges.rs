use rusqlite::Transaction;
use super::parser::ParsedSymbol;

pub fn insert_symbols_batch(tx: &Transaction, symbols: &[ParsedSymbol]) -> rusqlite::Result<()> {
    let mut stmt = tx.prepare(
        "INSERT INTO symbols (qualified_name, name, kind, language, path, line_start, line_end, signature, docstring, name_tokens, is_entry_point)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
    )?;
    
    for sym in symbols {
        stmt.execute(rusqlite::params![
            sym.qualified_name, sym.name, sym.kind.as_str(), sym.language, sym.path,
            sym.line_start, sym.line_end, sym.signature, sym.docstring, sym.name_tokens, sym.is_entry_point as i32
        ])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use crate::db::schema::init_db;
    use crate::types::SymbolKind;

    #[test]
    fn test_insert_symbols_transaction() {
        let mut conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        
        let tx = conn.transaction().unwrap();
        let symbols = vec![ParsedSymbol {
            qualified_name: "test.hello".to_string(),
            name: "hello".to_string(),
            kind: SymbolKind::Function,
            language: "python".to_string(),
            path: "test.py".to_string(),
            line_start: 1,
            line_end: 2,
            signature: "".to_string(),
            docstring: "".to_string(),
            name_tokens: "hello".to_string(),
            is_entry_point: false,
        }];
        insert_symbols_batch(&tx, &symbols).unwrap();
        tx.commit().unwrap();
        
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM symbols", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }
}
