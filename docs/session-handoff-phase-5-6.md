# Session Handoff — Phase 5 + Phase 6

> Tài liệu bàn giao cho session tiếp theo. Đọc trước khi bắt đầu code.

---

## Trạng thái hiện tại

**Branch**: `claude/code-intelligence-setup-6q6zn7`
**PR**: #2 (draft) — đã bao gồm Phase 0.5 → Phase 4
**CI**: Green (clippy, fmt, test — 94 tests pass)
**Repo chỉ được sửa**: `Code-Intelligence`. **TUYỆT ĐỐI KHÔNG** thay đổi `SUPER-MCP` (chỉ dùng tham chiếu).

### Đã hoàn thành

| Phase | Nội dung | Commit |
|-------|----------|--------|
| 0.5 | CI pipeline, debt registry, ADR-0001 | `96c0b19` |
| 1B | Analysis module ports (coverage, dead_code, codeowners, hotspot, diff_impact), C-1 regression test | `96c0b19` |
| 2 | FormalResolver (stack-graphs), ConservativeResolver, EdgeConfidence, ADR-0002 | `0417e15` |
| 3 | FTS5 dual-column BM25, output sanitizer (10 patterns), search module (5 SearchKind) | `a2c3969` |
| 4 | 16 MCP tool handlers via rmcp `#[tool]`, clap CLI, ServerHandler, Mutex\<Connection\> | `f7b79a4` |

### Codebase structure

```
crates/
├── ci-core/src/
│   ├── lib.rs              # pub mod: db, graph, config, types, analysis, resolver, search, sanitize
│   ├── db/schema.rs        # SQLite schema: symbols, file_index, call_edges, import_edges, fts_exact, fts_tokens
│   ├── db/queries.rs       # batch_callees, batch_callers
│   ├── graph/coreness.rs   # K-core O(V+E)
│   ├── graph/hub.rs        # Hub detection (C-2 fix)
│   ├── graph/path.rs       # Bidirectional BFS (F1,F2,F3,F10)
│   ├── graph/tokenize.rs   # Identifier tokenization
│   ├── resolver/formal.rs  # Stack Graphs Python resolver
│   ├── resolver/conservative.rs  # Alias-tracking 6 languages
│   ├── search.rs           # FTS5 search: symbol, text, file, semantic(stub), hybrid(RRF k=20)
│   ├── sanitize.rs         # Credential patterns → [REDACTED:label]
│   ├── config.rs           # Config loading + defaults
│   ├── types.rs            # 11 enums (SearchKind, EdgeConfidence, TerminatedBy, etc.)
│   └── analysis/           # coverage, dead_code, codeowners, hotspot, diff_impact (stubs with types)
├── ci-server/src/
│   ├── lib.rs              # serve_stdio(), doctor(), default_db_path()
│   └── tools.rs            # 16 MCP tools, CodeIntelligenceServer, ServerHandler impl
└── ci-cli/src/
    └── main.rs             # clap CLI: ci serve, ci index(stub), ci doctor
```

### Key technical details

- **rmcp 0.1.5** — `#[tool(tool_box)]` on impl block generates tool_box + ServerHandler methods. `#[tool]` macro only supports `name`, `description`, `vis` attributes. **KHÔNG có `annotations()`** — rmcp 0.1.5 doesn't support tool annotations in macro.
- **Mutex\<Connection\>** — rusqlite::Connection is NOT Sync. Server uses `Arc<Mutex<Connection>>` with `db()` helper. For `prepare()` calls, must bind guard to local variable (`let conn = self.db(); let stmt = conn.prepare(...)`) to avoid temporary dropped while borrowed.
- **Tool return type** — rmcp tools return `String` (implements `IntoContents`). No `Json<T>` wrapper. Serialize via `serde_json::to_string_pretty(&output).unwrap_or_default()`.
- **Parameters** — `rmcp::handler::server::tool::Parameters<T>` for tool input deserialization.
- **Tests** — 94 tests across ci-core (search: 11, sanitize: 11, graph: 12, db: 2, resolver: various). ci-server has 0 unit tests currently.

---

## Phase 5 — Capability Mới

**Ref**: `docs/migration-plan-v2.md` → "Phase 5 — Capability Mới"

### 5.1 — `symbol_metrics_history` table

Thêm bảng mới vào `ci-core/src/db/schema.rs`:

```sql
CREATE TABLE IF NOT EXISTS symbol_metrics_history (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    qualified_name  TEXT NOT NULL,
    snapshot_at     TEXT NOT NULL,       -- ISO8601 timestamp
    caller_count    INTEGER NOT NULL DEFAULT 0,
    callee_count    INTEGER NOT NULL DEFAULT 0,
    coreness        INTEGER NOT NULL DEFAULT 0,
    is_hub          INTEGER NOT NULL DEFAULT 0,
    churn_count     INTEGER NOT NULL DEFAULT 0,
    complexity      REAL,
    UNIQUE(qualified_name, snapshot_at)
);
CREATE INDEX IF NOT EXISTS idx_smh_symbol ON symbol_metrics_history(qualified_name);
CREATE INDEX IF NOT EXISTS idx_smh_time   ON symbol_metrics_history(snapshot_at);
```

Cập nhật `init_db()` để include migration. Tham khảo pattern migration idempotent đã có (CREATE TABLE IF NOT EXISTS).

### 5.2 — `ci fitness-check` CLI command

**File**: `crates/ci-cli/src/main.rs` — thêm subcommand `FitnessCheck`

```rust
FitnessCheck {
    #[arg(long, default_value = ".")]
    project_root: PathBuf,
    #[arg(long)]
    config: Option<PathBuf>,           // thresholds.toml
    #[arg(long, default_value = "false")]
    json: bool,                        // JSON output for CI parsing
}
```

**Logic** (`crates/ci-core/src/fitness.rs` — file mới):
1. Load thresholds từ TOML (hoặc defaults):
   ```toml
   [thresholds]
   max_hub_count = 50
   max_avg_coreness = 15.0
   max_dead_code_pct = 10.0
   max_hotspot_risk = 0.75
   min_edge_coverage_pct = 60.0
   ```
2. Query DB cho current metrics:
   - Hub count: `SELECT COUNT(*) FROM symbols WHERE is_hub = 1`
   - Avg coreness: `SELECT AVG(coreness) FROM symbols WHERE coreness > 0`  
     (Lưu ý: `symbols` table chưa có column `coreness` — cần thêm hoặc compute on-the-fly từ graph)
   - Dead code %: delegate tới `analysis::dead_code` (nếu đã port đầy đủ, hoặc stub)
   - Hotspot risk: max risk từ `analysis::hotspot`
   - Edge coverage: `SELECT COUNT(DISTINCT from_symbol) FROM call_edges` / total symbols
3. Compare với thresholds
4. Exit code: 0 nếu pass, 1 nếu fail
5. Output: human-readable default, JSON nếu `--json`

**Tham khảo**: SUPER-MCP `npm run ci` pattern — fitness check chạy như CI gate trong target project.

### 5.3 — Structured telemetry

**File**: `crates/ci-server/src/telemetry.rs` (file mới)

Telemetry events qua `tracing`:
```rust
tracing::info!(
    tool = tool_name,
    duration_ms = elapsed.as_millis(),
    result_size = result.len(),
    "tool_execution_completed"
);
```

Cần wrap tool execution trong timing logic. Có 2 cách:
1. **Middleware approach**: Wrap `call_tool` trong ServerHandler impl để tự động time mọi tool call.
2. **Per-tool approach**: Thêm timing vào mỗi tool function.

Recommend option 1 — override `call_tool` trong `impl ServerHandler`:
```rust
async fn call_tool(&self, req: CallToolRequestParam, ctx: RequestContext<RoleServer>) -> Result<CallToolResult, McpError> {
    let start = std::time::Instant::now();
    let tool_name = req.name.clone();
    let result = Self::tool_box().call(ToolCallContext::new(self, req, ctx)).await;
    tracing::info!(tool = %tool_name, duration_ms = start.elapsed().as_millis(), "tool_call");
    result
}
```

**Lưu ý**: `#[tool(tool_box)]` trên `impl ServerHandler` đã generate `call_tool`. Nếu muốn override, cần KHÔNG dùng `@derive` cho `call_tool` mà viết tay. Có thể cần restructure: dùng `#[tool(tool_box)]` chỉ trên plain impl block (generates `fn tool_box()`), rồi viết `impl ServerHandler` manually với custom `call_tool` + `list_tools` delegation.

### 5.4 — Snapshot writer

**File**: `crates/ci-core/src/fitness.rs` hoặc `crates/ci-core/src/db/metrics.rs`

Function `snapshot_metrics(conn, timestamp)`:
1. Query current symbols + coreness + caller_count
2. INSERT INTO symbol_metrics_history
3. Gọi từ `ci index` sau khi index xong, hoặc từ `ci fitness-check`

### 5.5 — Tests cần viết

- `fitness.rs`: threshold pass/fail logic, default thresholds, TOML parsing
- `telemetry.rs`: verify tracing events emitted (dùng `tracing_test` crate hoặc custom subscriber)
- `db/schema.rs`: verify `symbol_metrics_history` table created, insert/query works
- Integration: `ci fitness-check` exit code 0/1

### 5.6 — Exit criteria

- [ ] `symbol_metrics_history` table exists, migration idempotent
- [ ] `ci fitness-check` exit 0 khi metrics within thresholds
- [ ] `ci fitness-check` exit 1 khi vượt threshold, message rõ ràng
- [ ] `ci fitness-check --json` output parseable JSON
- [ ] Telemetry events in stderr khi tool called
- [ ] CI green (clippy, fmt, tests)

---

## Phase 6 — Distribution

**Ref**: `docs/migration-plan-v2.md` → "Phase 6 — Distribution"

### 6.1 — Cross-compile static binary

**File**: `.github/workflows/release.yml` (file mới)

```yaml
name: Release
on:
  push:
    tags: ['v*']

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-musl
            os: ubuntu-latest
          - target: aarch64-unknown-linux-musl
            os: ubuntu-latest
          - target: aarch64-apple-darwin
            os: macos-latest
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - name: Install musl tools (Linux)
        if: contains(matrix.target, 'musl')
        run: sudo apt-get install -y musl-tools
      - name: Install cross (aarch64-linux)
        if: matrix.target == 'aarch64-unknown-linux-musl'
        run: cargo install cross
      - name: Build
        run: |
          if [ "${{ matrix.target }}" = "aarch64-unknown-linux-musl" ]; then
            cross build --release --target ${{ matrix.target }}
          else
            cargo build --release --target ${{ matrix.target }}
          fi
      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: ci-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/ci

  release:
    needs: build
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/download-artifact@v4
      - name: Create release
        uses: softprops/action-gh-release@v2
        with:
          files: ci-*/ci
```

**Lưu ý quan trọng**: tree-sitter grammars cần compile native. Khi cross-compile cho musl, cần verify tree-sitter-python/etc build thành công. Nếu fail → fallback: build trên CI native runner cho mỗi platform.

### 6.2 — Containerfile

**File**: `Containerfile` (root)

```dockerfile
FROM rust:1.85-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
RUN cargo build --release --target x86_64-unknown-linux-musl

FROM scratch
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/ci /ci
ENTRYPOINT ["/ci"]
CMD ["serve", "--project-root", "/project", "--db-path", "/data/index.db"]
```

### 6.3 — compose.yaml example

**File**: `compose.yaml` (root)

```yaml
services:
  code-intelligence:
    build: .
    read_only: true
    cap_drop: [ALL]
    security_opt: [no-new-privileges:true]
    pids_limit: 64
    mem_limit: 256m
    volumes:
      - ./:/project:ro
      - ci-data:/data
    command: ["serve", "--project-root", "/project", "--db-path", "/data/index.db"]

volumes:
  ci-data:
```

### 6.4 — `ci init` command

**File**: `crates/ci-cli/src/main.rs` — thêm subcommand

```rust
Init {
    #[arg(long, default_value = ".")]
    project_root: PathBuf,
}
```

Logic:
1. Detect project root (tìm `.git/`, `Cargo.toml`, `package.json`, `pyproject.toml`)
2. Tạo `.codeindex/` directory
3. Tạo `.codeindex/config.json` với defaults từ `config.rs`
4. Print hướng dẫn: "Run `ci index` to build the index, then `ci serve` to start MCP server"

### 6.5 — Tests cần viết

- Release workflow: test manually bằng `act` (GitHub Actions local runner) hoặc push test tag
- Containerfile: `docker build .` + `docker run --rm ci doctor` 
- `ci init`: verify config file created, idempotent (không overwrite existing)

### 6.6 — Exit criteria

- [ ] Binary downloadable cho 3 platform (x86_64-linux-musl, aarch64-linux-musl, aarch64-apple-darwin)
- [ ] `ci init` tạo config file đúng
- [ ] Container image build thành công
- [ ] `docker run code-intelligence doctor` pass
- [ ] Release workflow green on tag push
- [ ] CI green (clippy, fmt, tests)

---

## Gotchas & Lessons learned

1. **rmcp 0.1.5 API quirks**:
   - `#[tool]` macro chỉ support `name`, `description`, `vis`. KHÔNG có `annotations()`.
   - `#[tool(tool_box)]` trên plain impl → generates `fn tool_box()`. Trên `impl ServerHandler` → generates `list_tools()` + `call_tool()`.
   - Tool functions return `String` (not `Json<T>`). Serialize output manually.
   - `Parameters<T>` is at `rmcp::handler::server::tool::Parameters`.

2. **rusqlite::Connection is NOT Sync** → phải wrap trong `Mutex`. Khi dùng `prepare()`, PHẢI bind guard vào local variable:
   ```rust
   let conn = self.db();           // MutexGuard lives here
   let mut stmt = conn.prepare(...)  // borrows conn
   ```
   KHÔNG ĐƯỢC: `self.db().prepare(...)` — temporary guard dropped.

3. **CI on this repo** uses Rust 1.96.0+ which has newer clippy lints (e.g., `useless_conversion`). Always test clippy with `-- -D warnings` locally trước khi push.

4. **`cargo fmt --all` và `cargo clippy --workspace`** — luôn chạy CẢ HAI và verify TỪNG CÁI riêng biệt. Đừng chain `&&` rồi chỉ check exit code cuối.

5. **SUPER-MCP repo** (`/home/user/SUPER-MCP`) — CHỈ ĐỌC, KHÔNG SỬA. Dùng để tham khảo patterns.

---

## Thứ tự thực hiện đề xuất

### Phase 5 (ước lượng ~5-7 ngày)
1. `symbol_metrics_history` table + migration
2. `ci-core/src/fitness.rs` — threshold logic + TOML parsing
3. `ci fitness-check` CLI subcommand
4. Telemetry middleware trong `ci-server`
5. Tests + clippy + fmt
6. Commit + push + update PR

### Phase 6 (ước lượng ~3-5 ngày)
1. `ci init` CLI subcommand
2. `.github/workflows/release.yml`
3. `Containerfile` + `compose.yaml`
4. Test cross-compile (ít nhất x86_64-musl)
5. Tests + clippy + fmt
6. Commit + push + update PR

---

## Files quan trọng cần đọc trước khi bắt đầu

| File | Tại sao |
|------|---------|
| `docs/migration-plan-v2.md` | Chi tiết Phase 5, 6 requirements |
| `crates/ci-core/src/db/schema.rs` | Hiểu DB schema hiện tại, pattern migration |
| `crates/ci-server/src/tools.rs` | Hiểu rmcp integration pattern, ServerHandler |
| `crates/ci-server/src/lib.rs` | serve_stdio(), doctor() |
| `crates/ci-cli/src/main.rs` | CLI structure, thêm subcommands ở đây |
| `crates/ci-core/src/config.rs` | Config defaults |
| `crates/ci-core/src/types.rs` | All type enums |
| `/home/user/SUPER-MCP/` | Reference ONLY — telemetry patterns, container patterns |
