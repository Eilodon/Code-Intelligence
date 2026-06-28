# Code Intelligence MCP — Design v2.7.2

> 16 tools. 4 layers. Zero new external dependencies.


---

## Architecture: 4 Layers — 16 Tools

**Layer 1 — Index Engine** (background process, persistent)
**Layer 2 — Tool Surface** (MCP server, stdio transport) — **16 tools**
**Layer 3 — Behavioral Guidance** (AGENTS.md + `workflow_guide` embedded) — 8 stages
**Layer 4 — Tool Presets** (selective registration, zero logic change)

Behavioral layer không phải add-on, là half the system.

---

## Design Rationale: Vấn đề mỗi quyết định kiến trúc giải quyết

**Lookup-then-inspect tốn 3 round-trip.** Pattern phổ biến nhất trong mọi session — agent
muốn biết "symbol X là gì?" — về bản chất cần 3 bước: tìm xem X ở đâu, xem structure file
đó, đọc metadata + health của X. Mỗi bước là 1 round-trip riêng qua MCP transport.
`locate` compound cả 3 bước vào 1 call — giảm pattern phổ biến nhất từ 3 calls xuống 1.

**Tool description luôn chiếm context, bất kể agent đang ở stage nào.** Mỗi tool
description nằm cố định trong context window của agent ở mọi turn, dù agent chỉ đang làm
việc tập trung ở một workflow stage cụ thể (chỉ orient, chỉ edit, ...). Token overhead này
không giảm theo workflow — nó là phí cố định cho toàn bộ session. Tool Presets (Layer 4)
giải quyết bằng selective registration: chỉ tools relevant với stage hiện tại mới xuất
hiện trong context của agent.

**Quyết định "gọi tool gì tiếp theo" đòi hỏi 1 vòng suy luận riêng.** Guidance kiểu
`workflow_guide`/AGENTS.md nằm *ngoài* response — agent phải internalize rồi tự áp dụng
cho từng context khác nhau, tốn thêm 1 inference round mỗi lần quyết định bước kế tiếp.
`suggested_next` giải quyết bằng cách nhúng hint trực tiếp vào mọi response — agent không
cần suy luận, chỉ cần đọc field.

**Static analysis một mình không phân biệt được "không có caller" với "không bao giờ chạy".**
`dead_code_confidence: "high"` chỉ dựa trên "không có static callers + private scope" sẽ
false-positive với dynamic dispatch, reflection, hay decorator-registered handlers — những
symbol này thực ra được gọi ở runtime nhưng static analysis không thấy được edge. Coverage
Reader giải quyết bằng cách cross-reference runtime coverage data: có lcov/coverage hit
trong `[line_start, line_end]` → symbol *được* execute → downgrade confidence, không báo
nhầm là dead.

**Agent mặc định dùng native tool (grep/Read) nếu description không nói rõ phải dùng tool
nào thay thế.** Một description chỉ liệt kê schema/behavior không đủ để agent ưu tiên tool
chuyên dụng over native tool — cần ngôn ngữ trigger rõ ràng. Mọi tool description trong hệ
thống này follow 1 khung 4 phần cố định để giải quyết đúng vấn đề này (pattern này đã được
chứng minh hiệu quả ở các IDE/agent tooling khác, ví dụ JetBrains):
1. **USE WHEN** — trigger condition rõ ràng
2. **USE THIS INSTEAD OF** — explicit replacement của native tool tương ứng
3. **NOT FOR** — negative example, tránh agent dùng sai tool
4. **vs X** — disambiguation từ tool dễ nhầm lẫn nhất

Khung 4 phần này là quy ước bắt buộc cho mọi tool mới thêm vào hệ thống — không chỉ là
lịch sử rewrite, mà là convention sống cho future growth.

---

## Layer 1: Index Engine

### Core Engine

tree-sitter parse toàn bộ codebase → extract symbols (signature + docstring + line range),
import edges, call edges, inheritance edges → SQLite FTS5 dual-column search → incremental
indexing 2-level (node hash + edge invalidation lan ra `imported_by`) → file watcher
(`watchfiles`). Phase 2 thêm k-core coreness computation (O(V+E)) sau khi toàn bộ edges
ready → `symbols.coreness` → `is_hub` revised với OR(degree, coreness) condition, được
tính trong một function audited duy nhất (`update_is_hub_flags`).

**Coverage data** (in-memory, loaded on startup): `CoverageReader.load(project_root)` detect
và parse coverage file tự động — lcov, Python `.coverage`, Go `coverage.out`, Cobertura XML.
Kết quả inject vào `_compute_dead_code_confidence()`. Re-load khi `indexing_status(retry_embeddings=true)`.

**Session tracking** (in-memory, powers `session_context`): server ghi compact log mỗi
tool call — `{tool, input, compact_summary, timestamp, session_started_at}`. Không ghi
full response.

---

### FTS5 Dual-Column Search

SQLite built-in tokenizers không split camelCase hay snake_case. Solution: **pre-tokenize
tại index time** với hai FTS5 virtual tables:

```sql
CREATE VIRTUAL TABLE fts_exact USING fts5(
  symbol_id UNINDEXED,
  name,
  docstring,
  tokenize='unicode61'
);

-- Pre-processing: "getUserById" → "get user by id"
--                "user_service" → "user service"
--                "HTTPSClient"  → "https client"
CREATE VIRTUAL TABLE fts_tokens USING fts5(
  symbol_id UNINDEXED,
  name_tokens,
  tokenize='unicode61'
);
```

**Pre-tokenization algorithm** (pure Python, no dependencies):

```python
import re

def tokenize_identifier(name: str) -> str:
    s = re.sub(r'[_\-]+', ' ', name)
    s = re.sub(r'([a-z0-9])([A-Z])', r'\1 \2', s)
    s = re.sub(r'([A-Z]+)([A-Z][a-z])', r'\1 \2', s)
    return s.lower().strip()
```

**Search routing**: `search(kind="symbol")` query chạy trên cả hai tables, merge bằng
BM25 score. Exact match (`fts_exact`) nhận weight boost × 1.5. `kind="text"` chỉ dùng
`fts_exact` trên content column.

---

### Phase-based Indexing

```
scanning → parsing → building_edges → ready
```

- **`scanning`**: liệt kê file, hash check. Chưa có symbol nào.
- **`parsing`**: symbols đã extract, edges **chưa build**.
- **`building_edges`**: đang build call/import/inheritance edges + confidence labels
  + coreness computation + `is_hub` flag update (một pass duy nhất, audited).
- **`ready`**: full graph sẵn sàng. `edges_ready: true` khi và chỉ khi `phase == "ready"`.

Embeddings không thuộc phase ladder này — track độc lập.

---

### Entry Point Detection

Entry points được detect trong Phase 1 (parsing) và lưu vào `symbols.is_entry_point`.
Một symbol là entry point nếu thoả MỘT TRONG các điều kiện:

```
1. Framework decorators (Python/TS/Go):
   @app.route, @router.get/post/put/delete/patch,
   @app.on_event, @click.command, @celery.task,
   app.add_api_route (FastAPI/Flask/Django/Click/Celery)

2. Convention names:
   main() / __main__ / Main() trong bất kỳ ngôn ngữ nào

3. Entry-point module heuristics:
   File nằm ở root level VÀ filename thuộc:
   index.ts, main.py, app.py, server.py, cmd/*/main.go,
   __main__.py, entrypoint.*, wsgi.py, asgi.py

4. Config escape hatch:
   Symbol được list trong config.json → entry_points[]
```

`is_entry_point: true` ảnh hưởng: `dead_code_confidence` (không bao giờ là dead code),
`edit_context.callers[]` priority ranking (sub-priority (c): closest to entry points), `repo_overview.entry_points[]`.

---

### Call Graph Conservative Resolver

**Vấn đề**: tree-sitter thuần túy extract call edge bằng name-only matching.
`notifier.send()` → edge tới *tất cả* symbol tên `send` trong codebase. Interface
dispatch (TS), duck typing (Python), trait methods (Rust) → false edge ngập.

**Tại sao không dùng `tree-sitter-analyzer` (TSA)**:

Lý do kiến trúc (chính): Code Intelligence MCP tự nó là MCP server — không depend
runtime vào MCP server khác, tránh nested-MCP coupling và phụ thuộc vào internal API
không được cam kết stable. Self-contained resolver mượn design philosophy từ TSA, không
mượn package.

Lý do phụ: TSA kéo 33+ transitive deps (`mcp`, `uvicorn`, `numpy`, `networkx`, ...) —
không phù hợp minimal-dep philosophy.

*TSA v1.28.0 thực ra đã có call-edge extraction mature — quyết định self-contained đứng vì
lý do kiến trúc, không vì capability.*

**Solution**: Self-contained resolver trong `codeindex/resolver/` — implement bằng
tree-sitter-languages (đã là dep của codeindex).

**Algorithm — 3-tier resolution với alias tracking, audited**:

```
Per file, Phase 2:

Step 1 — Import Extraction:
  Walk AST nodes: import_statement, import_from_statement (Python)
                  import_statement với named_imports/default/namespace (TS/JS)
                  use_declaration (Rust), import_declaration (Go/Java)
  → import_map: { symbol_name → source_module_path }

Step 2 — Type Annotation Extraction:
  Walk AST nodes: typed_parameter (Python function params)
                  assignment với type field (Python: x: TypeName = ...)
                  required_parameter với type_annotation (TypeScript)
                  variable_declarator với type_annotation (TypeScript/Java)
  → type_map: { var_name → type_name }

Step 2.5 — Local Assignment Alias Extraction:
  Walk AST nodes: assignment / augmented_assignment (Python)
                  variable_declarator (TS/JS), local_variable_declaration (Java)
                  let_declaration (Rust), short_var_declaration / var_declaration (Go)
  → alias_map: { x → y }, với điều kiện:
    y là bare identifier (không phải call, attribute, ternary, hay complex expr)
    y ∈ file_symbols OR y ∈ import_map  (y đã resolvable)
    x ∉ type_map, x ∉ file_symbols, x ∉ import_map
    x KHÔNG bị assign nhiều lần trong file [F7]
  Conservative contract: `x = func()`, `x = obj.attr`, `x = a if cond else b` → skip.
  Unknown beats mis-classified.

Step 3 — Call Expression Resolution:
  callee_name ∈ file_symbols / import_map / alias_map (resolvable target) → Tier 1 "resolved"
  method call với receiver type/alias resolvable qua import_map → Tier 2 "inferred"
  còn lại → Tier 3 "textual"
```

**[F7] Multi-assignment Guard**:

Nếu một biến được assign nhiều lần trong cùng file (`x = notify_user` rồi sau đó
`x = send_email`), alias_map dùng last-write-wins → resolver confident-resolve sai.
Pre-pass detect multi-assigned LHS, skip hoàn toàn khỏi alias_map:

```python
def _extract_aliases(
    self,
    ast_root,
    file_symbols: set[str],
    import_map: dict[str, str],
    type_map: dict[str, str]
) -> dict[str, str]:
    """
    Returns alias_map: {alias_name → resolved_symbol_name}
    Conservative contract:
      - Chỉ track x = y với y là bare identifier đã resolvable
      - [F7] Skip variables được assign nhiều lần (ambiguous aliasing)
      - Unknown beats mis-classified — bất kỳ case phức tạp nào → skip
    """
    # Pre-pass: detect multiply-assigned LHS
    # Scope: file-level (conservative — không phân biệt function scopes)
    # Tradeoff: x = f trong func A và x = g trong func B → cả hai bị skip.
    # Justified: false positive resolution worse than false negative.
    lhs_seen: set[str] = set()
    multi_assigned: set[str] = set()
    for node in self._walk(ast_root, self.ASSIGNMENT_NODES):
        lhs = self._get_assignment_lhs(node)
        if lhs:
            if lhs in lhs_seen:
                multi_assigned.add(lhs)
            lhs_seen.add(lhs)

    # Main pass: build alias_map
    alias_map: dict[str, str] = {}
    for node in self._walk(ast_root, self.ASSIGNMENT_NODES):
        lhs = self._get_assignment_lhs(node)
        rhs = self._get_assignment_rhs_identifier(node)  # None nếu RHS phức tạp

        if (lhs
                and rhs
                and lhs not in multi_assigned      # [F7] skip if re-assigned anywhere
                and lhs not in file_symbols
                and lhs not in import_map
                and lhs not in type_map
                and (rhs in file_symbols or rhs in import_map)):
            alias_map[lhs] = rhs

    return alias_map
```

```python
ASSIGNMENT_NODES = {
    "python":     ["assignment", "augmented_assignment"],
    "typescript": ["variable_declarator"],
    "javascript": ["variable_declarator"],
    "java":       ["local_variable_declaration"],
    "rust":       ["let_declaration"],
    "go":         ["short_var_declaration", "var_declaration"],
    # C/C++: defer — templates + qualifiers phức tạp
    # C#/Kotlin/Swift: Tier 1/2 coverage đủ, alias tracking lower priority
}
```

**Rust `let_declaration` handler**:

```python
def _get_rust_assignment_lhs_rhs(self, node) -> tuple[str | None, str | None]:
    """
    Handle: let x = y;  (tree-sitter: let_declaration)
    Safe case ONLY: pattern is bare identifier, value is bare identifier.
    Skip: let mut x = ..., let (a, b) = ..., if let Some(x) = ...,
          let x: Type = ..., let x = func(), let x = obj.method()
    """
    if node.type != "let_declaration":
        return None, None
    pattern = node.child_by_field_name("pattern")
    value = node.child_by_field_name("value")
    if not pattern or not value:
        return None, None
    if pattern.type != "identifier" or value.type != "identifier":
        return None, None
    # mutable_specifier is not a named field — must iterate children
    if any(child.type == "mutable_specifier" for child in node.children):
        return None, None
    # type annotation IS a named field in tree-sitter-rust
    if node.child_by_field_name("type") is not None:
        return None, None
    return pattern.text.decode("utf-8"), value.text.decode("utf-8")
```

**Go `short_var_declaration` / `var_declaration` handler**:

```python
def _get_go_assignment_lhs_rhs(self, node) -> tuple[str | None, str | None]:
    """
    Handle: x := y  (tree-sitter: short_var_declaration)
    Handle: var x = y  (tree-sitter: var_declaration → var_spec)
    Safe case ONLY: single LHS identifier, single RHS bare identifier.
    Skip: x, y := a, b  (multi-assign — F7 guard catches this too)
    """
    if node.type == "short_var_declaration":
        left = node.child_by_field_name("left")
        right = node.child_by_field_name("right")
        if not left or not right:
            return None, None
        left_children = [c for c in left.children if c.type != ","]
        right_children = [c for c in right.children if c.type != ","]
        if len(left_children) != 1 or len(right_children) != 1:
            return None, None
        if left_children[0].type != "identifier" or right_children[0].type != "identifier":
            return None, None
        return (left_children[0].text.decode("utf-8"),
                right_children[0].text.decode("utf-8"))

    elif node.type == "var_declaration":
        # var x = y  →  var_declaration → var_spec (name, value)
        # In tree-sitter-go, var_spec's "value" field is always an "expression_list",
        # never a bare "identifier" — must unwrap expression_list first.
        specs = [c for c in node.children if c.type == "var_spec"]
        if len(specs) != 1:
            return None, None
        spec = specs[0]
        name_node = spec.child_by_field_name("name")
        value_list = spec.child_by_field_name("value")   # expression_list, not identifier
        if not name_node or not value_list:
            return None, None
        # Skip typed var declarations: var x SomeType = y
        if spec.child_by_field_name("type") is not None:
            return None, None
        # Unwrap expression_list — must have exactly 1 child (rejects "var x, y = a, b")
        val_children = [c for c in value_list.children if c.type != ","]
        if len(val_children) != 1 or val_children[0].type != "identifier":
            return None, None
        if name_node.type != "identifier":
            return None, None
        return (name_node.text.decode("utf-8"),
                val_children[0].text.decode("utf-8"))

    return None, None
```

**Dispatch** (F7 guard unchanged):

```python
def _get_assignment_lhs_rhs(self, node, language: str) -> tuple[str | None, str | None]:
    if language in ("python",):
        return self._get_python_assignment_lhs_rhs(node)
    elif language in ("typescript", "javascript"):
        return self._get_ts_js_assignment_lhs_rhs(node)
    elif language == "java":
        return self._get_java_assignment_lhs_rhs(node)
    elif language == "rust":
        return self._get_rust_assignment_lhs_rhs(node)
    elif language == "go":
        return self._get_go_assignment_lhs_rhs(node)
    return None, None
```

**Tier 1/2 modifications** — không đổi (alias-aware lookup đã có sẵn):

```python
# Tier 1 — resolved
if callee_name in file_symbols:
    confidence = "resolved"
elif callee_name in import_map:
    confidence = "resolved"
    resolved_path = import_map[callee_name]
elif callee_name in alias_map:
    target = alias_map[callee_name]   # invariant: target đã resolvable
    if target in import_map:
        confidence = "resolved"
        resolved_path = import_map[target]
    elif target in file_symbols:
        confidence = "resolved"

# Tier 2 — inferred (receiver type lookup + alias fallback)
resolved_receiver = type_map.get(receiver_name) or alias_map.get(receiver_name)
if resolved_receiver and resolved_receiver in import_map:
    confidence = "inferred"
    resolved_path = import_map[resolved_receiver]
```

**Implementation**: AST navigation dùng tree-sitter `child_by_field_name()` API (stable
trong tree-sitter ≥ 0.20). Per-language node type constants trong
`codeindex/resolver/lang_constants.py`, bao gồm `ASSIGNMENT_NODES`.

**Conservative contract**: khi không chắc, prefer `"textual"` hơn wrong `"resolved"`.
*Unknown beats mis-classified.*

**Language coverage**:

| Language   | Import | Type Annotation              | Call | Alias Tracking | Effective Coverage                  |
|------------|--------|------------------------------|------|----------------|-------------------------------------|
| Python     | ✓      | ✓ (typed_param + annotation) | ✓    | ✓              | Full (resolved/inferred/textual)     |
| TypeScript | ✓      | ✓ (type_annotation node)     | ✓    | ✓              | Full                                |
| JavaScript | ✓      | ✗ (dynamic)                  | ✓    | ✓              | Full (Tier 1/2 — dynamic)           |
| Java       | ✓      | ✓ (explicit type decl)       | ✓    | ✓              | Full                                |
| Rust       | ✓      | ✓ (type bound)               | ✓    | ✓              | Full (Tier 1/2 + alias)             |
| Go         | ✓      | ✗ (structural typing)        | ✓    | ✓              | Full (Tier 1/2 + alias)             |
| C / C++    | ✓      | ✗                            | ✓    | ✗              | Partial (Tier 1/2 only — defer)     |
| C#         | ✓      | ✓                            | ✓    | ✗              | Full (Tier 1/2 unchanged)           |
| Kotlin     | ✓      | ✓                            | ✓    | ✗              | Full (Tier 1/2 unchanged)           |
| Swift      | ✓      | ✓                            | ✓    | ✗              | Full (Tier 1/2 unchanged)           |
| Ruby       | ✓      | ✗ (dynamic)                  | ✓    | ✗              | Partial                             |
| PHP        | ✓      | ✓ (7.4+ type hints)          | ✓    | ✗              | Partial                             |
| Other      | ✗      | ✗                            | ✗    | ✗              | All textual — không crash           |

Language không có trong bảng hoặc grammar thiếu → resolver silent fallback, tất cả edges
là `"textual"`. Không crash, không error.

---

### Coverage Reader

**Vị trí trong pipeline**: Cuối Phase 0 (sau DB init, migration, và scan_files; trước Phase 1 parse), chạy một lần, cache kết quả trong memory.
Không write vào DB — chỉ cần khi tính `dead_code_confidence`.

**Trigger**: Auto-detect trên startup, tìm coverage file trong project root.

```python
# codeindex/coverage_reader.py
# Pure Python. Zero new dependencies.

import sqlite3
import xml.etree.ElementTree as ET
from pathlib import Path
from dataclasses import dataclass, field

@dataclass
class CoverageData:
    """In-memory coverage map. {absolute_path: set of covered line numbers}"""
    source: str  # "lcov" | "python" | "go" | "cobertura" | "none"
    covered_lines: dict[str, set[int]] = field(default_factory=dict)

    def is_covered(self, abs_path: str, line_start: int, line_end: int) -> bool:
        """True if ANY line in [line_start, line_end] appears in coverage data."""
        file_cov = self.covered_lines.get(abs_path, set())
        return any(ln in file_cov for ln in range(line_start, line_end + 1))

COVERAGE_SEARCH_PATHS = [
    # (relative_path, format)
    # Priority rationale: project-root files override nested/tool-specific paths.
    # Within same format, shallow path wins (more likely to be project-wide coverage).
    # lcov first: most universal format (C/C++/JS/TS/Go/Rust all emit lcov).
    ("coverage/lcov.info", "lcov"),
    ("lcov.info",          "lcov"),
    (".nyc_output/lcov.info", "lcov"),   # Node.js/NYC — lower priority than project-root lcov
    (".coverage",          "python"),
    ("coverage.out",       "go"),
    ("coverage/coverage.out", "go"),
    ("coverage.xml",       "cobertura"),
    ("coverage/coverage.xml", "cobertura"),
]

class CoverageReader:

    @classmethod
    def load(cls, project_root: Path) -> CoverageData:
        """
        Auto-detect and parse first coverage file found.
        Returns CoverageData(source='none') if nothing found — never raises.
        """
        for relative, fmt in COVERAGE_SEARCH_PATHS:
            path = project_root / relative
            if not path.exists():
                continue
            try:
                if fmt == "lcov":
                    return cls._parse_lcov(path, project_root)
                elif fmt == "python":
                    return cls._parse_python_coverage(path, project_root)
                elif fmt == "go":
                    return cls._parse_go_coverage(path, project_root)
                elif fmt == "cobertura":
                    return cls._parse_cobertura(path, project_root)
            except (ValueError, KeyError, IndexError,
                    sqlite3.DatabaseError, ET.ParseError):
                # Parse errors (incl. SQLite malformed DB + malformed XML) → skip, try next.
                # sqlite3.DatabaseError covers both OperationalError (Py≤3.11) and
                # NotADatabaseError (Py3.12+). ET.ParseError is a SyntaxError subclass —
                # not caught by ValueError without explicit listing.
                continue
            except (OSError, PermissionError) as e:
                # IO errors → warn user, not silently swallow.
                print(f"[codeindex] Warning: Cannot read coverage file {path}: {e}")
                continue
        return CoverageData(source="none")

    @classmethod
    def _parse_lcov(cls, path: Path, project_root: Path) -> CoverageData:
        """
        LCOV format:
          SF:<source_file_path>
          DA:<line_number>,<hit_count>[,<checksum>]
          ...
          end_of_record
        """
        covered: dict[str, set[int]] = {}
        current_file = None
        with path.open(encoding="utf-8", errors="replace") as f:
            for line in f:
                line = line.strip()
                if line.startswith("SF:"):
                    raw = line[3:]
                    p = Path(raw)
                    current_file = str(p if p.is_absolute() else (project_root / p))
                    covered.setdefault(current_file, set())
                elif line.startswith("DA:") and current_file is not None:
                    try:
                        parts = line[3:].split(",")
                        line_no = int(parts[0])
                        hits = int(parts[1])
                        if hits > 0:
                            covered[current_file].add(line_no)
                    except (ValueError, IndexError):
                        pass
                elif line == "end_of_record":
                    current_file = None
        return CoverageData(source="lcov", covered_lines=covered)

    @classmethod
    def _parse_python_coverage(cls, path: Path, project_root: Path) -> CoverageData:
        """
        Python .coverage file: SQLite3 database (no extra deps — stdlib sqlite3).
        Schema: line_bits table with file_id + numbits bitmap.
        Use coverage.py's arc table for line info.
        """
        covered: dict[str, set[int]] = {}
        con = sqlite3.connect(str(path))
        try:
            try:
                files = {row[0]: row[1] for row in
                         con.execute("SELECT id, path FROM file")}
                for file_id, file_path in files.items():
                    abs_path = str(Path(file_path) if Path(file_path).is_absolute()
                                   else project_root / file_path)
                    lines = set()
                    for (numbits,) in con.execute(
                        "SELECT numbits FROM line_bits WHERE file_id=?", (file_id,)
                    ):
                        # numbits is a blob encoding line numbers as bit positions
                        # Each byte: bits 0-7 = lines (byte_index*8+1)..(byte_index*8+8)
                        for byte_idx, byte_val in enumerate(numbits):
                            for bit in range(8):
                                if byte_val & (1 << bit):
                                    lines.add(byte_idx * 8 + bit + 1)
                    covered[abs_path] = lines
            except sqlite3.OperationalError:
                # Older .coverage format: arc table (from_line, to_line)
                try:
                    files = {row[0]: row[1] for row in
                             con.execute("SELECT id, path FROM file")}
                    for file_id, file_path in files.items():
                        abs_path = str(Path(file_path) if Path(file_path).is_absolute()
                                       else project_root / file_path)
                        lines = set()
                        for (from_l, to_l) in con.execute(
                            "SELECT fromno, tono FROM arc WHERE file_id=?", (file_id,)
                        ):
                            if from_l > 0:
                                lines.add(from_l)
                            if to_l > 0:
                                lines.add(to_l)
                        covered[abs_path] = lines
                except sqlite3.OperationalError:
                    # Pre-5.x schema: line_data table (coverage.py < 5.0)
                    files = {row[0]: row[1] for row in
                             con.execute("SELECT id, path FROM file")}
                    for file_id, file_path in files.items():
                        abs_path = str(Path(file_path) if Path(file_path).is_absolute()
                                       else project_root / file_path)
                        lines = set()
                        for (line_no,) in con.execute(
                            "SELECT lineno FROM line_data WHERE file_id=?", (file_id,)
                        ):
                            if line_no > 0:
                                lines.add(line_no)
                        covered[abs_path] = lines
        finally:
            con.close()
        return CoverageData(source="python", covered_lines=covered)

    @classmethod
    def _parse_go_coverage(cls, path: Path, project_root: Path) -> CoverageData:
        """
        Go coverage.out format:
          mode: set
          github.com/pkg/file.go:10.5,15.2 3 1
          (file:startline.col,endline.col stmtcount hitcount)
        """
        covered: dict[str, set[int]] = {}
        with path.open(encoding="utf-8", errors="replace") as f:
            for line in f:
                line = line.strip()
                if line.startswith("mode:"):
                    continue
                parts = line.split(" ")
                if len(parts) != 3:
                    continue
                try:
                    hit_count = int(parts[2])
                    if hit_count == 0:
                        continue
                    location = parts[0]
                    colon_idx = location.rfind(":")
                    if colon_idx < 0:
                        continue
                    file_part = location[:colon_idx]
                    range_part = location[colon_idx + 1:]
                    start_str, end_str = range_part.split(",")
                    start_line = int(start_str.split(".")[0])
                    end_line = int(end_str.split(".")[0])
                    # Go coverage uses module path (e.g. github.com/user/repo/pkg/auth/user.go).
                    # Longest suffix match: iterate from full path down, keep the LONGEST
                    # suffix that exists on disk. This avoids monorepo collisions where
                    # shorter suffixes (e.g. "main.go") match multiple packages.
                    # Known limitation: in monorepos with duplicate directory structures
                    # (e.g. moduleA/cmd/main.go and moduleB/cmd/main.go), the first
                    # disk hit at a given depth wins. go.mod-aware resolution deferred to v2.8.
                    file_parts = Path(file_part).parts
                    all_candidates = []
                    matched_candidate = None
                    for n in range(len(file_parts), 0, -1):
                        suffix = Path(*file_parts[-n:])
                        candidate = project_root / suffix
                        if candidate.exists():
                            matched_candidate = candidate
                            break  # longest match found — stop immediately
                    if matched_candidate:
                        abs_path = str(matched_candidate)
                        covered.setdefault(abs_path, set())
                        covered[abs_path].update(range(start_line, end_line + 1))
                except (ValueError, IndexError):
                    pass
        return CoverageData(source="go", covered_lines=covered)

    @classmethod
    def _parse_cobertura(cls, path: Path, project_root: Path) -> CoverageData:
        """
        Cobertura XML (stdlib xml.etree — no deps).
        <line number="10" hits="3"/>
        """
        covered: dict[str, set[int]] = {}
        tree = ET.parse(str(path))
        for cls_elem in tree.findall(".//class"):
            filename = cls_elem.get("filename", "")
            abs_path = str(Path(filename) if Path(filename).is_absolute()
                           else project_root / filename)
            lines: set[int] = set()
            for line_elem in cls_elem.findall(".//line"):
                try:
                    hits = int(line_elem.get("hits", "0"))
                    if hits > 0:
                        line_no = int(line_elem.get("number", "0"))
                        if line_no >= 1:
                            lines.add(line_no)
                except ValueError:
                    pass
            # Cobertura emits one <class> per class; multiple classes (inner/anonymous)
            # share the same filename in Java. Union to preserve all covered lines.
            covered.setdefault(abs_path, set()).update(lines)
        return CoverageData(source="cobertura", covered_lines=covered)
```

**Integration vào `_compute_dead_code_confidence()`** (trong indexer):

```python
def _compute_dead_code_confidence(
    self,
    symbol_path: str,
    line_start: int,
    line_end: int,
    caller_count: int,
    is_entry_point: bool,
    is_private: bool,
    # is_private detection per language (Phase 1, AST-based):
    #   Python:     name starts with "_" (single underscore) AND not "__dunder__"
    #   TypeScript: not in "export" statement descendants
    #   JavaScript: not in "export_statement" descendants (ESM); module.exports absent
    #   Java:       access_modifier == "private" on class/method/field
    #   Rust:       no "pub"/"pub(crate)"/"pub(super)" visibility_modifier
    #   Go:         name starts with lowercase letter (unexported)
    #   C#:         access_modifier == "private" or "internal"
    #   Kotlin:     visibility_modifier == "private" or "internal"
    #   Swift:      access_level_modifier == "private" or "fileprivate"
    #   C/C++:      conservative false (no reliable private detection without full TU)
    #   Ruby/PHP:   conservative false (dynamic scoping)
    #   Default:    false (unknown beats mis-classified)
    scope_clear: bool,    # True khi: module-level function/class (không phải method trong class)
                          #           VÀ không bị exclude bởi __all__ nếu __all__ tồn tại
                          #           VÀ file không phải vendor/generated
                          # False khi: scope không xác định được (class method với dynamic dispatch,
                          #            __getattr__ magic, hay symbol trong generated code)
    # scope_clear detection per language (Phase 1, AST-based):
    #   Python:     symbol parent is module (not class_definition body); if __all__ exists
    #               in file, symbol.name must appear in __all__ list; file not in vendor/generated
    #   TypeScript: symbol at module scope (parent is program node), not nested in class/namespace
    #   JavaScript: same as TypeScript
    #   Java:       top-level class (parent is program); inner classes → False
    #   Rust:       item at crate root (parent is source_file), not inside impl/mod block
    #   Go:         function/type at package level (parent is source_file)
    #   C/C++:      namespace-level or file-level (not inside class/struct body)
    #   C#/Kotlin:  top-level class/function (not nested)
    #   Swift:      top-level declaration (not inside extension/class body)
    #   Default:    False (conservative — scope unclear → lower dead_code_confidence)
    coverage: CoverageData,
) -> tuple[str, str]:
    """
    Returns (dead_code_confidence, dead_code_source).
    dead_code_source: "static" | "static+coverage"
    """
    if is_entry_point or caller_count > 0:
        source = "static+coverage" if coverage.source != "none" else "static"
        return "none", source

    runtime_covered = (
        coverage.source != "none"
        and coverage.is_covered(symbol_path, line_start, line_end)
    )

    if runtime_covered:
        # Runtime execution confirmed — static graph blind spot (dynamic dispatch,
        # reflection, scheduled job, decorator-registered callback).
        return "low", "static+coverage"

    # Static-only path (coverage unavailable or not hit)
    source = "static+coverage" if coverage.source != "none" else "static"
    if is_private:
        return "high", source
    if scope_clear:
        return "medium", source
    return "low", source
```

`dead_code_source: "static+coverage"` có nghĩa là cả static analysis lẫn coverage data đều
được dùng. Agent có thể trust `"high"` mạnh hơn khi `dead_code_source == "static+coverage"`.

**Server startup** (trong `serve` command):

```python
# Phase 0 — sau DB init + migration, trước Phase 1:
coverage_data = CoverageReader.load(project_root)
if coverage_data.source != "none":
    print(f"[codeindex] Coverage data loaded: {coverage_data.source}, "
          f"{len(coverage_data.covered_lines)} files")
# coverage_data passed to indexer; re-loaded on indexing_status(retry_embeddings=true)
```

---

### WAL Mode

```python
conn.execute("PRAGMA journal_mode=WAL")
```

Multiple concurrent readers + one writer không block nhau.

---

### Auto-gitignore

```python
import fnmatch

def ensure_gitignore(project_root: Path) -> None:
    if not (project_root / ".git").is_dir():
        return

    gitignore = project_root / ".gitignore"
    entries = [".codeindex/index.db-wal", ".codeindex/index.db-shm"]
    existing = gitignore.read_text(encoding="utf-8") if gitignore.exists() else ""
    existing_patterns = [line.strip() for line in existing.splitlines() if line.strip() and not line.startswith("#")]

    def _is_covered(entry: str, patterns: list[str]) -> bool:
        """Check if entry is already covered by existing patterns.
        Understands Git semantics: directory patterns (trailing /) cover all files within."""
        for p in patterns:
            if fnmatch.fnmatch(entry, p):
                return True
            # Git directory pattern: "dir/" covers all paths starting with "dir/"
            if p.endswith("/") and entry.startswith(p):
                return True
            # Git directory pattern without trailing slash but with slash inside
            if "/" in p and not p.endswith("/") and fnmatch.fnmatch(entry, p + "*"):
                return True
        return False

    to_add = [e for e in entries if not _is_covered(e, existing_patterns)]

    if to_add:
        with gitignore.open("a", encoding="utf-8") as f:
            f.write("\n# Code Intelligence MCP — WAL files (auto-generated)\n")
            f.write("\n".join(to_add) + "\n")
        print(f"[codeindex] Added to .gitignore: {', '.join(to_add)}")
```

Silent khi entries đã exist. Không modify `.gitignore` nếu không có `.git/` directory.

---

### K-core Coreness Computation — audited (F4, F11)

Chạy cuối Phase 2, sau khi toàn bộ `call_edges` đã insert.

**Lý do**: Pure in-degree (`caller_count`) miss bridge functions — node kết nối modules,
in-degree vừa phải nhưng đứng ở điểm giao quan trọng. K-core captures mutual reinforcement:
node A có coreness cao khi nó liên kết với nhiều nodes có coreness cao (recursive). Research
basis: Pan 2014, Meyer 2014, Qu 2021 (IEEE TSE, 47 citations) — k-core outperforms degree
centrality, betweenness, closeness cho software dependency graphs. Qu 2021: +11.5–12.6%
bug prediction accuracy trên 18 Java systems.

**Undirected interpretation**: mỗi directed edge `(from → to)` contribute adjacency cả
hai chiều. Cả ba papers đều dùng undirected variant — phù hợp với hub identification.

**Peeling algorithm** (O(V+E) thật — [F11] v2.6.1 implement `min(degree[v] for v in remaining)`
là O(V·√E+E), không phải O(V+E) như claim. v2.6.2 fix bằng bucket-by-degree với pointer
chỉ tăng, đạt O(V+E) thật. Cascading peel, handles cycles/isolates — semantics không đổi):

```python
from collections import defaultdict

def compute_coreness(conn: sqlite3.Connection) -> dict[str, int]:
    """
    [F11] O(V+E) qua bucket-by-degree với pointer chỉ tăng (k_ptr không reset mỗi
    vòng). Giữ nguyên semantics round-based/cascading.

    QUAN TRỌNG [F4]: Keys trong dict là qualified_name — fully-qualified identifier
    đồng nhất với call_edges.from_symbol và call_edges.to_symbol.
    KHÔNG dùng symbols.name vì name không unique cross-file.
    Returns: {qualified_name → coreness_value}
    """
    adj: dict[str, set[str]] = defaultdict(set)
    for (from_sym, to_sym) in conn.execute(
        "SELECT from_symbol, to_symbol FROM call_edges"
    ):
        adj[from_sym].add(to_sym)
        adj[to_sym].add(from_sym)

    if not adj:
        return {}

    degree = {node: len(neighbors) for node, neighbors in adj.items()}
    max_deg = max(degree.values())

    # buckets[d] = set các node hiện có degree đúng bằng d
    buckets: list[set[str]] = [set() for _ in range(max_deg + 1)]
    for node, d in degree.items():
        buckets[d].add(node)

    coreness: dict[str, int] = {}
    remaining_count = len(degree)
    k_ptr = 0   # [F11] CHỈ tăng, không reset — đây là fix chính cho complexity

    while remaining_count > 0:
        # Amortized O(max_deg) cho TOÀN BỘ run (k_ptr không lùi), không phải
        # O(max_deg) mỗi vòng như min()-scan của bản gốc.
        while k_ptr <= max_deg and not buckets[k_ptr]:
            k_ptr += 1
        if k_ptr > max_deg:
            break

        to_peel = buckets[k_ptr]
        while to_peel:
            v = to_peel.pop()
            coreness[v] = k_ptr
            remaining_count -= 1
            for u in adj[v]:
                if u in coreness:
                    continue
                du = degree[u]
                if du <= k_ptr:
                    continue
                buckets[du].discard(u)
                degree[u] = du - 1
                if degree[u] <= k_ptr:
                    buckets[k_ptr].add(u)
                else:
                    buckets[degree[u]].add(u)

    return coreness
```

**Vì sao đạt O(V+E) thật**: `k_ptr` chỉ tăng, tổng số bước của vòng `while k_ptr <= max_deg`
cộng dồn suốt toàn bộ hàm ≤ `max_deg + 1`. Mỗi edge bị "chạm" tối đa 1 lần thực sự.
Mỗi node bị peel đúng 1 lần.
→ O(V) (peel) + O(E) (decrement) + O(max_deg) (k_ptr advance, `max_deg ≤ V`) = **O(V+E)**.

**[F4] Batch UPDATE dùng `qualified_name`**:

v2.6 dùng `UPDATE symbols SET coreness = ? WHERE name = ?` — `name` **không unique**
cross-file. Fix:

```python
updates = [(v, sym) for sym, v in coreness.items()]
conn.execute("UPDATE symbols SET coreness = 0")   # explicit baseline for ALL symbols (including isolated nodes not in adj)
conn.executemany(
    "UPDATE symbols SET coreness = ? WHERE qualified_name = ?",
    updates
)
# executemany = single transaction, significantly faster cho thousands of rows.
# Isolated nodes (no edges) retain coreness = 0 from the blanket UPDATE above,
# independent of DB migration DEFAULT value — no fragile dependency on schema defaults.
```

**`is_hub` revised — `update_is_hub_flags`, [F6] `caller_count_percentile` định nghĩa
inline**:

```python
import bisect

def update_is_hub_flags(conn: sqlite3.Connection, config) -> None:
    """
    Tính caller_count_percentile, p75_coreness, và evaluate is_hub cho toàn bộ symbols.
    [F6] caller_count_percentile: fraction of symbols with caller_count <= this
         symbol's caller_count, among symbols với caller_count >= 1.
    Query bao gồm symbols với coreness > 0 (kể cả caller_count == 0) để detect
    dispatcher hubs — top-level orchestrators có out-degree cao, in-degree = 0.
    """
    rows = conn.execute(
        "SELECT qualified_name, caller_count, coreness "
        "FROM symbols WHERE caller_count >= 1 OR coreness > 0"
    ).fetchall()

    conn.execute("UPDATE symbols SET is_hub = 0")

    if not rows:
        return

    caller_rows = [r for r in rows if r[1] >= 1]
    sorted_counts = sorted(r[1] for r in caller_rows)
    total = len(sorted_counts) if sorted_counts else 1

    def percentile_rank(caller_count: int) -> float:
        if not sorted_counts:
            return 0.0
        return bisect.bisect_right(sorted_counts, caller_count) / total

    all_coreness = [r[2] for r in rows if r[2] > 0]
    if not all_coreness:
        p75_coreness = config.hub_threshold.min_callers
    else:
        all_coreness.sort()
        pct = config.hub_threshold.coreness_pct           # default 75
        idx = max(0, int(len(all_coreness) * pct / 100) - 1)
        p75_coreness = max(all_coreness[idx], config.hub_threshold.min_callers)

    top_threshold = 1.0 - config.hub_threshold.top_pct / 100.0

    updates = []
    for qname, caller_count, coreness in rows:
        caller_pct = percentile_rank(caller_count)
        is_hub = (
            # [F12] Tách threshold — min_callers (cao) cho degree-hub, min_callers_bridge
            # (thấp hơn) cho bridge-hub. Bắt "moderate in-degree, high coreness".
            (caller_count >= config.hub_threshold.min_callers and caller_pct >= top_threshold)      # degree-hub
            or
            (caller_count >= config.hub_threshold.min_callers_bridge and coreness >= p75_coreness)  # bridge-hub [F12]
        )
        updates.append((is_hub, qname))

    conn.executemany(
        "UPDATE symbols SET is_hub = ? WHERE qualified_name = ?",
        updates
    )
```

**[F12] Tách threshold degree-hub / bridge-hub**:
- `min_callers` (default 5) vẫn giữ cho degree-hub.
- `min_callers_bridge` (default 2) cho bridge-hub — mở cho "moderate in-degree, high coreness"
  (caller_count 2–4) như example gốc mô tả.

**[F5] Incremental update strategy — scale guidance + debounce**:

| Codebase size      | Python CPython estimate | Recommendation                                   |
|--------------------|--------------------------|---------------------------------------------------|
| ≤ 50k edges         | < 50ms                   | Global recompute — OK                              |
| 50k–500k edges       | 50ms–500ms               | Global recompute + 500ms debounce                  |
| > 500k edges         | > 500ms                  | Global recompute + debounce; log elapsed; consider incremental (v2.8) |

Debounce 500ms cooldown trên `on_change` trigger đủ để prevent thrashing trong active
coding sessions (10+ files/giây).

---

### Semantic Embedding Pipeline

Track độc lập với phase ladder. Opt-in, disabled by default.

```
embeddings_status: "disabled" | "downloading" | "embedding" | "ready" | "failed"
```

**State machine**:

```
disabled    → (enable trong config)           → downloading
downloading → (download success)              → embedding
downloading → (download fail / timeout)       → failed
embedding   → (embed complete)                → ready
embedding   → (OOM / disk full / crash)       → failed
failed      → (retry_embeddings: true)        → downloading
failed      → (server restart)               → downloading
failed      → (không làm gì)                → stays "failed" — KHÔNG tự retry vô hạn
ready       → (on_change: symbol updated)    → re-embed changed symbols only → stays "ready"
```

**Stack**: `fastembed` (ONNX runtime, ~60-100MB, không PyTorch) + `sqlite-vec` (KNN
virtual table trong cùng `index.db`). Model mặc định `BAAI/bge-base-en-v1.5` (768-dim).
Option chất lượng cao hơn: `nomic-ai/nomic-embed-text-v1.5` (768-dim, 8192 context).

**Hybrid fusion** — Reciprocal Rank Fusion với configurable k:

```
RRF_score(doc) = 1/(k + rank_fts) + 1/(k + rank_vec)
```

`k = config.search.rrf_k` (default 20). k=20 cho discrimination tốt hơn trên short
result lists (top-10) so với k=60 được thiết kế cho large IR corpora. Deduplicated,
re-ranked bằng RRF score.

---

## Layer 2: Tool Surface — 16 Tools

---

### Shared Type: `SuggestedNext`

Tất cả 16 tools đều emit field `suggested_next?: SuggestedNext` (optional, absent khi
không có gợi ý rõ ràng):

```typescript
SuggestedNext = {
  tool: string,
  reason: string,
  args?: Record<string, string | number | boolean>
}
```

**Logic per tool** (pure function, không có side effects, không persist):

| Tool | Condition | `tool` | `reason` | `args` |
|---|---|---|---|---|
| `repo_overview` | `indexing_phase != "ready"` | `"indexing_status"` | `"Monitor until phase=ready before using graph tools"` | — |
| `repo_overview` | `embeddings_status == "failed"` | `"indexing_status"` | `"Recover embeddings"` | `{retry_embeddings: true}` |
| `repo_overview` | phase ready, embeddings ready OR disabled | `"locate"` | `"Start exploration"` | — |
| `search` | top result exists, kind=symbol | `"locate"` | `"Full context in 1 call (replaces symbol_info)"` | `{query: results[0].name, kind: "symbol"}` |
| `search` | results empty, kind != hybrid, kind != semantic | `"search"` | `"Try hybrid for broader recall"` | `{kind: "hybrid"}` |
| `search` | results empty, kind == semantic | `"search"` | `"Semantic index may not cover this — try text or hybrid search"` | `{kind: "text"}` |
| `search` | results empty, kind == hybrid | `"search"` | `"Embeddings may not cover this query — try exact text search or broaden wording"` | `{kind: "text"}` |
| `locate` | `top_result.symbol.is_hub == true` | `"edit_context"` | `"Hub detected — mandatory pre-edit check"` | `{symbol: ..., path: ...}` |
| `locate` | `top_result.symbol.dead_code_confidence == "high"` | `"callers"` | `"Verify dead code — no static callers found"` | `{symbol: ...}` |
| `locate` | `top_result.symbol.ambiguous == true` | `"symbol_info"` | `"Disambiguate top result"` | `{name: candidates[0].name, path: candidates[0].path}` |
| `locate` | default; results non-empty | `"source"` | `"Read implementation"` | `{target: results[0].name}` |
| `locate` | top_result absent (empty results) | `"search"` | `"No match — broaden with hybrid search"` | `{kind: "hybrid"}` |
| `symbol_info` | `is_hub == true` | `"edit_context"` | `"Hub — check blast radius before modifying"` | `{symbol: name, path: path}` |
| `symbol_info` | `health.test_files == []` | `"search"` | `"No tests found — search for coverage"` | `{query: name + " test", kind: "text"}` |
| `symbol_info` | default | `"source"` | `"Read implementation"` | `{target: name}` |
| `source` | `metadata.is_hub == true` *(chỉ khi `include_metadata=true`)* | `"edit_context"` | `"Hub — mandatory pre-edit context"` | — |
| `source` | default | `"callers"` | `"Check who uses this before modifying"` | `{symbol: ...}` |
| `callers` | `any edge_confidence=="textual"` OR `total_direct > 10` | `"edit_context"` | `"High blast radius or uncertain edges — verify before modifying"` | — |
| `callers` | `0 < total_direct <= 10` AND no textual edge | `"source"` | `"Read top caller implementation"` | `{target: direct[0].caller_symbol}` |
| `callees` | `total_direct > 0` | `"path"` | `"Trace specific call chain"` | — |
| `dependencies` | `imported_by_total > 20` | `"callers"` | `"High fan-in — check symbol blast radius"` | — |
| `path` | `exists == true` | `"source"` | `"Read meeting node implementation"` | — |
| `path` | `terminated_by == "timeout"` | `"path"` | `"Retry with smaller max_hops"` | `{max_hops: 4}` |
| `path` | `terminated_by == "max_hops"` | `"path"` | `"Path may exceed hop limit — retry with larger max_hops, or check the reverse direction"` | `{max_hops: <current+4>, from_symbol: <to_symbol>, to_symbol: <from_symbol>}` |
| `edit_context` | always | `"diff_impact"` | `"MANDATORY after changes — verify blast radius"` | — |
| `session_context` | `frontier non-empty` | `"file_overview"` | `"Explore top frontier file"` | `{path: frontier[0].path}` |
| `session_context` | `frontier empty` | `"repo_overview"` | `"Frontier exhausted — refresh map"` | — |
| `diff_impact` | `aggregate_risk in [critical, high]`, `unindexed_files == []` | `"callers"` | `"Verify high-risk callers manually"` | `{symbol: affected_symbols[0].name}` |
| `diff_impact` | `aggregate_risk == "medium"`, `unindexed_files == []` | `"callers"` | `"Medium-risk changes — spot-check key callers"` | `{symbol: affected_symbols[0].name}` |
| `diff_impact` | `aggregate_risk == "unknown"`, `unindexed_files == []` | `"indexing_status"` | `"Risk unknown — check index state"` | — |
| `diff_impact` | `unindexed_files non-empty` | `"indexing_status"` | `"Wait for index before treating as safe"` | — |
| `hotspots` | default | `"file_overview"` | `"Inspect highest-risk file"` | `{path: hotspots[0].path}` |
| `understand` | `ambiguous` present | `"symbol_info"` | `"Ambiguous — retry with specific candidate"` | `{name: ambiguous.candidates[0].name, path: ambiguous.candidates[0].path}` |
| `understand` | `is_hub == true` | `"edit_context"` | `"Hub — mandatory pre-edit check"` | `{symbol: name, path: path}` |
| `understand` | default | `"edit_context"` | `"Pre-edit: verify blast radius before modifying"` | `{symbol: name, path: path}` |
| `file_overview` | any `is_hub == true` | `"locate"` | `"Inspect hub symbol"` | `{query: top_hub.name}` |
| `file_overview` | default | `"source"` | `"Read a symbol implementation"` | — |
| `indexing_status` | `phase == "ready"` | `"locate"` | `"Index ready — begin exploration"` | — |
| `indexing_status` | `phase != "ready"` | `"indexing_status"` | `"Still indexing — poll again or use search/source while edges build"` | — |

**Priority rules** (within each tool, conditions evaluated top-to-bottom, first match wins):

- **repo_overview**: Condition 1 (phase not ready) beats Condition 2 (embeddings failed). Embeddings retry chỉ có ý nghĩa khi `phase == "ready"` — retry trước khi index xong là vô nghĩa.
- **diff_impact**: Condition `unindexed_files non-empty` beats `aggregate_risk in [critical, high]`. aggregate_risk không thể trust khi có unindexed files — có thể under-estimated. Giải quyết index trước, sau đó re-evaluate risk.

**Note `path` + `max_hops`**: `"timeout"` gợi ý giảm search space bằng `max_hops` nhỏ hơn.
`"max_hops"` nghĩa là path thật có thể dài hơn limit — gợi ý *tăng* `max_hops`. `args` swap
`from_symbol`/`to_symbol` vì bidirectional BFS có thể asymmetric — retry chiều ngược lại
với cùng budget có thể tìm ra path mà chiều gốc chưa kịp chạm tới.

**Implementation**: pure function, preset-aware.

```python
def compute_suggested_next(
    tool_name: str,
    output: dict,
    available_tools: set[str] | None = None  # None = all tools
) -> SuggestedNext | None:
    hint = _raw_suggested_next(tool_name, output)
    if hint is None:
        return None
    if available_tools is not None and hint.tool not in available_tools:
        return None  # filter out tools not in current preset
    return hint
```

Server truyền `available_tools` từ `PRESET_TOOL_SETS[preset]` khi gọi compute.
Default `None` = no filtering (`--preset=full`).

---

### Tool 1 — `repo_overview`

*ALWAYS call this FIRST at the start of every session — never skip. USE WHEN: starting a new session, switching projects, or after server restart. NOT FOR: per-file details (use file_overview), searching for symbols (use search/locate). Call indexing_status() only when you need file-level counts or embedding error details.*

```
Input: {
  path?: string,
  include_health?: bool,   // default false
  top_n?: number           // default 20, sort by symbol_count desc; fallback path asc when symbol_count is null
}

Output: {
  languages: string[],
  indexing_phase: "scanning" | "parsing" | "building_edges" | "ready",
  embeddings_status: "disabled" | "downloading" | "embedding" | "ready" | "failed",
  module_map: {
    name, path,
    symbol_count: number | null,   // null khi phase != "ready" — KHÔNG dùng 0
    key_exports: string[]  // Top ≤10 exported (public) symbols theo caller_count desc; [] khi phase != "ready"
  }[],
  total_modules: number,
  truncated: bool,
  entry_points: { symbol, path, kind }[],
  stats: { files, symbols, edges },
  health_summary?: { dead_code_count, untested_modules, undocumented_hubs },
  note?: string,       // chỉ có khi indexing_phase != "ready"
  workflow_guide: string,
  suggested_next?: SuggestedNext
}
```

`symbol_count: null` (không phải `0`) khi phase chưa `"ready"`.

`health_summary.undocumented_hubs` có thể tăng sau khi upgrade lên v2.6+, vì `is_hub`
giờ bao gồm bridge hubs trước đây bị miss bởi pure degree detection — expected behavior,
không phải regression.

**`workflow_guide` exact content**:

```
ORIENT:   repo_overview() first, always. Check indexing_phase + embeddings_status.
SCAN:     hotspots() optional — proactive risk map after orientation (Stage 1.5).
LOCATE:   locate(query) → 1 call replaces search+file_overview+symbol_info.
INSPECT:  symbol_info(name) → check is_hub+coreness+health+dead_code_source before source.
READ:     source(symbol) or understand(query) → read implementation. Never native Read tool.
TRACE:    callers(symbol) | callees(symbol) | path(from,to) — require edges_ready:true.
REORIENT: session_context() after 10+ calls or new sub-task.
EDIT:     edit_context(symbol) before any change — check risk_assessment + edge_confidence.
VERIFY:   diff_impact() before commit — check suggested_reviewers for review routing.
```

---

### Tool 2 — `search`

*USE THIS INSTEAD OF native grep, text search, or file browsing tools. USE WHEN: you don't have an exact file path and line number. kind="hybrid" → highest recall (preferred when embeddings ready). NOT FOR: inspecting a file you already have (use file_overview). vs locate: search returns a result list; locate returns search + symbol metadata in one call.*

```
Input: {
  query: string,
  kind: "symbol" | "text" | "file" | "semantic" | "hybrid",
  limit?: number   // default 10
}

Output: {
  results: {
    name, kind, path,
    line_start?: number,
    line_end?: number,
    preview,
    match_type: "exact" | "fts" | "semantic" | "hybrid"
  }[],
  truncated: bool,
  degraded: bool,
  suggestions?: string[],
  embeddings_status?: "disabled" | "downloading" | "embedding" | "ready" | "failed",
  suggested_next?: SuggestedNext
}
```

**Sort policy**:
- `kind="symbol"`: exact match first → BM25 (dual-column weighted) → caller_count desc
- `kind="text"`: BM25 → recency (file mtime desc)
- `kind="file"`: exact match first → path depth asc
- `kind="semantic"`: cosine similarity desc
- `kind="hybrid"`: RRF score desc (k = `config.search.rrf_k`), deduplicated

**Chunk boundary cho `kind="text"`**: N = `config.search.text_chunk_context_lines` (default 10).

```
Case 1 — Match trong function/method/class body:
  → chunk = toàn bộ AST node
  → Nếu node > config.search.text_max_chunk_lines (default 50):
      truncate về N/2 dòng trên + N/2 dòng dưới match line
  → preview = 2-3 dòng từ match line

Case 2 — Match ở module-level:
  → chunk = [max(1, match_line - N), min(file_end, match_line + N)]
  → preview = 2-3 dòng bắt đầu từ match line

Case 3 — Match trong comment hoặc string literal:
  → chunk = containing AST node nếu có, else rule Case 2
  → preview = 3 dòng từ match line
```

**`degraded`**: `true` khi `kind="hybrid"` request nhưng embeddings chưa ready → kết
quả FTS5-only.

**Behavior khi `embeddings_status != "ready"`**:

| `embeddings_status`         | `kind="semantic"`                             | `kind="hybrid"`                                    |
|-----------------------------|-----------------------------------------------|----------------------------------------------------|
| `"disabled"`                | Error `FEATURE_UNAVAILABLE`                   | `degraded: true`, FTS5-only, note gợi ý enable embeddings |
| `"downloading"/"embedding"` | `results: []`, note gợi ý dùng `kind="text"` | `degraded: true`, FTS5-only, note                  |
| `"failed"`                  | Error `EMBEDDING_FAILED`                      | Error `EMBEDDING_FAILED`                            |
| `"ready"`                   | Bình thường, `degraded: false`                | Bình thường, `degraded: false`                      |

---

### Tool 3 — `file_overview`

*USE WHEN: you have a file path and want to see its symbols, structure, and inferred role. vs source: file_overview shows ALL symbols in a file; source reads ONE symbol's body. vs dependencies: file_overview shows what's INSIDE the file; dependencies shows what the file IMPORTS/IS IMPORTED BY.*

```
Input: { path: string, limit?: number }   // limit default 20, sort by caller_count desc

Output: {
  language: string,
  inferred_role: "service" | "model" | "router" | "utility" | "test" | "config" | null,
  symbols: {
    name, kind, signature,
    line_start: number,
    line_end: number,
    caller_count: number | null,
    is_hub: bool | null,
    coreness: number | null        // null khi edges_ready: false
  }[],
  total_symbols: number,
  truncated: bool,
  edges_ready: bool,
  note?: string,
  suggested_next?: SuggestedNext
}
```

**`inferred_role` heuristics** — first-match-wins, `null` nếu không match:

```
1. "test"    → path/filename: *test*, *spec*, *fixture*
2. "router"  → filename contains: router, routes, controller, handler
3. "config"  → filename: config.*, settings.*, constants.*, .env*, *.ini, *.toml, *.yaml
4. "model"   → filename contains: model, entity, schema, dto, domain
5. "service" → filename contains: service, manager, provider, repository
6. "utility" → filename contains: util, helper, common, shared, lib
7. null      → no pattern matched
```

---

### Tool 4 — `symbol_info`

*USE WHEN: you have a symbol name and want metadata + health signals BEFORE reading source. Check is_hub + coreness before deciding whether to modify — hub symbols need edit_context. NOT FOR: reading source (use source), finding symbols (use search/locate). vs source: symbol_info is metadata-only (no code body).*

```
Input: { name: string, path?: string }

// NORMAL RESPONSE
Output: {
  name, kind, signature, docstring,
  path, line_start, line_end,
  language,
  caller_count: number | null,
  is_hub: bool | null,
  coreness: number | null,       // null khi edges_ready: false
  health: Health,
  edges_ready: bool,
  note?: string,
  suggested_next?: SuggestedNext
}

// AMBIGUOUS RESPONSE
Output: {
  ambiguous: true,
  candidates: { name, kind, path, class_context: string | null, caller_count: number | null, language: string, line_start: number, line_end: number, signature?: string }[]
}
```

`coreness` là raw coreness number từ k-core peeling. Agent tự judge mức độ "high
coreness" dựa vào context — không có threshold cứng trong output.

---

### Tool 5 — `source`

*USE THIS INSTEAD OF native Read file tool — reads symbol-precise code, always fresh from disk. USE WHEN: you need to read the actual implementation of a specific function/class/method. Use source(target, include_metadata=true) to skip a prior symbol_info call. Use source("path:line_start-line_end") for exact location. NEVER use native Read tool on a full file — it floods context with unrelated code.*

```
Input: { target: string, context_lines?: number, include_metadata?: bool }

// NORMAL RESPONSE
Output: {
  content: string,
  path, line_start, line_end,
  token_estimate: number,
  data_source: "disk",
  cached: bool,     // true → symbol location resolved from index cache; file content always re-read from disk
  metadata?: {
    language: string,
    caller_count: number | null,
    is_hub: bool | null,
    coreness: number | null,
    health: Health,
    edges_ready: bool
  },
  suggested_next?: SuggestedNext
}

// AMBIGUOUS RESPONSE
Output: { ambiguous: true, candidates: { name, kind, path, class_context, caller_count, language, line_start: number, line_end: number, signature?: string }[] }
```

`context_lines` mở rộng đối xứng từ symbol range, cap tại file boundaries. Khi
`target = "path:line_start-line_end"` — exact format — không bao giờ ambiguous.

---

### Tool 6 — `callers`

*USE WHEN: you need to know who calls a specific symbol — blast radius scan, refactoring impact. USE THIS for SYMBOL-LEVEL call sites. NOT for file-level imports (use dependencies). vs edit_context: callers is for exploration; edit_context is the mandatory pre-edit tool.*

```
Input: { symbol: string, path?: string, max_depth?: number, limit?: number }
       // max_depth default 1, cap tại config.callers.max_depth_cap (default 4); limit default: no cap — all direct results returned

Output: {
  direct: { caller_symbol, caller_path, line: number, preview, edge_confidence: EdgeConfidence }[],
  total_direct: number,
  truncated: bool,
  transitive_count?: number | null,   // present iff max_depth > 1
  transitive_capped?: bool,           // present iff max_depth > 1
  edges_ready: bool,
  note: string,
  ambiguous?: bool,
  candidates?: { name, path, kind, line_start: number, line_end: number }[],
  suggested_next?: SuggestedNext
}
```

**`transitive_count` semantics**: tổng unique symbols reachable trong `max_depth` hops,
không cap bởi `limit`. `null` khi BFS timeout (kèm `transitive_capped: true`). Khi
`max_depth == 1`: cả hai field **absent**. Transitive BFS bounded bởi
`config.callers.transitive_timeout_ms` (default 3000ms) — khi timeout, trả
`transitive_count: null` và `transitive_capped: true`.

`direct[].line` là call site — singular, không phải definition range.

**Performance note**: query `SELECT from_symbol FROM call_edges WHERE to_symbol = ?`
hưởng lợi từ `idx_call_edges_to` — không còn full scan trên codebase lớn.

---

### Tool 7 — `callees`

*USE WHEN: you need to trace what a symbol calls — understanding logic flow, internal deps. NOT for finding who calls this symbol (use callers). vs callers: callers=upward (who calls X); callees=downward (what X calls).*

```
Input: { symbol: string, path?: string, max_depth?: number, limit?: number }
       // max_depth default 1, cap tại config.callees.max_depth_cap (default 4); limit default: no cap — all direct results returned

Output: {
  direct: { callee_symbol, callee_path, line: number, preview, edge_confidence: EdgeConfidence }[],
  total_direct: number,
  truncated: bool,
  transitive_count?: number | null,
  transitive_capped?: bool,
  edges_ready: bool,
  note: string,
  ambiguous?: bool,
  candidates?: { name, path, kind, line_start: number, line_end: number }[],
  suggested_next?: SuggestedNext
}
```

Same transitive semantics và depth cap như `callers`. Transitive BFS bounded bởi
`config.callees.transitive_timeout_ms` (default 3000ms).

---

### Tool 8 — `dependencies`

*USE WHEN: you need to understand file-level architectural connections. USE THIS for FILE-LEVEL import graph. NOT for symbol-level call sites (use callers/callees). vs callers/callees: dependencies is file-level; callers/callees is symbol-level.*

```
Input: { path: string }

Output: {
  imports: { module, resolved_path?: string, symbols_used: string[] }[],
  imports_total: number,
  imports_truncated: bool,
  imported_by: { path, symbols_used: string[] }[],
  imported_by_total: number,
  imported_by_truncated: bool,
  edges_ready: bool,
  note: string,
  suggested_next?: SuggestedNext
}
```

Không nhận symbol name → không cần Ambiguity Contract. `imports` capped tại
`config.dependencies.max_imports` (default 100). `imported_by` capped tại
`config.dependencies.max_imported_by` (default 100).

---

### Tool 9 — `path`

*USE WHEN: you need to trace if and how symbol A can reach symbol B through call chain. Bidirectional BFS — cycles terminate cleanly. path is DIRECTED: A→B ≠ B→A. terminated_by=null + exists=true/false → certain result. terminated_by="timeout"/"max_hops" → exists=null → inconclusive.*

```
Input: {
  from_symbol: string, from_path?: string,
  to_symbol: string, to_path?: string,
  max_paths?: number,    // default 5
  max_hops?: number,     // default 8, hard ceiling 20
  timeout_ms?: number    // default 5000
}

Output: {
  exists: bool | null,
  direction: "from→to",
  routes: { steps: { symbol, path, line_start, line_end, kind, edge_confidence?: EdgeConfidence }[], length: number }[],
  total_found: number,   // total meeting nodes discovered by BFS (may exceed len(routes) when truncated by max_paths)
  truncated: bool,
  terminated_by: null | "path_count" | "max_hops" | "timeout",
  hops_clamped: bool,    // true nếu request max_hops vượt hard ceiling (20) và đã bị clamp về 20; false = max_hops dùng như request hoặc default
  edges_ready: bool,
  ambiguous?: bool,
  candidates?: { name, path, kind, line_start: number, line_end: number }[],
  note: string,
  suggested_next?: SuggestedNext
}
```

`routes[].steps[0]` (source node) không có incoming edge → `edge_confidence` absent
(field bị omit, không phải `null`).

**Mapping `terminated_by` → `exists`**:

| `terminated_by` | `exists`         | Ý nghĩa                                                      |
|-----------------|------------------|--------------------------------------------------------------|
| `null`          | `true` / `false` | Search hoàn tất — kết quả chắc chắn                         |
| `"path_count"`  | `true`           | Found, dừng vì đủ `max_paths`                                |
| `"max_hops"`    | `null`           | Chưa reach trong giới hạn hop — không loại trừ path sâu hơn |
| `"timeout"`     | `null`           | Search bị interrupt                                          |

**BFS algorithm — Bidirectional, audited (F1, F2, F3, F10)**

v2.5.2 dùng unidirectional BFS: worst case O(V+E) — visit gần hết graph trước khi reach
`to_symbol`. v2.6 nâng cấp lên Bidirectional BFS với 4 vấn đề được fix:

- **F1 — N queries/level**: batch 1 query/level thay vì N queries (N = frontier size).
  Frontier 500 nodes với SQLite local disk ~0.1–0.5ms/query = 250ms/level → overhead át
  lợi ích thuật toán. Fix: batch query.
- **F2 — tie-break luôn nghiêng forward**: `<=` → effectively unidirectional. Fix:
  alternate theo `tie_toggle` khi bằng nhau.
- **F3 — `meeting_nodes` là list**: O(n) duplicate check mỗi lần thêm. Fix: `set[str]`.
- **F10 — empty-frontier deadlock + tie-break parity không robust**:
  - Bug 1: `len(forward_frontier) == 0` luôn nhỏ hơn `len(backward_frontier) > 0` → khi
    forward cạn, code mãi chọn expand forward (expand `set()` = noop). Fix: exhaustion flags.
  - Bug 2: global `step` tăng mọi vòng → parity tại 2 lần tie cách nhau bởi vòng không-tie
    lẻ trùng nhau. Fix: `tie_toggle` chỉ flip khi thực sự vào nhánh tie.

**Batch DB Helpers — F1**:

```python
def db_callees_batch(self, nodes: set[str]) -> dict[str, list[tuple[str, str]]]:
    """1 SQL query cho toàn bộ frontier — thay vì N queries (N = frontier size)."""
    if not nodes:
        return {}
    placeholders = ",".join("?" * len(nodes))
    rows = self.conn.execute(
        f"SELECT from_symbol, to_symbol, edge_confidence "
        f"FROM call_edges WHERE from_symbol IN ({placeholders})",
        list(nodes)
    ).fetchall()
    result: dict[str, list] = defaultdict(list)
    for from_s, to_s, conf in rows:
        result[from_s].append((to_s, conf))
    return dict(result)

def db_callers_batch(self, nodes: set[str]) -> dict[str, list[tuple[str, str]]]:
    """1 SQL query cho toàn bộ backward frontier. Uses idx_call_edges_to."""
    if not nodes:
        return {}
    placeholders = ",".join("?" * len(nodes))
    rows = self.conn.execute(
        f"SELECT to_symbol, from_symbol, edge_confidence "
        f"FROM call_edges WHERE to_symbol IN ({placeholders})",
        list(nodes)
    ).fetchall()
    result: dict[str, list] = defaultdict(list)
    for to_s, from_s, conf in rows:
        result[to_s].append((from_s, conf))
    return dict(result)
```

Với batch, overhead giảm từ ~250ms/level xuống ~1–2ms/level → claim speedup bidirectional
(2.9–6.89×, Haeupler et al. 2024; Dong et al. 2025) **trở thành thực tế** thay vì lý thuyết.

**Algorithm — F1, F2, F3, F10 áp dụng đầy đủ**:

```python
def bidirectional_bfs_path(
    self, from_sym: str, to_sym: str,
    max_hops: int, max_paths: int, timeout_ms: int
) -> tuple[list, bool | None, str | None]:
    """
    Returns: (routes, exists, terminated_by)
    routes: list of paths, mỗi path là list of (symbol, incoming_edge_confidence).
    """
    if from_sym == to_sym:
        return [[(from_sym, None)]], True, None   # self-loop: length = 0 (0 edges traversed)

    start = time.monotonic()
    deadline = start + timeout_ms / 1000

    forward_pred:  dict[str, tuple | None] = {from_sym: None}
    backward_pred: dict[str, tuple | None] = {to_sym:   None}
    forward_frontier:  set[str] = {from_sym}
    backward_frontier: set[str] = {to_sym}

    f_depth = 0
    b_depth = 0
    meeting_nodes: set[str] = set()   # [F3] set thay vì list

    # [F10] Exhaustion tracked explicitly — KHÔNG suy ra từ len(frontier) == 0.
    forward_exhausted = False
    backward_exhausted = False

    # [F10] Counter tie riêng — chỉ flip khi THỰC SỰ vào nhánh tie.
    tie_toggle = True

    while not (forward_exhausted and backward_exhausted):
        if time.monotonic() > deadline:
            return [], None, "timeout"
        if f_depth + b_depth >= max_hops:
            return [], None, "max_hops"

        # [F2+F10] Vertex-balanced: expand smaller frontier.
        # Khi equal → alternate qua tie_toggle (chỉ flip trong nhánh tie này).
        if forward_exhausted:
            expand_forward = False
        elif backward_exhausted:
            expand_forward = True
        elif len(forward_frontier) < len(backward_frontier):
            expand_forward = True
        elif len(backward_frontier) < len(forward_frontier):
            expand_forward = False
        else:
            expand_forward = tie_toggle
            tie_toggle = not tie_toggle

        if expand_forward:
            callee_map = self.db_callees_batch(forward_frontier)   # [F1] batch
            new_f: set[str] = set()
            for node in forward_frontier:
                for callee, edge in callee_map.get(node, []):
                    if callee not in forward_pred:
                        forward_pred[callee] = (node, edge)
                        new_f.add(callee)
                        if callee in backward_pred:
                            meeting_nodes.add(callee)
            forward_frontier = new_f
            if not forward_frontier:
                forward_exhausted = True       # [F10]
            else:
                f_depth += 1
        else:
            caller_map = self.db_callers_batch(backward_frontier)   # [F1] batch
            new_b: set[str] = set()
            for node in backward_frontier:
                for caller, edge in caller_map.get(node, []):
                    if caller not in backward_pred:
                        backward_pred[caller] = (node, edge)
                        new_b.add(caller)
                        if caller in forward_pred:
                            meeting_nodes.add(caller)
            backward_frontier = new_b
            if not backward_frontier:
                backward_exhausted = True      # [F10]
            else:
                b_depth += 1

        if meeting_nodes:
            break

    if not meeting_nodes:
        return [], False, None  # cả hai phía exhausted — exists: false, chắc chắn

    routes = []
    for meeting in meeting_nodes:
        fwd = []
        node = meeting
        while node is not None:
            pred = forward_pred[node]
            fwd.append((node, pred[1] if pred else None))
            node = pred[0] if pred else None
        fwd.reverse()

        bwd = []
        node = meeting
        while True:
            pred = backward_pred.get(node)
            if pred is None:
                break
            next_node, edge = pred
            bwd.append((next_node, edge))
            node = next_node

        routes.append(fwd + bwd)
        if len(routes) >= max_paths:
            # total_found = len(meeting_nodes) — may exceed len(routes)
            return routes, True, "path_count"

    return routes, True, None
    # Caller sets output.total_found = len(meeting_nodes) from BFS,
    # output.truncated = (terminated_by == "path_count")
```

**Path reconstruction trace** (correctness verification): với graph
`from_sym → A → meeting → B → to_sym`:
`fwd` (sau reverse) = `[(from_sym,None),(A,conf_from→A),(meeting,conf_A→meeting)]`;
`bwd` = `[(B,conf_meeting→B),(to_sym,conf_B→to_sym)]`; route ghép lại đúng từng bước.

**Correctness trên directed graph**: `path` không require *shortest* path — chỉ cần
*any* path thoả `max_hops`. Khi meeting node M tìm thấy, path `from → ... → M → ... → to`
tồn tại và total length ≤ forward_depth + backward_depth ≤ max_hops. Classic directed-graph
caveat (meeting node có thể không trên shortest path) không ảnh hưởng vì không claim shortest.

**Path enumeration limitation**: BFS predecessor dicts (`forward_pred`/`backward_pred`)
store 1 predecessor per node — mỗi meeting node chỉ yield 1 path. `max_paths` giới hạn
số meeting nodes được report, không phải tổng simple paths qua mỗi meeting node. Nếu graph
có ít meeting nodes, `len(routes)` có thể < `max_paths` dù có nhiều simple paths tồn tại.
`total_found` reflect số meeting nodes đã yield paths (xem below).

**Cycle detection**: visited sets (`forward_pred`/`backward_pred` keys) prevent revisit —
cycles không cause infinite loop; BFS terminates cleanly. Result (`exists: true/false`) phụ
thuộc vào reachability, không phải sự tồn tại của cycle.

---

### Tool 10 — `edit_context`

*ALWAYS CALL THIS before any code modification — mandatory, never skip. USE WHEN: you are about to edit, refactor, or delete a symbol. NOT FOR: read-only inspection (use symbol_info + source). NOT post-edit (use diff_impact).*

```
Input: { symbol: string, path?: string }

Output: {
  target: { signature, source, path, line_start, line_end, data_source: "disk" },
  callers: { symbol, signature, path, line, edge_confidence: EdgeConfidence }[],
  callers_truncated: bool,
  callers_total: number,
  caller_selection: "priority_ranked",
  callees: { symbol, signature, path, line, edge_confidence: EdgeConfidence }[],
  callees_truncated: bool,
  callees_total: number,
  callee_selection: "priority_ranked",
  blast_radius: { direct_callers, transitive_callers, files_affected },
  risk_assessment: { level: "low" | "medium" | "high" | "critical", reasons: string[] },
  edges_ready: bool,
  index_freshness: { last_sync_ms: number, pending_files: number, stale_callers: bool },
  note: string,
  ambiguous?: bool,
  candidates?: { name, path, kind, line_start: number, line_end: number }[],
  suggested_next?: SuggestedNext
}
```

**Priority order — `callers`** (confidence-first):
1. `edge_confidence: "resolved"` — sub-order: (a) referenced trong test files (b) `is_hub: true`
   (c) closest to entry points — đo bằng BFS hop count từ entry point symbols (main, handler, CLI
   root, exported API); fewer hops = higher priority (d) caller_count desc — tie-break khi
   (a)–(c) bằng nhau
2. `edge_confidence: "inferred"` — cùng sub-order
3. `edge_confidence: "textual"` — cùng sub-order

**Priority order — `callees`**: 1. `"resolved"` 2. `"inferred"` 3. `"textual"`, trong
cùng tier sort by `line` asc.

`edges_ready: false` → cold-start. `index_freshness.stale_callers: true` → index ready
nhưng file vừa đổi chưa re-parse.

**Risk planning: Bridge-hub vs Degree-hub**:

`risk_assessment` dùng `is_hub` làm input chính. `is_hub` capture thêm bridge functions
(caller_count medium, coreness cao) — không chỉ pure high-degree hubs. Khi đọc output:

- `is_hub: true` + `caller_count` **thấp** (không nằm trong top 5%) = **bridge hub**.
  Edit ảnh hưởng cross-module integration — risk cao hơn apparent degree suggest.
- `is_hub: true` + `caller_count` **cao** = degree hub (classic behavior).
- Dùng `coreness` trong `symbol_info`/`source.metadata` để phân biệt:
  `coreness >> p75` + `caller_count` thấp = strong bridge-hub signal.

---

### Tool 11 — `session_context`

*USE WHEN: after 10+ tool calls without convergence, or when starting a new sub-task. session_started_at: save on first call as T₀. Changed T₀ = server restarted.*

```
Input: {}

Output: {
  explored: {
    symbols: { name, path, caller_count: number | null, is_hub: bool | null }[],
    symbols_total: number, symbols_truncated: bool,
    files: string[], files_total: number
  },
  frontier: { path, reason, connection_count }[],
  frontier_degraded: bool,
  frontier_note: string,
  already_fetched: { symbol?: string, path, line_start, line_end }[],
  session_stats: { tool_calls, unique_files_explored },
  session_started_at: string,
  suggested_next?: SuggestedNext
}
```

**`explored.symbols[]` ordering khi truncated**: MRU-ordered (LRU-eviction) — sort by `last_accessed_at DESC`,
keep top 50. Most-recently-used symbols first; least-recently-used evicted khi vượt 50.

**`frontier` algorithm**:

```
frontier = sorted(
  union(
    {f | f ∈ imports_of(explored_files), f ∉ explored_files},
    {f | f ∈ files_containing_callers(explored_symbols), f ∉ explored_files}
  ),
  key=lambda f: connection_count(f), reverse=True
)[:20]
```

`reason`: `"imported_by_explored"`, `"contains_callers_of_explored"`, `"both"`.
`frontier_degraded: true` khi `edges_ready: false`.

**`session_started_at` — restart detection protocol**: lần gọi đầu tiên, agent lưu giá
trị làm T₀. Khác T₀ ở lần gọi sau = server đã restart — `already_fetched`/`explored`
không liên tục với trước.

---

### Tool 12 — `diff_impact`

*CALL THIS after every code change, BEFORE commit or push — never skip. USE WHEN: you have uncommitted changes and want to verify blast radius. NOT FOR: pre-edit analysis (use edit_context). diff=<text> → no git needed. staged=true or commits="..." → requires git in PATH. vs edit_context: edit_context=pre-edit (proactive risk check); diff_impact=post-edit (verification before commit).*

```
Input: { diff?: string, staged?: bool, commits?: string }   // đúng một trong ba

Output: {
  affected_symbols: {
    symbol, path, line_start, line_end, kind,
    change_type: "modified" | "added" | "deleted" | "renamed",
    signature_changed: bool,
    risk_assessment: { level: "low" | "medium" | "high" | "critical", reasons: string[] }
  }[],
  affected_symbols_total: number,
  affected_symbols_truncated: bool,
  aggregate_risk: "low" | "medium" | "high" | "critical" | "unknown",
  blast_radius: { direct_callers, transitive_callers, files_affected },
  high_risk_callers: { symbol, path, line, reason, edge_confidence: EdgeConfidence }[],
  high_risk_callers_truncated: bool,
  edges_ready: bool,
  edge_confidence_note?: string,
  unindexed_files: string[],
  suggested_reviewers?: ReviewerSuggestion[],  // present khi risk >= medium AND owners found
  note?: string,
  suggested_next?: SuggestedNext
}
```

**`signature_changed` detection** — range-overlap-check:

```python
def get_signature_range(symbol_node) -> tuple[int, int]:
    params_node = symbol_node.child_by_field_name("parameters")
    return_type_node = symbol_node.child_by_field_name("return_type")
    end_node = return_type_node or params_node or symbol_node
    return (symbol_node.start_point[0] + 1, end_node.end_point[0] + 1)

def is_signature_changed(signature_range, hunk_ranges) -> bool:
    sig_start, sig_end = signature_range
    return any(not (hunk_end < sig_start or hunk_start > sig_end)
               for hunk_start, hunk_end in hunk_ranges)
```

**Risk escalation rule**:

```python
if affected_symbol.signature_changed:
    reasons.append("signature modified — all call sites may need update")
    if RISK_ORDER[level] < RISK_ORDER["high"]:
        level = "high"
```

**`aggregate_risk` algorithm**:

```python
RISK_ORDER = {"low": 0, "medium": 1, "high": 2, "critical": 3}
aggregate_risk = max(
    (s.risk_assessment.level for s in affected_symbols),
    key=lambda level: RISK_ORDER[level],
    default="low"
)
if unindexed_files:
    aggregate_risk = "unknown"
```

`"unknown"` luôn override bất kỳ indexed-file risk nào — kể cả `"critical"`. Lý do: không thể trust risk estimate khi có unindexed files, bất kể indexed portion nói gì.

**`change_type: "renamed"` theo input mode**:

| Input mode        | Rename detection                                                              |
|-------------------|--------------------------------------------------------------------------------|
| `staged: true`    | `git diff --cached -M` — detect qua content similarity                       |
| `commits: string` | `git diff -M <commits>`                                                       |
| `diff: string`    | Không detect rename — appear như delete + add; `note` warns                  |

**Validation Rules**:

| Input case                                                       | Behavior                                                                                                           |
|------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------|
| Không có cái nào trong 3 option                                  | Error `INVALID_INPUT`: "Provide exactly one of diff, staged, or commits."                                          |
| Có ≥ 2 trong 3 option                                            | Error `INVALID_INPUT`: "Provide exactly one, not multiple."                                                        |
| `diff: ""` hoặc không parse được                                 | Error `INVALID_INPUT` với parse failure message                                                                    |
| `staged`/`commits` nhưng git không trong PATH                    | Error `FEATURE_UNAVAILABLE`, `suggestions: ["Provide diff text via diff parameter"]`                               |
| Diff hợp lệ nhưng chỉ touch file non-code                        | `affected_symbols: []`, `aggregate_risk: "low"`                                                                    |
| `staged: true` nhưng `git diff --cached` trả empty              | `affected_symbols: []`, `aggregate_risk: "low"`, note: "No staged changes found."                                  |
| `commits` với empty range (same commit)                          | `affected_symbols: []`, `aggregate_risk: "low"`, note: "Commit range resolves to no changes."                      |
| File trong diff chưa index hoặc hash outdated                    | `affected_symbols: []`, `aggregate_risk: "unknown"`, `unindexed_files: [...]`                                      |

`edge_confidence` không bao giờ bị discount trong `diff_impact` — `edge_confidence_note` được
expose, agent tự quyết định mức tin cậy. KHÔNG treat `"textual"` edges là safe, chỉ treat
là uncertain.

**Implementation**: parse diff → `(file, line_start, line_end)` mỗi hunk →
`SELECT * FROM symbols WHERE path=? AND line_start<=? AND line_end>=?` → blast_radius
logic shared với `edit_context`.

**Truncation sort** (khi `affected_symbols_total > config.diff_impact.max_affected_symbols`):
```python
affected_symbols.sort(
    key=lambda s: (
        RISK_ORDER[s.risk_assessment.level],  # higher risk first
        1 if s.signature_changed else 0,       # signature changes first
        s.blast_radius.direct_callers,          # higher blast radius first
    ),
    reverse=True
)
affected_symbols = affected_symbols[:config.diff_impact.max_affected_symbols]
```
Khi `affected_symbols_truncated: true`, symbols có risk thấp hơn bị cắt — không miss critical items.

**CODEOWNERS Integration** (chỉ tính khi `edges_ready: true` và `aggregate_risk >= medium`):

```python
# server.py — startup:
codeowners_patterns = codeowners.load_codeowners(project_root)  # cached once at startup
if codeowners_patterns:
    print(f"[codeindex] CODEOWNERS loaded: {len(codeowners_patterns)} patterns")

# diff_impact handler — nhận codeowners_patterns từ server state:
if aggregate_risk in ("medium", "high", "critical") and edges_ready:
    reviewers = []
    seen_files = set()
    for sym in affected_symbols[:10]:  # top 10 most-impacted
        if sym.path in seen_files:
            continue
        seen_files.add(sym.path)
        owners = (codeowners.find_owners(codeowners_patterns, sym.path)
                  if codeowners_patterns else [])
        source = "CODEOWNERS"
        if not owners:
            owners = codeowners.get_git_blame_owners(project_root, sym.path)
            source = "git_blame"
        if owners:
            reviewers.append({"path": sym.path, "owners": owners, "source": source})
    if reviewers:
        result["suggested_reviewers"] = reviewers
    # Không emit field khi empty
```

**`codeowners.py`** (pure Python, no new deps):

```python
import fnmatch
import subprocess
from pathlib import Path

CODEOWNERS_PATHS = [
    ".github/CODEOWNERS",
    "CODEOWNERS",
    "docs/CODEOWNERS",
    ".gitlab/CODEOWNERS",
]

def load_codeowners(project_root: Path) -> list[tuple[str, list[str]]]:
    """
    Parse CODEOWNERS file. Returns list of (pattern, [owners]).
    GitHub/GitLab: LAST matching rule wins.
    """
    for relative in CODEOWNERS_PATHS:
        path = project_root / relative
        if not path.exists():
            continue
        patterns = []
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            parts = line.split()
            if len(parts) >= 2:
                pattern, *owners = parts
                patterns.append((pattern, owners))
        return patterns
    return []

def find_owners(patterns: list[tuple[str, list[str]]], file_path: str) -> list[str]:
    """
    GitHub CODEOWNERS: last matching rule wins.
    '*' does not cross '/': "src/*.py" matches "src/x.py" but NOT "src/nested/x.py".
    Trailing '/' = explicit directory. No slash = basename match anywhere.
    Slash present (no trailing) = root-anchored, segment-by-segment glob.
    Extension-less files (Makefile, Dockerfile, Procfile) matched by basename, not
    misclassified as directory patterns.
    """
    matched: list[str] = []
    file_path_normalized = file_path.lstrip("/")

    def _match_path_pattern(pattern_parts: list[str], file_parts: list[str]) -> bool:
        """Segment-by-segment match: '*' cannot cross '/'."""
        if len(pattern_parts) > len(file_parts):
            return False
        return all(fnmatch.fnmatch(fp, pp) for pp, fp in zip(pattern_parts, file_parts))

    for pattern, owners in patterns:
        pattern_normalized = pattern.lstrip("/")

        if pattern_normalized.endswith("/"):
            # Explicit directory: match files under this directory.
            # If pattern contains glob chars (e.g. "*.egg-info/"), use
            # fnmatch on directory components; otherwise plain prefix match.
            dir_pattern = pattern_normalized[:-1]
            if any(c in dir_pattern for c in "*?["):
                parts = file_path_normalized.split("/")
                matched_dir = False
                for i in range(len(parts)):
                    prefix = "/".join(parts[:i+1])
                    if fnmatch.fnmatch(prefix, dir_pattern):
                        matched_dir = True
                        break
                if matched_dir:
                    matched = owners
            else:
                if file_path_normalized.startswith(pattern_normalized):
                    matched = owners
        elif "/" not in pattern_normalized:
            # No slash → matches any file in any directory by basename
            if fnmatch.fnmatch(Path(file_path_normalized).name, pattern_normalized):
                matched = owners
        else:
            # Path pattern with slash → root-anchored, segment-by-segment.
            # Implicit-directory case ("src/auth" matching "src/auth/anything") falls
            # out naturally since prefix-match still works.
            pattern_parts = pattern_normalized.split("/")
            file_parts = file_path_normalized.split("/")
            if _match_path_pattern(pattern_parts, file_parts):
                matched = owners

    return matched

def get_git_blame_owners(
    project_root: Path,
    file_path: str,
    top_n: int = 3,
    timeout: float = 5.0,
    since: str = "1 year ago"  # avoid stale suggestions from infrequently changed files
) -> list[str]:
    """Fallback: recent committers from git log."""
    try:
        result = subprocess.run(
            ["git", "log", f"--since={since}", "--follow",
             "-n", "10", "--format=%ae", "--", file_path],
            cwd=project_root, capture_output=True, text=True, timeout=timeout
        )
        authors = []
        seen = set()
        for email in result.stdout.splitlines():
            email = email.strip()
            if email and email not in seen:
                seen.add(email)
                authors.append(email)
                if len(authors) >= top_n:
                    break
        return authors
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return []
```

---

### Tool 13 — `indexing_status`

*USE WHEN: you need file-level index stats, embedding error details, or to trigger embedding recovery. NOT a replacement for repo_overview() at session start — repo_overview has indexing_phase already. retry_embeddings=true → triggers re-download of embedding model.*

```
Input: { retry_embeddings?: bool }   // default false

Output: {
  phase: "scanning" | "parsing" | "building_edges" | "ready",
  edges_ready: bool,
  embeddings_status: "disabled" | "downloading" | "embedding" | "ready" | "failed",
  embeddings_error?: { reason: "download_failed" | "model_corrupt" | "oom" | "embed_failed", message: string, retry_count: number },
  stats: {
    files_indexed: number, files_total: number,
    symbols_indexed: number | null,   // null khi phase == "scanning"
    edges_indexed: number | null      // null khi phase != "ready"
  },
  last_updated: string,
  suggested_next?: SuggestedNext
}
```

`retry_embeddings: true` → server clear error state, trigger re-download → response ngay
`embeddings_status: "downloading"`.

---

### Tool 14 — `locate`

*Compound: search + file_overview + symbol_info in 1 call. Replaces the most common 3-call chain (66% reduction). USE INSTEAD OF calling search then file_overview then symbol_info separately. NOT FOR: reading source (use source after locate), pre-edit (use edit_context).*

```
Input: {
  query: string,
  kind?: "symbol" | "text" | "file" | "semantic" | "hybrid",  // default "symbol"
  limit?: number,     // default 5, applies to search results
  depth?: "search_only" | "with_file" | "with_symbol"         // default "with_symbol"
}

Output: {
  // === Search results (same schema as search tool) ===
  results: {
    name, kind, path,
    line_start?: number,
    line_end?: number,
    preview,
    match_type: "exact" | "fts" | "semantic" | "hybrid"
  }[],
  truncated: bool,
  degraded: bool,
  suggestions?: string[],
  depth_adjusted?: "with_file" | "search_only",  // present iff auto-downgrade occurred; extensible for future depth levels

  // === Top result enrichment (absent if results empty or depth="search_only") ===
  top_result?: {
    file: {
      language: string,
      inferred_role: "service" | "model" | "router" | "utility" | "test" | "config" | null,
      symbols: {
        name, kind, signature,
        line_start, line_end,
        caller_count: number | null,
        is_hub: bool | null,
        coreness: number | null
      }[],
      total_symbols: number,
      file_truncated: bool
    },
    symbol?: SymbolInfo | AmbiguousResult  // omitted if depth="with_file"
  },

  edges_ready: bool,
  note?: string,
  embeddings_status?: "disabled" | "downloading" | "embedding" | "ready" | "failed",
  suggested_next?: SuggestedNext
}
```

**Behavior**:

- `depth="search_only"` → identical to `search` tool, no enrichment.
- `depth="with_file"` → search + file_overview of top result's file.
- `depth="with_symbol"` (default) → search + file_overview + symbol_info of top result.
- `kind ∈ {"text", "file"}` + `depth="with_symbol"` → **auto-downgrade to `depth="with_file"`**.
  Reason: text/file results carry no symbol name — `results[0].name` is a text snippet or
  filename, not a callable symbol — so `symbol_info()` cannot be called meaningfully.
  Response sets `depth_adjusted: "with_file"` to signal enrichment was capped. `kind ∈ {"symbol",
  "hybrid", "semantic"}` is unaffected — full enrichment proceeds as requested.

**Top result selection**: `results[0]` sau sort. Nếu top result ambiguous →
`top_result.symbol = AmbiguousResult`. Agent chọn candidate, gọi `symbol_info(name, path=candidate.path)` riêng.

**Performance**: search → file_overview → symbol_info trong same process, không có MCP
round-trip giữa các bước. Latency overhead ~1-2ms vs 3 separate calls (~15-30ms × 3).

**Preset membership**: `orient`, `trace`, `edit`, `compound`, `full`.

---

### Tool 15 — `hotspots`

*Proactive churn × complexity analysis. New capability — no equivalent in previous versions. USE WHEN: starting exploration of a codebase or after orientation to identify high-risk files before diving in.*

```
Input: {
  top_n?: number,           // default 10
  since?: string,           // default "6 months ago" | ISO date | "YYYY-MM-DD"
  min_churn?: number,       // minimum commit count to include, default 2
  include_symbols?: bool    // default false — include top hub symbols per file
}

Output: {
  hotspots: {
    path: string,
    language: string,
    churn: {
      commit_count: number,
      unique_authors: number,
      last_changed: string | null  // ISO 8601; null when git unavailable
    },
    complexity: {
      symbol_count: number,
      hub_count: number,             // count of is_hub==true symbols in file
      connected_coreness_count: number,   // count of symbols with coreness > 0
      avg_caller_count: number
    },
    hotspot_score: number,   // 0.0–1.0; (churn × complexity) khi git available, complexity-only khi git không available (hotspot_method="index_only")
    risk_level: "low" | "medium" | "high" | "critical",
    top_symbols?: {          // present iff include_symbols=true
      name, kind, is_hub, coreness, caller_count: number | null
    }[]
  }[],
  git_available: bool,
  since: string,
  total_files_analyzed: number,  // candidate count BEFORE top_n slice
  hotspot_method: "git+index" | "index_only",
  note: string,
  suggested_next?: SuggestedNext
}
```

**Algorithm** (Adam Thornhill's method: hotspot_score = normalize(churn) × normalize(complexity)):

```python
def compute_hotspots(
    project_root: Path,
    conn: sqlite3.Connection,
    top_n: int = 10,
    since: str = "6 months ago",
    min_churn: int = 2,
    include_symbols: bool = False,
) -> tuple[list[HotspotResult], int]:
    """
    Returns (results, total_files_analyzed).
    total_files_analyzed is the candidate count BEFORE the top_n slice.
    """

    # --- Step 1: Churn from git (optional) ---
    churn_map: dict[str, dict] = {}
    git_available = False
    try:
        import subprocess
        result = subprocess.run(
            ["git", "log", f"--since={since}",
             "--name-only", "--format=|||%ae|||%aI"],  # %aI = strict ISO 8601
            cwd=project_root, capture_output=True, text=True, timeout=30
        )
        if result.returncode == 0:
            git_available = True
            current_author, current_date = None, None
            for line in result.stdout.splitlines():
                if line.startswith("|||"):
                    parts = line.split("|||")
                    current_author = parts[1].strip() if len(parts) > 1 else None
                    current_date = parts[2].strip() if len(parts) > 2 else None
                elif line.strip():
                    abs_path = str(project_root / line.strip())
                    if abs_path not in churn_map:
                        churn_map[abs_path] = {
                            "commit_count": 0,
                            "authors": set(),
                            "last_changed": current_date or None
                        }
                    churn_map[abs_path]["commit_count"] += 1
                    if current_author:
                        churn_map[abs_path]["authors"].add(current_author)
    except (subprocess.TimeoutExpired, FileNotFoundError):
        pass

    # --- Step 2: Complexity from index (always available) ---
    rows = conn.execute("""
        SELECT
            path,
            COUNT(*) as symbol_count,
            SUM(CASE WHEN is_hub = 1 THEN 1 ELSE 0 END) as hub_count,
            AVG(COALESCE(caller_count, 0)) as avg_caller_count,
            SUM(CASE WHEN coreness > 0 THEN 1 ELSE 0 END) as connected_coreness_count,
            MAX(language) as language
        FROM symbols
        WHERE path IS NOT NULL
        GROUP BY path
    """).fetchall()
    complexity_map: dict[str, dict] = {}
    for path, sym_count, hub_count, avg_callers, high_core, language in rows:
        complexity_map[path] = {
            "symbol_count": sym_count,
            "hub_count": hub_count or 0,
            "avg_caller_count": round(avg_callers or 0, 2),
            "connected_coreness_count": high_core or 0,
            "language": language,
        }

    # Complexity score per file: weighted combination
    def complexity_score(c: dict) -> float:
        return (
            c["symbol_count"] * 0.3 +
            c["hub_count"] * 3.0 +          # hubs heavily weighted
            c["connected_coreness_count"] * 1.5 +
            c["avg_caller_count"] * 0.5
        )

    # --- Step 3: Merge + normalize ---
    if git_available:
        candidates = {
            path: data for path, data in churn_map.items()
            if data["commit_count"] >= min_churn and path in complexity_map
        }
    else:
        # No git: rank purely by complexity. min_churn not applicable.
        # commit_count=0 is semantically correct: "no churn data available".
        candidates = {path: {"commit_count": 0, "authors": set(), "last_changed": None}
                      for path in complexity_map}

    if not candidates:
        return [], 0

    total_files_analyzed = len(candidates)  # capture BEFORE top_n slice

    churn_scores = {p: d["commit_count"] for p, d in candidates.items()}
    compl_scores = {p: complexity_score(complexity_map[p]) for p in candidates}

    max_churn = max(churn_scores.values()) or 1
    max_compl = max(compl_scores.values()) or 1

    results = []
    for path in candidates:
        norm_compl = compl_scores[path] / max_compl
        if git_available:
            norm_churn = churn_scores[path] / max_churn
            score = norm_churn * norm_compl
        else:
            score = norm_compl  # no git: pure complexity score
        level = (
            "critical" if score >= 0.75 else
            "high"     if score >= 0.50 else
            "medium"   if score >= 0.25 else
            "low"
        )
        cd = candidates[path]
        cm = complexity_map[path]
        results.append(HotspotResult(
            path=path,
            language=cm["language"],
            churn_commit_count=cd["commit_count"],
            churn_unique_authors=len(cd.get("authors", set())),
            churn_last_changed=cd.get("last_changed"),
            symbol_count=cm["symbol_count"],
            hub_count=cm["hub_count"],
            connected_coreness_count=cm["connected_coreness_count"],
            avg_caller_count=cm["avg_caller_count"],
            hotspot_score=round(score, 4),
            risk_level=level
        ))

    results.sort(key=lambda r: r.hotspot_score, reverse=True)
    top_results = results[:top_n]

    if include_symbols:
        for r in top_results:
            r.top_symbols = conn.execute("""
                SELECT name, kind, is_hub, coreness, caller_count
                FROM symbols WHERE path = ?
                ORDER BY COALESCE(caller_count, 0) DESC, coreness DESC
                LIMIT 5
            """, (r.path,)).fetchall()

    return top_results, total_files_analyzed
```

**`hotspot_method`**: `"git+index"` khi git available và có churn data. `"index_only"` khi
git không có hoặc timeout — fallback pure complexity ranking, vẫn useful.

**`note` field** (luôn có trong output):
- Git available: `"Analyzed commits since {since}."`
- Git available nhưng `hotspots: []` vì filter: `"No files exceeded min_churn={min_churn} commits since {since}. Try reducing min_churn."`
- Git không available: `"Git unavailable: ranking by complexity only. min_churn parameter not applied."`

**Risk level thresholds** (tunable qua config):

```json
"hotspots": {
  "default_top_n": 10,
  "default_since": "6 months ago",
  "default_min_churn": 2,
  "risk_critical_threshold": 0.75,
  "risk_high_threshold": 0.50,
  "risk_medium_threshold": 0.25
}
```

**Preset membership**: `orient`, `compound`, `full`.

---

### Tool 16 — `understand`

*Compound: locate + source + callers summary in 1 call. USE INSTEAD OF calling locate then source then callers separately. USE WHEN: you want to find, read, AND understand usage context of a symbol. NOT FOR: pre-edit (use edit_context — more complete blast radius). NOT FOR: browsing results list (use locate with depth="search_only").*

```
Input: {
  query: string,
  kind?: "symbol" | "hybrid",  // default "symbol"
  // kind restricted to "symbol" | "hybrid" only — understand requires a symbol name
  // for source + callers resolution. kind="text"/"file"/"semantic" → Error INVALID_INPUT:
  // "understand requires kind='symbol' or kind='hybrid'. Use locate() for text/file search."
}

Output: {
  // === Resolved case (all fields below absent when ambiguous is present) ===
  name?: string,
  path?: string,
  kind?: string,
  language?: string,
  signature?: string,
  docstring?: string,
  is_hub?: bool,
  coreness?: number | null,
  health?: Health,

  // === From source (resolved case only) ===
  source?: string,
  line_start?: number,
  line_end?: number,

  // === Callers summary (resolved case only, top 5 by confidence, resolved first) ===
  callers_summary?: {
    name: string,
    path: string,
    edge_confidence: "resolved" | "inferred" | "textual",
    line: number
  }[],
  total_callers?: number,
  edges_ready?: bool,

  // === Ambiguous case (discriminant: present ↔ resolved fields absent) ===
  // Agent: pick candidate from ambiguous.candidates[], gọi symbol_info(name=candidate.name, path=candidate.path).
  // understand không nhận path param nên không thể tự disambiguate; symbol_info có thể.
  ambiguous?: AmbiguousResult,

  suggested_next?: SuggestedNext
}
```

**Implementation**: ~60 lines composition — `search_logic` → `symbol_info_logic` →
`source_logic` → `callers_logic(limit=5)`. Không có new logic, chỉ là orchestration. Không
có MCP round-trips giữa steps (same process).

**Preset membership**: `compound`, `full`.

---

## Layer 4: Tool Presets

**Concept**: CLI flag lọc tools nào được register với MCP server. Không thay đổi bất cứ
logic bên trong nào — chỉ là selective tool registration.

**Motivation**: 16 tools × ~70 tokens/description = ~1120 tokens overhead cố định. Với
preset, agent trong từng workflow stage chỉ thấy tools relevant.

### CLI

```bash
python -m codeindex serve --project /path/to/repo                    # default, all 16 tools
python -m codeindex serve --project /path/to/repo --preset=orient    # discovery phase
python -m codeindex serve --project /path/to/repo --preset=trace     # call tracing phase
python -m codeindex serve --project /path/to/repo --preset=edit      # editing phase
python -m codeindex serve --project /path/to/repo --preset=compound  # compound tools only
python -m codeindex serve --project /path/to/repo --preset=full      # explicit all 16
```

### Preset Definitions

```
--preset=orient (5 tools, ~350 tokens):
  repo_overview    # orientation
  locate           # compound: find + inspect
  dependencies     # file-level architecture
  hotspots         # proactive risk scan
  indexing_status  # check index progress

--preset=trace (6 tools, ~420 tokens):
  repo_overview    # orientation
  locate           # find symbols
  callers          # upward tracing
  callees          # downward tracing
  path             # call chain between two points
  session_context  # reorientation

--preset=edit (6 tools, ~420 tokens):
  repo_overview    # orientation
  locate           # find target symbol
  source           # read implementation
  edit_context     # mandatory pre-edit
  diff_impact      # mandatory post-edit
  indexing_status  # check freshness

--preset=compound (9 tools, ~630 tokens):
  repo_overview
  locate           # compound search
  hotspots         # proactive analysis
  source
  understand       # compound: locate + source + callers
  edit_context
  diff_impact
  session_context
  indexing_status  # embedding recovery + index progress

--preset=full (16 tools, ~1120 tokens):
  All 16 tools — default, no filtering
```

### Implementation

```python
PRESET_TOOL_SETS: dict[str, set[str] | None] = {
    "orient":   {"repo_overview", "locate", "dependencies", "hotspots", "indexing_status"},
    "trace":    {"repo_overview", "locate", "callers", "callees", "path", "session_context"},
    "edit":     {"repo_overview", "locate", "source", "edit_context", "diff_impact", "indexing_status"},
    "compound": {"repo_overview", "locate", "hotspots", "source", "understand",
                 "edit_context", "diff_impact", "session_context", "indexing_status"},
    "full":     None,  # None = all tools
}

def register_tools(mcp_server, preset: str = "full") -> None:
    # Validate before lookup — None return is ambiguous between "full" and unrecognized preset.
    if preset not in PRESET_TOOL_SETS:
        valid = list(PRESET_TOOL_SETS.keys())
        raise ValueError(
            f"[codeindex] Unknown preset: {preset!r}. Valid options: {valid}"
        )
    allowed = PRESET_TOOL_SETS[preset]
    for tool_name, tool_fn in ALL_TOOLS.items():
        if allowed is None or tool_name in allowed:
            mcp_server.register_tool(tool_name, tool_fn)
```

**Config.json support** (alternative to CLI):

```json
{
  "preset": "orient"   // can set in config.json as alternative to CLI flag
}
```

CLI flag overrides config.json nếu cả hai có.

**Dynamic preset limitation**: `--preset` set một lần khi boot server — không thể thay đổi
giữa session. Nếu agent bắt đầu với `--preset=trace` nhưng cần edit tools ở Stage 7,
agent phải restart server với `--preset=edit` hoặc `--preset=full`. Khuyến nghị:
dùng `--preset=full` (default) nếu workflow span nhiều stages; dùng preset cụ thể chỉ
khi workflow đã xác định rõ scope (chỉ orient, chỉ trace, chỉ edit).

---

## Layer 3: Behavioral Guidance

### AGENTS.md — Navigational Workflow (v2.7.2)

```markdown
## Code Intelligence — Navigational Workflow

### STAGE 1: ORIENT — Nhìn tổng bản đồ
Luôn bắt đầu đây. Không bao giờ skip.
→ repo_overview() — project map, entry points, module list
→ Check indexing_phase: nếu != "ready", graph tools chưa reliable — dùng search +
  file_overview + source trong lúc chờ
→ Check embeddings_status:
  → "downloading"/"embedding" → search(kind="semantic"/"hybrid") chưa full
  → "failed" → indexing_status(retry_embeddings=true) để trigger recovery
  → "ready" → tất cả search kinds available
→ health_summary.undocumented_hubs tăng sau upgrade = expected (bridge hubs lộ ra)
→ Chỉ gọi indexing_status() khi cần file-level counts, last_updated, embeddings_error

### STAGE 1.5: SCAN (Optional) — Rủi ro proactive sau orientation
Gọi SAU Stage 1 (repo_overview), TRƯỚC Stage 2, nếu cần hiểu rủi ro toàn codebase.
ĐIỀU KIỆN: chỉ gọi khi Stage 1 confirm indexing_phase == "ready".
Nếu indexing_phase != "ready" → gọi indexing_status() trước, không gọi hotspots().
→ hotspots() — files có risk_level="critical"/"high" cần cẩn thận đặc biệt
  → hotspot_method: "index_only" = git không available, ranking chỉ theo complexity
  → include_symbols=true → xem hub symbols trong hotspot file
→ Nếu không cần overview rủi ro, bỏ qua Stage 1.5 và bắt đầu Stage 2 ngay sau Stage 1

### STAGE 2: LOCATE — Khoanh vùng
→ Biết tên symbol hoặc concept → locate(query, kind="symbol")
  → Replaces: search → file_overview → symbol_info (3 calls → 1 call)
  → depth="with_symbol" (default) → full: search + file structure + symbol metadata
  → depth="with_file" → search + file structure only (khi chỉ cần structure)
  → depth="search_only" → pure search list (khi cần browse nhiều results)
  → kind="text"/"file" → depth tự động downgrade về "with_file" nếu request "with_symbol",
    check field depth_adjusted trong response
  → top_result.symbol.is_hub: true → đặc biệt cẩn thận, suggested_next sẽ gợi ý edit_context
→ Vẫn có thể dùng search() riêng lẻ khi muốn fine-grained control
→ Biết file path rồi → file_overview(path) trực tiếp (không cần locate)
→ Muốn biết file architecture → dependencies(path)

### STAGE 3: INSPECT — Đọc metadata, chưa đọc source
→ symbol_info(name)
→ ambiguous: true → chọn candidate, gọi lại với path
→ is_hub: null → edges chưa ready, không kết luận được
→ is_hub: true → đặc biệt cẩn thận trước khi touch
→ coreness field:
  → coreness: null  → edges_ready: false
  → coreness: 0     → isolated node, không thuộc k-core nào
  → coreness > 0    → số càng lớn, node càng deeply embedded trong dense subgraph
  → is_hub: true với coreness cao nhưng caller_count không cao → bridge hub
→ edges_ready: false → caller_count, is_hub, coreness đều null
→ health.dead_code_confidence + dead_code_source:
  → dead_code_source: "static" = chỉ static analysis
  → dead_code_source: "static+coverage" = confirmed bằng coverage data
  → "high" + "static+coverage" = strong signal: không có static callers VÀ không có
    coverage hits → thực sự dead, safe to remove
  → "high" + "static" = static-only, có thể false positive qua dynamic dispatch
  → "medium" + "static+coverage" = scope_clear nhưng không có coverage hits và không có
    static callers → candidate for removal, nhưng verify thêm. Có thể là API endpoint,
    config symbol, hay module-level init không được gọi trực tiếp nhưng dùng externally.
  → "low" + "static+coverage" = runtime-covered nhưng không có static callers →
    dynamic dispatch, reflection, hay decorator-registered — KHÔNG remove
→ health.caller_count_by_confidence.textual cao → verify callers thủ công
→ health.test_files rỗng → chưa có test coverage
→ Đã chắc cần đọc? → bỏ qua Stage 3, dùng source(symbol, include_metadata: true)

### STAGE 4: READ — Vào trong
→ source(symbol) — sau khi đã symbol_info()
→ source(symbol, include_metadata: true) — bỏ qua two-step nếu đã biết cần đọc
  [kiểm tra metadata.edges_ready để biết caller_count/is_hub/coreness có reliable không]
→ source("path:line_start-line_end") — khi biết exact location
→ understand(query) — compound: find + read + callers summary (3 tools → 1 call)
  → Trade-off: understand gộp locate+source+callers, tiện cho exploration nhưng
    KHÔNG dùng cho pre-edit workflow — edit_context có blast_radius + risk_assessment
    đầy đủ hơn. Dùng understand khi đọc hiểu, dùng edit_context khi sắp sửa code.
  → Khi preset restrict (e.g. --preset=trace không có understand): dùng locate → source → callers riêng lẻ
→ ambiguous: true → chọn candidate (candidates có đủ line_start/line_end), gọi lại với path:line_start-line_end
→ KHÔNG dùng native Read tool trên full file

### STAGE 5: TRACE — Follow connections
Pre-condition: edges_ready: true
→ Ai gọi symbol này → callers(symbol)
  → edge_confidence: "textual" → treat as hint, verify thủ công
  → edge_confidence: "resolved"/"inferred" → reliable
  → transitive_count: null + transitive_capped: true → BFS timeout, count uncertain
→ Symbol này gọi gì → callees(symbol)
→ Call flow A→B → path(from_symbol="A", to_symbol="B")
  → Bidirectional BFS — cycles không gây loop, terminate cleanly
  → terminated_by: null + exists: false = không có path (chắc chắn)
  → terminated_by: "max_hops"/"timeout" + exists: null = chưa kết luận
  → path là directed: A→B ≠ B→A
  → terminated_by: "max_hops" → suggested_next gợi ý retry với max_hops lớn hơn, chiều ngược lại
→ Phân biệt: dependencies(path) file-level; callers/callees(symbol) symbol-level

### STAGE 6: REORIENT — Khi mất phương hướng hoặc bắt đầu sub-task mới
→ session_context() — đã explore đâu, frontier là gì
  → explored.symbols[] là MRU-ordered (LRU-eviction) — 50 most-recent
  → frontier_degraded: true → edges chưa ready, frontier chỉ từ import graph
  → session_started_at: lưu giá trị lần gọi ĐẦU TIÊN làm T₀. Khác T₀ = server restart
    Giới hạn: T₀ có thể mất nếu context-compaction — treat như server restart nếu xảy ra.
→ Gọi sau 10+ tool calls chưa converge, hoặc khi chuyển sub-task

### STAGE 7: EDIT — Khi đã hiểu đủ
Pre-condition: edges_ready: true
→ edit_context(symbol) trước mọi thay đổi — BẮT BUỘC
→ index_freshness.stale_callers: true → callers list có thể outdated
→ Xem risk_assessment.level, reasons[], edge_confidence của từng caller và callee
→ critical/high → verify blast_radius trước khi đổi signature
→ callers với edge_confidence: "textual" → confirm manually trước khi assume safe
→ callees sorted by confidence desc, then line asc
→ Risk planning — bridge-hub vs degree-hub:
  → is_hub: true + caller_count không nằm top 5% = bridge hub. Risk ảnh hưởng
    cross-module integration — cẩn thận dù caller_count thấp.
  → is_hub: true + caller_count cao = degree hub.
  → coreness >> p75 + caller_count thấp = strong bridge-hub signal.
→ diff_impact output v2.7:
  → suggested_reviewers[] — owners từ CODEOWNERS + git blame
    → source: "CODEOWNERS" = explicit ownership
    → source: "git_blame" = CODEOWNERS không cover file này, dùng recent committers
    → Absent nếu aggregate_risk="low"
    → Absent nếu không có CODEOWNERS VÀ git không available
    → source: "git_blame" chỉ xuất hiện nếu git available VÀ CODEOWNERS không cover file này
    → CODEOWNERS-sourced reviewers KHÔNG bị block bởi git unavailable (2 nguồn độc lập nhau)

### STAGE 8: VERIFY CHANGESET — Sau khi đổi code, trước khi commit/push
Pre-condition: có uncommitted changes hoặc staged diff
→ diff_impact(diff=<paste>) — primary, không cần git
→ diff_impact(staged=true) hoặc diff_impact(commits="HEAD~N..HEAD") — khi git available
→ aggregate_risk: "unknown" → unindexed_files non-empty, KHÔNG treat là safe
→ aggregate_risk: "critical"/"high" → review high_risk_callers[], verify edge_confidence
→ affected_symbols[].signature_changed: true →
  → risk tự động escalate lên minimum "high"
  → tất cả callers cần update call sites
→ affected_symbols_truncated: true → higher-risk symbols được include trước, truncation
  ở cuối danh sách — không miss critical items
→ edge_confidence_note xuất hiện → blast_radius có thể sai cả hai chiều (over/under-count)
→ suggested_reviewers[] → routing cho code review
→ Complement của Stage 7: Stage 7 pre-edit (reactive), Stage 8 post-edit (proactive)

### PRESET GUIDANCE — Chọn tools phù hợp workflow
Nếu server được start với --preset:
  --preset=orient: dùng locate + hotspots + dependencies cho discovery
  --preset=trace:  dùng locate + callers + callees + path cho tracing
  --preset=edit:   dùng locate + edit_context + diff_impact cho editing
  --preset=full:   tất cả tools available (default)

suggested_next hint trong mỗi response chỉ gợi ý tool TRONG preset hiện tại.
Nếu gợi ý tool không available trong preset, chọn equivalent trong set hiện có.
```

---

## Shared Field Types

Contract cứng — thêm field vào shared type tự động propagate sang mọi nơi dùng type đó.

### `Health`

*(dùng ở `symbol_info.health`, `source.metadata.health`, `understand.health`)*

```typescript
Health = {
  has_docstring: boolean,
  test_files: string[],
  dead_code_confidence: "none" | "low" | "medium" | "high",
  dead_code_source: "static" | "static+coverage",
  caller_count_by_confidence: { resolved: number, inferred: number, textual: number } | null
}
```

`dead_code_confidence`: `"none"` có callers HOẶC entry point; `"low"` không callers nhưng
runtime-covered (dynamic dispatch/reflection), HOẶC scope không xác định được (scope_clear=false);
`"medium"` không callers, không runtime-covered, scope xác định được (scope_clear=true) nhưng
không private; `"high"` không callers, private scope, không entry point.
`caller_count_by_confidence: null` khi `edges_ready: false`.

`dead_code_source: "static"` khi chỉ có static analysis. `"static+coverage"` khi coverage
data có và được dùng. `dead_code_source` luôn present khi `Health` object tồn tại — mandatory
within the type. `Health` itself is optional in `understand` (absent khi ambiguous case), nhưng
khi present, tất cả fields bao gồm `dead_code_source` đều phải có.

### `SuggestedNext`

*(dùng ở tất cả 16 tool responses)*

```typescript
SuggestedNext = {
  tool: string,
  reason: string,
  args?: Record<string, string | number | boolean>
}
```

Optional field trong mọi tool output — absent khi không có gợi ý rõ ràng hoặc tool bị
filter bởi preset.

### `AmbiguousResult`

*(dùng ở `locate.top_result.symbol`, `understand.ambiguous`; shape identical used inline by `symbol_info`, `source`, `callers`, `callees`, `path`, `edit_context`)*

```typescript
AmbiguousResult = {
  ambiguous: true,      // discriminant — normal (non-ambiguous) responses omit this field entirely
  candidates: {
    // --- Base fields (ALL 6 Ambiguity Contract tools) ---
    name: string,
    path: string,
    kind: string,
    line_start: number,
    line_end: number,
    // --- Extended fields (symbol_info, source, locate, understand only) ---
    class_context?: string | null,
    caller_count?: number | null,
    language?: string,
    signature?: string,
  }[]
}
```

Tools chỉ resolve symbol (`callers`, `callees`, `path`, `edit_context`) trả candidates
với 5 base fields. Tools có metadata sẵn (`symbol_info`, `source`, `locate`, `understand`)
trả full shape bao gồm cả extended fields. Agent dùng `path:line_start-line_end` format
để disambiguate bất kể tool nào.

### `ReviewerSuggestion`

*(dùng ở `diff_impact.suggested_reviewers`)*

```typescript
ReviewerSuggestion = {
  path: string,
  owners: string[],
  source: "CODEOWNERS" | "git_blame"
}
```

### `EdgeConfidence`

*(dùng ở `callers.direct[]`, `callees.direct[]`, `path.routes[].steps[]`,
`edit_context.callers[]`, `edit_context.callees[]`, `diff_impact.high_risk_callers[]`)*

```
EdgeConfidence = "resolved" | "inferred" | "textual"
```

- `"resolved"`: callee defined cùng file, explicit import, hoặc alias confirmed. Reliable.
- `"inferred"`: callee type inferred qua import + type hints (hoặc alias type). Mostly reliable.
- `"textual"`: name-only — dễ false positive cả hai chiều.

### Ambiguity Contract

*(dùng ở `symbol_info`, `source`, `callers`, `callees`, `path`, `edit_context` —
KHÔNG dùng ở `dependencies` vì input là `path` exact)*

**NOT_FOUND case**: khi input không match bất kỳ symbol nào trong index, trả error
`NOT_FOUND` (xem §Unified Error Schema). Đây là case khác biệt với AMBIGUOUS: 0 matches
vs nhiều matches. Tất cả 6 tools trong contract xử lý cả hai case.

Khi input match nhiều candidate và không có `path`: trả `ambiguous: true`,
`candidates: [...]`, mọi field "kết quả chính" rỗng/null. Agent chọn candidate, gọi lại
với `path` cụ thể. 6 tools trong contract đều có `path?`/`from_path?`/`to_path?`.

**Candidate shape**: xem §AmbiguousResult type definition. Base 5 fields (`name`, `path`,
`kind`, `line_start`, `line_end`) có ở tất cả 6 tools. Extended fields (`class_context`,
`caller_count`, `language`, `signature`) chỉ có ở `symbol_info`, `source`, `locate`,
`understand`. Agent dùng `path:line_start-line_end` format để disambiguate — hoạt động
với cả base lẫn extended shape.

### Edge-Readiness Note Convention

*(dùng ở `callers`, `callees`, `dependencies`, `path`, `edit_context`, `diff_impact`,
`symbol_info`, `file_overview`, `locate`; `understand` có `edges_ready?: bool` optional nhưng không
có `note` convention — field này passthrough từ callers_logic)*

```
edges_ready == false:
  note = "[disclaimer: index đang build edges...]" + " " + "[static-analysis-limitation]"
edges_ready == true:
  note = "[static-analysis-limitation của tool đó]"
```

**Note field type**: `edit_context.note`, `callers.note`, `callees.note`,
`dependencies.note`, `path.note` — `string`, mandatory. `symbol_info.note`,
`file_overview.note`, `diff_impact.note`, `locate.note` — `string?`, optional.

**`diff_impact` note khi `edges_ready == false`**:
"Call graph đang build — affected_symbols và high_risk_callers có thể chưa đầy đủ. Kết
quả diff_impact lúc này chỉ nên dùng tham khảo, không nên dùng để quyết định an toàn
push code."

### Line-Range Convention

Symbol *definition* → `line_start`/`line_end` (range). Call/reference *site* → `line`
(singular).

| Field                                          | Convention                                       |
|------------------------------------------------|---------------------------------------------------|
| `symbol_info.line_start/line_end`              | definition range                                   |
| `file_overview.symbols[].line_start/end`       | definition range                                   |
| `search.results[].line_start?/line_end?`       | definition/chunk range (omit khi `kind="file"`)   |
| `source.line_start/line_end`                   | definition/range                                   |
| `edit_context.target.line_start/line_end`      | definition range                                   |
| `diff_impact.affected_symbols[].line_*`        | definition range                                   |
| `path.routes[].steps[].line_start/end`         | definition range                                   |
| `callers.direct[].line`                        | call site (singular)                              |
| `callees.direct[].line`                        | call site (singular)                              |
| `edit_context.callers[].line`                  | call site (singular)                              |
| `edit_context.callees[].line`                  | call site (singular)                              |
| `diff_impact.high_risk_callers[].line`         | call site (singular)                              |
| `session_context.already_fetched[].line_*`     | actual fetched range                              |

---

## Unified Error Schema

```json
{
  "error": {
    "code": "NOT_FOUND" | "AMBIGUOUS" | "INDEX_PARTIAL" | "PARSE_FAILED"
          | "TIMEOUT" | "DB_LOCKED" | "INVALID_INPUT" | "FEATURE_UNAVAILABLE"
          | "EMBEDDING_FAILED",
    "message": string,
    "recoverable": bool,
    "suggestions"?: string[]
  }
}
```

| Code                  | Recoverable | Meaning                                              |
|-----------------------|-------------|--------------------------------------------------------|
| `NOT_FOUND`           | false       | Symbol/file không tồn tại trong index                |
| `AMBIGUOUS`           | true        | Ambiguity không thể resolve in-band                  |
| `INDEX_PARTIAL`       | true        | Index chưa complete; retry sau                       |
| `PARSE_FAILED`        | false       | File có syntax error                                 |
| `TIMEOUT`             | true        | BFS/query timeout                                    |
| `DB_LOCKED`           | true        | SQLite write contention (rare với WAL)               |
| `INVALID_INPUT`       | true        | Bad params                                           |
| `FEATURE_UNAVAILABLE` | true        | Feature cần enable trong config                      |
| `EMBEDDING_FAILED`    | true        | Download/embedding lỗi                               |

**Error-to-tool mapping**:

| Error Code            | Emitted by                                                              |
|-----------------------|-------------------------------------------------------------------------|
| `NOT_FOUND`           | `symbol_info`, `source`, `callers`, `callees`, `path`, `edit_context`   |
| `AMBIGUOUS`           | (in-band via `ambiguous: true` — error form chỉ khi không có in-band path) |
| `INDEX_PARTIAL`       | `callers`, `callees`, `path`, `edit_context`, `diff_impact`              |
| `PARSE_FAILED`        | `source`, `file_overview`                                                |
| `TIMEOUT`             | `path`, `callers` (transitive), `callees` (transitive)                   |
| `DB_LOCKED`           | bất kỳ tool nào (rare với WAL)                                           |
| `INVALID_INPUT`       | `diff_impact`, `search`, `path`                                          |
| `FEATURE_UNAVAILABLE` | `search` (semantic/hybrid khi disabled), `diff_impact` (git unavailable) |
| `EMBEDDING_FAILED`    | `search` (semantic/hybrid khi embeddings failed)                         |

`AMBIGUOUS` error chỉ dùng khi không có natural in-band path.

---

## Implementation Decisions

**Tech stack**:

```
tree-sitter-languages    # Universal grammar, 100+ grammars, không cần compile
fastmcp                  # MCP server framework
watchfiles               # Async file watcher, cross-platform
sqlite3                  # stdlib, FTS5 + WAL built-in

codeindex/resolver/       # Self-contained, pure Python, zero new deps
coverage_reader.py        # Pure Python, stdlib only (sqlite3 + xml.etree)
codeowners.py             # Pure Python, stdlib only (fnmatch + subprocess)

# Optional — chỉ install khi semantic_search.enabled: true:
fastembed                # ONNX-based embeddings, ~60-100MB, không cần PyTorch
sqlite-vec               # KNN vector search SQLite extension
```

**Project structure**:

```
.codeindex/
  index.db
  index.db-wal          # auto-added to .gitignore by server
  index.db-shm
  config.json

codeindex/
  resolver/
    __init__.py         # ConservativeResolver: alias_map + F7 guard + Rust/Go handlers
    lang_constants.py   # Per-language node type constants, ASSIGNMENT_NODES

  coverage_reader.py    # CoverageReader, CoverageData — pure Python, zero deps
  codeowners.py         # CODEOWNERS parser, segment-by-segment glob, git_blame fallback
  hotspot.py            # compute_hotspots(), HotspotResult

  tools/
    locate.py           # compound: search + file_overview + symbol_info
    hotspots.py         # MCP tool wrapper for hotspot.py
    understand.py       # compound: locate + source + callers summary

  server.py             # preset filtering, coverage_reader init, codeowners init
  db_init.py            # ensure_gitignore(), WAL setup, migrate_to_v2_6()

AGENTS.md
```

**Stack Graphs note**: GitHub Stack Graphs (formally-grounded, incremental scope resolution)
— plan cho v2.8+ sau khi ConservativeResolver Python/TS/Java/Rust/Go production-verified ở
v2.7. Self-contained resolver ở v2.7 không preclude Stack Graphs migration — two complementary
approaches (current: alias-tracking + confidence tiers; future: formally-grounded binding).

**`config.json`**:

```json
{
  "preset": "full",
  "languages": ["typescript", "python", "rust", "go"],
  "ignore": ["node_modules", ".git", "dist", "build"],
  "entry_points": [],
  "hub_threshold": {
    "top_pct": 5,
    "min_callers": 5,
    "min_callers_bridge": 2,
    "coreness_pct": 75
  },
  "call_graph": {
    "resolver": "conservative",
    "confidence_tracking": true
  },
  "semantic_search": {
    "enabled": false,
    "model": "BAAI/bge-base-en-v1.5",
    "dimensions": 768,
    "index_on_startup": true
  },
  "search": {
    "text_chunk_context_lines": 10,
    "text_max_chunk_lines": 50,
    "rrf_k": 20
  },
  "path": {
    "default_max_hops": 8,
    "max_allowed_hops": 20,
    "timeout_ms": 5000
  },
  "callers":      { "max_depth_cap": 4, "transitive_timeout_ms": 3000 },
  "callees":      { "max_depth_cap": 4, "transitive_timeout_ms": 3000 },
  "dependencies": { "max_imports": 100, "max_imported_by": 100 },
  "edit_context": { "max_callers": 15, "max_callees": 20 },
  "diff_impact":  { "max_high_risk_callers": 10, "max_affected_symbols": 50 },
  "session_context": { "max_explored_symbols_in_response": 50 },
  "hotspots": {
    "default_top_n": 10,
    "default_since": "6 months ago",
    "default_min_churn": 2,
    "risk_critical_threshold": 0.75,
    "risk_high_threshold": 0.50,
    "risk_medium_threshold": 0.25
  }
}
```

`hub_threshold.coreness_pct: 75` — p75 coreness làm ngưỡng bridge-hub. `min_callers_bridge: 2`
[F12] — floor thấp hơn cho bridge-hub, cho phép "moderate in-degree, high coreness" (caller_count
2–4). `search.rrf_k: 20` — k=20 cho score discrimination tốt hơn k=60 trên top-10 code symbol
lists (rank=1 vs rank=10: k=60 → chỉ ~1.1× diff; k=20 → ~1.4× diff — discrimination rõ ràng
hơn cho danh sách ngắn). Nếu cần behavior cũ: set `"rrf_k": 60` trong config.json. `edit_context.max_callers: 15`
— up từ 10, giảm `callers_truncated: true` cases.

**Startup migration detection** (trong server.py, sau khi load config):

```python
if config.get("search", {}).get("rrf_k") == 60:
    print("[codeindex] Note: search.rrf_k=60 detected in config.json. "
          "Default is now rrf_k=20 (better score discrimination for code search). "
          "Your config.json overrides this — keeping rrf_k=60. "
          "Remove 'rrf_k' from config.json to use new default.")
```

`callers.max_depth_cap = callees.max_depth_cap = 4` (symmetric). Server nên log
`transitive_capped` events per tool+direction — dùng data thực tế để quyết định asymmetry
trong tương lai nếu cần (e.g. callers cần depth lớn hơn callees).

---

**Indexing pipeline**:

```
Phase 0: scan_files → hash_check → build_file_tree
         ensure_gitignore(project_root)   # idempotent
         migrate_to_v2_6(conn)            # idempotent, CREATE INDEX IF NOT EXISTS [F8]
         coverage_data = CoverageReader.load(project_root)   # in-memory, zero writes
         codeowners_patterns = codeowners.load_codeowners(project_root)   # cached

Phase 1: parse_with_tree_sitter
         → extract_symbols (signature, docstring, line_range, is_entry_point)
         → insert_sqlite
         → build_fts5_dual_column (pre-tokenization)

Phase 2: extract_raw_call_edges + import_edges + inheritance_edges
         → resolve_confidence (ConservativeResolver + alias_map, F7 guard, Rust/Go handlers)
         → insert_edges_with_confidence
         → compute_coreness(conn)                     # O(V+E) thật — bucket-by-degree [F11]
         → batch_update symbols.coreness               # executemany, qualified_name [F4]
         → update_is_hub_flags(conn, config)           # percentile_rank + p75 + is_hub [F6]
         → watch_for_changes
         → on_change: node_update + edge_invalidation + confidence_update
                    + debounced(500ms) recompute_coreness_global() + update_is_hub_flags()

[track riêng, opt-in]
if semantic_search.enabled:
  download_embedding_model (first run) → embed_symbols_batch → store_vectors(sqlite-vec)
  on_change: re-embed changed symbols only
  on_fail: embeddings_status → "failed", populate embeddings_error
```

---

**DB Schema**:

```sql
-- Coreness column trong symbols table
ALTER TABLE symbols ADD COLUMN coreness INTEGER DEFAULT 0;

-- Backward index trên call_edges (enables bidirectional BFS + faster callers queries)
CREATE INDEX IF NOT EXISTS idx_call_edges_to ON call_edges(to_symbol);
```

`idx_call_edges_to` benefit cả `path` tool (backward BFS) lẫn `callers` tool.
`symbols.qualified_name` là unique identifier — cùng format với
`call_edges.from_symbol`/`to_symbol` — coreness/is_hub UPDATE (F4) dùng cột này.

---

**DB Migration** — simplified idiom (F8):

```python
def migrate_to_v2_6(conn: sqlite3.Connection) -> None:
    """
    Idempotent migration — safe to run multiple times.
    Chạy trong `serve --project` startup, trước Phase 0.
    """
    # ALTER TABLE chưa support IF NOT EXISTS trên phần lớn SQLite versions
    # (chỉ từ 3.37+ với STRICT tables) — phải check thủ công qua PRAGMA.
    existing_cols = [row[1] for row in conn.execute("PRAGMA table_info(symbols)")]
    if "coreness" not in existing_cols:
        conn.execute("ALTER TABLE symbols ADD COLUMN coreness INTEGER DEFAULT 0")
        # coreness values sẽ populate sau Phase 2 hoàn tất;
        # existing rows giữ DEFAULT 0 cho đến lần index tiếp theo.

    # [F8] CREATE INDEX IF NOT EXISTS — atomic idiom trong SQLite 3.3.7+ (2006),
    # thay cho manual existence check thủ công của v2.6.
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_call_edges_to ON call_edges(to_symbol)"
    )

    conn.commit()
    print("[codeindex] Migration v2.6 complete")
```

---

## Implementation Order

### Base System (v2.6.2)

```
Step 1: DB schema (prerequisite cứng)
  1a. CREATE INDEX IF NOT EXISTS idx_call_edges_to        [F8]
  1b. ALTER TABLE symbols ADD COLUMN coreness INTEGER DEFAULT 0

Step 2: Phase 2 pipeline additions (độc lập nhau)
  2a. compute_coreness() với batch UPDATE qualified_name  [F4]
  2b. update_is_hub_flags() với caller_count_percentile   [F6]
      + debounce on_change 500ms                          [F5]
  2c. Alias extraction với multi-assign pre-pass           [F7]

Step 3: Path tool rewrite
  3a. Bidirectional BFS: batch queries + tie-break + set    [F1] [F2] [F3]
      + exhaustion flags + tie_toggle                       [F10]
  Note: [F9] was merged into [F8] during v2.6.2 consolidation — no standalone fix.

Step 4: Config additions
  4a. coreness_pct vào hub_threshold
  4b. min_callers_bridge vào hub_threshold                  [F12]
  4c. rrf_k vào search

Step 5: Tool surface
  5a. symbol_info + file_overview → add coreness field
  5b. source.metadata → coreness nếu include_metadata

Step 6: Behavioral layer
  6a. AGENTS.md: Stage 1-8 với bridge-hub guidance
  6b. workflow_guide string
```

### v2.7 Additions (dependency-safe)

```
=== Tier 1: Surface (không deps vào Tier 2/3) ===

Step 7: Tool Description Rewrites
  7a. Replace description strings cho tất cả 13 tools có mặt tại bước này:
      repo_overview, search, file_overview, symbol_info, source, callers, callees,
      dependencies, path, edit_context, session_context, diff_impact, indexing_status
      (locate, hotspots, understand được add ở Steps 10, 11, 16 tương ứng — skip ở đây)
  [Độc lập — có thể deploy trước]

Step 8: SuggestedNext type + logic
  8a. Define SuggestedNext type trong shared_types.py
  8b. Implement compute_suggested_next() per-tool logic (preset-aware, pure function)
  8c. Add suggested_next?: SuggestedNext vào output của tất cả tools

Step 9: Preset System
  9a. Implement PRESET_TOOL_SETS dict với validation trong server.py
  9b. Add --preset CLI argument + config.json "preset" support
  9c. Implement register_tools() với preset filter + ValueError trên unknown preset

=== Tier 2: Compound Tools (deps vào base tools stable) ===

Step 10: locate tool
  10a. Implement locate() trong tools/locate.py
       — Internally reuse search_logic(), file_overview_logic(), symbol_info_logic()
       — kind=text/file + depth=with_symbol → auto-downgrade + depth_adjusted field
  10b. Add tool description cho locate
  10c. Add suggested_next logic cho locate
  10d. Add to preset definitions: "orient", "trace", "edit", "compound", "full"

Step 11: hotspots tool
  11a. Implement compute_hotspots() trong hotspot.py (git+index, fallback index_only)
       — include_symbols populated, language field, total_files_analyzed tuple return
       — git log format %aI (strict ISO 8601)
  11b. Implement MCP wrapper trong tools/hotspots.py
  11c. Add config section "hotspots" với defaults
  11d. Add to preset: "orient", "compound"

=== Tier 3: Intelligence Upgrades (deps vào base index engine) ===

Step 12: Coverage Reader
  12a. Implement CoverageReader trong coverage_reader.py
       — lcov, python .coverage, go coverage.out, cobertura XML parsers
       — Cobertura: union per-file lines (multiple <class> per file)
  12b. Load on server startup, inject into indexer
  12c. Update _compute_dead_code_confidence() signature + logic (returns tuple)
  12d. Add dead_code_source field to Health type (always present)
  12e. Update Health output in symbol_info, source, understand

Step 13: Rust/Go Alias Tracking
  13a. Add rust + go to ASSIGNMENT_NODES in lang_constants.py
  13b. Implement _get_rust_assignment_lhs_rhs() (skip mutable, skip type annotation)
  13c. Implement _get_go_assignment_lhs_rhs() (short_var_decl + var_decl, unwrap expression_list)
  13d. Update _get_assignment_lhs_rhs() dispatch

Step 14: CODEOWNERS Integration
  14a. Implement codeowners.py (load_codeowners + find_owners + get_git_blame_owners)
       — Segment-by-segment glob ('*' stays within one path segment)
       — Extension-less files matched correctly by basename
  14b. Load CODEOWNERS on server startup (cached once, not per-invocation)
  14c. Inject suggested_reviewers into diff_impact when risk >= medium

=== Tier 4: Config + Compound Tools + Behavioral Layer ===

Step 15: Config Defaults
  15a. Update config.json: search.rrf_k: 60 → 20
  15b. Update config.json: edit_context.max_callers: 10 → 15
  15c. Add config.json "hotspots" section
  15d. Add startup migration detection log for rrf_k=60

Step 16: understand tool
  16a. Implement understand() trong tools/understand.py
       — Compose search_logic + symbol_info_logic + source_logic + callers_logic(limit=5)
  16b. Add to preset: "compound", "full"

Step 17: AGENTS.md + workflow_guide v2.7
  17a. Add Stage 1.5 (hotspots — sau Stage 1, conditional on indexing_phase==ready)
  17b. Update Stage 2 (locate)
  17c. Update Stage 3 (dead_code_source + medium+coverage interpretation)
  17d. Update Stage 7 (suggested_reviewers)
  17e. Add Preset Guidance section
  17f. Update workflow_guide string trong repo_overview

Không có circular dependencies. Tier 1 (Step 7-9) có thể ship trước Tier 2/3.
Step 10-11 (Tier 2) có thể làm song song với Step 12-14 (Tier 3).
Step 16-17 phải sau tất cả steps khác.
```

---

**Khởi động**:

```bash
python -m codeindex serve --project /path/to/repo
python -m codeindex serve --project /path/to/repo --preset=orient
```

```json
{
  "mcpServers": {
    "codeindex": {
      "command": "codeindex",
      "args": ["serve"]
    }
  }
}
```

---

## Navigational Map: 16 Tools × Intent

```
repo_overview     → nhìn tổng toàn bộ bản đồ + trạng thái index (phase + embeddings)
search            → tìm kiếm: tên, text, file, concept (semantic), hoặc hybrid
file_overview     → xem một khu vực: có gì, ai là landmark, inferred role
dependencies      → kết nối file-level: vùng này import ai, ai import vùng này
symbol_info       → thông tin tại một địa chỉ cụ thể: metadata + health + coreness signals
source            → vào trong tòa nhà đọc chi tiết, luôn fresh từ disk
callers           → ai đang đổ vào giao lộ này (upward), với confidence per edge
callees           → giao lộ này đổ đi đâu (downward), với confidence per edge
path              → tìm đường giữa hai điểm, Bidirectional BFS bounded, batched + audited
edit_context      → bản đồ đầy đủ trước khi đặt vật cản, freshness-aware
session_context   → you are here — đã đi đâu (MRU-50), frontier là gì
diff_impact       → blast radius + signature change signal + suggested_reviewers
indexing_status   → trạng thái bản đồ đang vẽ đến đâu + embedding recovery trigger
locate            → compound: search + file_overview + symbol_info in 1 call
hotspots          → proactive churn × complexity risk scan — files cần đặc biệt cẩn thận
understand        → compound: locate + source + callers summary in 1 call
```

---

## Maintenance Checklist

Trước khi merge bất kỳ patch nào thêm shared field vào một tool, grep xác nhận tool
đó đã được liệt trong đúng §Shared Field Types:

| Nếu patch thêm field...     | Phải có tool trong...                                         |
|-----------------------------|---------------------------------------------------------------|
| `edges_ready: bool`         | §Edge-Readiness Note Convention                               |
| `ambiguous`/`candidates`    | §Ambiguity Contract                                           |
| `health` / `Health` type    | §Health *(symbol_info.health, source.metadata.health, understand.health)* |
| `EdgeConfidence`            | §EdgeConfidence                                               |
| `coreness`                  | §symbol_info, §file_overview.symbols[], §source.metadata      |
| `suggested_next`            | §SuggestedNext type definition + logic table                  |
| `dead_code_source`          | §Health type (`dead_code_source` present, always non-optional)|
| `suggested_reviewers`       | §diff_impact output + §ReviewerSuggestion type                |
| Tool mới                    | phải có trong §Preset Definitions                             |

Không tin vào trí nhớ — grep tên field trong toàn bộ §Shared Field Types, confirm tool
đang sửa đã xuất hiện trước khi approve merge.

---

## Regression Test Targets

### `path` tool

- `bidirectional_bfs(A,B) == exists:true` ↔ `unidirectional_bfs(A,B) == exists:true` trên
  cùng graph — existence phải identical, chỉ speed khác
- `routes[0]` là valid path — walk từng step, verify edge tồn tại trong `call_edges`
- Cycle trong graph → terminate cleanly, không loop
- `terminated_by: "max_hops"` chỉ khi `f_depth + b_depth >= max_hops`
- **[F1]** Graph với frontier 1000 nodes → verify 1 DB query/BFS level — check qua `conn.set_trace_callback`
- **[F2]** Linear chain `A→B→C→D→E`, branching factor 1 → verify cả `f_depth` và `b_depth` tăng
- **[F3]** `assert isinstance(meeting_nodes, set)` — no duplicate meeting nodes
- **[F10-test-1]** `from_symbol` là leaf (0 callees), `to_symbol` có backward chain dài 5 hop
  trong `max_hops=8` → `exists: true`, KHÔNG rơi vào `"max_hops"`/`"timeout"`
- **[F10-test-2]** Cả hai phía cạn cùng lúc, không gặp nhau → `exists: false`, `terminated_by: null`
- **[F10-test-3]** Tie xảy ra ở bước 1, rồi 3 bước không-tie, rồi tie lại ở bước 5 →
  `tie_toggle` tại 2 lần tie đó phải khác nhau
- `path(terminated_by="max_hops")` → `suggested_next.tool == "path"`, `suggested_next.args.max_hops > original` ✓
- `path(terminated_by="max_hops")` → `suggested_next.args.from_symbol == original to_symbol` (reverse heuristic) ✓
- `path(terminated_by="timeout")` hint unchanged: still suggests smaller max_hops, not reversed ✓

### `is_hub` và coreness

- Symbol với `caller_count < min_callers_bridge` → `is_hub: false` bất kể coreness
- Symbol từng `is_hub: true` ở v2.6.1 → vẫn `is_hub: true` (min_callers_bridge ≤ min_callers)
- Codebase rỗng (0 edges) → tất cả `coreness: 0`, is_hub fall back về pure degree check
- **[F4]** Hai symbols khác file cùng tên `process` nhưng khác `qualified_name` → mỗi symbol nhận coreness riêng, không clobber nhau
- **[F6]** 10 symbols caller_counts `[1..10]` → symbol caller_count=8 phải có `percentile_rank = 0.8`
- **[F5]** Log elapsed time của `compute_coreness` + `update_is_hub_flags` tại 50k/200k/500k edges. Fail build nếu > 2s trên reference machine
- **[F11-test]** Graph "staircase" ở scale 50k+ edges → verify `compute_coreness` tăng gần-linear theo (V+E)
- `caller_count=2` (≥ min_callers_bridge, < min_callers), coreness >> p75 → `is_hub: true` ✓
- `caller_count=1` (< min_callers_bridge), coreness >> p75 → `is_hub: false` ✓

### Conservative Resolver

- Edge confidence từ v2.5.2 không bao giờ downgrade
- `alias_map` chỉ chứa bare-identifier assignments — complex RHS không vào alias_map
- Resolver không crash khi `alias_map` rỗng
- **[F7]** Single-assignment: `x = notify_user` → `alias_map["x"] == "notify_user"` ✓
- **[F7]** Multi-assignment: `x = notify_user` line 10, `x = send_email` line 50 → `"x" not in alias_map` ✓
- **[F7]** Cross-scope: `x = notify_user` trong func A, `x = send_email` trong func B → `"x" not in alias_map` (file-level conservative) ✓
- **[F7]** Complex RHS: `x = func()`, `x = obj.attr`, `x = a if c else b` → `"x" not in alias_map` ✓

### Rust/Go Alias Tracking

- Rust `let x = some_func;` → `alias_map["x"] == "some_func"` ✓
- Rust `let mut x = some_func;` → `"x" not in alias_map` (mutable, skip) ✓
- Rust `let (a, b) = pair;` → neither in alias_map (destructuring) ✓
- Rust `let x = func();` → `"x" not in alias_map` (call, not bare identifier) ✓
- Rust `let x: Type = y;` → `"x" not in alias_map` (type annotation) ✓
- Go `x := y` (single LHS, single RHS, both identifiers) → `alias_map["x"] == "y"` ✓
- Go `x, y := a, b` (multi LHS) → neither in alias_map ✓
- Go `x := func()` (RHS is call) → `"x" not in alias_map` ✓
- Go `var x = someFunc` → `alias_map["x"] == "someFunc"` ✓
- Go `var x int = someFunc` (typed) → `"x" not in alias_map` ✓
- F7 guard: Rust `let x = a;` at line 5, `let x = b;` at line 10 → `"x" not in alias_map` ✓
- F7 guard: Go `x := a` at line 5, `x = b` at line 10 → `"x" not in alias_map` ✓

### Schema consistency

- `coreness: null` khi `edges_ready: false`
- `coreness: 0` khi symbol isolated (`edges_ready: true`)
- Migration idempotent: chạy 2 lần không crash, không duplicate index
- `dead_code_source` present in ALL symbol_info responses (not optional) ✓
- `dead_code_source == "static"` when no coverage file found ✓
- `dead_code_confidence` logic không đổi khi coverage data không available — graceful
  degradation về static-only, không phá behavior cũ ✓

### `locate` tool

- `locate(query).results` semantically equivalent to `search(query).results` khi cùng query + kind
- `locate(query, depth="search_only")` == `search(query)` (output identical)
- `locate("X", depth="with_symbol").top_result.symbol.name == symbol_info("X").name`
- `locate("X").top_result.file.inferred_role == file_overview(locate("X").results[0].path).inferred_role`
- `locate(query_no_match).top_result` absent — không crash
- Ambiguous top result → `top_result.symbol.ambiguous == true`, `candidates` non-empty
- `locate(kind="text", depth="with_symbol")` → `depth_adjusted == "with_file"`, symbol_info NOT called ✓
- `locate(kind="file", depth="with_symbol")` → same downgrade ✓
- `locate(kind="symbol", depth="with_symbol")` → `depth_adjusted` absent (no downgrade) ✓

### `hotspots` tool

- `hotspot_score` ∈ [0.0, 1.0] cho mọi entry
- `risk_level` consistent với score thresholds (critical ≥ 0.75, high ≥ 0.50, medium ≥ 0.25)
- Git unavailable → `git_available: false`, `hotspot_method: "index_only"`, không crash
- Git unavailable + min_churn=5 → `note` field: "min_churn parameter not applied" ✓
- Git unavailable → `hotspot_score == norm_compl` (không nhân churn) ✓
- Git unavailable → `commit_count in output == 0` ✓
- Git available + min_churn=5 → chỉ files có commit_count >= 5 ✓
- `top_n=5` → at most 5 entries returned
- `total_files_analyzed` == candidate count BEFORE top_n slice (không phải 5) ✓
- `compute_hotspots()` luôn trả tuple `(list, int)`, KHÔNG bao giờ trả bare list kể cả khi
  `candidates` rỗng ✓
- Empty codebase (0 symbols) → `hotspots: []`, không crash
- `include_symbols=true` → `top_symbols` present cho mọi hotspot entry ✓
- mỗi hotspot entry có `language` non-null khi file có symbols ✓
- Symbol với coreness=0 → NOT counted in connected_coreness_count ✓
- Symbol với coreness=1 → counted in connected_coreness_count ✓
- Doc mô tả field nói "coreness > 0", không phải ">= p75" — khớp với SQL:
  `SUM(CASE WHEN coreness > 0 THEN 1 ELSE 0 END)` ✓
- `churn.last_changed` là `null` khi git unavailable ✓; khi git available, parses với `datetime.fromisoformat()` ✓ (format %aI, strict ISO 8601)

### Coverage Reader

- lcov DA:10,3 → line 10 in covered set ✓
- lcov DA:10,0 → line 10 NOT in covered set ✓
- Symbol at lines 10-20, lcov hit at line 15 → `is_covered(path, 10, 20) == True` ✓
- Symbol at lines 10-20, lcov hits at 5 and 25 only → `is_covered(path, 10, 20) == False` ✓
- Corrupt/unreadable coverage file → `CoverageData(source="none")`, no exception ✓
- No coverage file → `CoverageData(source="none")` ✓
- Cobertura XML với 2 `<class>` elements cùng `filename` → `covered_lines` là UNION, không mất line ✓
- `dead_code_confidence` with coverage hit + no callers → `"low"`, `dead_code_source="static+coverage"` ✓
- `dead_code_confidence` with no coverage + no callers + private → `"high"`, `dead_code_source="static"` ✓

### `suggested_next` hints

- `repo_overview` phase=ready, embeddings=ready → `suggested_next.tool == "locate"` ✓
- `repo_overview` phase=parsing → `suggested_next.tool == "indexing_status"` ✓
- `symbol_info` is_hub=true → `suggested_next.tool == "edit_context"` ✓
- `source` is_hub=true in metadata → `suggested_next.tool == "edit_context"` ✓
- `edit_context` → `suggested_next.tool == "diff_impact"` ALWAYS ✓
- `diff_impact` aggregate_risk=low, indexed files all → `suggested_next` absent ✓
- `suggested_next.args` consistent với tool input schema (no unknown keys) ✓
- `locate()` trong `--preset=orient` → `suggested_next.tool` nằm trong `{"repo_overview", "locate", "dependencies", "hotspots", "indexing_status"}` ✓
- `--preset=full` (available_tools=None) → no filtering, all hints allowed ✓
- `search(kind="hybrid")` empty results → `suggested_next.args.kind == "text"` (NOT "semantic") ✓
- `search(kind="text")` empty results → `suggested_next` suggests "hybrid" ✓

### CODEOWNERS Integration

- Pattern `src/auth/` matches `src/auth/login.py` → owners correct ✓
- Pattern `*.ts` matches `src/user.ts` → owners correct ✓
- Pattern `src/*.py` matches `src/user.py` ✓
- Pattern `src/*.py` does NOT match `src/auth/user.py` (nested — `*` không cross `/`) ✓
- Last matching rule wins: `*.py @team-a` then `src/*.py @team-b` → `src/x.py` → `[@team-b]` ✓
- Pattern "Makefile @build-team" matches file "Makefile" ✓
- Pattern "Dockerfile @ops" matches file "Dockerfile" ✓
- Pattern "Procfile @platform" matches file "Procfile" ✓
- Pattern "Jenkinsfile @ci" matches file "Jenkinsfile" ✓
- Pattern "src/" → NOT match file literally named "src" ✓
- Pattern "src" (không trailing slash) → match file literally named "src" (basename match, not directory prefix) ✓
- Pattern "src/" (trailing slash) → match "src/anything" qua explicit directory prefix ✓
- No CODEOWNERS file → fallback to git_blame (or empty if git unavailable) ✓
- `aggregate_risk == "low"` → `suggested_reviewers` absent ✓
- CODEOWNERS present but file not matched → git_blame fallback, source="git_blame" ✓
- `load_codeowners` gọi đúng 1 lần trên startup, không gọi trong diff_impact handler ✓
- Server startup log: "[codeindex] CODEOWNERS loaded: N patterns" ✓
- CODEOWNERS file không tồn tại → codeowners_patterns = [] → no error, git_blame fallback ✓

### Preset System

- `--preset=orient` → only 5 tools registered, `locate` available, `callers` NOT available ✓
- `--preset=full` → all 16 tools registered ✓
- Preset in config.json overridden by CLI flag ✓
- Unknown preset value → `ValueError` at startup with meaningful message ✓
- `locate` trong `--preset=full` → available (full = all 16 tools) ✓

---
