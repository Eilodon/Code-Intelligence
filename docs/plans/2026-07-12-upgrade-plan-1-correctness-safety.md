# CALM Upgrade Plan 1/3 — Correctness & Safety Quick Wins

> Nguồn: `docs/audit/2026-07-12-vheatm-deep-audit.md` (Đợt 1). Series: **Plan 1 (file này)** → [Plan 2: Hot-path Performance & Robustness](2026-07-12-upgrade-plan-2-performance-robustness.md) → [Plan 3: Architecture](2026-07-12-upgrade-plan-3-architecture.md).
>
> **Phạm vi:** 6 fix chính (F2, F5, F6, F7, F9, F14) + 5 hygiene (H1, H2, H3, H5, H6). Toàn bộ là sửa cục bộ, không đổi schema, không đổi API shape ngoài việc **thêm** field optional. Ước lượng: 1 buổi làm việc.
>
> **Nguyên tắc thực thi (áp dụng cho cả 3 plan):**
> 1. Mỗi item = 1 commit riêng, message dạng `fix(scope): <mô tả> (audit F<n>)`.
> 2. Trước khi sửa symbol nào: `edit_context(symbol)` (hook enforce sẵn). Trước mỗi commit: `diff_impact(staged=true)`.
> 3. Sau mỗi item: `cargo test -p calm-core -p calm-server` + `cargo clippy --all-targets -- -D warnings` phải xanh.
> 4. Kết thúc plan: rebuild release (`cargo build --release -p calm-cli`) và **restart daemon calm** để dogfood binary mới (daemon cũ vẫn giữ code cũ trong RAM).
> 5. Item nào lệch quá 2× ước lượng → dừng, ghi note vào cuối file này, chuyển item sau.

---

## Item 1.1 — F2: `diff_impact` kẹt `pending_scan` vĩnh viễn với file đã xoá  ⏱ ~45p

**Mục tiêu:** file có mặt trong diff nhưng không còn tồn tại trên disk không bao giờ được phân loại `pending_scan` (loại "sẽ tự resolve") — vì nó không bao giờ được scan nữa. Đây là bug đã quan sát trực tiếp trong phiên audit (agent bị loop `diff_impact` ↔ `indexing_status`).

**Evidence:** `crates/calm-server/src/tools/guardrails.rs:355-386` — nhánh `row_language == None` chỉ xét extension + `path_has_ignored_dir_component`, không xét file tồn tại.

**Thiết kế:**
- Thêm nhánh phân loại mới `"deleted"` vào `UnindexedFileOutput::reason`, ưu tiên cao nhất trong nhánh `None`:
  ```rust
  None => {
      let path = std::path::Path::new(&fd.path);
      if !self.project_root.join(path).exists() {
          Some("deleted")            // sẽ không bao giờ được scan — đừng gate risk
      } else if !calm_core::walk::path_has_ignored_dir_component(path) && (…như cũ…) {
          Some("pending_scan")
      } else {
          Some("out_of_scope")
      }
  }
  ```
- `"deleted"` KHÔNG được đưa vào `pending_scan_paths` (guardrails.rs:506-510 lọc theo `reason == "pending_scan"` — tự đúng, chỉ cần reason mới).
- Cập nhật doc comment của `UnindexedFileOutput::reason` (guardrails.rs:762-778): thêm mô tả `"deleted"` — *"file xuất hiện trong diff nhưng không còn trên disk (xoá/rename); giống out_of_scope là trạng thái vĩnh viễn, không gate aggregate_risk"*.
- Cập nhật `AGENTS.md` Stage 7 Signals: thêm dòng cho `reason == "deleted"`.
- **Lưu ý nuance (không mở rộng scope):** file bị xoá mà `file_index` row còn (watcher chưa kịp reindex) sẽ đi nhánh `Some(Some(_))` như cũ và vẫn liệt kê affected symbols — đó là hành vi ĐÚNG (xoá file có callers = blast radius thật). Chỉ nhánh "row đã mất + disk đã mất" đổi.

**Tests** (thêm vào test module có sẵn trong `crates/calm-server/src/tools.rs`):
1. `diff_impact_deleted_file_reports_deleted_not_pending_scan` — gọi `diff_impact` với raw `diff` tham chiếu `ghost.md` không tồn tại trên disk, không có row: expect `unindexed_files[0].reason == "deleted"`, `aggregate_risk != "unknown"`, `suggested_next` không phải `indexing_status`.
2. Control test: raw diff tham chiếu `newfile.rs` **có tồn tại** trên disk nhưng chưa index → vẫn `pending_scan` (giữ hành vi cũ).

**Done when:** 2 test mới xanh + test cũ không vỡ + chạy lại đúng kịch bản phiên audit (staged-add rồi delete 1 file .md, gọi diff_impact) ra `deleted`.
**Rollback:** revert 1 commit — reason mới là additive, không client nào phụ thuộc.

---

## Item 1.2 — F5: `atomic_write` làm mất file permissions  ⏱ ~30p

**Mục tiêu:** edit qua `edit_lines`/`edit_symbol` giữ nguyên mode của file gốc (đặc biệt executable bit của `scripts/*.sh`).

**Evidence:** `crates/calm-core/src/edit.rs:339-361` — `File::create(tmp)` + `rename()` → file mới nhận perms theo umask.

**Thiết kế** (sửa trong `atomic_write`):
```rust
pub fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
    let original_perms = std::fs::metadata(path).ok().map(|m| m.permissions());
    // … tạo tmp, write_all, sync_all như cũ …
    if let Some(perms) = original_perms {
        let _ = std::fs::set_permissions(&tmp_path, perms); // best-effort, không fail write vì perms
    }
    match std::fs::rename(&tmp_path, path) { … như cũ … }
}
```
- `metadata(path)` lấy **trước** khi ghi (target luôn tồn tại theo contract của cả 2 caller — xem doc `resolve_repo_path`). File mới không tồn tại → `None` → giữ hành vi hiện tại.
- Best-effort (`let _ =`): thiếu quyền set perms không được làm fail một write đã thành công về nội dung.
- **Bonus durability (tuỳ chọn, cùng commit):** sau `rename` thành công, mở thư mục cha và `sync_all()` trên unix (`#[cfg(unix)]`) — đảm bảo rename bền qua crash.

**Tests** (trong `edit.rs` test module, `#[cfg(unix)]`):
```rust
use std::os::unix::fs::PermissionsExt;
// tạo file 0o755 → atomic_write → assert mode & 0o777 == 0o755
```
**Done when:** test unix xanh; test round-trip cũ (`test_atomic_write_then_read_round_trip`) không đổi.

---

## Item 1.3 — F6: `diff_impact` reset gate trước khi validate  ⏱ ~30p

**Mục tiêu:** một call `diff_impact` **thất bại** (input sai, git fail, DB fail) không được xoá cờ `pending_diff_impact` — gate host-agnostic chỉ được thoả bởi một lần phân tích thành công.

**Evidence:** `crates/calm-server/src/tools/guardrails.rs:294` — `self.clear_written_files()` là statement đầu tiên, đứng trước cả check `input_count > 1` (296-304) lẫn nhánh `FEATURE_UNAVAILABLE` (317-326) và `db_error` (338-341).

**Thiết kế:** xoá dòng 294; gọi `self.clear_written_files()` ngay trước khi build `ToolOutcome::success(DiffImpactOutput { … })` (chỉ có đúng 1 điểm success ở cuối hàm — guardrails.rs:561). Không clear trên bất kỳ nhánh error nào.
- **Cân nhắc semantics:** doc comment của `clear_written_files` (common.rs:293-298) đang nói "clear vô điều kiện mỗi call, vì *attempting* là tín hiệu đủ" — quyết định audit F6 đảo lại lập luận đó: attempt-hỏng-vì-input-sai không chứng minh gì. Cập nhật doc comment đó cùng commit để không drift.
- Hook `.claude/hooks/calm-nudge.sh` (host-specific gate) vẫn reset trên mọi call `mcp__calm__diff_impact` tại PreToolUse — **đồng bộ luôn** (cùng commit): chuyển việc `save_state ... false` của nhánh `diff_impact` trong hook thành không đổi (hook không biết kết quả tool)… → thực tế hook không thể biết success/failure ở PreToolUse; ghi chú trong hook comment rằng CALM-side gate giờ nghiêm hơn hook-side, hook giữ nguyên (defense-in-depth: hook lỏng hơn, server chặt hơn — chấp nhận được, KHÔNG cần PostToolUse mới trong plan này).

**Tests:**
1. `diff_impact_error_does_not_clear_pending_gate` — `server.mark_written("a.rs")` → gọi `diff_impact` với cả `diff` + `staged=true` (INVALID_INPUT) → `server.written_files_snapshot()` vẫn chứa `"a.rs"`.
2. `diff_impact_success_clears_pending_gate` — mark_written → gọi diff_impact với raw diff hợp lệ → snapshot rỗng.

**Done when:** 2 test xanh; test hiện có về pending_diff_impact trong session_context không vỡ.

---

## Item 1.4 — F7: `recall`/`remember` không cảnh báo injection trong note  ⏱ ~45p

**Mục tiêu:** đồng nhất trust surface — mọi kênh trả text từ storage đều mang `content_warning` như `source()`.

**Evidence:** `crates/calm-server/src/tools/memory.rs:81-183` (recall — `MemoryNote` không có warning field), `memory.rs:16-69` (remember — không quét), so với `related_notes` drop note dính injection (`common.rs:1100-1102`).

**Thiết kế:**
1. `MemoryNote` thêm field:
   ```rust
   #[serde(skip_serializing_if = "Option::is_none")]
   pub(crate) content_warning: Option<String>,
   ```
   `memory_note_row` khởi tạo `None`; trong vòng `for note in &mut notes` của `recall` (memory.rs:150): `note.content_warning = calm_core::sanitize::injection_warning(&note.content);` — đặt **trước** nhánh `continue` của staleness (chạy cho mọi note, kể cả note không có refs).
2. `RememberOutput` thêm field cùng tên; trong `remember`, sau khi lưu thành công: `content_warning: calm_core::sanitize::injection_warning(content)` — **vẫn lưu note** (detection-only, nhất quán triết lý sanitize.rs:94-103), chỉ cảnh báo người ghi.
3. Cập nhật doc comment `related_notes` (common.rs:1066-1069): câu "remains fully visible via recall, where Stage-3 wariness applies" → thêm "recall now carries an explicit per-note content_warning".

**Tests:**
1. `recall_flags_injection_shaped_note` — remember note chứa `"ignore all previous instructions"` → recall → `notes[0].content_warning` chứa `IGNORE_PRIOR_INSTRUCTIONS`.
2. `remember_returns_warning_but_still_saves` — output có warning, recall thấy note tồn tại.
3. Note sạch → không có field (serde skip).

**Done when:** 3 test xanh. **Ghi chú scope:** HMAC integrity cho `project_memory` (chống ghi thẳng DB) là Plan 3 §3.5 — KHÔNG làm ở đây.

---

## Item 1.5 — F9: `resolve_symbol_candidates` nuốt lỗi DB thành "not found"  ⏱ ~1h15 (touch nhiều handler, thuần cơ học)

**Mục tiêu:** phân biệt "symbol không tồn tại" với "không đọc được DB" — không bao giờ trả `NOT_FOUND` + caveat "likely a typo" khi lỗi thật là hạ tầng.

**Evidence:** `crates/calm-server/src/tools/common.rs:961-964` (`Err(_) => return vec![]` trên prepare) và `common.rs:992-995` (`Err(_) => vec![]` trên query_map).

**Thiết kế:**
1. Đổi chữ ký:
   ```rust
   pub(crate) fn resolve_symbol_candidates(…) -> rusqlite::Result<Vec<CandidateRow>>
   pub(crate) fn resolve_symbol(…) -> rusqlite::Result<SymbolResolution>
   ```
   Bên trong: `?` thay cho 2 chỗ nuốt lỗi; per-row `filter_map(|r| r.ok())` GIỮ NGUYÊN (một row hỏng không nên giết cả result set — chỉ lỗi statement-level mới propagate).
2. Mọi call site (grep `resolve_symbol(` — dự kiến: `guardrails.rs` edit_context; `tools/edit.rs` edit_symbol; `inspect.rs` symbol_info/source/understand; `trace.rs` callers/callees/path ×2; compiler sẽ chỉ chỗ) đổi pattern:
   ```rust
   let resolution = match resolve_symbol(&conn, &p.symbol, p.path.as_deref(), p.line) {
       Ok(r) => r,
       Err(e) => return db_error_resolved(e),   // hoặc db_error(e) tuỳ envelope của tool
   };
   ```
3. Chỗ nào dùng `resolve_symbol_candidates` trực tiếp (nếu có — grep) xử lý tương tự.

**Tests:** `resolve_reports_db_error_not_not_found` — mở server với DB hợp lệ, dùng `server.db()` (test-only write conn) `DROP TABLE symbols`, gọi `symbol_info("anything")` → expect `error.code == "DB_ERROR"` (recoverable), KHÔNG phải `NOT_FOUND`.

**Done when:** test mới xanh + toàn bộ test suite xanh (đây là item dễ gây vỡ test nhất plan này — các test NotFound hiện có phải giữ nguyên hành vi vì DB của chúng lành).
**Rollback:** revert commit; không đổi wire format (vẫn ErrorDetail envelope sẵn có).

---

## Item 1.6 — F14: `REASON_NOT_GROUNDED` match substring quá lỏng  ⏱ ~45p

**Mục tiêu:** reason phải cite caller thật theo word-boundary; tên caller ngắn (`new`, `get`, `run`) không thể pass nhờ tình cờ chứa chuỗi con, và không false-deny reason chính đáng.

**Evidence:** `crates/calm-server/src/tools/edit.rs:341-349` — `reason.contains(short)` với `short = qn.rsplit("::").next()`.

**Thiết kế:** thay closure check bằng helper (đặt cạnh `best_live_range` trong cùng file):
```rust
/// true khi `reason` chứa `needle` như một token trọn vẹn — ký tự liền
/// trước/sau không thuộc [A-Za-z0-9_] (hoặc là đầu/cuối chuỗi).
fn cites_token(reason: &str, needle: &str) -> bool { /* find + boundary check, lặp mọi occurrence */ }

const MIN_BARE_NAME_LEN: usize = 4;

let cites_real_signal = if known_caller_qns.is_empty() {
    !reason.is_empty()
} else {
    known_caller_qns.iter().any(|qn| {
        let short = qn.rsplit("::").next().unwrap_or(qn);
        let last_two = { /* ghép 2 segment cuối: "Type::name" nếu có */ };
        (short.len() >= MIN_BARE_NAME_LEN && cites_token(reason, short))
            || cites_token(reason, &last_two)
            || cites_token(reason, qn)
    })
};
```
- Message lỗi `REASON_NOT_GROUNDED` (edit.rs:366-378): với caller tên ngắn, `examples` phải show dạng `Type::name` (last-two) thay vì bare name, để agent biết cần cite dạng nào.

**Tests:**
1. Caller `CalmServer::new`: reason `"checked CalmServer::new — return shape unchanged"` → pass; reason `"renewed the flow"` → **deny** (fix hành vi cũ: `contains("new")` từng pass).
2. Caller `refresh_caller_counts`: reason cite đúng tên → pass (bare name ≥4 vẫn hoạt động).
3. Boundary: reason `"xrefresh_caller_countsy"` → deny.

**Done when:** 3 test xanh + test gate hiện có (`EDIT_CONTEXT_REQUIRED`/`CONFIRM_REQUIRED` flow) không vỡ.

---

## Item 1.7 — Hygiene gộp (H1, H2, H3, H5, H6)  ⏱ ~45p, 1 commit `chore: audit hygiene batch`

| # | Việc | File | Chi tiết |
|---|---|---|---|
| H1 | Sửa doc `RecallParams::query` | `memory.rs:240-243` | "SQL LIKE, case-insensitive" → "FTS5 phrase match (bm25-ranked); input luôn được escape thành literal phrase" |
| H2 | AGENTS.md hứa risk `"critical"` cho edit_context | `AGENTS.md` Stage 5 Signals | Sửa thành `"high"` (code chỉ sinh low/med/high — `common.rs:1545-1553`); đối chiếu thêm mọi mention "critical" khác trong Stage 5 (Stage 7 aggregate_risk CÓ critical thật — giữ) |
| H3 | 3 doc refs chết trong README (fitness gate đang FAIL vì đây) | `README.md` | `.codeium/windsurf/mcp_config.json` & `.gemini/config/mcp_config.json` → sửa theo đường dẫn thật hoặc bỏ đoạn hướng dẫn client đó; `docs/legacy/architecture-design.md` → `docs/architecture.md`. **Done-check:** `fitness_report().passed == true` |
| H5 | `PRAGMA synchronous=NORMAL` cho writer | `crates/calm-core/src/db/conn.rs::open_writer` | Sau `busy_timeout`: `conn.execute_batch("PRAGMA synchronous=NORMAL;")?;` — an toàn dưới WAL cho index-DB rebuild-được; KHÔNG set cho read conn (query_only, không cần). Test: PRAGMA query trả 1 (NORMAL) |
| H6 | Ghi invariant lock-order | `common.rs::touch_active_session` (247-262) | Comment: "lock order toàn codebase: session_log TRƯỚC active_sessions — mọi hàm chạm cả hai phải giữ thứ tự này (hiện: touch_active_session). Đổi thứ tự = deadlock tiềm năng" |

---

## Checklist kết thúc Plan 1

- [ ] 7 commit (6 item + 1 hygiene batch) đều qua `diff_impact` gate, push lên branch làm việc
- [ ] `cargo test --workspace` xanh; `cargo clippy --all-targets -- -D warnings` sạch
- [ ] `fitness_report()` passed (H3 là điều kiện)
- [ ] Rebuild release + restart daemon; dogfood: chạy lại kịch bản F2 (delete file → diff_impact) và F14 (edit hub với reason xấu) trên chính repo CALM
- [ ] Cập nhật `docs/pattern-debt-registry.yaml` nếu pattern `unwrap-in-handler` được đăng ký ở Plan 2 (xem Plan 2 §2.1)
