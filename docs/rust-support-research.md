---
title: "Nghiên cứu: Hỗ trợ Rust top-tier cho ci"
date: 2026-07-03
status: RESEARCH — đề xuất, chưa phải quyết định (ADR sẽ chốt sau khi review)
related: ADR-0002, ADR-0004, docs/comparison.md
---

# Rust support cho `ci` — nghiên cứu có kiểm chứng thực nghiệm

> **TL;DR**: Kết luận "không có đường rẻ cho Rust — mọi hướng đều cần compiler thật + code
> build được" là **sai một nửa quan trọng**. Thực nghiệm trên máy thật (2026-07-03) chứng minh
> `rust-analyzer scip` (chế độ batch, không phải LSP server) chạy tốt trên **workspace không
> compile được** (lỗi type + lỗi syntax), tốn 4–22s và 1–2GB RAM cho repo nhỏ-vừa, output chứa
> đầy đủ resolution mà không heuristic nào với tới (method call trên receiver không annotation,
> dyn dispatch, cross-crate moniker). Điều nó cần không phải "build thành công" mà là
> "`cargo metadata` load được" — một điều kiện yếu hơn rất nhiều.
>
> Lời giải đề xuất: **2 tầng** — (1) nâng cấp tầng syntactic Rust-native (sửa 1 bug thật +
> 4 nâng cấp, zero dependency mới, luôn chạy, robust tuyệt đối) và (2) SCIP overlay từ
> `rust-analyzer scip` như tầng Formal opt-in/batch/additive-only, đúng khung ADR-0004 nhưng
> **rẻ và đơn giản hơn phương án live-LSP** mà ADR-0004 phác thảo.

---

## 1. Phương pháp

Khác với các vòng desk-research trước, vòng này **chạy thí nghiệm thật** trên máy dev:

- Tạo workspace 2-crate cố tình hỏng (lỗi type trong `core`, lỗi syntax trong `app`,
  trait + dyn dispatch, cross-crate call, method call không annotation).
- Chạy `rust-analyzer scip` (v1.96.0, 2026-05-25) trên: workspace hỏng đó, chính repo `ci`,
  và ripgrep vừa clone (cold).
- Decode SCIP protobuf bằng `scip` CLI v0.9.0, soi từng occurrence.
- Chạy `ci index` (binary release hiện tại) trên cùng workspace hỏng, dump SQLite,
  so từng edge với SCIP — đo **baseline gap thật** thay vì suy đoán.

## 2. Kết quả thực nghiệm

### 2.1 `rust-analyzer scip` KHÔNG cần code build được

Workspace 2-crate, `cargo check` **fail** (E0308 + syntax error):

| Kiểm tra | Kết quả |
|---|---|
| Exit code `rust-analyzer scip .` | **0**, index sinh ra trong **4.0s** |
| Symbol trong file có lỗi type (`type_error_fn`) | ✅ có DEF |
| Symbol là chính hàm lỗi syntax (`broken_syntax`) | ✅ vẫn có DEF |
| `let e = Engine::new(); e.start()` — receiver **không annotation** | ✅ ref → `engine/impl#[Engine]start()` |
| `r.run()` với `r: &dyn Runner` | ✅ ref → `Runner#run()` (trait method — đúng ngữ nghĩa) |
| Cross-crate: `main.rs` gọi `demo-core` | ✅ moniker đầy đủ `demo-core 0.1.0 engine/Engine#` |
| `impl Runner for FastRunner` | ✅ moniker `impl#[FastRunner][Runner]run()` — quan hệ Type–Trait mã hoá ngay trong symbol string |
| Toán tử `+`, `println!` | ✅ resolve về `core`/`std` (lọc bỏ được dễ dàng qua moniker prefix) |

Điều kiện thật sự: **`cargo metadata` phải chạy được** (mọi `Cargo.toml` well-formed, deps
resolve được — lần đầu cần network hoặc cache `~/.cargo` sẵn; repo đang được dev active thì
gần như luôn sẵn). Nó cũng chạy build script/proc-macro qua `cargo check` phía dưới (tạo
`target/`) — tắt được qua `--config-path` nếu cần chế độ zero-side-effect, đổi lấy độ phủ
proc-macro thấp hơn.

### 2.2 Chi phí đo được

| Corpus | Wall | RAM peak | Index |
|---|---|---|---|
| Workspace 2-crate hỏng | 4.0s | không đáng kể | 5.6KB |
| **Chính repo `ci`** (3 crates + deps nặng: stack-graphs, grammars, tokio) | **21.5s** | 1.9GB | 4.3MB — 58 docs, 44,141 occurrences (6,685 defs / 37,456 refs) |
| ripgrep, fresh clone (cold, fetch deps trong lúc chạy) | 20.1s | 1.1GB | 7.7MB |
| `cargo metadata --no-deps` trên `ci` | **44ms** | — | crate-name → src-root map |

So sánh đúng bản chất: đây **không phải** chi phí kiểu "LSP cold-start 30s + hang 75s mỗi
session" trong ADR-0004 — đây là **batch job chạy nền một lần, cache được theo (RA version,
Cargo.lock hash, dirty-files hash)**, không giữ process, không JSON-RPC, không lifecycle.

### 2.3 Baseline gap của `ci` hiện tại trên Rust (đo trên cùng workspace)

`ci index` hôm nay (9 symbols, 4 call edges) so với ground truth:

1. **BUG — `pub use` vô hình hoàn toàn**: `parse_rust_import` làm
   `text.strip_prefix("use ")` (`imports.rs:156`) → `"pub use engine::Engine"` trả `None`.
   Re-export façade (`lib.rs` pattern phổ biến bậc nhất Rust) biến mất khỏi graph.
2. **Cross-crate import chết**: `use demo_core::Engine` → `import_edges.to_path = NULL`.
   Không có mapping crate-name (`demo-core`→`demo_core`) → thư mục crate. Đồng thời
   `resolve_module_to_path` strip `crate/`/`super/`/`self/` rồi thử `src/…` từ **repo root**
   (`pipeline.rs:559-575`) — sai với mọi workspace nhiều crate, kể cả chính `ci`.
3. **Method call chỉ đạt `textual` khi thiếu annotation**: `let e = Engine::new(); e.start()`
   → textual (đúng target nhờ may mắn tên `start` duy nhất). Tier-2 hiện chỉ đọc
   `let x: Foo` và typed params — bỏ qua quy ước constructor `Foo::new()`/`Foo::default()`.
4. **Trait method declaration không là symbol**: `Runner::run` không tồn tại trong DB;
   field `trait` của `impl_item` bị vứt → không trả lời được "ai implement `Runner`".
   Edge `call_dynamic → FastRunner::run` được gắn `resolved` **do trùng tên may mắn**
   (bare-name `run` nằm trong `file_symbols`) — semantics thật là dispatch qua trait.
5. **Không index**: `mod` declarations (→ không có module tree), `enum_item`, `const/static`,
   `type_item`, `macro_definition`, `macro_invocation`.

## 3. Đối chiếu với báo cáo desk-research trước

| Kết luận cũ | Kiểm chứng |
|---|---|
| Scope-graphs/stack-graphs không mô hình được trait resolution (constraint solving) | ✅ Đúng, giữ nguyên. Không viết `.tsg` cho Rust — lý thuyết capped + upstream archived 09/2025. |
| Meta Glean dùng rust-analyzer SCIP mode | ✅ Đúng — và thêm: Sourcegraph `scip-rust` (v0.0.6, 05/2026) cũng chỉ là **wrapper quanh `rust-analyzer scip`**. Hội tụ industry tuyệt đối. |
| `cargo-call-stack` cần LLVM-IR + nightly + bó tay dyn | ✅ Đúng, loại. |
| "Cả LSP subprocess lẫn embed `ra_ap_*` đều không rẻ và an toàn" | ⚠️ Đúng cho **2 hướng đã xét** — nhưng bỏ sót hướng thứ ba: **batch `rust-analyzer scip`**, không phải LSP (không process sống, không cold-start mỗi session), không phải embed (không đụng API unstable, không phình binary). |
| "Cần compiler thật + code build được" | ❌ **Bác bỏ bằng thực nghiệm** — cần `cargo metadata` load được, không cần build thành công. Lỗi type/syntax trong workspace không chặn index. |
| Sợi dây `hir_def`-only cần spike thật | ✅ Spike đã chạy — và câu trả lời hay hơn kỳ vọng: không cần lát cắt `hir_def` riêng vì (a) phần "module-tree/import resolution không cần type info" **tự implement được ~vài trăm dòng** trên nền tree-sitter sẵn có (mục 4, Tầng 0), (b) phần cần type info thì batch scip đã cho với chi phí chấp nhận được. Đóng sợi dây này. |
| rustdoc JSON (chưa xét trong báo cáo) | Loại: vẫn nightly-only 2026, và không chứa reference/call-site data — chỉ có ích cho API surface, thứ tree-sitter đã làm. |

## 4. Lời giải đề xuất — 2 tầng, đúng triết lý "sống trong môi trường"

Nguyên tắc phân vai (theo yêu cầu thiết kế của `ci`): không cạnh tranh với những gì môi
trường agent đã làm tốt — diagnostics là việc của `cargo check`/IDE; go-to-def một-lần-một
là việc của LSP trong IDE. Niche của `ci` là **whole-repo call graph + risk metrics +
token-efficient context, robust trên code hỏng, chạy được cả headless CLI**. Mọi đề xuất
dưới đây chỉ phục vụ việc làm graph đó **đúng hơn**, không biến `ci` thành language server.

### Tầng 0 — Rust-native syntactic upgrade (luôn chạy, zero dependency mới)

Rust là ngôn ngữ hiếm hoi mà **import/module resolution là thuần cú pháp + quy ước** —
không dynamic như Python/JS. Phần này heuristic làm được gần-hoàn-hảo, không cần compiler:

- **R0.1 — Fix bug `pub use`** (gap #1): parse `use_declaration` bằng cấu trúc node tree-sitter
  (`visibility_modifier`, `scoped_use_list`, `use_as_clause`, `use_wildcard`) thay vì string
  split — sửa luôn nested groups `use a::{b::{c,d}, e}` mà string-split hiện bóp méo.
- **R0.2 — Workspace crate map** (gap #2): đọc `cargo metadata --no-deps` (44ms) khi có cargo;
  fallback parse TOML thủ công khi không có (giữ nguyên zero-dependency install). Map
  `package.name` (chuẩn hoá `-`→`_`) → src root (`lib.rs`/`main.rs`, `[lib] path`).
- **R0.3 — Module tree thật** (gap #5): index `mod foo;` + `#[path]` + quy ước
  `foo.rs`/`foo/mod.rs` → thay thế đường strip-prefix sai trong `resolve_module_to_path`
  bằng resolution `crate::`/`super::`/`self::` đúng theo vị trí file trong module tree.
  Đây chính là phần "DefCollector không cần type info" — fixed-point chỉ cần cho glob
  re-export, có thể bound hoặc bỏ qua glob ở v1.
- **R0.4 — Re-export chain**: sau R0.1+R0.3, follow `pub use` chains (non-glob) khi resolve
  import target — `use mylib::Engine` tìm thấy `engine::Engine` qua façade.
- **R0.5 — Trait surface** (gap #4): trait method declarations thành symbols; lưu quan hệ
  `(impl_type, trait)` từ 2 field sẵn có của `impl_item`. Mở khoá: "ai implement X",
  candidate edges cho dyn call (đúng pattern `MAX_CALLEE_CANDIDATES` sẵn có), và chấm dứt
  chuyện dyn-dispatch được gắn `resolved` nhờ trùng tên.
- **R0.6 — Constructor inference** (gap #3): `let x = Foo::new(...)`/`Foo::default()`/
  `Foo { .. }` → `type_map[x] = Foo` (tier-`inferred`, đúng contract hiện hành).
- **R0.7 (tuỳ chọn, sau)**: `enum_item`/`const`/`type_item`/`macro_definition` thành symbols;
  `macro_invocation` → edge tới `macro_rules!` cùng crate.

Trần của Tầng 0 (chấp nhận, đo và dán nhãn confidence thay vì giấu): receiver qua biểu thức
tuỳ ý / method chaining, generic bounds, code sinh bởi proc-macro.

### Tầng 1 — SCIP overlay từ `rust-analyzer scip` (Formal tier, opt-in, batch, additive-only)

Đúng 6 nguyên tắc ADR-0004 (opt-in qua config, detect-once fail-silent, additive-only,
chạy sau `ready`, tái dùng rank `Formal`, per-language decision) — nhưng thay transport:
**batch subprocess sinh file SCIP, không phải LSP client sống**. Đơn giản hơn phương án
gopls-pilot của ADR-0004 ở mọi trục vận hành (không process lifecycle, không request
budget per-call, không leak).

- **Detect**: `rust-analyzer` trên PATH → rustup component → binary bundle trong VS Code
  extension (`~/.vscode/extensions/rust-lang.rust-analyzer-*/server/`). Máy Rust dev gần như
  luôn có ≥1 đường (máy dev này có cả 3).
- **Chạy**: nền, sau `indexing_phase=ready`, timeout cứng (đề xuất 120s — đo được 22s cho
  `ci`), nice/ionice thấp. Cache theo (RA version, `Cargo.lock` hash, set file dirty).
- **Ingest**: đối chiếu SCIP occurrence `(file, line, moniker)` với call site
  `(from_path, call_line, callee_name)` sẵn có → nâng edge lên `Formal`. Def moniker →
  `(path, range)` → `qualified_name`. Moniker `impl#[Type][Trait]method()` cho quan hệ
  trait-impl formal. **Không bao giờ** tạo/xoá/hạ edge — đúng ADR-0004 §3.
- **Bonus kiến trúc**: module ingest này là **SCIP chung**, không Rust-riêng — cùng code
  đường sau nhận `scip-typescript`/`scip-java`/`scip-clang`, và cho phép mô hình Glean-style:
  CI pipeline build index artifact tập trung, agent tải về — máy yếu không phải trả 2GB RAM.
- **Rủi ro & đối phó**: (a) help text ghi rõ subcommand "no stability guarantees" → golden
  test nhỏ chạy khi detect version mới, lệch thì tắt overlay cho session, log một lần;
  (b) staleness giữa 2 lần chạy → edge mới sinh ra giữa chừng chỉ có confidence syntactic
  cho tới lần chạy sau — chấp nhận được vì additive-only nghĩa là không bao giờ *tệ hơn*
  baseline; (c) repo không phải Cargo (Bazel/Buck) → RA hỗ trợ `rust-project.json`, còn
  không có thì Tầng 0 vẫn nguyên vẹn.

### Tầng 2 — (chỉ khi đo thấy cần) live-LSP theo đúng pilot plan ADR-0004

Nếu sau khi ship Tầng 1, staleness thành vấn đề thật (đo bằng tỉ lệ query chạm edge chưa
được xác nhận), lúc đó mới xét rust-analyzer LSP sống / `ra-multiplex`. Phát hiện batch-SCIP
này nên được ghi vào ADR-0004 như một cập nhật: với Rust (và mọi ngôn ngữ có SCIP indexer
trưởng thành), **batch SCIP đi trước live LSP** trong thứ tự cân nhắc.

## 5. Benchmark khép vòng — SCIP làm oracle

Tác dụng phụ giá trị nhất của Tầng 1: SCIP output là **ground truth để đo Tầng 0**.
Chạy trên chính `ci` (dogfood) + 2-3 repo Rust thật (vd ripgrep): precision/recall của
call edges syntactic so với SCIP, trước/sau mỗi hạng mục R0.x — biến "hỗ trợ Rust tốt
chưa" thành con số theo dõi được qua `benchmarks/` harness sẵn có.

## 6. Những hướng đã xét và loại (kèm lý do một dòng)

- **Viết `.tsg` stack-graphs cho Rust**: lý thuyết capped (trait solving = constraint
  satisfaction, không phải name resolution), upstream archived, effort khổng lồ.
- **Embed `ra_ap_*`**: API không cam kết ổn định, phình binary, gánh maintenance — batch
  subprocess đạt cùng chất lượng với zero coupling.
- **`cargo-call-stack`/LLVM-IR**: cần codegen thật + nightly flags, chết với dyn dispatch.
- **rustdoc JSON**: nightly-only (xác nhận 2026), không có reference data.
- **Lát cắt `hir_def`-only tự trích**: mục đích của nó (resolution không cần build) đã đạt
  bằng đường rẻ hơn nhiều (Tầng 0 tự viết + batch scip). Đóng.

## 7. Effort ước lượng

| Hạng mục | Effort | Phụ thuộc |
|---|---|---|
| R0.1–R0.6 (syntactic) | ~2–4 ngày, thuần Rust trên codebase sẵn | không |
| Fixture workspace Rust + parity tests | ~0.5 ngày | không |
| Tầng 1 (SCIP ingest + runner + cache) | ~3–5 ngày | `scip` protobuf parse (crate `scip` hoặc prost tự sinh — vet license/size trước) |
| Benchmark oracle hoá | ~1 ngày | Tầng 1 |

## Phụ lục — lệnh tái lập thí nghiệm

```bash
# workspace hỏng: xem scratchpad session 2026-07-03; cấu trúc 2 crate,
# core/src/lib.rs chứa lỗi type E0308, app/src/main.rs chứa lỗi syntax
cargo check          # → FAIL (2 lỗi)
rust-analyzer scip . --output broken.scip   # → exit 0, ~4s
scip print --json broken.scip               # → đầy đủ defs/refs kể cả file lỗi

/usr/bin/time -v rust-analyzer scip /path/to/ci --output ci.scip
# → 21.5s wall, 1.9GB peak, 44,141 occurrences

cargo metadata --no-deps --format-version 1   # → 44ms, crate→src-root map
```

Nguồn ngoài: [rust-analyzer SCIP CLI](https://rust-lang.github.io/rust-analyzer/rust_analyzer/cli/scip/index.html) ·
[sourcegraph/scip-rust — wrapper quanh `rust-analyzer scip`](https://github.com/sourcegraph/scip-rust) ·
[Glean lsif-rust](https://glean.software/docs/indexer/lsif-rust/) ·
[RA persistent cache #4712 (vẫn mở)](https://github.com/rust-lang/rust-analyzer/issues/4712) ·
[Port RA sang salsa 3.0](https://hackmd.io/@salsa/B19OUlA71l) ·
[rustdoc JSON RFC 2963](https://rust-lang.github.io/rfcs/2963-rustdoc-json.html) ·
[Rust project goals 04/2026](https://blog.rust-lang.org/2026/05/18/project-goals-2026-04/) ·
[Trait solving — rustc-dev-guide](https://rustc-dev-guide.rust-lang.org/traits/resolution.html) ·
[ra-multiplex](https://github.com/pr2502/ra-multiplex)
