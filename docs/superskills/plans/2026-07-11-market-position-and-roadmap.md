# CALM — Market Position & Roadmap (2026-07-11)

Nghiên cứu chiến lược: xu hướng coding agent 2026, harness/loop engineering, và khoảng trống trong chính roadmap của CALM — để xác định nên đầu tư tiếp vào đâu để dẫn đầu category "code intelligence for AI agents".

Phương pháp: 3 luồng research song song (2 external qua WebSearch, 1 internal audit qua CALM's own docs), tổng hợp bởi phiên làm việc chính. Độ tin cậy của từng claim được giữ nguyên như agent gốc báo cáo — không làm phẳng "well-supported" thành "single-source" hay ngược lại.

---

## 1. Bức tranh thị trường 2026 (external, đã verify qua WebSearch)

**Đã xác nhận đa nguồn:**
- Thị trường phân cực IDE-first (Cursor, $2B ARR, Feb 2026) vs. agent-first (Cognition/Devin, $26B valuation, Devin ARR $37M→$492M YoY, nuốt luôn Windsurf sau khi thương vụ OpenAI đổ vỡ).
- **Claude Code dẫn đầu về satisfaction, không phải usage-share**: JetBrains khảo sát 4/2026 — Claude Code 46% "most-loved" vs Cursor 19%, Copilot 9% — dù Copilot vẫn dẫn về raw adoption (29%).
- **MCP đã thành hạ tầng, không còn là canh bạc**: donated cho Linux Foundation's Agentic AI Foundation (12/2025), SDK downloads ~97M/tháng (3/2026, từ ~100K cuối 2024). Registry 9,652–17,468 server tuỳ cách đếm.
- **A2A (Agent-to-Agent protocol)** — Linux Foundation + Google, v1.0 tháng 4/2026, 150+ tổ chức. Định vị bổ sung cho MCP (MCP = agent↔tool, A2A = agent↔agent), không cạnh tranh.
- **MCP có vấn đề bảo mật thật**: Wiz Research — MCP server hiện diện trong 80% cloud environment quan sát được (đầu 2026), lỗ hổng cốt lõi là auth/authz chưa được đặc tả rõ trong spec. Vụ Postmark MCP (9/2025, một bản update bị compromise âm thầm BCC email) là ví dụ cảnh báo hay được trích dẫn nhất.

**Single-source, cần thận trọng khi trích dẫn ra ngoài** (từ một blog research, chưa verify độc lập qua GitHub API):
- CodeGraph (~47.4k sao), GitNexus (~42k), Serena (~25.2k) là 3 "breakout leader" trong sub-category "code intelligence for agents". grepai claim giảm 97% input token.
- **Phát hiện đáng chú ý nhất: CALM không xuất hiện trong khảo sát 14 tool này** — đây là vấn đề visibility/distribution, không phải vấn đề kỹ thuật (xem mục 5).

**Định hướng ngành (nguồn: Anthropic's own "2026 Agentic Coding Trends Report", đọc trực tiếp — tự nhận là dự đoán của Anthropic, không phải consensus trung lập):**
- Chuyển từ single-agent sang **multi-agent orchestrator/specialist**; agent chạy dài hơi (giờ-đến-ngày, ví dụ Claude Code hoàn thành 1 feature 12.5M dòng ở Rakuten trong 7 giờ, 99.9% accuracy).
- **Con người chỉ "fully delegate" được 0–20% task dù dùng AI trong ~60% công việc** — vì delegation hiệu quả cần "active supervision, validation, judgment" cho việc quan trọng. Đây chính là lý do tồn tại của pre-edit safety gate.
- Guardrail vẫn còn khoảng trống thật trong ngành: 9/30 agent trong 1 khảo sát AI Agent Index **không có guardrail nào được document**. OpenAI Codex giờ *bắt buộc* JSON "Plan" + persona "Reviewer" trước khi sửa file/network — tức là đối thủ lớn nhất đang tự đi tới đúng mô hình CALM đã có sẵn từ đầu (hard gate trước khi edit).

---

## 2. Harness & Loop Engineering — nguyên lý hiện tại, đối chiếu với CALM

**Kết luận quan trọng nhất cho định vị chiến lược:** [Harness-Bench](https://arxiv.org/html/2605.27922v1) đo được **swing 10–20 điểm phần trăm trên SWE-Bench-style score chỉ từ thay đổi harness, giữ nguyên model** — chứng minh bằng số rằng lớp "harness/tooling" quanh model không phải là tính năng phụ, mà là alpha thật. Đây là bằng chứng khách quan mạnh nhất để CALM dùng khi định vị: CALM không phải "tiện ích thêm", CALM tác động trực tiếp đến khả năng agent hoàn thành task đúng — nhưng **CALM hiện chưa tự đo được điều này** (xem Tier 2 bên dưới).

Đối chiếu nguyên lý đã được cộng đồng đồng thuận với thiết kế CALM hiện có:

| Nguyên lý harness/loop engineering (research 2026) | CALM đã có |
|---|---|
| Loop cần termination condition + no-progress detection rõ ràng | `session_context.possibly_stuck` (10+ tool call không tiến triển) |
| Externalized state thay vì giữ sống trong context | `remember`/`recall` sống sót qua restart, tách biệt session_context |
| Tool trả về **high-signal-only** output (Anthropic's "Writing effective tools") | `source()` đọc đúng 1 symbol thay vì cả file; noise-penalty ranking |
| Sub-agent isolation để cô lập context | Chưa có tương đương — CALM không tự orchestrate sub-agent |
| Hard gate ở tool-call boundary, không phải ở reasoning layer (Checkmarx) | `edit_context`/`diff_impact` hook-enforced — đúng chính xác mô hình này |
| Compaction khi gần giới hạn context | Không áp dụng trực tiếp — CALM ngăn context phình từ đầu (targeted read) hơn là nén sau |

CALM đang **tình cờ đã đúng hướng** với phần lớn best-practice hiện tại của ngành, mà không cần thiết kế lại — điều cần làm là **đo lường và công bố** điều này một cách có bằng chứng (đúng tinh thần "proof not promises" đã có sẵn), không phải xây thêm cơ chế mới.

---

## 3. Khoảng trống nội bộ — những gì CALM đã nghĩ tới nhưng chưa làm

Từ audit trực tiếp `docs/pattern-debt-registry.yaml`, `docs/superskills/plans/`, `docs/adr/`, `benchmarks/README.md`:

**Documentation/process debt (rẻ, nên sửa ngay):**
- `docs/adr/0004-lsp-optional-confidence-upgrade.md:3` — status vẫn ghi "Proposed (draft — chờ review, chưa implement)" dù đã shipped từ lâu, được xác nhận ngay trong "Update 2026-07-10" section của chính file đó.
- `docs/adr/0002-formal-resolver-stack-graphs.md:15` — vẫn ghi "TypeScript/JavaScript/Java: Future" dù cả 3 đã ship.
- `docs/superskills/plans/2026-07-10-25-language-expansion.md` — dừng ở "Phase B done", nhưng thực tế Phase A, toàn bộ Phase C (9 ngôn ngữ), và Phase D (D.0–D.4) đều đã xong tính đến HEAD hiện tại. Đây là *lần thứ 4* agent phát hiện kiểu lệch pha "implementation đi trước, doc không theo kịp" trong phiên hôm nay (README, provider.rs, lang_constants.rs, giờ là ADR + plan doc) — đủ để coi là một **pattern có hệ thống**, không phải sự cố đơn lẻ.

**Kỹ thuật gần xong, đáng hoàn thiện (leverage cao, effort thấp-trung bình):**
- ADR-0005 daemon/forwarder: code tự nhận "no idle-timeout yet, no version-handshake enforcement yet" (`crates/calm-server/src/daemon.rs:11-12`) — đúng 2 risk-mitigation mà chính ADR yêu cầu trước khi coi là production-ready.
- Go SCIP còn giới hạn single-module (`go.work` multi-module bị hoãn có chủ đích).
- `DEBT-006` (duy nhất còn mở trong pattern-debt registry): ý tưởng dùng `ty check` làm tier `TypeChecked` bị từ chối sau POC (chỉ báo lỗi, không xác nhận resolution dương) — nhưng để lại 2 hướng chưa quyết: (a) `has_type_error` như health signal riêng, (b) live-LSP để lấy positive-resolution data thật, "chi phí khác hẳn, cần đánh giá riêng."

**Benchmark còn thiếu — đây là khoảng trống chiến lược nhất:**
`benchmarks/README.md` liệt kê 5 track vẫn ở trạng thái **Planned, chưa xây**: B1 (AST accuracy vs regex), B5 (tốc độ incremental indexing), B7 (task-correctness regression qua refactor thật), B8 (model-tier leveling — model rẻ + calm vs model đắt không có calm), B9 (scaling curve theo repo size).

**B7 và B8 chính là loại bằng chứng mà nghiên cứu harness-engineering ở mục 2 nói là quan trọng nhất** (Harness-Bench đo tác động harness lên task success, không phải lên token cost). CALM hiện có B2 (precision/recall call-graph), B11 (so găng thật với 4 MCP server đối thủ) — đều là proof mạnh, nhưng **chưa có con số nào chứng minh CALM cải thiện tỷ lệ hoàn thành task thật**, đúng loại bằng chứng thị trường đang coi là gold-standard.

**Khoảng trắng thật sự — chưa từng được cân nhắc ở đâu trong docs (không phải "đã hoãn", mà là chưa từng nghĩ tới):**
Multi-repo indexing, IDE-native (non-MCP) integration, agent-to-agent coordination, test generation, PR/code-review automation, CI-native feature — **0 mention** trong toàn bộ `docs/`. Đây là đất trống thật, không phải nợ kỹ thuật.

---

## 4. Khuyến nghị ưu tiên — 4 tier

### Tier 0 — Vệ sinh tài liệu (làm ngay, rủi ro ~0)
Sửa status ADR-0004, ADR-0002 khớp thực tế; refresh `2026-07-10-25-language-expansion.md` để phản ánh Phase A/C/D đã xong. Cân nhắc thêm 1 dòng vào quy trình release/commit (adr-commit skill đã có sẵn) để bắt buộc đối chiếu status ADR mỗi khi 1 plan/phase đóng — vì đây đã lặp lại đủ nhiều lần trong 1 ngày để coi là lỗi quy trình, không phải lỗi người.

### Tier 1 — Hoàn thiện cái đã 80% xong (leverage cao, effort thấp-trung bình)
1. Đóng 2 gap còn lại của ADR-0005 v1: idle-timeout thật, version-handshake enforcement thật.
2. Sau đó chuyển default entry point (`scripts/mcp-launcher.sh`) từ `calm serve` sang `calm connect` — biến "an toàn khi nhiều agent chạy song song trên 1 repo" từ tính năng ẩn (chỉ dogfood nội bộ) thành **tuyên bố định vị công khai**, đúng lúc thị trường đang chuyển sang multi-agent/agent-fleet (Antigravity 2.0, A2A). Đây là cầu nối rẻ nhất giữa cái CALM đã xây và xu hướng lớn nhất của ngành.
3. Go SCIP multi-module support — gap đã biết, phạm vi rõ.

### Tier 2 — Đầu tư benchmark (leverage chiến lược cao nhất theo đúng nghiên cứu harness-engineering)
Xây **B7** (task-correctness regression qua refactor thật) và **B8** (model-tier leveling). Đây là khoản đầu tư có ROI định vị cao nhất tìm được trong toàn bộ nghiên cứu: nếu B8 cho ra con số kiểu "model rẻ + CALM ≈ model đắt không có CALM" trên một tập task cụ thể, đó là claim vừa cụ thể, vừa đúng thứ thị trường đang định giá (cost-consciousness + harness-quality-as-alpha), vừa chưa có đối thủ nào trong khảo sát 14 tool công bố con số tương đương.

### Tier 3 — Đất trống thật, đặt cược chiến lược (effort cao hơn, nhưng dùng hạ tầng đã có sẵn, không cần kiến trúc mới)
- **PR/blast-radius review**: đóng gói `diff_impact` + `fitness_report` + `hotspots` thành 1 MCP Prompt "review_pr" chấm điểm rủi ro cho cả PR, không chỉ 1 symbol. Không đối thủ nào trong khảo sát làm tốt việc này, và CALM đã có sẵn mọi nguyên liệu.
- **Test-generation hint**: dùng coreness × dead-code/coverage để gợi ý "hàm nào cần test nhất" — cũng chỉ là tổ hợp lại dữ liệu đã có, không cần thu thập gì mới.
- **Nhận diện agent đồng thời như một khái niệm hạng nhất** (không cần implement full A2A): mở rộng `session_context` để agent A biết "agent B đang sửa file X" — đi trước xu hướng fleet mà không phải cam kết cả một chuẩn giao thức mới.

### Tier 4 — Visibility (rẻ nhất về kỹ thuật, có thể là đòn bẩy lớn nhất)
Toàn bộ công sức kỹ thuật ở trên vô nghĩa về mặt thị trường nếu CALM tiếp tục vắng mặt trong chính bài khảo sát liệt kê 14 đối thủ cùng category. Hành động cụ thể, rẻ: nộp CALM vào các directory/khảo sát MCP-server tương tự; viết 1 bài kỹ thuật (không phải marketing) dựa trên chính research hôm nay về harness/loop engineering + benchmark B11 — CALM có đủ bằng chứng thật để đóng góp nội dung có giá trị vào một mảng hiện đang bị content-farm SEO chiếm phần lớn (agent nghiên cứu external ghi nhận rõ điều này). Đồng thời làm nổi bật rõ hơn khía cạnh bảo mật (local-only, redaction, prompt-injection flagging) — đúng lúc Wiz Research vừa công bố MCP là bề mặt tấn công đang tăng, đây là câu chuyện "CALM đã làm đúng từ đầu" có thể kể ngay mà không cần code thêm.

---

## 5. Kế hoạch triển khai chi tiết (grounded trong code thật, cập nhật 2026-07-11 chiều)

3 agent research mới (đọc trực tiếp source, đối chiếu `git log`) đào sâu 4 tier ở mục 4 thành việc làm cụ thể. Độ tin cậy phần này **cao hơn** mục 1 (external): mọi con số/file:line dưới đây được verify trực tiếp qua code, không phải suy luận từ tài liệu.

### 5.0 Đính chính quan trọng trước khi triển khai

**Tiền đề của Tier 1 (mục 4) sai một phần.** Comment "no idle-timeout yet... no version-handshake enforcement yet" ở `crates/calm-server/src/daemon.rs:11-15` — cả 2 thứ đó **đã ship cùng ngày** (`37585b8` v1/M3 16:21, `65923c7` v1/M4 16:34, 2026-07-10), chỉ là comment không được cập nhật sau đó. Đây là lần thứ **5** trong ngày phát hiện kiểu lệch pha "implementation đi trước, doc không theo kịp" (README, `provider.rs`, `lang_constants.rs`, ADR/plan doc, giờ là chính `daemon.rs`) — càng củng cố nhận định ở mục 3 rằng đây là lỗi quy trình hệ thống. **ADR-0005's status header** ("Proposed — Deferred... Không nên implement cho tới lúc đó") giờ là dòng lỗi thời nhất trong toàn bộ doc set — cần sửa cùng đợt với Tier 0.

Việc thật còn lại của Tier 1 hẹp hơn mục 4 mô tả: chỉ còn **(a)** chuyển default launcher sang `calm connect` và **(b)** Go SCIP multi-module — cả hai đều genuinely chưa bắt đầu.

### 5.1 Thứ tự triển khai đề xuất (ưu tiên theo leverage/effort)

| # | Việc | Tier | Effort | Phụ thuộc |
|---|---|---|---|---|
| 1 | Sửa status ADR-0004, ADR-0002, ADR-0005, plan doc 25-lang, comment `daemon.rs:11-15` | 0 | < 1 giờ | **✅ Đã xong** |
| 2 | Script lint đối chiếu ADR có "Update" section vs status header | 0 | ~30 phút | **✅ Đã xong** (`scripts/check-adr-staleness.sh`) |
| 3 | `review_pr` MCP Prompt | 3a | < 1 giờ | **✅ Đã xong** |
| 4 | `test_gap_hotspots` tool mới | 3b | ~0.5 ngày | **✅ Đã xong** |
| 5 | Rust: mở rộng `Connect`/`spawn_detached_daemon` nhận `--preset`/`--db-path` | 1 | ~0.5 ngày | **✅ Đã xong**, có integration test (`calm_connect_forwards_preset_to_the_daemon_it_spawns`) |
| 6 | `mcp-launcher.sh`: default sang `calm connect` (Unix-only, có fallback) | 1 | ~0.5 ngày | **✅ Đã xong**, smoke-test live 3 kịch bản (no-args/extra-args/env opt-out) |
| 7 | Concurrent-agent awareness (`active_sessions` trong `session_context`) | 3c | ~1 ngày + review | nên làm sau #6 (daemon mode phổ biến hơn — giờ đã là default) — cần specialist-review vì đụng vùng code từng có bug production |
| 8 | Go SCIP `go.work` multi-module | 1 | ~2-3 ngày | không (research upstream scip-go trước khi chốt approach) |
| 9 | B7 (bản scripted, không cần LLM API) | 2 | ~2-3 ngày | không |
| 10 | Agent-loop harness (net-new) | 2 | ~2-4 ngày | nên làm sau #9 để validate oracle trước |
| 11 | B8 pilot (Haiku vs Opus/Sonnet, 4-6 task × 2 tier × 3-5 lần) | 2 | ~1-2 ngày + ~$20-50 chi phí API | việc #10 |
| 12 | Tier 4 (visibility) | 4 | liên tục, cần fen tự thực hiện phần công khai | không, có thể làm song song bất cứ lúc nào |

Việc #1-4 gần như miễn phí và không rủi ro — nên làm ngay, gộp chung một đợt. Việc #9 (B7 scripted) có ROI định vị cao nhất trong toàn bộ danh sách (theo đúng kết luận Harness-Bench ở mục 2) nhưng effort không nhỏ — nên xếp sau khi dọn xong nhóm rẻ.

### 5.2 Tier 0 — chi tiết (đã có sẵn text thay thế, chỉ cần áp dụng)

- **ADR-0004** dòng 3: `- **Status**: Proposed (draft — chờ review, chưa implement)` → thay bằng bản phản ánh đúng: live-LSP overlay đã ship pilot cho Rust/rust-analyzer (2026-07-10, feature `lsp-overlay`, opt-in), như chính "Update 2026-07-10" section của file đó (dòng 282) đã xác nhận.
- **ADR-0002** dòng 15-16 ("TypeScript/JavaScript/Java: Future") → thay bằng "Shipped", trỏ đúng `load_typescript`/`load_javascript`/`load_java` tại `crates/calm-core/src/resolver/formal.rs:385/425/451`.
- **ADR-0005** status header (dòng 3-7) → "Proposed — Deferred" sai hoàn toàn; M2-M5 (`d553c3f`→`ef75371`, 2026-07-10) đã ship daemon+forwarder+idle-timeout+version-handshake có test. Đổi thành "Accepted — Implemented (v1), partial: preset/db-path forwarding qua `calm connect` còn thiếu (xem #5)".
- **`docs/superskills/plans/2026-07-10-25-language-expansion.md`** dòng 3 → cập nhật Phase A/B/C(9/9)/D(D.0-D.4) đã xong, Phase E đang chạy (CI all-languages job + benchmark tier-distribution đã có, `68fb216`/`3cccd2b`).
- **`daemon.rs:11-15`** comment → xoá "no idle-timeout yet/no version-handshake enforcement yet", ghi rõ cả 2 đã ship + test nào cover.
- **Process fix**: `adr-commit` là skill **global** (`~/.claude/skills/`), sửa nó ảnh hưởng mọi project khác trên máy, và định dạng ADR của nó (`docs/superskills/adrs/`) không khớp convention thật của CALM (`docs/adr/000N-*.md`, living doc có "Update" section). Đừng sửa skill global. Thay vào đó: 1 script nhỏ local trong repo, grep `docs/adr/*.md` tìm file có section "Update" rồi cảnh báo cần soát lại status header — chạy như một bước doc-lint/CI, đúng tinh thần "feature-flag parity check" plan 25-lang đã tự đề xuất ở §1.9.

### 5.3 Tier 1 — chi tiết (việc #5+#6 đã xong 2026-07-11 chiều; #8 vẫn mở)

**(a) `mcp-launcher.sh` default sang `calm connect` — ÐÃ XONG.** 2 blocker đã xử lý đúng thứ tự:
1. `Commands::Connect` vẫn `#[cfg(unix)]` ở cấp enum-variant — không đổi (không thể/không nên fallback runtime cho non-Unix); launcher tự check `uname -s` trước khi thử `connect`.
2. `Connect` giờ nhận thêm `--preset`/`--db-path`, forward đúng qua `connect_or_spawn`/`spawn_detached_daemon` — xác minh bằng integration test thật (`calm_connect_forwards_preset_to_the_daemon_it_spawns`), không chỉ build sạch.

Launcher chỉ bật `calm connect` khi **không có extra arg nào** (tránh rủi ro pha trộn `--foo bar` dạng 2-token với positional token) và trên Unix; có `CI_MCP_LAUNCHER_NO_DAEMON=1` để opt-out. Smoke-test live 3 kịch bản (không arg → daemon files xuất hiện + JSON-RPC round-trip thật; có extra arg → fallback `calm serve`; env opt-out → fallback) đều đúng thiết kế.

**(b) Go SCIP `go.work` multi-module** — effort Medium, không Small, **vẫn mở**. Code hiện tại chạy `scip-go index --module-root <root>` đúng 1 lần (`scip/runner.rs:116-128`), không có enumerate/merge. Cần: (1) parse `go.work`'s `use` directives, (2) chạy scip-go per-module rồi merge output (rebase path) — **hoặc** xác nhận scip-go có flag `--workspace` native trước (chưa verify upstream, có thể giảm xuống Small nếu có), (3) mở rộng cache-key fingerprint cho mọi `go.mod`/`go.sum` thành viên, không chỉ root.

### 5.4 Tier 2 — chi tiết (B7/B8)

**B7 (task-correctness regression)** — xây được **gần như miễn phí ngay bây giờ**, không cần gọi LLM API:
- Tái dùng: `benchmarks/lib/naive_workflow.py` (mô phỏng arm "không calm"), `benchmarks/lib/mcp_client.py` (arm "có calm" — gọi `edit_context`/`diff_impact`/`edit_symbol` theo kịch bản cố định), cơ chế git-worktree cô lập của B11 (`README.md:33-53`), kỹ thuật oracle `grep_oracle_callers` của B11.
- Corpus: 4-6 task thật (refactor có test suite sẵn) trên repo pinned commit — theo đúng quy mô nhỏ B2/B11 đã dùng, không cần hàng trăm task.
- Oracle: **test pass/fail là chính** (deterministic, không cần LLM judge) + callsite-recall (tái dùng kỹ thuật B11). Diff-similarity chỉ dùng như tín hiệu phụ, **không** dùng LLM-judge làm cổng chính — tránh đưa thêm nhiễu từ 1 model khác vào đúng lúc đang đo model đầu tiên.

**B8 (model-tier leveling)** — cần hạ tầng **hoàn toàn mới**: không có bất kỳ agent-loop/LLM-API driver nào trong `benchmarks/` hiện tại (đã grep xác nhận 0 hit `anthropic`/`openai`/`agent_loop`). Cần xây 1 vòng lặp agent thật (Anthropic SDK Tool Runner hoặc tự viết), dùng lại `MCPClient` cho phần gọi tool. Đề xuất: Haiku 4.5 (rẻ, có calm) vs Opus 4.8/Sonnet 5 (đắt, không có calm) trên cùng task set — đúng claim "model rẻ + CALM ≈ model đắt không CALM" có ROI định vị cao nhất. Chi phí pilot nhỏ (4-6 task × 2 tier × 3-5 lần, có prompt caching): ước chừng vài chục USD, không phải hàng trăm.

**Thứ tự bắt buộc**: xây B7 bản scripted trước (validate oracle, gần như miễn phí) → xây agent-loop harness 1 lần → dùng lại harness đó cho cả B7 bản LLM-driven lẫn B8.

### 5.5 Tier 3 — chi tiết

- **(a) `review_pr`**: `diff_impact` **đã hỗ trợ sẵn** `commits="A..B"` — báo cáo gốc ở mục 4 phóng đại khoảng trống này. Chỉ cần thêm 1 MCP Prompt mới (< 1 giờ, cùng khuôn với `review_symbol`/`debug_symbol`/`onboard_area`) chỉ agent gọi `diff_impact(commits=...)` → `hotspots(top_n=5)` → `fitness_report()` rồi tổng hợp. Bản Tool riêng (fuse 3 lời gọi + tự tính 1 risk grade hợp nhất server-side) chỉ đáng làm sau nếu dữ liệu dùng thực tế cho thấy agent hay tự chain cả 3 — ước ~0.5-1 ngày khi đó.
- **(b) `test_gap_hotspots`**: tool mới, ghép lại 3 thứ đã có sẵn và đã test (`coreness` query pattern từ `repo_overview`, `compute_dead_code_confidence`, `test_files` lookup từ `build_health`) — không cần thuật toán mới. ~0.5 ngày.
- **(c) Concurrent-agent awareness**: `CalmServer::for_connection()` (`tools/common.rs:68-73`) đã **cố ý** tách riêng `session_log` cho mỗi connection trong khi share mọi field khác — đúng chỗ cần "un-privatize một phần". Thêm 1 field share mới `active_sessions: Arc<Mutex<HashMap<u64, SessionSummary>>>`, hook vào `track_file`/`track_symbol`/`mark_written` đã có sẵn, deregister tại điểm `daemon.rs:151` đã giảm `active_connections`. Code nhỏ, nhưng đụng đúng vùng từng sinh bug production thật (WAL bloat, SIGTERM hang, cross-process edit race) — cần specialist-review/vheatm audit, không chỉ "diff nhỏ = an toàn". Chỉ có ý nghĩa dưới daemon mode (stdio-only `calm serve` luôn đúng 1 connection) — nên làm sau việc #6 (launcher default) để daemon mode thực sự phổ biến trước khi đầu tư tính năng này.

### 5.6 Tier 4 — chi tiết (cần hành động của fen, mình không tự làm thay phần công khai)

Đây là phần duy nhất không phải code — và có 3 mức độ mình có thể hỗ trợ khác nhau:
1. **Nộp CALM vào directory/khảo sát MCP-server** (ví dụ nguồn khảo sát 14 tool tìm được ở mục 1) — hành động công khai/bên ngoài, mình sẽ **không tự nộp thay** fen, chỉ chuẩn bị sẵn nội dung (mô tả ngắn, link, số liệu B11) nếu fen muốn.
2. **Bài viết kỹ thuật** dựa trên chính research harness/loop-engineering + B11 hôm nay — mình có thể **draft** nội dung ngay trong repo (file .md), nhưng việc đăng công khai (blog, HN, X) cần fen tự duyệt và đăng.
3. **Security posture** (local-only, redaction, prompt-injection flagging, `scan_text` mới) — việc này làm **trực tiếp trong repo** (vd. thêm `SECURITY.md` hoặc mở rộng README) không cần hành động bên ngoài nào — an toàn để mình làm ngay nếu fen muốn, không cần chờ duyệt riêng.

Nói fen biết nếu muốn mình bắt đầu từ đâu trong 12 việc ở bảng 5.1 — mặc định mình sẽ đề xuất bắt đầu từ việc #1-4 (rẻ, không rủi ro, có thể làm ngay trong phiên này).

---

## Ghi chú về độ tin cậy

Phần lớn số liệu external ở mục 1 được từ WebSearch với 2 agent riêng biệt; bộ phân loại an toàn (claude-sonnet-5) không khả dụng để review 2 kết quả đó khi trả về — đã đọc và thấy nội dung hợp lý, có trích nguồn, tự gắn cờ rõ phần nào single-source/cần verify thêm (đặc biệt: số sao GitHub của CodeGraph/GitNexus/Serena chưa verify qua API, nên xử lý như ước lượng, không phải số chính thức khi trích dẫn ra ngoài). Phần internal audit (mục 3) trích trực tiếp từ file:line trong repo, đã đối chiếu qua CALM's own `search`/`file_overview` — độ tin cậy cao hơn.
