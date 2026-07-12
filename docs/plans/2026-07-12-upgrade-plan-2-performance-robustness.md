# CALM Upgrade Plan 2/3 — Hot-path Performance & Robustness

> Nguồn: `docs/audit/2026-07-12-vheatm-deep-audit.md` (Đợt 2). Series: [Plan 1: Correctness & Safety](2026-07-12-upgrade-plan-1-correctness-safety.md) → **Plan 2 (file này)** → [Plan 3: Architecture](2026-07-12-upgrade-plan-3-architecture.md).
>
> **Phạm vi:** F4 (panic surface), F13 (sanitize 13-pass), F12 (config/conn per-call), F11 (edit_context hot path), F8 (decode-budget bypass), H4 (diff_impact double-serialization). Mục tiêu chung: **cắt latency cảm nhận được trên 2 hot path của agent** (`source`-family reads và `edit_context`→`edit_*`), và loại panic khỏi tool handlers. Ước lượng: 2-3 buổi.
>
> **Điều kiện tiên quyết:** Plan 1 đã merge (đặc biệt Item 1.5 — F9 đổi chữ ký `resolve_symbol`, tránh conflict).
>
> **Đo lường (bắt buộc, trước khi sửa):** `timed_tool` đã log duration mọi tool call (`crates/calm-server/src/telemetry.rs`). Trước khi bắt đầu plan: chạy kịch bản chuẩn trên repo CALM và ghi baseline vào bảng cuối file — `repo_overview`, `locate("edit_context")`, `source("edit_lines_impl")`, `edit_context("apply_hunks")`, `edit_symbol` 1 lần sửa nhỏ, `diff_impact(staged)`. So sánh lại sau từng item. Không có số baseline = item chưa bắt đầu.

---

## Item 2.1 — F4: Quét sạch panic surface trong tool handlers  ⏱ ~half day

**Mục tiêu:** (a) một panic không bao giờ poison `edit_lock` và brick edit path; (b) lỗi DB statement-level trong handler trả `DB_ERROR` envelope thay vì panic.

**Evidence:** `tools/edit.rs:122` (`edit_lock.lock().unwrap()` — poison = mọi edit sau panic vĩnh viễn); `guardrails.rs:66,79,93,106,400,413` và tổng non-test: guardrails 7, trace 12, orient 9, inspect 3, recover 3, locate 2, testgap 1 unwraps; 65 `lock()/read()/write().unwrap()` toàn crate server.

**Thiết kế — 3 lớp, 3 commit riêng:**

**(a) No-poison lock helpers** — thêm vào `tools/common.rs`:
```rust
/// Poison-tolerant lock: mọi state sau các Mutex/RwLock này đều là
/// counter/map phụ trợ (session tracking) hoặc guard rỗng `()` — một
/// panic giữa chừng không để lại invariant vỡ đáng bảo vệ, nên vào
/// tiếp luôn đúng hơn là panic dây chuyền.
pub(crate) trait LockExt<T> {
    fn lock_ok(&self) -> std::sync::MutexGuard<'_, T>;
}
impl<T> LockExt<T> for std::sync::Mutex<T> {
    fn lock_ok(&self) -> std::sync::MutexGuard<'_, T> {
        self.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
// tương tự RwLockExt: read_ok() / write_ok()
```
Thay thế cơ học: ưu tiên **bắt buộc** cho `edit_lock` (edit.rs:122), `session_log`, `active_sessions`, `phase`, `embed_status`, `embedder`, `owns_indexer_lock`, `coverage`, `last_*_error`. Chỗ đã dùng `if let Ok(...)` giữ nguyên (đã an toàn) — chỉ diệt `.unwrap()`.

**(b) prepare/query unwrap sweep** — pattern thay thế trong handler body (closure của `timed_tool`, return type là `ToolOutcome`/`ResolvedOutcome`):
```rust
// TRƯỚC:  let mut stmt = conn.prepare(SQL).unwrap();
// SAU:
let mut stmt = match conn.prepare(SQL) {
    Ok(s) => s,
    Err(e) => return db_error(e),          // hoặc db_error_resolved(e)
};
// .query_map(...).unwrap() → cùng pattern; .filter_map(|r| r.ok()) per-row GIỮ NGUYÊN
```
Duyệt theo file, thứ tự: `guardrails.rs` → `trace.rs` → `orient.rs` → `inspect.rs` → `recover.rs` → `locate.rs` → `testgap.rs`. Sau sweep: `grep -n "\.unwrap()" crates/calm-server/src/tools/*.rs` (non-test) chỉ còn các chỗ chứng minh-được-không-fail (ghi comment `// infallible: <lý do>` tại từng chỗ giữ lại).

**(c) Pattern debt đăng ký:** dùng chính CALM — `pattern_debt_register(symbol="edit_lines_impl", note="unwrap-in-handler class — audit F4")` sau khi fix xong để phiên sau re-check bằng `pattern_debt_status`.

**Tests:**
1. `edit_lock_poison_does_not_brick_edits` — spawn thread panic trong khi giữ `edit_lock` (test truy cập field trực tiếp trong crate) → gọi `edit_lines` preview → vẫn trả kết quả, không panic.
2. Test DROP TABLE từ Plan 1 Item 1.5 mở rộng: sau khi drop `call_edges`, gọi `callers(...)` → `DB_ERROR`, không panic (trước fix: panic tại trace.rs prepare unwrap).

**Done when:** grep đếm unwrap non-test trong tools/ ≤ 5 (đều có comment infallible); 2 test xanh.
**Ghi chú scope:** `catch_unwind` toàn cục quanh handler body KHÔNG làm ở plan này (generic `T` của `timed_tool` không fabricate được; cần rmcp middleware — để ngỏ, ghi vào Plan 3 backlog nếu còn panic thực tế sau sweep).

---

## Item 2.2 — F13: `sanitize_source_output` 13-pass → RegexSet prefilter  ⏱ ~1h

**Mục tiêu:** case phổ biến nhất (code sạch, không credential) tốn đúng **một** pass RegexSet thay vì 13 pass `replace_all` + 13 allocation. Hot path: mọi `source()`, mọi caller/callee preview, mọi signature/docstring (`common.rs:918-921`, `common.rs:1578`).

**Evidence:** `sanitize.rs:79-88`.

**Thiết kế:**
```rust
static CREDENTIAL_SET: LazyLock<regex::RegexSet> = LazyLock::new(|| {
    regex::RegexSet::new(CREDENTIAL_PATTERNS_SRC).unwrap()   // cùng nguồn pattern string
});
```
- Refactor nguồn pattern: tách mảng `&'static str` pattern-sources dùng chung cho cả `RegexSet` lẫn `Vec<CredentialPattern>` (giữ per-pattern `Regex` cho replace + label) — một nguồn sự thật, không drift.
- `sanitize_source_output`: `let matched = CREDENTIAL_SET.matches(code); if !matched.matched_any() { return code.to_string(); }` — chỉ chạy `replace_all` cho các index có trong `matched`.
- `contains_credentials`: → `CREDENTIAL_SET.is_match(code)`.
- `scan_patterns` (injection, `sanitize.rs:260-266`): cùng kỹ thuật — `INJECTION_SET.matches(text)` map index → label, một pass thay vì 21 lần `is_match`.
- Giữ nguyên API public (vẫn trả `String`) — 20+ caller không đổi.

**Tests:** toàn bộ test sanitize hiện có phải xanh nguyên trạng (chúng chính là spec); thêm 1 test khớp nhau: với 5 fixture (sạch, 1 credential, nhiều credential, injection, mixed), output mới == output của implementation cũ (giữ bản cũ tạm thời dưới `#[cfg(test)] fn sanitize_reference()` để so, xoá sau khi merge).

**Done when:** tests xanh; đo lại `source("edit_lines_impl", include_metadata=true)` trong bảng baseline — kỳ vọng giảm rõ trên file lớn.

---

## Item 2.3 — F12: cache `load_config` (mtime-based) + ghi chú connection  ⏱ ~1h30

**Mục tiêu:** loại 19 lần parse `config.json` từ disk rải khắp tool calls; mỗi call chỉ còn 1 `stat()`.

**Evidence:** 19 call site `load_config` trong `crates/calm-server/src/tools/*.rs`; nặng nhất: mỗi `search`/`locate` (qua `apply_personalization_boost` — `common.rs:326`) và mỗi `edit_context` (`guardrails.rs:42`).

**Thiết kế** — thêm vào `crates/calm-core/src/config.rs`:
```rust
/// Cache theo (canonical project_root) → (mtime config.json lúc đọc, Config).
/// Hit khi mtime hiện tại == mtime cache (kể cả cùng-None khi file absent).
/// Miss/đổi → đọc lại như load_config. Trả Config clone (Config cần derive Clone).
pub fn load_config_cached(project_root: &Path) -> Result<Config, ConfigError> { … }
static CONFIG_CACHE: LazyLock<RwLock<HashMap<PathBuf, (Option<SystemTime>, Config)>>> = …;
```
- Kiểm tra `Config` và mọi struct con đã `#[derive(Clone)]` chưa (config.rs:8-93 — thêm nếu thiếu).
- Server-side: đổi **toàn bộ 19 site** sang `load_config_cached` (grep `load_config(` trong `crates/calm-server/`). CLI/indexer (`calm-cli`, pipeline.rs:1469) TUỲ CHỌN giữ bản uncached — chạy một-lần-mỗi-lệnh, không đáng đổi.
- Correctness: mtime-check giữ nguyên hành vi "sửa config.json giữa phiên có hiệu lực ngay call sau" — không đổi semantics, chỉ đổi chi phí.
- **Connection per-call (`make_read_conn`, 29 callers):** ĐO trước — nếu sau F12+F13 mà open-conn vẫn nổi trong baseline thì mới làm read-pool (2-4 conn `r2d2`-style tự viết mini). Dự đoán: dưới WAL, open+PRAGMA ~vài chục µs — **không làm nếu số đo không chứng minh**; ghi kết quả đo vào bảng cuối file.

**Tests:** unit cho cache: (1) hit không đọc file (đổi nội dung file mà không đổi mtime khó mô phỏng — thay bằng: đọc 2 lần, lần 2 trả giá trị dù file đã bị xoá tạm? không — đơn giản: touch file đổi mtime → giá trị mới được đọc); (2) file absent → default, sau đó tạo file → nhận config mới.

**Done when:** grep `load_config(` trong calm-server = 0 (chỉ còn `load_config_cached`); tests xanh; baseline `locate`/`edit_context` cải thiện.

---

## Item 2.4 — F11: `edit_context` hot path — preview batching + co-change cache  ⏱ ~half day

**Mục tiêu:** `edit_context` (tool bắt-buộc-trước-mọi-edit) không đọc lại nguyên file N lần cho N caller và không spawn `git log` mỗi call.

**Evidence:** `common.rs:1561-1590` (`line_preview` = `fs::read_to_string` nguyên file **mỗi row**; 7 call site); `guardrails.rs:129-140` (`compute_co_changes` → git subprocess mỗi call, `analysis/cochange.rs:34+`).

**Thiết kế — 2 commit:**

**(a) Preview batching** — thêm vào `common.rs`:
```rust
/// Đọc mỗi file đúng một lần, trả preview theo thứ tự items.
/// items: (path, Option<line>). Giữ nguyên semantics line_preview
/// (trim, sanitize, cap 160 chars, None cho line lỗi/EOF).
pub(crate) fn line_previews_batched(
    project_root: &Path,
    items: &[(String, Option<i64>)],
) -> Vec<Option<String>> {
    // group index theo path → đọc 1 lần/file → lines().nth cho từng line
}
```
- Refactor call sites: `edit_context` callers+callees (guardrails.rs:57-109) và `trace.rs` callers/callees — thu thập rows trước (không preview trong `query_map` closure), rồi gắn preview một lượt. `line_preview` cũ giữ cho chỗ chỉ cần 1 dòng (nếu còn) hoặc xoá nếu hết caller.
- Sanitize từng preview vẫn qua `sanitize_source_output` (sau F13 đã rẻ).

**(b) Co-change cache** — trong `calm-core/src/analysis/cochange.rs`:
```rust
/// Cache theo (project_root, target_path, since) → (Instant, CoChangeResult),
/// TTL 60s. Git history chỉ đổi khi có commit — TTL ngắn là đủ đúng
/// cho mục đích advisory của co_changed_files.
pub fn compute_co_changes_cached(…) -> CoChangeResult { … }
```
- `CoChangeResult`/`CoChangeEntry` derive Clone. `guardrails.rs:130` đổi sang bản cached.
- Cân nhắc thêm (cùng commit, config-gated): `cochange.enabled: bool` default true trong config — repo khổng lồ có thể tắt hẳn.

**Tests:** (a) golden: fixture 1 symbol có 3 callers cùng file + 1 caller file khác → previews mới == previews cũ từng-row (giữ reference impl trong test như 2.2). (b) cache: 2 call liên tiếp — call 2 không spawn git (đo bằng inject counter hoặc đơn giản: TTL logic unit test với mock Instant — chấp nhận test TTL thuần logic).

**Done when:** baseline `edit_context` trên symbol nhiều caller (`timed_tool` của `edit_context("make_read_conn")` — 29 callers) giảm rõ rệt; tests xanh.

---

## Item 2.5 — F8: decode-budget ưu tiên hoá + báo exhausted  ⏱ ~1h30

**Mục tiêu:** 40 decoy base64-alphabet đứng trước không còn che được payload encoded đứng sau; khi budget thật sự cạn, agent được BIẾT scan chưa phủ hết thay vì nhận kết quả "sạch" im lặng.

**Evidence:** `sanitize.rs:245` (`MAX_TOTAL_DECODE_ATTEMPTS = 40`), `sanitize.rs:268-287` (FIFO, trừ budget bất kể decode kết quả).

**Thiết kế:**
1. Tách 2 budget: `MAX_DECODE_TRIES = 400` (mọi attempt, bound CPU thô — decode fail rẻ) và `MAX_SUCCESSFUL_DECODES = 40` (chỉ trừ khi `try_decode_to_text` trả non-empty — thứ đắt là re-scan + đệ quy).
2. Ưu tiên: thu thập candidates trước (`find_iter().collect()`), sort theo **độ dài giảm dần** trước khi decode — payload lệnh tiếng người encode ra thường dài hơn SHA/hash 40-64 char; decoy ngắn không còn chặn đầu hàng.
3. `detect_injection_patterns` giữ chữ ký cũ (wrapper); thêm:
   ```rust
   pub struct InjectionScan { pub hits: Vec<&'static str>, pub decode_scan_exhausted: bool }
   pub fn detect_injection_patterns_ext(code: &str) -> InjectionScan
   ```
   `exhausted = true` khi một trong hai budget chạm 0 mà vẫn còn candidate chưa thử.
4. `scan_text` (`tools/security.rs`): output thêm field
   ```rust
   /// true = còn decode-candidate chưa được quét vì chạm budget — một kết
   /// quả "sạch" kèm cờ này KHÔNG phải kết luận sạch; chia nhỏ text và quét lại.
   #[serde(skip_serializing_if = "std::ops::Not::not")]  // chỉ hiện khi true
   pub(crate) decode_scan_exhausted: bool,
   ```
   `source`/`understand` content_warning: giữ nguyên (message-only), không đổi.

**Tests:**
1. `payload_after_200_decoys_still_detected` — 200 chuỗi 40-char hex-like + 1 payload base64("ignore previous instructions") ở CUỐI → hits chứa `IGNORE_PRIOR_INSTRUCTIONS` (fail trên code hiện tại — viết TRƯỚC khi fix, đúng red-green).
2. `exhausted_flag_set_when_budget_hit` — input vượt cả 2 budget → `decode_scan_exhausted == true` qua `scan_text`.
3. Test bound cũ (`test_decode_budget_bounds_many_candidates_without_hanging`) giữ nguyên và vẫn phải trả nhanh (<~1s) — priority-sort không được phá CPU bound.

**Done when:** 3 test xanh; test sanitize cũ nguyên trạng.

---

## Item 2.6 — H4: `diff_impact` bỏ vòng HashMap→JSON→struct  ⏱ ~1h

**Mục tiêu:** dựng `AffectedSymbolOutput` trực tiếp thay vì `HashMap<String, serde_json::Value>` rồi serialize→deserialize ngược (vừa tốn vừa giấu lỗi field-name bằng `filter_map(...ok())` — một typo key làm symbol biến mất im lặng khỏi output).

**Evidence:** `guardrails.rs:482-528` (build map + round-trip), phối hợp `calm_core::analysis::diff_impact::{compute_aggregate_risk, sort_affected_symbols}` đang nhận `&[HashMap<...>]`.

**Thiết kế:**
1. **Đọc `crates/calm-core/src/analysis/diff_impact.rs` trước khi sửa** (chưa audit chi tiết file này) — xác định 2 hàm trên cần gì: dự kiến chỉ cần `(risk_level: &str, direct_callers: i64, signature_changed: bool)` per symbol.
2. Refactor 2 hàm core sang nhận slice của struct nhẹ (định nghĩa trong core):
   ```rust
   pub struct AffectedSymbolFacts { pub risk_level: RiskTier, pub direct_callers: i64, /* … đúng những gì 2 hàm dùng */ }
   ```
   hoặc generic qua accessor closures — chọn phương án ít xâm lấn sau khi đọc.
3. Server build `Vec<AffectedSymbolOutput>` thẳng trong vòng lặp (guardrails.rs:417-502), map sang facts cho core fns.
4. Không đổi wire format — snapshot test: serialize output trước/sau trên cùng fixture diff phải byte-identical (trừ thứ tự field nếu serde_json map ordering đổi — so bằng `serde_json::Value` equality, không so string).

**Done when:** snapshot equality test xanh; không còn `serde_json::to_value(m)` round-trip trong guardrails.rs.

---

## Bảng baseline & kết quả đo (điền khi thực thi)

Đo bằng `grep tool_execution_completed` trong `.calm/daemon.log` của chính daemon phục vụ session. "Baseline" đo trên daemon **debug** (build tại thời điểm bắt đầu Plan 2, trước bất kỳ thay đổi nào); "Final" đo trên daemon **release** build lại từ HEAD sau khi cả 6 item merge — nên phần chênh lệch bao gồm cả debug→release, không thuần là tác dụng của Plan 2. Cột "Final (warm)" mới là phép so sánh táo-với-táo thật sự: gọi lại **đúng cùng** `edit_context` lần thứ hai ngay sau lần đầu trên cùng symbol, đo tác dụng thực của cache co-change + config (audit F11b/F12) — đây là kịch bản "sửa file A, edit_context lại A" mà edit_context vốn được thiết kế để phục vụ lặp lại.

| Kịch bản | Baseline (debug, trước Plan 2) | Final (release, sau Plan 2 — cold) | Final (release — warm, gọi lại) | Ghi chú |
|---|---|---|---|---|
| `repo_overview` | 8ms | 8ms | — | không đổi nhiều — tool này không phải trọng tâm Plan 2 |
| `locate("edit_context")` | 16ms | 5ms | — | ~3.2× — hưởng lợi từ F13 (sanitize snippet rẻ hơn) |
| `source("edit_lines_impl", metadata=true)` | 42ms | 27ms | — | ~1.6× — F13 trên file lớn (901+ dòng, đã tăng thêm sau các sweep) |
| `edit_context("make_read_conn")` (29 callers, trải trên ~25 file khác nhau) | 131ms | 107ms | **10ms** | Cold: cải thiện khiêm tốn vì 29 caller trải rộng nhiều file (preview-batching ít trùng file để gộp trên chính symbol này). Warm (gọi lại lần 2, cache co-change 60s TTL + config cache còn hiệu lực): **10.7×** — đúng kịch bản edit_context lặp lại mà audit F11b nhắm tới |
| `edit_symbol` sửa 1 dòng (không hub) | _chưa đo_ | _chưa đo_ | | phần reindex đo ở Plan 3 — không thuộc phạm vi Plan 2 |
| `diff_impact(staged)` unstaged rỗng | 4ms | 5ms | — | nhiễu đo lường ở mức này (0 file thay đổi cả hai lần) |

## Checklist kết thúc Plan 2

- [x] 6 item = 8 commit, đều qua `diff_impact` gate — chưa push (chờ user xác nhận)
  - 2.2 (F13): `b228b58`
  - 2.5 (F8): `2d36552`
  - 2.1(a) (F4 lock-poison): `15e24d9`
  - 2.6 (H4): `d9b7cb2`
  - 2.1(b) (F4 prepare/query sweep): `509b8a7`
  - 2.3 (F12): `3c98d15`
  - 2.4(a) (F11 preview batching): `7814dc1`
  - 2.4(b) (F11 co-change cache): `66a5e66`
- [x] Bảng đo điền đủ cột (baseline debug + final release cold/warm) — `edit_symbol` sửa 1 dòng để ngỏ cho Plan 3 (đúng phạm vi tài liệu gốc)
- [x] `pattern_debt_register` cho class `unwrap-in-handler` (anchor `edit_lines_impl`, sau 2.1) và `per-row-file-read` (anchor `line_previews_batched`, sau 2.4) — `pattern_debt_status` sẽ re-check phiên sau
- [x] Rebuild release + restart daemon (SIGTERM debug daemon cũ → self-heal spawn bản release đúng HEAD, không dirty) + dogfood trực tiếp trên chính CALM (toàn bộ các lệnh đo ở trên chạy qua MCP thật, không phải test giả lập)
- [x] Cập nhật audit report: đánh dấu F4/F8/F11/F12/F13/H4 = RESOLVED kèm commit hash (xem `docs/audit/2026-07-12-vheatm-deep-audit.md`)
