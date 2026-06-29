import sqlite3
import os

db_dir = ".antigravity"
os.makedirs(db_dir, exist_ok=True)
db_path = os.path.join(db_dir, "codeindex.db")
if os.path.exists(db_path):
    os.remove(db_path)

conn = sqlite3.connect(db_path)
conn.execute("CREATE TABLE file_index (path TEXT PRIMARY KEY, hash TEXT NOT NULL, language TEXT, symbol_count INTEGER NOT NULL DEFAULT 0, last_indexed REAL NOT NULL, mtime REAL);")
conn.execute("CREATE TABLE symbols (id INTEGER PRIMARY KEY AUTOINCREMENT, qualified_name TEXT NOT NULL, name TEXT NOT NULL, kind TEXT NOT NULL, language TEXT NOT NULL, path TEXT NOT NULL, line_start INTEGER NOT NULL, line_end INTEGER NOT NULL, signature TEXT NOT NULL DEFAULT '', docstring TEXT NOT NULL DEFAULT '', name_tokens TEXT NOT NULL DEFAULT '', caller_count INTEGER NOT NULL DEFAULT 0, is_hub INTEGER NOT NULL DEFAULT 0, coreness INTEGER, is_entry_point INTEGER NOT NULL DEFAULT 0, file_hash TEXT NOT NULL DEFAULT '', indexed_at REAL NOT NULL DEFAULT 0);")
conn.execute("CREATE TABLE call_edges (id INTEGER PRIMARY KEY AUTOINCREMENT, from_symbol TEXT NOT NULL, to_symbol TEXT NOT NULL, call_site_line INTEGER, edge_confidence TEXT NOT NULL DEFAULT 'textual', from_path TEXT, to_path TEXT);")

# Insert files
conn.execute("INSERT INTO file_index (path, hash, language, symbol_count, last_indexed, mtime) VALUES ('src/a.rs', 'hash_a', 'rust', 1, 0, 0)")
conn.execute("INSERT INTO file_index (path, hash, language, symbol_count, last_indexed, mtime) VALUES ('src/b.rs', 'hash_b', 'rust', 1, 0, 0)")
conn.execute("INSERT INTO file_index (path, hash, language, symbol_count, last_indexed, mtime) VALUES ('src/c.rs', 'hash_c', 'rust', 1, 0, 0)")

# Insert symbols (A calls B, B calls C)
conn.execute("INSERT INTO symbols (qualified_name, name, kind, language, path, line_start, line_end, caller_count, is_hub, coreness) VALUES ('A', 'A', 'function', 'rust', 'src/a.rs', 1, 10, 0, 0, 0)")
conn.execute("INSERT INTO symbols (qualified_name, name, kind, language, path, line_start, line_end, caller_count, is_hub, coreness) VALUES ('B', 'B', 'function', 'rust', 'src/b.rs', 1, 10, 1, 0, 0)")
conn.execute("INSERT INTO symbols (qualified_name, name, kind, language, path, line_start, line_end, caller_count, is_hub, coreness) VALUES ('C', 'C', 'function', 'rust', 'src/c.rs', 1, 10, 1, 0, 0)")

# Insert edges
conn.execute("INSERT INTO call_edges (from_symbol, to_symbol, call_site_line, from_path, to_path) VALUES ('A', 'B', 5, 'src/a.rs', 'src/b.rs')")
conn.execute("INSERT INTO call_edges (from_symbol, to_symbol, call_site_line, from_path, to_path) VALUES ('B', 'C', 5, 'src/b.rs', 'src/c.rs')")

conn.commit()
conn.close()
print("Synthetic DB created.")
