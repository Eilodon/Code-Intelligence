---
title: Code Intelligence MCP — Full Implementation Design
date: 2026-06-29
author: ybao
SPEC_APPROVED: true
SPEC_ESCALATION: false
ESCALATION_FINDING: ""
---

# Code Intelligence MCP — Full Implementation Design

## 1. Tầm nhìn & Triết lý

Đây là **Cognitive OS cho AI Agent** — không phải text search tool. Ba triết lý cốt lõi:

1. **Zero-Round-Trip**: Compound tools (`locate`, `understand`) gộp 3 MCP calls thành 1 in-process call. Latency: ~2ms thay vì ~90ms × 3.
2. **Sat-Nav Guidance**: `suggested_next` nhúng vào mọi response — agent không cần tự suy luận "gọi gì tiếp".
3. **Token Efficiency**: Tool Presets chỉ expose tools relevant với workflow stage hiện tại.

**Scope của spec này**: Tất cả layers trừ MCP protocol wiring (caller có boilerplate riêng). Output của spec là:
- `_handle_xxx(params: dict, *, ctx: ServerContext) -> dict` hoạt động đúng
- Indexer pipeline hoàn chỉnh
- CLI `serve` command minimal

---

## 2. Dependency Stack

| Package | Version | Justification |
|---------|---------|---------------|
| `tree-sitter-languages` | 1.10.2 | 80+ pre-compiled grammars — không tự build |
| `pydantic` | 2.12.5 | Trust boundary + config validation (đã install) |
| `watchfiles` | 1.1.1 | File watcher (đã install) |
| `fastembed` | 0.8.0 | Opt-in ONNX embeddings, no PyTorch |
| `sqlite-vec` | 0.1.9 | Opt-in KNN vector search |
| `typer` | 0.24.1 | CLI (đã install) |
| `defusedxml` | latest | XXE prevention (optional import, đã có trong code) |

`mcp` SDK — do caller's boilerplate handle, không list ở đây.

---

## 3. Module Structure

```
codeindex/
├── schemas.py           # Pydantic I/O models — Trust Boundary Layer
├── config.py            # Config model + loader (config.json / .codeindex/config.json)
├── context.py           # ServerContext — dependency container cho mọi handler
├── cli.py               # typer CLI: serve command
│
├── db/
│   ├── __init__.py
│   ├── schema.py        # CREATE TABLE + triggers + migrations (idempotent)
│   └── queries.py       # Shared query helpers: batch callers, batch callees, etc.
│
├── indexer/
│   ├── __init__.py
│   ├── indexer.py       # Phase orchestration: scanning→parsing→building_edges→ready
│   ├── watcher.py       # watchfiles integration + 500ms debounce
│   ├── embedder.py      # fastembed + sqlite-vec state machine (opt-in)
│   └── parsers/
│       ├── __init__.py  # Parser registry + LanguageParser ABC
│       ├── base.py      # ParseResult, Symbol, CallSite, Import dataclasses
│       ├── python_.py   # Python parser (function_definition, class_definition, ...)
│       ├── typescript_.py  # TS + JS parser
│       ├── java_.py     # Java parser
│       ├── rust_.py     # Rust parser
│       ├── go_.py       # Go parser
│       └── generic_.py  # Fallback: textual edges only, no crash
│
├── tools/
│   ├── __init__.py      # Re-export _handle_xxx, _xxx_logic
│   ├── meta.py          # repo_overview, indexing_status, source
│   ├── search.py        # search, file_overview, symbol_info
│   ├── compound.py      # locate, understand (in-process orchestration)
│   ├── graph.py         # callers, callees, dependencies, path
│   └── analysis.py      # edit_context, diff_impact, hotspots, session_context
│
# Giữ nguyên (không sửa):
├── analyzer.py          # _compute_dead_code_confidence
├── codeowners.py        # CODEOWNERS matching + git blame
├── coverage_reader.py   # Coverage parsing (lcov, python, go, cobertura)
├── diff_impact.py       # get_git_diff helper
├── hotspot.py           # compute_hotspots
├── path_algo.py         # PathFinder bidirectional BFS
├── resolver/            # ConservativeResolver
│
# Update:
├── db_init.py           # Extend: thêm main schema tables + triggers
└── server.py            # Update: wire _handle_xxx → tools/, giữ suggested_next logic
```

---

## 4. Database Schema

### 4.1 Bảng chính

```sql
-- symbols: source of truth cho mọi tool
CREATE TABLE IF NOT EXISTS symbols (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    qualified_name  TEXT NOT NULL,    -- "pkg.ClassName.method" — unique cross-file
    name            TEXT NOT NULL,    -- bare name "method"
    kind            TEXT NOT NULL,    -- function|class|method|interface|variable|enum|...
    language        TEXT NOT NULL,
    path            TEXT NOT NULL,
    line_start      INTEGER NOT NULL,
    line_end        INTEGER NOT NULL,
    signature       TEXT NOT NULL DEFAULT '',
    docstring       TEXT NOT NULL DEFAULT '',
    name_tokens     TEXT NOT NULL DEFAULT '',  -- tokenize_identifier(name), pre-computed
    caller_count    INTEGER NOT NULL DEFAULT 0,
    is_hub          INTEGER NOT NULL DEFAULT 0,
    coreness        INTEGER,           -- NULL cho đến khi building_edges xong
    is_entry_point  INTEGER NOT NULL DEFAULT 0,
    file_hash       TEXT NOT NULL DEFAULT '',
    indexed_at      REAL NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_symbols_qualified ON symbols(qualified_name);
CREATE INDEX IF NOT EXISTS idx_symbols_path     ON symbols(path);
CREATE INDEX IF NOT EXISTS idx_symbols_name     ON symbols(name);
CREATE INDEX IF NOT EXISTS idx_symbols_hub      ON symbols(is_hub) WHERE is_hub = 1;

-- call_edges: call graph (from ConservativeResolver)
CREATE TABLE IF NOT EXISTS call_edges (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    from_symbol     TEXT NOT NULL,
    to_symbol       TEXT NOT NULL,
    call_site_line  INTEGER,
    edge_confidence TEXT NOT NULL DEFAULT 'textual',  -- resolved|inferred|textual
    from_path       TEXT,
    to_path         TEXT
);

CREATE INDEX IF NOT EXISTS idx_call_edges_from ON call_edges(from_symbol);
CREATE INDEX IF NOT EXISTS idx_call_edges_to   ON call_edges(to_symbol);
CREATE INDEX IF NOT EXISTS idx_call_edges_fpath ON call_edges(from_path);

-- import_edges: file-level dependency graph
CREATE TABLE IF NOT EXISTS import_edges (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    from_path     TEXT NOT NULL,
    to_path       TEXT,             -- NULL nếu external package / unresolved
    module_name   TEXT NOT NULL,
    symbols_used  TEXT DEFAULT '[]' -- JSON array: ["func_a", "ClassB"]
);

CREATE INDEX IF NOT EXISTS idx_import_from ON import_edges(from_path);
CREATE INDEX IF NOT EXISTS idx_import_to   ON import_edges(to_path);

-- file_index: trạng thái incremental indexing
CREATE TABLE IF NOT EXISTS file_index (
    path          TEXT PRIMARY KEY,
    hash          TEXT NOT NULL,
    language      TEXT,
    symbol_count  INTEGER NOT NULL DEFAULT 0,
    last_indexed  REAL NOT NULL,
    mtime         REAL
);
```

### 4.2 FTS5 Virtual Tables (content-backed)

```sql
-- fts_exact: search trên name gốc + docstring
CREATE VIRTUAL TABLE IF NOT EXISTS fts_exact USING fts5(
    name,
    docstring,
    content='symbols',
    content_rowid='id',
    tokenize='unicode61'
);

-- fts_tokens: search trên tokenized name (camelCase/snake_case split)
CREATE VIRTUAL TABLE IF NOT EXISTS fts_tokens USING fts5(
    name_tokens,
    content='symbols',
    content_rowid='id',
    tokenize='unicode61'
);
```

### 4.3 FTS5 Sync Triggers (tự động, không cần application logic)

```sql
-- INSERT
CREATE TRIGGER IF NOT EXISTS symbols_ai AFTER INSERT ON symbols BEGIN
    INSERT INTO fts_exact(rowid, name, docstring)
        VALUES (new.id, new.name, new.docstring);
    INSERT INTO fts_tokens(rowid, name_tokens)
        VALUES (new.id, new.name_tokens);
END;

-- DELETE
CREATE TRIGGER IF NOT EXISTS symbols_ad AFTER DELETE ON symbols BEGIN
    INSERT INTO fts_exact(fts_exact, rowid, name, docstring)
        VALUES ('delete', old.id, old.name, old.docstring);
    INSERT INTO fts_tokens(fts_tokens, rowid, name_tokens)
        VALUES ('delete', old.id, old.name_tokens);
END;

-- UPDATE (name hoặc docstring thay đổi)
CREATE TRIGGER IF NOT EXISTS symbols_au
    AFTER UPDATE OF name, docstring, name_tokens ON symbols BEGIN
    INSERT INTO fts_exact(fts_exact, rowid, name, docstring)
        VALUES ('delete', old.id, old.name, old.docstring);
    INSERT INTO fts_exact(rowid, name, docstring)
        VALUES (new.id, new.name, new.docstring);
    INSERT INTO fts_tokens(fts_tokens, rowid, name_tokens)
        VALUES ('delete', old.id, old.name_tokens);
    INSERT INTO fts_tokens(rowid, name_tokens)
        VALUES (new.id, new.name_tokens);
END;
```

### 4.4 Semantic Embeddings (opt-in, tạo khi enabled)

```sql
-- Tạo khi config.semantic_search.enabled = true
CREATE VIRTUAL TABLE IF NOT EXISTS embedding_vecs USING vec0(
    symbol_id INTEGER,
    embedding FLOAT[768]   -- BAAI/bge-base-en-v1.5 dimensions
);
```

---

## 5. Config System (`codeindex/config.py`)

Pydantic model mirror `ConfigJson` TypeScript type. Defaults khớp với architecture doc.

```python
from pydantic import BaseModel, Field
from pathlib import Path
import json

class HubThresholdConfig(BaseModel):
    top_pct: float = 5.0
    min_callers: int = 5
    min_callers_bridge: int = 2
    coreness_pct: float = 75.0

class SemanticSearchConfig(BaseModel):
    enabled: bool = False
    model: str = "BAAI/bge-base-en-v1.5"
    dimensions: int = 768
    index_on_startup: bool = False

class SearchConfig(BaseModel):
    text_chunk_context_lines: int = 10
    text_max_chunk_lines: int = 50
    rrf_k: int = 20

class PathConfig(BaseModel):
    default_max_hops: int = 8
    max_allowed_hops: int = 20
    timeout_ms: int = 5000

class DepthConfig(BaseModel):
    max_depth_cap: int = 4
    transitive_timeout_ms: int = 3000

class HotspotsConfig(BaseModel):
    default_top_n: int = 10
    default_since: str = "6 months ago"
    default_min_churn: int = 2
    risk_critical_threshold: float = 0.75
    risk_high_threshold: float = 0.50
    risk_medium_threshold: float = 0.25

class Config(BaseModel):
    preset: str = "full"
    languages: list[str] = ["python", "typescript", "javascript",
                             "java", "rust", "go"]
    ignore: list[str] = ["node_modules", ".git", "__pycache__",
                         "*.min.js", "dist", "build", ".venv"]
    entry_points: list[str] = []
    hub_threshold: HubThresholdConfig = Field(default_factory=HubThresholdConfig)
    semantic_search: SemanticSearchConfig = Field(default_factory=SemanticSearchConfig)
    search: SearchConfig = Field(default_factory=SearchConfig)
    path: PathConfig = Field(default_factory=PathConfig)
    callers: DepthConfig = Field(default_factory=DepthConfig)
    callees: DepthConfig = Field(default_factory=DepthConfig)
    hotspots: HotspotsConfig = Field(default_factory=HotspotsConfig)

def load_config(project_root: Path) -> Config:
    """Load config.json or .codeindex/config.json. Falls back to defaults."""
    for candidate in [
        project_root / "config.json",
        project_root / ".codeindex" / "config.json",
    ]:
        if candidate.exists():
            return Config.model_validate(json.loads(candidate.read_text()))
    return Config()
```

---

## 6. Pydantic Trust Boundary (`codeindex/schemas.py`)

### 6.1 Serialization Convention

```python
from pydantic import BaseModel, model_serializer, ConfigDict
from typing import ClassVar

# Fields listed in _absent_when_none bị DROP hoàn toàn khỏi JSON khi None
# (không emit "field": null). Mọi Output model kế thừa từ đây.
class _BaseOutput(BaseModel):
    model_config = ConfigDict(populate_by_name=True)
    _absent_when_none: ClassVar[frozenset[str]] = frozenset()

    @model_serializer(mode="wrap")
    def _serialize(self, handler) -> dict:
        d = handler(self)
        for field in self._absent_when_none:
            if d.get(field) is None:
                d.pop(field, None)
        return d
```

### 6.2 Áp dụng convention

```python
class RepoOverviewOutput(_BaseOutput):
    # "suggested_next", "note", "health_summary" → absent khi None
    _absent_when_none = frozenset({"suggested_next", "note", "health_summary"})

    languages: list[str]
    indexing_phase: Literal["scanning", "parsing", "building_edges", "ready"]
    embeddings_status: Literal["disabled", "downloading", "embedding", "ready", "failed"]
    module_map: list[ModuleMapEntry]
    total_modules: int
    truncated: bool
    entry_points: list[EntryPoint]
    stats: RepoStats
    workflow_guide: str
    # Optional-absent:
    health_summary: HealthSummary | None = None
    note: str | None = None
    suggested_next: SuggestedNext | None = None
```

### 6.3 UnifiedError (không kế thừa _BaseOutput)

```python
class UnifiedError(BaseModel):
    """Returned khi tool gặp lỗi. Mutually exclusive với normal output."""
    error: ErrorBody

class ErrorBody(BaseModel):
    code: Literal[
        "NOT_FOUND", "INDEX_PARTIAL", "PARSE_FAILED", "TIMEOUT",
        "DB_LOCKED", "INVALID_INPUT", "FEATURE_UNAVAILABLE", "EMBEDDING_FAILED"
    ]
    message: str
    recoverable: bool
    suggestions: list[str] | None = None

    @model_serializer(mode="wrap")
    def _serialize(self, handler) -> dict:
        d = handler(self)
        if d.get("suggestions") is None:
            d.pop("suggestions", None)
        return d
```

### 6.4 Error Boundary trong `_make_tool_fn`

```python
# codeindex/server.py — error boundary wrap tất cả handlers
def _make_tool_fn(handler, name: str, available_tools: set[str] | None):
    async def tool_fn(params: dict, *, ctx: Any = None) -> dict:
        try:
            output: dict = await handler(params, ctx=ctx)
        except sqlite3.OperationalError as e:
            return UnifiedError(error=ErrorBody(
                code="DB_LOCKED",
                message=str(e),
                recoverable=True,
            )).model_dump()
        except TimeoutError as e:
            return UnifiedError(error=ErrorBody(
                code="TIMEOUT",
                message=str(e),
                recoverable=True,
            )).model_dump()
        except ValidationError as e:
            return UnifiedError(error=ErrorBody(
                code="INVALID_INPUT",
                message=str(e),
                recoverable=True,
            )).model_dump()
        except Exception:
            # Unhandled → log + re-raise (không nuốt server-fatal errors)
            logger.exception("Unhandled error in tool %s", name)
            raise

        suggestion = compute_suggested_next(name, output, available_tools)
        for k in _PRIVATE_KEYS:
            output.pop(k, None)
        if suggestion is not None:
            sn: dict = {"tool": suggestion.tool, "reason": suggestion.reason}
            if suggestion.args:
                sn["args"] = suggestion.args
            output["suggested_next"] = sn
        return output
    return tool_fn
```

---

## 7. ServerContext (`codeindex/context.py`)

```python
import sqlite3
import threading
from dataclasses import dataclass, field
from pathlib import Path
from collections import OrderedDict, deque

from .config import Config
from .coverage_reader import CoverageData
from .indexer.embedder import EmbedderState

@dataclass
class SessionState:
    started_at: str
    tool_calls: int = 0
    # LRU-eviction: OrderedDict, evict oldest khi > 50
    explored_symbols: OrderedDict = field(default_factory=OrderedDict)
    explored_files: set[str] = field(default_factory=set)
    # FIFO-capped tại 200 entries
    already_fetched: deque = field(default_factory=lambda: deque(maxlen=200))
    unique_files_explored: int = 0

@dataclass
class IndexerState:
    phase: str = "scanning"   # scanning|parsing|building_edges|ready
    files_indexed: int = 0
    files_total: int = 0
    symbols_indexed: int | None = None
    edges_indexed: int | None = None
    last_updated: str = ""
    embedder: EmbedderState | None = None

@dataclass
class ServerContext:
    project_root: Path
    db_path: Path
    config: Config
    write_conn: sqlite3.Connection    # Dành riêng cho Indexer (1 writer)
    write_lock: threading.Lock        # Bảo vệ write_conn
    coverage_data: CoverageData
    codeowners_patterns: list
    indexer_state: IndexerState = field(default_factory=IndexerState)
    session: SessionState | None = None  # Khởi tạo sau khi server accept 1st request

    def make_read_conn(self) -> sqlite3.Connection:
        """Tạo read connection mới — gọi trong thread pool, đóng sau khi dùng."""
        conn = sqlite3.connect(str(self.db_path), check_same_thread=False)
        conn.execute("PRAGMA journal_mode=WAL")
        conn.execute("PRAGMA query_only=ON")  # Safety: prevent accidental writes
        conn.row_factory = sqlite3.Row
        return conn
```

**Pattern dùng trong mọi tool handler**:

```python
async def _handle_callers(params: dict, *, ctx: ServerContext) -> dict:
    validated = CallersInput.model_validate(params)
    return await asyncio.to_thread(_callers_sync, validated, ctx)

def _callers_sync(params: CallersInput, ctx: ServerContext) -> dict:
    conn = ctx.make_read_conn()
    try:
        result = _callers_logic(params, conn, ctx.config)
        return result.model_dump()
    finally:
        conn.close()
```

---

## 8. Indexer Pipeline (`codeindex/indexer/`)

### 8.1 ParseResult Dataclasses (`parsers/base.py`)

```python
@dataclass
class Symbol:
    name: str
    qualified_name: str       # "module.Class.method"
    kind: str                 # function|class|method|interface|variable|...
    line_start: int
    line_end: int
    signature: str
    docstring: str
    name_tokens: str          # tokenize_identifier(name) — pre-computed
    is_entry_point: bool = False

@dataclass
class CallSite:
    callee_name: str
    line: int
    in_symbol: str            # qualified_name của symbol chứa call site

@dataclass
class ImportEdge:
    module_name: str
    resolved_path: str | None
    symbols_used: list[str]

@dataclass
class ParseResult:
    path: str
    language: str
    file_hash: str
    symbols: list[Symbol]
    call_sites: list[CallSite]
    imports: list[ImportEdge]
```

### 8.2 LanguageParser ABC (`parsers/__init__.py`)

```python
from abc import ABC, abstractmethod
from tree_sitter_languages import get_language, get_parser

class LanguageParser(ABC):
    def __init__(self, language_name: str):
        self.lang = get_language(language_name)
        self.parser = get_parser(language_name)

    @abstractmethod
    def parse(self, source: bytes, path: str, file_hash: str) -> ParseResult: ...

    def _text(self, node) -> str:
        return node.text.decode("utf-8", errors="replace") if node else ""

# Parser registry
_PARSERS: dict[str, type[LanguageParser]] = {}

def register(ext_or_lang: str):
    def decorator(cls):
        _PARSERS[ext_or_lang] = cls
        return cls
    return decorator

def get_parser_for(language: str) -> LanguageParser | None:
    cls = _PARSERS.get(language)
    return cls() if cls else None
```

### 8.3 Phase Orchestration (`indexer/indexer.py`)

```python
class Indexer:
    def __init__(self, ctx: ServerContext):
        self.ctx = ctx

    async def run_full_index(self) -> None:
        """scanning → parsing → building_edges → ready"""
        # Phase 1: scanning
        self.ctx.indexer_state.phase = "scanning"
        dirty_files = await asyncio.to_thread(self._scan_files)

        # Phase 2: parsing
        self.ctx.indexer_state.phase = "parsing"
        parse_results = await self._parse_all(dirty_files)

        # Phase 3: building_edges
        self.ctx.indexer_state.phase = "building_edges"
        await self._build_edges(parse_results)
        await asyncio.to_thread(self._compute_graph_metrics)

        # Phase 4: ready
        self.ctx.indexer_state.phase = "ready"

    def _scan_files(self) -> list[str]:
        """Return list of paths that need re-parsing (new or hash-changed)."""
        all_files = _walk_project(self.ctx.project_root, self.ctx.config)
        conn = self.ctx.make_read_conn()
        try:
            indexed = {row["path"]: row["hash"]
                       for row in conn.execute("SELECT path, hash FROM file_index")}
        finally:
            conn.close()
        dirty = []
        for path in all_files:
            file_hash = _sha256(path)
            if indexed.get(path) != file_hash:
                dirty.append(path)
        self.ctx.indexer_state.files_total = len(all_files)
        return dirty

    async def _parse_all(self, paths: list[str]) -> list[ParseResult]:
        results = []
        for path in paths:
            result = await asyncio.to_thread(self._parse_single, path)
            if result:
                results.append(result)
                self._write_parse_result(result)
        return results

    def _parse_single(self, path: str) -> ParseResult | None:
        language = _detect_language(path, self.ctx.config)
        if not language:
            return None
        source = Path(path).read_bytes()
        file_hash = _sha256_bytes(source)
        parser = get_parser_for(language) or GenericParser()
        return parser.parse(source, path, file_hash)

    def _write_parse_result(self, result: ParseResult) -> None:
        """Atomically replace symbols + imports cho một file."""
        with self.ctx.write_lock:
            self.ctx.write_conn.execute("BEGIN IMMEDIATE")
            try:
                # Xóa data cũ
                self.ctx.write_conn.execute(
                    "DELETE FROM symbols WHERE path=?", (result.path,))
                self.ctx.write_conn.execute(
                    "DELETE FROM import_edges WHERE from_path=?", (result.path,))
                # Insert symbols mới (triggers tự sync FTS5)
                for sym in result.symbols:
                    self.ctx.write_conn.execute("""
                        INSERT INTO symbols
                          (qualified_name, name, kind, language, path,
                           line_start, line_end, signature, docstring,
                           name_tokens, is_entry_point, file_hash, indexed_at)
                        VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)
                    """, (sym.qualified_name, sym.name, sym.kind,
                          result.language, result.path,
                          sym.line_start, sym.line_end,
                          sym.signature, sym.docstring,
                          sym.name_tokens, sym.is_entry_point,
                          result.file_hash, time.time()))
                # Insert imports
                for imp in result.imports:
                    self.ctx.write_conn.execute("""
                        INSERT INTO import_edges
                          (from_path, to_path, module_name, symbols_used)
                        VALUES (?,?,?,?)
                    """, (result.path, imp.resolved_path,
                          imp.module_name, json.dumps(imp.symbols_used)))
                # Update file_index
                self.ctx.write_conn.execute("""
                    INSERT OR REPLACE INTO file_index
                      (path, hash, language, symbol_count, last_indexed)
                    VALUES (?,?,?,?,?)
                """, (result.path, result.file_hash, result.language,
                      len(result.symbols), time.time()))
                self.ctx.write_conn.commit()
            except Exception:
                self.ctx.write_conn.execute("ROLLBACK")
                raise

    async def _build_edges(self, parse_results: list[ParseResult]) -> None:
        """Chạy ConservativeResolver → insert call_edges."""
        # Build lookup maps (file_symbols, import_map) từ DB
        conn = self.ctx.make_read_conn()
        try:
            file_symbols = _load_file_symbols(conn)
            import_map   = _load_import_map(conn)
        finally:
            conn.close()

        resolver = ConservativeResolver()
        for result in parse_results:
            edges = await asyncio.to_thread(
                self._resolve_file_edges, resolver, result,
                file_symbols, import_map)
            self._write_call_edges(result.path, edges)

    def _resolve_file_edges(self, resolver, result, file_symbols, import_map):
        # Dùng ConservativeResolver đã implement
        ...

    def _write_call_edges(self, from_path: str, edges: list[tuple]) -> None:
        with self.ctx.write_lock:
            self.ctx.write_conn.execute("BEGIN IMMEDIATE")
            try:
                self.ctx.write_conn.execute(
                    "DELETE FROM call_edges WHERE from_path=?", (from_path,))
                self.ctx.write_conn.executemany("""
                    INSERT INTO call_edges
                      (from_symbol, to_symbol, call_site_line,
                       edge_confidence, from_path, to_path)
                    VALUES (?,?,?,?,?,?)
                """, edges)
                self.ctx.write_conn.commit()
            except Exception:
                self.ctx.write_conn.execute("ROLLBACK")
                raise

    def _compute_graph_metrics(self) -> None:
        """compute_coreness + update_is_hub_flags — chạy trong thread pool."""
        from .db_init import compute_coreness, update_is_hub_flags
        with self.ctx.write_lock:
            compute_coreness(self.ctx.write_conn)
            update_is_hub_flags(self.ctx.write_conn, self.ctx.config)
```

### 8.4 Incremental Update (file watcher trigger)

```python
async def reindex_changed(ctx: ServerContext, changed_paths: list[str]) -> None:
    """Cascade 1-hop: primary dirty + direct importers."""
    primary = set(changed_paths)

    # Tìm files import primary files (1-hop cascade)
    conn = ctx.make_read_conn()
    try:
        placeholders = ",".join("?" * len(primary))
        rows = conn.execute(
            f"SELECT DISTINCT from_path FROM import_edges WHERE to_path IN ({placeholders})",
            list(primary)
        ).fetchall()
        cascade = {row[0] for row in rows if row[0]}
    finally:
        conn.close()

    all_dirty = primary | cascade

    # Re-parse tất cả (bao gồm cascade) để rebuild edges đúng
    indexer = Indexer(ctx)
    for path in all_dirty:
        result = await asyncio.to_thread(indexer._parse_single, path)
        if result:
            indexer._write_parse_result(result)

    # Rebuild edges cho all_dirty trong 1 transaction
    conn2 = ctx.make_read_conn()
    try:
        file_symbols = _load_file_symbols(conn2)
        import_map   = _load_import_map(conn2)
    finally:
        conn2.close()

    resolver = ConservativeResolver()
    for path in all_dirty:
        # Lấy call_sites từ DB hoặc re-parse (tùy implementation)
        edges = await asyncio.to_thread(
            indexer._resolve_file_edges, resolver, path, file_symbols, import_map)
        indexer._write_call_edges(path, edges)

    # Recompute graph metrics với debounce
    await _debounced_recompute(ctx)

# 500ms debounce cho coreness recompute
_recompute_task: asyncio.Task | None = None

async def _debounced_recompute(ctx: ServerContext, delay: float = 0.5) -> None:
    global _recompute_task
    if _recompute_task and not _recompute_task.done():
        _recompute_task.cancel()
    _recompute_task = asyncio.create_task(_do_recompute(ctx, delay))

async def _do_recompute(ctx: ServerContext, delay: float) -> None:
    await asyncio.sleep(delay)
    await asyncio.to_thread(ctx._compute_graph_metrics)
```

### 8.5 File Watcher (`indexer/watcher.py`)

```python
from watchfiles import awatch, Change

async def watch_project(ctx: ServerContext) -> None:
    ignore = set(ctx.config.ignore)
    async for changes in awatch(str(ctx.project_root)):
        dirty = []
        for change_type, path in changes:
            if change_type in (Change.added, Change.modified):
                if not _should_ignore(path, ignore):
                    dirty.append(path)
        if dirty:
            await reindex_changed(ctx, dirty)

def _should_ignore(path: str, patterns: set[str]) -> bool:
    p = Path(path)
    return any(p.match(pat) for pat in patterns)
```

### 8.6 Semantic Embeddings (`indexer/embedder.py`)

```python
from dataclasses import dataclass, field
from enum import Enum

class EmbedStatus(str, Enum):
    DISABLED    = "disabled"
    DOWNLOADING = "downloading"
    EMBEDDING   = "embedding"
    READY       = "ready"
    FAILED      = "failed"

@dataclass
class EmbedError:
    reason: str     # download_failed|model_corrupt|oom|embed_failed
    message: str
    retry_count: int = 0

@dataclass
class EmbedderState:
    status: EmbedStatus = EmbedStatus.DISABLED
    error: EmbedError | None = None

class Embedder:
    def __init__(self, ctx: ServerContext):
        self.ctx = ctx
        self.state = EmbedderState()

    async def start(self) -> None:
        """Start embedding pipeline nếu config.semantic_search.enabled."""
        if not self.ctx.config.semantic_search.enabled:
            return
        self.state.status = EmbedStatus.DOWNLOADING
        try:
            model_name = self.ctx.config.semantic_search.model
            model = await asyncio.to_thread(self._load_model, model_name)
            self.state.status = EmbedStatus.EMBEDDING
            await asyncio.to_thread(self._embed_all, model)
            self.state.status = EmbedStatus.READY
        except MemoryError:
            self.state.error = EmbedError("oom", "Out of memory during embedding")
            self.state.status = EmbedStatus.FAILED
        except Exception as e:
            self.state.error = EmbedError("embed_failed", str(e))
            self.state.status = EmbedStatus.FAILED

    def retry(self) -> None:
        """indexing_status(retry_embeddings=True) → clear error, restart."""
        self.state.error = None
        asyncio.create_task(self.start())

    def _load_model(self, model_name: str):
        from fastembed import TextEmbedding
        return TextEmbedding(model_name)

    def _embed_all(self, model) -> None:
        import sqlite_vec
        conn = self.ctx.write_conn
        with self.ctx.write_lock:
            sqlite_vec.load(conn)
            # Batch embed symbols
            rows = conn.execute(
                "SELECT id, name, signature, docstring FROM symbols"
            ).fetchall()
            texts = [f"{r[1]} {r[2]} {r[3]}" for r in rows]
            embeddings = list(model.embed(texts))  # fastembed handles batching
            conn.execute("BEGIN IMMEDIATE")
            try:
                conn.execute("DELETE FROM embedding_vecs")
                for (id_, *_), emb in zip(rows, embeddings):
                    conn.execute(
                        "INSERT INTO embedding_vecs(symbol_id, embedding) VALUES (?,?)",
                        (id_, emb.tolist())
                    )
                conn.commit()
            except Exception:
                conn.execute("ROLLBACK")
                raise
```

---

## 9. Language Parsers

### 9.1 Python Parser (ví dụ minh họa pattern)

```python
@register("python")
class PythonParser(LanguageParser):
    def __init__(self):
        super().__init__("python")

    def parse(self, source: bytes, path: str, file_hash: str) -> ParseResult:
        tree = self.parser.parse(source)
        module_name = Path(path).stem
        symbols, call_sites, imports = [], [], []
        self._walk(tree.root_node, source, module_name,
                   symbols, call_sites, imports, [])
        return ParseResult(
            path=path, language="python", file_hash=file_hash,
            symbols=symbols, call_sites=call_sites, imports=imports
        )

    def _walk(self, node, source, module, symbols, call_sites, imports,
              scope_stack):
        if node.type == "function_definition":
            name = self._text(node.child_by_field_name("name"))
            qname = ".".join(scope_stack + [name]) if scope_stack else name
            symbols.append(Symbol(
                name=name,
                qualified_name=f"{module}.{qname}",
                kind="method" if scope_stack else "function",
                line_start=node.start_point[0] + 1,
                line_end=node.end_point[0] + 1,
                signature=self._extract_signature(node, source),
                docstring=self._extract_docstring(node),
                name_tokens=tokenize_identifier(name),
            ))
            scope_stack = scope_stack + [name]

        elif node.type == "class_definition":
            name = self._text(node.child_by_field_name("name"))
            qname = ".".join(scope_stack + [name])
            symbols.append(Symbol(
                name=name, qualified_name=f"{module}.{qname}",
                kind="class",
                line_start=node.start_point[0] + 1,
                line_end=node.end_point[0] + 1,
                signature=f"class {name}",
                docstring=self._extract_docstring(node),
                name_tokens=tokenize_identifier(name),
            ))
            scope_stack = scope_stack + [name]

        elif node.type == "call":
            func_node = node.child_by_field_name("function")
            if func_node:
                callee = self._text(func_node)
                in_sym = ".".join(scope_stack) if scope_stack else "<module>"
                call_sites.append(CallSite(
                    callee_name=callee,
                    line=node.start_point[0] + 1,
                    in_symbol=f"{module}.{in_sym}",
                ))

        elif node.type == "import_from_statement":
            # from x import y
            ...

        for child in node.children:
            self._walk(child, source, module, symbols,
                       call_sites, imports, scope_stack)
```

Các language parsers khác (TS, Java, Rust, Go) follow same pattern với node types tương ứng.

### 9.2 Language Coverage

| Language | Parser file | Symbol kinds | Entry point detection |
|----------|------------|-------------|----------------------|
| Python | `python_.py` | function, class, method | `if __name__ == "__main__"`, `@app.route`, `@cli.command` |
| TypeScript | `typescript_.py` | function, class, interface, type, method | `export default`, `main()` |
| JavaScript | `typescript_.py` | function, class, method | same as TS |
| Java | `java_.py` | class, interface, method, constructor | `public static void main` |
| Rust | `rust_.py` | fn, struct, trait, impl, enum | `fn main`, `#[tokio::main]` |
| Go | `go_.py` | func, type, method | `func main()`, `func init()` |
| Others | `generic_.py` | (textual only) | none |

---

## 10. Tool Handler Architecture

### 10.1 Pattern: Internal Logic vs Handler

Mỗi tool có 2 functions:

```python
# 1. Internal logic (sync, dùng bởi compound tools in-process)
def _search_logic(params: SearchInput, conn: sqlite3.Connection,
                  config: Config) -> SearchOutput:
    # ... sync DB queries ...
    return SearchOutput(...)

# 2. Handler wrapper (async, dùng bởi _make_tool_fn / MCP boilerplate)
async def _handle_search(params: dict, *, ctx: ServerContext) -> dict:
    validated = SearchInput.model_validate(params)
    output = await asyncio.to_thread(_search_sync, validated, ctx)
    # _make_tool_fn inject _kind và các private keys
    output["_kind"] = validated.kind
    return output

def _search_sync(params: SearchInput, ctx: ServerContext) -> dict:
    conn = ctx.make_read_conn()
    try:
        return _search_logic(params, conn, ctx.config).model_dump()
    finally:
        conn.close()
```

### 10.2 FTS5 BM25 Search Logic

```python
def _search_logic(params: SearchInput, conn: sqlite3.Connection,
                  config: Config) -> SearchOutput:
    if params.kind == "symbol":
        # Dual-column weighted BM25
        rows_exact  = conn.execute("""
            SELECT s.qualified_name, s.name, s.path, s.line_start, s.kind,
                   -bm25(fts_exact) * 1.5 AS score
            FROM fts_exact
            JOIN symbols s ON s.id = fts_exact.rowid
            WHERE fts_exact MATCH ?
            LIMIT ?
        """, (params.query, params.limit * 2)).fetchall()

        rows_tokens = conn.execute("""
            SELECT s.qualified_name, s.name, s.path, s.line_start, s.kind,
                   -bm25(fts_tokens) * 1.0 AS score
            FROM fts_tokens
            JOIN symbols s ON s.id = fts_tokens.rowid
            WHERE fts_tokens MATCH ?
            LIMIT ?
        """, (params.query, params.limit * 2)).fetchall()

        # Merge by qualified_name, sum scores
        scores: dict[str, float] = {}
        data: dict[str, dict] = {}
        for row in [*rows_exact, *rows_tokens]:
            qname = row["qualified_name"]
            scores[qname] = scores.get(qname, 0.0) + row["score"]
            data.setdefault(qname, dict(row))

        top = sorted(data.keys(), key=lambda q: scores[q], reverse=True)
        results = [_format_symbol_result(data[q], "exact" if ...) for q in top[:params.limit]]
        return SearchOutput(results=results, truncated=len(top) > params.limit, ...)

    elif params.kind == "hybrid":
        # RRF merge: FTS + semantic (nếu embeddings ready)
        ...
```

### 10.3 Compound Tools (in-process)

```python
# tools/compound.py
def _locate_logic(params: LocateInput, conn: sqlite3.Connection,
                  config: Config) -> LocateOutput:
    # Step 1: search (in-process, không qua MCP)
    search_out = _search_logic(
        SearchInput(query=params.query, kind=params.kind or "symbol",
                    limit=params.limit or 10),
        conn, config
    )
    if not search_out.results:
        return LocateOutput(results=[], truncated=False, degraded=False, edges_ready=...)

    # Step 2: file_overview (nếu depth != search_only)
    if params.depth != "search_only":
        top_path = search_out.results[0].path
        file_out = _file_overview_logic(
            FileOverviewInput(path=top_path), conn, config)

    # Step 3: symbol_info (nếu depth == with_symbol VÀ kind có symbol name)
    top_result = None
    if (params.depth == "with_symbol" or params.depth is None) \
            and params.kind not in ("text", "file"):
        top_name = search_out.results[0].name
        sym_out = _symbol_info_logic(
            SymbolInfoInput(name=top_name, path=top_path), conn, config)
        top_result = LocateTopResult(file=file_out, symbol=sym_out)
    elif params.depth in ("with_file", None):
        top_result = LocateTopResult(file=file_out)

    return LocateOutput(
        results=search_out.results,
        top_result=top_result,
        ...
    )

def _understand_logic(params: UnderstandInput, conn: sqlite3.Connection,
                      config: Config) -> UnderstandOutput:
    # 4-step chain: locate → source → callers(limit=5)
    locate_out = _locate_logic(
        LocateInput(query=params.query, kind=params.kind or "symbol",
                    depth="with_symbol"),
        conn, config
    )
    if not locate_out.top_result or not locate_out.top_result.symbol:
        return UnderstandOutput(status="ambiguous", ...)

    sym = locate_out.top_result.symbol
    source_out = _source_logic(
        SourceInput(target=sym.name, include_metadata=True), conn, config)
    callers_out = _callers_logic(
        CallersInput(symbol=sym.name, path=sym.path, limit=5), conn, config)

    return UnderstandOutput(
        status="found",
        name=sym.name, path=sym.path, kind=sym.kind,
        signature=sym.signature, docstring=sym.docstring,
        source=source_out.content,
        callers_summary=[...],
        ...
    )
```

### 10.4 Ambiguity Contract (trong symbol_info, source, callers, callees, path, edit_context)

```python
def _resolve_symbol(name: str, path: str | None,
                    conn: sqlite3.Connection) -> sqlite3.Row | AmbiguousResult:
    """Trả về Row nếu found + unique. Trả AmbiguousResult nếu nhiều match. Raise nếu NOT_FOUND."""
    if path:
        rows = conn.execute(
            "SELECT * FROM symbols WHERE name=? AND path=?", (name, path)
        ).fetchall()
    else:
        rows = conn.execute(
            "SELECT * FROM symbols WHERE name=?", (name,)
        ).fetchall()

    if len(rows) == 0:
        raise NotFoundError(f"Symbol '{name}' not found in index")
    if len(rows) == 1:
        return rows[0]
    # Nhiều hơn 1 → AmbiguousResult
    return {"ambiguous": True, "candidates": [_format_candidate(r) for r in rows[:10]]}
```

---

## 11. CLI (`codeindex/cli.py`)

```python
import typer
import asyncio
from pathlib import Path

app = typer.Typer()

@app.command()
def serve(
    project_root: Path = typer.Argument(Path("."), help="Project root directory"),
    config_path: Path = typer.Option(None, "--config", help="Path to config.json"),
    preset: str = typer.Option("full", "--preset",
                                help="Tool preset: orient|trace|edit|compound|full"),
    db_path: Path = typer.Option(None, "--db", help="SQLite DB path"),
):
    """Start Code Intelligence MCP server."""
    asyncio.run(_serve(project_root.resolve(), preset, db_path, config_path))

async def _serve(project_root: Path, preset: str,
                 db_path: Path | None, config_path: Path | None):
    from .config import load_config
    from .context import ServerContext, IndexerState
    from .db.schema import init_db
    from .indexer.indexer import Indexer
    from .indexer.watcher import watch_project
    from .coverage_reader import CoverageReader
    from .codeowners import load_codeowners
    import sqlite3, threading

    db_path = db_path or (project_root / ".codeindex" / "index.db")
    db_path.parent.mkdir(parents=True, exist_ok=True)

    config = load_config(config_path or project_root)
    if preset != "full":
        config.preset = preset

    # Write connection (Indexer only)
    write_conn = sqlite3.connect(str(db_path), check_same_thread=False)
    write_conn.execute("PRAGMA journal_mode=WAL")
    write_conn.row_factory = sqlite3.Row
    init_db(write_conn)   # CREATE TABLE IF NOT EXISTS + triggers

    ctx = ServerContext(
        project_root=project_root,
        db_path=db_path,
        config=config,
        write_conn=write_conn,
        write_lock=threading.Lock(),
        coverage_data=CoverageReader.load(project_root),
        codeowners_patterns=load_codeowners(project_root),
    )

    # Background tasks
    indexer = Indexer(ctx)
    asyncio.create_task(indexer.run_full_index())
    asyncio.create_task(watch_project(ctx))
    if config.semantic_search.enabled and config.semantic_search.index_on_startup:
        asyncio.create_task(ctx.embedder.start())

    # MCP server — caller's boilerplate picks up ctx và registered handlers
    # server.py exports: ALL_TOOLS, register_tools(mcp_server, preset)
    from .server import register_tools
    # Caller's boilerplate: mcp_server = create_mcp_server(); register_tools(mcp_server, preset)
    # mcp_server.run_stdio(ctx=ctx)  ← caller's responsibility
```

---

## 12. Session Tracking

`session_context` tool đọc từ `ctx.session`:

```python
# tools/analysis.py
def _session_context_logic(ctx: ServerContext) -> SessionContextOutput:
    if ctx.session is None:
        from datetime import datetime, timezone
        ctx.session = SessionState(
            started_at=datetime.now(timezone.utc).isoformat()
        )

    sess = ctx.session
    # Frontier: files chưa explore, connected với explored via imports
    frontier = _compute_frontier(sess, ctx)

    return SessionContextOutput(
        explored=ExploredSection(
            symbols=list(sess.explored_symbols.values())[-50:],
            symbols_total=len(sess.explored_symbols),
            symbols_truncated=len(sess.explored_symbols) > 50,
            files=list(sess.explored_files),
            files_total=len(sess.explored_files),
        ),
        frontier=frontier,
        already_fetched=list(sess.already_fetched),
        session_stats=SessionStats(
            tool_calls=sess.tool_calls,
            unique_files_explored=sess.unique_files_explored,
        ),
        session_started_at=sess.started_at,
    )
```

---

## 13. Testing Strategy

Dùng trực tiếp regression targets từ `docs/architecture-design.md`:

### Priority 1 — Schema & Serialization
- Null vs Absent: `suggested_next` absent khi None ✓
- `coreness: null` khi `edges_ready: false` ✓
- `dead_code_source` luôn present ✓

### Priority 2 — FTS5 & Search
- `tokenize_identifier("getUserByEmail")` → `"get user by email"` ✓
- FTS trigger: INSERT symbol → fts_exact searchable ngay ✓
- DELETE symbol → fts_exact clean, không còn orphan entries ✓
- BM25 merge: `fts_exact × 1.5 + fts_tokens × 1.0` ✓

### Priority 3 — Graph & Path
- Bidirectional BFS tests [F1]–[F10] từ architecture doc ✓
- `compute_coreness` không bị self-loop inflate (đã fix) ✓
- k-core peeling O(V+E) ✓

### Priority 4 — Incremental Indexing
- File change → symbols update trong transaction duy nhất ✓
- Cascade 1-hop: file A imports B, B changes → A re-parsed ✓
- FTS5 triggers sync tự động, không manual ✓

### Priority 5 — suggested_next
- Toàn bộ bảng 30+ conditions từ architecture doc ✓
- Preset filtering: `orient` preset không suggest `callers` ✓

---

## 14. Implementation Order

```
Week 1  — Foundation
  Day 1-2: db/schema.py (tables + triggers)
  Day 3:   config.py, context.py
  Day 4-5: schemas.py (Pydantic models cho 16 tools)

Week 2-3 — Indexer
  Day 1-2: parsers/base.py + python_.py
  Day 3-4: parsers/typescript_.py, java_.py, rust_.py, go_.py, generic_.py
  Day 5:   indexer.py (full index run)
  Day 6:   watcher.py (incremental + cascade)
  Day 7:   db/queries.py (shared helpers)

Week 3-4 — Tool Handlers
  Day 1-2: tools/meta.py (repo_overview, indexing_status, source)
  Day 3-4: tools/search.py (search, file_overview, symbol_info)
  Day 5:   tools/compound.py (locate, understand)
  Day 6:   tools/graph.py (callers, callees, dependencies, path)
  Day 7:   tools/analysis.py (edit_context, diff_impact, hotspots, session_context)

Week 4   — Polish
  Day 1:   cli.py (serve command)
  Day 2-3: embedder.py (fastembed + sqlite-vec, opt-in)
  Day 4-5: Tests (regression targets từ architecture doc)
```

---

## 15. Environment Preconditions

> *(Bắt buộc check trước khi ship — ref Tikai H9 gotcha)*

| Precondition | Check | Consequence nếu miss |
|-------------|-------|----------------------|
| `tree-sitter-languages` installed | `import tree_sitter_languages` | Indexer không parse được, mọi tools trả empty |
| `.codeindex/` directory writable | `os.access(db_path.parent, os.W_OK)` | DB không tạo được, server crash |
| SQLite WAL mode enabled | `PRAGMA journal_mode` → `wal` | Concurrent reader/writer conflict |
| FTS5 triggers created trước khi index | Check trigger existence in `sqlite_master` | FTS desync ngay từ đầu |
| `watchfiles` event loop integration | Chạy `watch_project` trong asyncio loop | File changes không trigger reindex |
| `fastembed` model downloaded (opt-in) | State machine `DOWNLOADING` → `READY` | `kind=hybrid` không available |

---

*SPEC_APPROVED: true — Tiến hành audit-design.*

---

## Risk Assessment (audit-design)
<!-- audit-design: DO NOT DUPLICATE — update this section, do not append a second one -->
<!-- last-run: 2026-06-29 | trigger: NORMAL -->

**Tier:** 2 (Persistent DB state, file writes, external CLI/model download, multi-connection concurrency)
**Date:** 2026-06-29

### Failure Modes

1. **`_write_parse_result` blocks asyncio event loop** — HIGH — mitigation in plan: NO
   `_parse_all` (async) calls `self._write_parse_result(result)` synchronously after each `await asyncio.to_thread(_parse_single)`. `_write_parse_result` acquires `threading.Lock` and executes SQLite writes on the event loop thread. If embedder or watcher holds `write_lock`, this blocks MCP stdio for the lock duration. Fix: wrap `_write_parse_result` in `asyncio.to_thread`.

2. **Schema migration gap — existing DB crashes on new columns** — HIGH — mitigation in plan: NO
   `CREATE TABLE IF NOT EXISTS` silently skips if table already exists. New columns (`symbols.name_tokens`, `symbols.is_entry_point`, `file_index.mtime`) won't be added to existing DBs. Any query touching these columns → `OperationalError: no such column`. Existing `db_init.py` has `migrate_to_v2_6()` but spec defines no `migrate_to_v2_7()`. Fix: add `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` migration block in `db/schema.py`.

3. **Watcher + initial index race corrupts call_edges** — MED — mitigation in plan: NO
   `run_full_index` and `watch_project` start as concurrent tasks. Watcher can fire `reindex_changed` during `building_edges` phase, writing call_edges for paths that the indexer's `_build_edges` will subsequently overwrite using a stale `file_symbols` snapshot loaded before the watcher ran. Result: call graph is internally inconsistent post-startup. Fix: watcher should queue events during `building_edges` phase and drain after `phase=ready`.

### Layer Signals

- **L1 Logic**: `_resolve_file_edges` body is `...` (stub) — the core function wiring `ParseResult` → `ConservativeResolver` → `call_edges` is not defined in spec. `_hybrid` search branch and `_format_symbol_result(data[q], "exact" if ...)` also stubs. These are the **largest gap** — spec describes inputs/outputs but not internal logic. Mark **ASSUMED**.
- **L2 Concurrency**: `_debounced_recompute` uses module-level `_recompute_task` global. Safe within single asyncio event loop but untestable in isolation. Low risk, pattern debt.
- **L3 Data**: (a) `qualified_name` UNIQUE constraint has no collision policy — two files defining `module.ClassName` will crash with constraint violation. **ASSUMED**. (b) `file_index.mtime` defined in schema but never written in `_write_parse_result` INSERT statement — silently stays NULL. (c) `symbols_used` stored as JSON text with no length bound.
- **L6 Observability**: `logger` referenced in `_make_tool_fn` error boundary but never configured/defined in spec. No startup-time validation log. Spec Section 15 lists preconditions but doesn't wire them into actual startup checks.

### Assumptions to Verify

- **ASSUMED**: `_resolve_file_edges` implementation — bridges ParseResult → ConservativeResolver. Contract (inputs, confidence labeling, how `file_symbols` map is keyed) not specified.
- **ASSUMED**: `tokenize_identifier(name)` — called everywhere (`Symbol` dataclass, Python parser, test targets) but never defined in spec. If missing, all `name_tokens` = empty string → `fts_tokens` returns nothing silently.
- **ASSUMED**: Schema migration strategy for existing databases.
- **ASSUMED**: `qualified_name` collision policy (two files with same qualified symbol name).
- **ASSUMED**: `_walk_project`, `_sha256`, `_sha256_bytes`, `_detect_language`, `_load_file_symbols`, `_load_import_map`, `_compute_frontier` — all referenced but undefined in spec.
- **ASSUMED**: `mtime` write path in `_write_parse_result` (column defined but not populated).

### Abductive Hypotheses

**Abductive 1 — sqlite-vec load ordering:** `init_db(write_conn)` runs `CREATE VIRTUAL TABLE IF NOT EXISTS embedding_vecs USING vec0(...)`. This fails with "no such module: vec0" because `sqlite_vec.load(conn)` is only called inside `Embedder._embed_all`, which runs later. These are correct components in the wrong order. Fix: call `sqlite_vec.load(write_conn)` before `init_db`, or defer `CREATE VIRTUAL TABLE` to `Embedder.start()`.

**Abductive 2 — `tokenize_identifier` silent degradation at scale:** At indexing time, `name_tokens` is pre-computed by `tokenize_identifier(name)`. If `tokenize_identifier` is missing or returns empty string for any name, `fts_tokens` index becomes empty. `fts_exact` still works (name + docstring), but BM25 weighted merge silently degrades to `fts_exact`-only scoring. No error is raised — the `fts_tokens` table just returns no rows. This would only manifest as "search misses camelCase queries" at scale and be hard to attribute to a missing function.

### Gate Result

```
PASS WITH FLAGS — proceed to writing-plans
writing-plans MUST include mitigations for:
  [HIGH-1] Wrap _write_parse_result in asyncio.to_thread
  [HIGH-2] Add schema migration (ALTER TABLE ADD COLUMN IF NOT EXISTS) for new columns
  [ABD-1]  sqlite-vec extension load before CREATE VIRTUAL TABLE
  [ASSUMED] Define tokenize_identifier + all undefined helper functions
  [MED-3]  Watcher queuing during building_edges phase
```
