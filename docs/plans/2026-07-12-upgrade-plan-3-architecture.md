# CALM Upgrade Plan 3/3 — Architecture: Incremental Graph, Ranking, Hub Tiering & Agent-UX

> Nguồn: `docs/audit/2026-07-12-vheatm-deep-audit.md` (Đợt 3 + mục Chiến lược). Series: [Plan 1](2026-07-12-upgrade-plan-1-correctness-safety.md) → [Plan 2](2026-07-12-upgrade-plan-2-performance-robustness.md) → **Plan 3 (file này)**.
>
> **Phạm vi:** F1 (incremental reindex — item giá trị nhất toàn dự án), F3 (personalization ranking), F10 (hub tiering), F15 (entry_points), và 4 hạng mục chiến lược cho agent-UX. Đây là plan duy nhất có thay đổi kiến trúc + schema. Ước lượng: 1-2 tuần, làm theo phase, **mỗi phase ship được độc lập**.
>
> **Điều kiện tiên quyết:** Plan 1 + Plan 2 merged. Đặc biệt: bảng baseline Plan 2 đã có số — Plan 3 phải chứng minh cải thiện bằng chính bảng đó mở rộng.
>
> **Rủi ro chung & phanh an toàn:** mọi thay đổi graph-related phải giữ **golden equivalence** (§3.1 Phase B) — incremental result == full-rebuild result. Sai lệch = revert, không "sửa cố". Config flag tắt được từng tính năng mới.

---

## §3.1 — F1: Incremental reindex — gỡ trần scale  ⏱ 4 phase, ship độc lập

**Bức tranh evidence (từ audit):**
- `tools/edit.rs:420-470` — reindex chạy ĐỒNG BỘ trong `edit_lock`, mỗi edit.
- `pipeline.rs:1464-1597` — `reindex_changed` walk + đọc + hash **toàn bộ repo** để tìm file đổi, dù edit path đã biết chính xác file nào đổi.
- `pipeline.rs:880` — `rebuild_graph` mở đầu bằng `DELETE FROM call_edges` (toàn bộ), re-resolve mọi call_site; đồng thời giết mọi `formal` upgrade từ SCIP/LSP overlay (chính comment tại `tools/edit.rs:444-456` xác nhận + workaround chạy lại overlay ~20s nền mỗi edit).
- `pipeline.rs:1585-1594` — `CrateMap`/`Psr4Map`/`NamespaceMap` build lại từ disk-walk mỗi lần.

### Phase A — Dirty-path reindex cho edit path  ⏱ ~1 ngày · ship riêng

**Mục tiêu:** `edit_lines`/`edit_symbol` không walk repo nữa — reindex đúng file vừa ghi.

**Thiết kế** — hàm mới trong `pipeline.rs`, tái dùng ruột `reindex_changed`:
```rust
/// Reindex đúng các rel_paths cho trước — KHÔNG walk repo. Dùng bởi edit
/// path (biết chính xác file vừa ghi) và watcher (biết path từ event).
/// File không còn tồn tại → remove_file_rows (deletion). Hash không đổi
/// → skip parse. Trả cùng ReindexSummary như reindex_changed.
pub fn reindex_paths(
    conn: &mut Connection,
    project_root: &Path,
    rel_paths: &[String],
) -> rusqlite::Result<ReindexSummary>
```
Các bước bên trong (mirror `reindex_changed_cancellable`, bỏ walk):
1. Load config + formal resolver 1 lần (như 1469-1477).
2. Per path: `exists()`? không → trong tx: `remove_file_rows`, `summary.deleted += 1`. Có → ext→lang / recognized_unparsed check (không nhận diện → skip); đọc, `hash_content`, so `file_index.hash` — trùng → skip; khác → `extract_file_data` → `remove_file_rows` + `persist_file` + `upsert_file_index` (đúng trình tự 1559-1574).
3. `!summary.is_noop()` → graph update: Phase A tạm gọi `rebuild_graph` như cũ (vẫn thắng lớn: bỏ được full-walk + full-hash); Phase B thay bằng incremental.
4. Call site: `tools/edit.rs:423` — `reindex_changed(&mut write_conn, &self.project_root)` → `reindex_paths(&mut write_conn, &self.project_root, &[path.to_string()])`.
5. Watcher (`crates/calm-server/src/watcher.rs`): **đọc file trước khi sửa** — nếu event batch đã mang path list, đổi sang `reindex_paths(batch)`; overflow/rename-mù (notify báo rescan) → fallback `reindex_changed` full. Nếu watcher hiện gom event không giữ path — để nguyên watcher ở Phase A, ghi chú lại.

**Tests:** fixture 3 file; sửa 1 file qua `reindex_paths` → symbols file đó cập nhật, `file_index` 2 file kia giữ nguyên `indexed_at` (chứng minh không re-scan); xoá file → rows biến mất; path không tồn tại + chưa từng index → no-op không lỗi.
**Done when:** edit trên repo CALM không còn đọc 195 file (thêm counter tạm/log debug xác nhận); test xanh.
**Rollback:** call site đổi 1 dòng — revert dễ.

### Phase B — Incremental graph update  ⏱ ~2-3 ngày · giá trị lớn nhất

**Mục tiêu:** edit 1 file chỉ đụng các edge liên quan tới file đó; edge của phần repo không đổi — kể cả `formal` upgrade từ SCIP/LSP — **sống nguyên**.

**Bước 0 (bắt buộc):** đọc trọn `rebuild_graph` (`pipeline.rs:557-~880`) + `refresh_caller_counts` + phần coreness/hub trước khi code — audit mới đọc phần đầu; xác nhận danh sách bước global hiện tại (dự kiến: resolve edges → insert → caller_count → coreness k-core → `update_is_hub_flags` → import_edges rebuild `pipeline.rs:1363`).

**Thiết kế** — hàm mới `incremental_graph_update(tx, changed_paths, old_names, hub_config, maps)`:
1. **Thu delta tên symbol:** `reindex_paths` (Phase A) capture `old_names` = `SELECT name FROM symbols WHERE path = ?` TRƯỚC `remove_file_rows`; sau persist lấy `new_names`; `names_delta = old ∪ new`.
2. **Xoá edge lỗi thời, có chủ đích (thay cho DELETE toàn bộ):**
   - `DELETE FROM call_edges WHERE from_path IN changed` (out-edges rebuild từ call_sites của chính file đó — đã có index `idx_call_edges_fpath`).
   - `DELETE FROM call_edges WHERE to_symbol NOT IN (SELECT qualified_name FROM symbols)` — quét dangling do symbol biến mất/rename (bounded: chỉ chạy khi `names_delta` non-empty; tối ưu thêm bằng `AND to_path IN changed` cho case thường).
3. **Re-resolve có phạm vi:** resolve lại `call_sites WHERE from_path IN changed` (toàn bộ site của file đổi) **CỘNG** `call_sites WHERE callee_name IN names_delta` (site ở file KHÁC trước đây unresolved/resolved-khác nay có thể đổi đích do tên mới xuất hiện/biến mất). Bounded theo độ trùng tên, không theo kích thước repo.
4. **Candidate maps:** resolution cần `by_name` toàn cục — Phase B chấp nhận 1 lần `SELECT name, qualified_name, path, class_context, signature, language FROM symbols` (đọc, không resolve; rẻ hơn resolve-all-sites nhiều lần). **Phase B+ (backlog):** daemon giữ `by_name` cache in-memory trong `CalmServer`, invalidate theo changed paths — ghi vào backlog, không làm ngay.
5. **Metrics:** `caller_count` — recompute chỉ cho `to_symbol` bị đụng (tập target của edges xoá + chèn); `coreness` (k-core) + `update_is_hub_flags` + import_edges của changed files — chạy global như cũ (in-memory trên 5K edges là trivial; đo trên repo lớn ở bước acceptance, nếu >100ms thì đưa vào backlog riêng, KHÔNG chặn Phase B).
6. **`ruled_out_by_scip` / formal edges:** edge của file KHÔNG đổi giữ nguyên row → giữ flag/tier — chính là cái thắng lớn so với hiện tại (hiện tại chết toàn bộ mỗi edit). Overlay nền sau edit (`tools/edit.rs:457-464`) GIỮ NGUYÊN ở Phase B (giờ nó chỉ còn phải vá edges của changed files).
7. **Phanh an toàn:**
   - Config `indexing.incremental_graph: bool` default `true`; `false` → `rebuild_graph` như cũ.
   - `changed_paths.len() > 50` (branch switch, mass refactor) → tự fallback full rebuild.
   - `calm index --full` (CLI) luôn là full rebuild — escape hatch vĩnh viễn.

**Tests — golden equivalence là vua:**
1. `incremental_equals_full_rebuild` — fixture đa file đa ngôn ngữ (tái dùng `tests/fixtures/multi_lang_workspace` nếu còn/khôi phục được): áp N=5 vòng mutation ngẫu nhiên nhỏ (sửa body, đổi tên fn, thêm file, xoá file) — sau mỗi vòng: chạy incremental trên DB A, full `rebuild_graph` trên DB B clone cùng cây → so **tập** `(from_symbol,to_symbol,call_site_line,edge_confidence,edge_kind)` + `caller_count` + `is_hub` per symbol: PHẢI bằng nhau tuyệt đối.
2. `unchanged_file_edges_survive_by_rowid` — edit file X; edge A→B (A,B ∉ X) giữ nguyên `rowid` (chứng minh không bị delete+reinsert).
3. `scip_flag_survives_edit_of_other_file` — set tay `ruled_out_by_scip=1` trên 1 edge; edit file khác → flag còn.
4. `rename_reroutes_cross_file_edge` — file A gọi `foo` ở file B; rename `foo`→`bar` trong B → edge cũ biến mất, call_site của A re-resolve (textual/unresolved hoặc sang đích mới nếu A cũng đổi).

**Done when:** golden test 5-vòng xanh ổn định (chạy 3 lần liên tiếp); đo `edit_symbol` end-to-end trên CALM repo — mục tiêu **< 150ms** phần reindex+graph (điền bảng đo).
**Rollback:** flag config về `false` — code cũ còn nguyên.

### Phase C — Đưa embedding ra khỏi edit lock  ⏱ ~half day

**Evidence:** `tools/edit.rs:428-442` — `embed_pending` + `embed_pending_chunks` chạy sync trong lock; semantic-search freshness không đáng giữ response.
**Thiết kế:** sau `tx.commit()` + drop 2 guard → `std::thread::spawn` chạy embed (mở writer riêng qua `open_writer`, busy_timeout lo va chạm). **Bước 0:** đọc `embedding.rs:315-407` xác nhận `embed_pending*` idempotent (chọn row theo trạng thái pending/NULL trong DB — nếu đúng, 2 thread trùng nhau vô hại; nếu không, thêm `static EMBED_BG: Mutex<()>` serialize). Coalesce như scip overlay (`run_all_coalesced` pattern) nếu agent edit dồn dập.
**Test:** edit → response về trước khi embed xong (inject sleep vào embedder test double); semantic search sau đó vẫn thấy nội dung mới (eventual).
**Done when:** phần embed biến khỏi latency `edit_symbol` trong bảng đo.

### Phase D — Cache resolution maps  ⏱ ~half day

**Evidence:** `pipeline.rs:1585-1594` — `CrateMap::build`/`Psr4Map::build`/`NamespaceMap::build` walk disk mỗi reindex.
**Thiết kế:** cache theo project_root, invalidate bằng mtime của manifest nguồn (bước 0: đọc 3 hàm build xác định input thật — dự kiến `Cargo.toml` set, `composer.json`, `*.csproj`); TTL fallback 60s cho trường hợp manifest mới xuất hiện. Áp cho cả `reindex_paths` lẫn full pipeline.
**Done when:** log debug xác nhận cache hit trên edit thứ 2 liên tiếp.

---

## §3.2 — F3: Personalization boost — chuyển sang normalize + bounded  ⏱ ~half day

**Evidence:** `common.rs:349-362` (`r.score += 0.15 × boost` cộng thẳng); thang điểm thật: RRF top-1 ≈ 0.048-0.17 (`search.rs:22-31`), grep/file = 1.0 hằng (`search.rs:354,569`), bm25 symbol 1-30+ (`search.rs:177,206`), semantic 0-1. Boost đè bẹp RRF, vô dụng trên bm25 — mâu thuẫn contract trong chính doc comment `common.rs:307-309`.

**Thiết kế (chọn phương án ít xâm lấn — min-max normalize):** trong `apply_personalization_boost`:
1. Normalize score của result set về [0,1]: `s_norm = (s - min) / (max - min)` (max==min → tất cả 0.5).
2. `s_final = s_norm + weight × boost` (weight giữ config `personalization_weight` 0.15), re-sort theo `s_final`, ghi `s_final` vào `r.score`.
3. **Invariant mới (ghi vào doc comment + test):** hai kết quả có khoảng cách normalize > `weight` KHÔNG BAO GIỜ bị hoán vị bởi boost — đúng lời hứa "never overrides a strong match", giờ đúng theo toán trên MỌI kind.
4. Trade-off minh bạch: `score` trả về đổi thang (thành normalized) — field vốn opaque với agent, và `personalized: true` đã được report sẵn (`locate.rs:56+`). Ghi 1 dòng vào doc field score nếu có schema comment.
5. **Phương án thay thế đã cân nhắc, để backlog:** proximity như nguồn RRF thứ 4 (rank-fusion thuần, đẹp hơn về lý thuyết nhưng đổi scoring path của cả 7 kind — làm sau nếu min-max lộ khuyết điểm thực tế).

**Tests:**
1. Property test nhỏ: sinh ngẫu nhiên 50 bộ (scores theo 4 thang: rrf/bm25/const-1.0/cosine; boosts) → assert invariant #3 với mọi cặp.
2. Regression: kịch bản audit — RRF scores [0.071(đúng nhất), …, 0.036(rank 8, được boost 1.0)] → sau boost, rank-1 GIỮ NGUYÊN (gap normalize > 0.15). Fail trên code hiện tại.
3. Test cũ trong `personalization_tests` (common.rs:1596-1709) giữ xanh (chúng test compute_proximity_boosts, không đụng).

**Done when:** 3 nhóm test xanh; dogfood: locate 1 query sau khi đã explore vài file — kết quả top không bị file "hàng xóm" chiếm chỗ vô lý.

---

## §3.3 — F10: Hub tiering — gate mạnh đúng chỗ, hết friction tràn lan  ⏱ ~1 ngày

**Evidence:** `graph/hub.rs:66-68` (bridge-hub = `callers ≥ 2 && coreness ≥ p75`), default `min_callers_bridge: 2` (`config.rs:86-93`) → thực đo trên chính CALM: 185/2797 = 9.8% hub; mỗi hub-touch đòi đủ 3 lớp gate (`tools/edit.rs:264-380`) → agent học cách né edit tools của CALM = phản tác dụng an toàn.

**Thiết kế — 2 phần, 2 commit:**

**(a) Phân loại hub_kind + siết default:**
1. Schema: `ALTER TABLE symbols ADD COLUMN hub_kind TEXT` (nullable; theo pattern migration `table_info` sẵn có trong `schema.rs:331,416`). `update_is_hub_flags` ghi `'degree' | 'bridge' | 'both' | NULL`.
2. Default mới: `min_callers_bridge: 2 → 4`; giữ `coreness_pct 75` trước (đổi 1 biến 1 lần); **bước calibrate bắt buộc:** chạy lại indexing trên chính CALM, ghi hub_pct mới vào plan — mục tiêu 3-5%; chưa đạt → nâng `coreness_pct` 75→90, đo lại.
3. `fitness_report`/`repo_overview.health_summary` thêm breakdown `hub_degree_count`/`hub_bridge_count` (additive fields).

**(b) Gate theo tier** — trong `tools/edit.rs` (`compute_touch_risk` trả thêm hub_kind mạnh nhất; `symbols_overlapping_ranges` SELECT thêm cột):
- `degree`/`both` hoặc `risk == "high"` (>10 callers): giữ nguyên 3 lớp `EDIT_CONTEXT_REQUIRED` → `CONFIRM_REQUIRED` → `REASON_NOT_GROUNDED`.
- `bridge`-only (và risk ≤ medium): chỉ `CONFIRM_REQUIRED` (bỏ ép edit_context-this-session + reason-grounding) — message lỗi nói rõ "bridge hub: confirm là đủ, nhưng edit_context vẫn được khuyến nghị".
- AGENTS.md Stage 5/6 + doc `EditLinesParams::confirm` cập nhật khớp.

**Tests:** unit cho `update_is_hub_flags` phân loại đúng 3 kind; gate test: symbol bridge-only + confirm:true không cần reason → applied; symbol degree-hub thiếu edit_context → `EDIT_CONTEXT_REQUIRED` như cũ.
**Done when:** hub_pct trên CALM 3-5%; test xanh; dogfood 5 edit thật thấy friction giảm (ghi nhận xét vào plan).
**Rollback:** default config revert được; column nullable vô hại.

---

## §3.4 — F15: entry_points hết nhiễu  ⏱ ~half day

**Evidence:** output `repo_overview` phiên audit: 20 entry đầu gồm `Config`, `HubThresholdConfig::default`, `LspConfig::default`… (struct config!) — đè mất `main`/`serve` thật; sai mental model từ call đầu tiên của mọi phiên + ~600 token nhiễu.

**Thiết kế:**
1. **Bước 0:** đọc default `entry_points` patterns trong `config.rs` + chỗ parser/pipeline set `is_entry_point` (grep `is_entry_point` trong `calm-core`) — xác định vì sao struct match (nghi: pattern match theo path/tên file `config`?, hoặc match mọi kind).
2. Fix tối thiểu chắc ăn: `is_entry_point` chỉ áp cho `kind ∈ {function, method}` — struct/class/const không bao giờ là entry point.
3. Siết pattern default về: `main`, `__main__`, `serve`, `run`, `handler`, `cli` (đối chiếu list thật trước khi sửa); match theo **tên symbol**, không theo substring path.
4. `repo_overview` sort entry_points: function tên `main` trước, rồi theo caller_count desc; cap 10 (đang 20).

**Tests:** fixture có `struct Config` + `fn main` + `fn helper` → entry_points chỉ chứa `main`. Chạy trên chính CALM: list phải gồm `calm-cli main`, `serve_stdio*` — không còn Config structs.
**Done when:** repo_overview trên CALM ra entry points đúng nghĩa.

---

## §3.5 — Hạng mục chiến lược cho agent-UX (mỗi cái 1 commit nhỏ, độc lập)

### (a) `repo_overview` compact mode  ⏱ ~2h
Param mới `compact: bool` default `false`: bỏ `workflow_guide` (client có AGENTS.md rồi), bỏ `entry_points`, `core_symbols` cap 8, `module_map` cap 10. Đo token: hiện ~2.4K → mục tiêu ≤ 1K. AGENTS.md Stage 1 ghi chú "phiên tiếp theo trong cùng repo: repo_overview(compact=true) đủ".

### (b) `suggested_next.gate` — phân biệt gate thật vs gợi ý mềm  ⏱ ~2h
Field additive `gate: Option<bool>` trên `SuggestedNext` (`common.rs:494-500`): `true` CHỈ cho 2 hint hook-enforce (edit_context→diff_impact tại `guardrails.rs:271-274`, edit_*→diff_impact tại `tools/edit.rs:541-544`); mọi hint khác không set. AGENTS.md "Follow suggested_next" cập nhật: `gate:true` = bắt buộc, còn lại = advisory. Giảm cognition load phân loại hint cho agent.

### (c) `scan_text.decode_scan_exhausted` — đã nằm trong Plan 2 §2.5 (chỉ nhắc để không làm trùng).

### (d) HMAC integrity cho `project_memory` (chống cấy note ngoài luồng)  ⏱ ~1 ngày · ưu tiên thấp nhất
Key 32B random tại `.calm/memory.key` (0600, tạo lazy); cột mới `content_mac` (nullable); `remember` ghi `HMAC-SHA256(topic ‖ content)`; `recall` verify → field `integrity: "ok" | "unverified"(note cũ, pre-feature) | "mismatch"`; `related_notes` drop `mismatch`. Cần dep `hmac`+`sha2` (kiểm tra `hash_content` đang dùng gì — nếu blake3 sẵn, dùng keyed-blake3 khỏi thêm dep). Làm CUỐI CÙNG — chỉ khi các mục trên xong.

### (e) Backlog ghi nhận (KHÔNG làm trong plan này)
- Proximity-as-RRF-source (§3.2 phương án 2) · by_name in-memory cache trong daemon (§3.1 B+) · read-conn pool (Plan 2 §2.3 — chỉ khi số đo đòi) · ONNX embedder backend sau feature flag (giữ potion default; interface `Embedder::embed_batch` đã đủ trừu tượng) · `max_tokens_hint` per-call response budget · catch_unwind middleware (Plan 2 §2.1c).

---

## Trình tự thực thi & tiêu chí nghiệm thu toàn Plan 3

**Thứ tự khuyến nghị:** §3.4 (nhỏ, độc lập, thắng nhanh) → §3.1 Phase A → C → D → B (B sau cùng trong F1 vì cần golden test hạ tầng) → §3.2 → §3.3 → §3.5 a,b → d.

**Nghiệm thu cuối (điền số thật):**
| Metric | Trước | Mục tiêu | Sau |
|---|---|---|---|
| `edit_symbol` end-to-end (CALM repo, 1 dòng, non-hub) | _ms | < 200ms | |
| Phần reindex+graph trong edit | _ms | < 150ms | |
| Formal edges sống sót sau 1 edit (đếm `edge_confidence='formal'` trước/sau) | chết 100% | sống 100% trừ changed files | |
| hub_pct trên CALM | 9.8% | 3-5% | |
| `repo_overview(compact=true)` tokens (ước bằng chars/4) | ~2.4K | ≤ 1K | |
| Golden equivalence 5-vòng | n/a | 3 lần chạy liên tiếp xanh | |

**Sau khi xong toàn series:** cập nhật `docs/audit/2026-07-12-vheatm-deep-audit.md` — đánh dấu từng finding RESOLVED/DEFERRED kèm commit hash; `remember(topic="audit-2026-07-12-outcome", ...)` tóm tắt để phiên sau `recall` được; hẹn re-audit theo Attestation (sau Plan 3 hoặc 6 tháng).