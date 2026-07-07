# Resolution — Multi-language Call-Graph Tier Baseline

Đo tier distribution (`formal`/`resolved`/`inferred`/`textual`/`ambiguous`/`unresolved`) của
`calm` trên **8 repo OSS thật, bên ngoài self-repo**, cho 8 ngôn ngữ mục tiêu của
[`docs/superskills/plans/2026-07-07-eight-lang-formal-tier.md`](../../docs/superskills/plans/2026-07-07-eight-lang-formal-tier.md).

**Khác B2** ([`../b2_call_graph_quality/`](../b2_call_graph_quality/)): B2 đo precision/recall
Rust so với oracle `rust-analyzer scip`, self-repo only. Benchmark này **không có oracle** — chưa
ngôn ngữ nào trong 8 ngôn ngữ này có SCIP provider (Phase 2 của kế hoạch chưa làm) — nên chỉ đo
**phân bố tier** để trả lời một câu hỏi cụ thể trước khi đổ effort vào Phase 2: heuristic Phase 0/1
(same-dir tier, type_map, PSR-4, stack-graphs JS, …) đã kéo được bao nhiêu, và ngôn ngữ nào còn
khoảng trống lớn nhất đáng ưu tiên.

## Corpus

Repo nhỏ, pin theo commit SHA thật resolve lúc clone (không phải tag cứng — xem `results.json`
mỗi lần chạy để biết chính xác commit nào được đo):

| lang | repo | lý do chọn |
|---|---|---|
| go | gin-gonic/gin | nhỏ, real-world web framework |
| java | spring-projects/spring-petclinic | nhẹ (theo gợi ý gốc của kế hoạch, thay vì guava) |
| csharp | dotnet-architecture/eShopOnWeb | mẫu kiến trúc .NET thật |
| c | redis/redis | C lớn, thật — **xem phát hiện quan trọng bên dưới** |
| cpp | fmtlib/fmt | header-only, nhiều overload — stress-test tier `ambiguous` |
| js | expressjs/express | nhỏ, thật |
| php | monicahq/monica | app PHP/Laravel thật, có composer.json cho PSR-4 |
| sql | jOOQ/sakila | mirror đa dialect (postgres/mysql/sql-server/…) của sample DB sakila |

## Chạy

```bash
cargo build --release -p calm-cli   # không cần --features scip-overlay — Phase 2 provider
                                     # chưa tồn tại cho ngôn ngữ nào trong 8 cái này
benchmarks/.venv/bin/python benchmarks/resolution/run_benchmark.py
```

Corpus clone (shallow, `--depth 1`) vào `corpus/<lang>/` lần đầu, tái sử dụng lần sau (gitignored —
xem `.gitignore`'s `benchmarks/resolution/corpus/`). `--fresh-clone` để bắt buộc clone lại,
`--lang go,java` để chạy subset.

## 🐛 Phát hiện quan trọng nhất: bug crash thật, tìm thấy nhờ đo trên repo thật

Lần chạy đầu tiên **crash hoàn toàn** khi index redis (`UNIQUE constraint failed:
symbols.qualified_name`) — không sinh ra một số nào cho C. Root cause xác nhận bằng bisection nhị
phân (cắt dần file/dòng) + debug print tạm thời trên chính `server.h` (~4700 dòng, header trung tâm
của redis):

- C's symbol extractor coi mỗi lần **nhắc tên** một `struct` forward-declare làm tham số con trỏ
  (vd `struct redisObject *key` trong 1 function-pointer typedef) là một "symbol" occurrence riêng
  — không chỉ tại nơi struct đó thật sự được định nghĩa.
- `server.h` có hàng chục typedef kiểu `moduleType*Func` đều nhận `struct redisObject *` làm tham
  số → hàng chục "symbol" tên `redisObject` dồn vào 1 file.
- Dedup nội bộ (`extract_file_data`, `pipeline.rs`) chỉ thử **một lần** hậu tố `#{line_start}` khi
  trùng tên — đủ cho 2 bản trùng, nhưng **không đủ khi 3+ bản trùng đúng cùng (tên, dòng)** — trường
  hợp thật xảy ra ở dòng có **2 tham số cùng kiểu** trên cùng 1 dòng (vd
  `typedef void *(*moduleTypeCopyFunc)(struct redisObject *fromkey, struct redisObject *tokey, ...)`).
  Lần thử hậu tố thứ hai đụng lại chính hậu tố đã dùng ở lần đầu → INSERT lỗi UNIQUE constraint
  không được xử lý → **crash toàn bộ lần index**, không riêng gì 1 file.

**Đã sửa** (`crates/calm-core/src/indexer/pipeline.rs`, hàm `extract_file_data`): vòng lặp dedup
giờ thử hậu tố tăng dần (`#{line}`, `#{line}#2`, `#{line}#3`, …) cho đến khi thật sự unique, thay vì
thử đúng 1 lần. Test hồi quy `test_c_same_line_triple_name_collision_does_not_crash_indexing`
(`pipeline.rs`) tái tạo tối thiểu chính xác pattern này (5 typedef, dòng cuối nhắc `struct Foo` 2
lần) — xác nhận **không sửa fix thì crash y hệt**, có fix thì cả 6 occurrence đều thành symbol
row riêng biệt, không crash. Toàn bộ workspace 527 test xanh, clippy `-D warnings` sạch, fmt sạch
sau fix.

**Không sửa trong lượt này** (cố ý, ngoài phạm vi benchmark): nguyên nhân gốc (root cause đầu tiên
— coi type reference là symbol) vẫn còn — nghĩa là con số `symbols_total`/`ambiguous` cho C/C++
trong bảng dưới đây **bị nhiễu bởi noise thật** (nhiều "symbol" giả từ tham chiếu kiểu, không phải
định nghĩa thật). Đây là lý do `formal_pct`/`resolved_pct` không nên đọc tuyệt đối cho C/C++ ở bản
đo này — xem "Giới hạn" bên dưới.

## Kết quả đo lần đầu (2026-07-07, sau fix crash ở trên)

| lang | symbols | edges | resolved% | inferred% | textual% | ambiguous% | wall(s) |
|---|---:|---:|---:|---:|---:|---:|---:|
| go | 1,533 | 7,672 | 15.0% | 10.5% | 20.3% | **54.3%** | 5.6 |
| java | 227 | 254 | 16.9% | 10.2% | 44.1% | 28.7% | 5.4 |
| csharp | 596 | 318 | 40.9% | 23.0% | 16.0% | 20.1% | 5.4 |
| c | 11,238 | 40,573 | 37.1% | 0.0% | 51.5% | 11.5% | 9.2 |
| cpp | 5,052 | 51,399 | 4.8% | 0.2% | 2.5% | **92.5%** | 6.7 |
| js | 123 | 36 | 30.6% | 0.0% | 2.8% | 66.7% | 8.2 |
| php | 6,503 | 9,334 | 36.4% | 13.0% | 15.6% | 34.9% | 7.5 |
| sql | 0 | 0 | — | — | — | — | 5.5 |

`formal_pct` = 0.0% cho **mọi** ngôn ngữ — đúng như kỳ vọng, không phải bug: chưa ngôn ngữ nào có
Phase 2 SCIP provider. `overlay_match_rate` = `null` cho mọi dòng (không phải `0.0`) — có chủ đích,
xem docstring của `run_benchmark.py`.

## Diễn giải

- **`ambiguous` là trần chính, không phải `textual`** — đây là phát hiện quan trọng nhất, và khác
  với trực giác ban đầu (kế hoạch gốc coi `textual` là tier "chưa resolve được" chính). Trên
  fmt (cpp), 92.5% edge rơi vào `ambiguous`; go 54.3%; js 66.7%. Đây chính xác là cái mà
  `MAX_CALLEE_CANDIDATES=20` fan-out (kiến trúc thật đã xác minh ở plan §1.2) sinh ra: hàm/method
  tên phổ biến (`format`, `write`, `get`, …) trùng lặp khắp repo → resolver từ chối chọn bừa, gắn
  nhãn `ambiguous` thay vì đoán sai. Đây CHÍNH LÀ khoảng trống Phase 2 (SCIP overlay: match theo
  (file, line) chính xác, không quan tâm bao nhiêu candidate cùng tên) nhắm tới lấp — con số này là
  baseline "trước" để so khi Go/Java/C# provider (Phase 2) hạ cánh.
- **C++ (fmt) là ca cực đoan nhất, một phần do overload thật, một phần do noise đã biết** — fmt là
  thư viện header-only với hàng trăm hàm/overload tên giống nhau (`format`, `print`, `join`, …) nên
  fan-out thật sự cao; NHƯNG con số 92.5% cũng bị thổi phồng bởi chính bug "struct reference = symbol"
  mô tả ở trên (chưa sửa) — hai nguyên nhân cộng dồn, chưa tách được tỷ lệ đóng góp của mỗi cái ở bản
  đo này.
- **C# có `resolved%` cao nhất (40.9%)** — hợp lý: P1.5's type_map/ctor-inference mới thêm (commit
  `d7178b9`) hoạt động đúng thiết kế, và C# không nằm trong same-dir tier V1 (P1.3) nên toàn bộ đóng
  góp resolved ở đây đến từ type_map thật, không phải proxy thư mục.
- **`inferred%` = 0.0% cho C** — hợp lý, không phải bug: C không có method-call cú pháp
  receiver-kiểu (`obj.method()`) theo nghĩa OOP, nên Tier-2 (type_map receiver inference) không có
  gì để nâng cấp; toàn bộ đóng góp không-ambiguous của C đến từ Tier-1 (`resolved`, same-file/same-dir).
- **SQL = 0 symbols, đúng là 0 thật, không phải lỗi** — P3.3 (SQL support) chưa triển khai;
  `language_for_extension` (`lang_constants.rs`) chưa map `.sql` sang ngôn ngữ nào. Corpus sakila có
  hàng chục file `.sql` thật (đã xác nhận bằng `find`) nhưng `calm index` bỏ qua hoàn toàn — số 0 này
  là bằng chứng trực tiếp, đo được, cho "trần" SQL hiện tại, đúng tinh thần "không che số xấu" của
  `benchmarks/README.md`.

## Giới hạn

- **Không có oracle** — khác B2, đây không đo đúng/sai (precision/recall), chỉ đo phân bố tier.
  Không suy luận "resolved% cao = tốt hơn" giữa 2 ngôn ngữ khác nhau mà không xét đặc thù ngôn ngữ đó
  (C không có method call nên không có gì để "inferred", không có nghĩa C kém hơn C#).
- **1 repo/ngôn ngữ, 1 lần đo** — không phải benchmark suite đa-repo theo quy mô (xem `docs/superskills/plans/...`'s §7 gợi ý mở rộng: gin/guava/redis/fmt/express/monica/sakila + eShopOnWeb là điểm khởi đầu, chưa phải danh sách cuối).
- **C/C++ noise đã biết, chưa tách** — xem mục bug ở trên; số `ambiguous`/`symbols_total` cho C/C++
  cao hơn thực tế một phần chưa định lượng được.
- **`.calm/config.json`'s `semantic_search.enabled=false`** — benchmark này tắt embeddings để đo
  nhanh và tránh nhiễu thời gian không liên quan; `wall_time_sec` vì vậy KHÔNG phản ánh thời gian
  `calm index` thật với embeddings bật (mặc định thật của `calm`).
- Pin theo commit SHA resolve lúc clone (ghi trong `results.json`), không phải tag release cứng —
  chạy lại sẽ luôn lấy state hiện tại vì các repo dùng shallow clone trên nhánh mặc định; muốn tái
  lập chính xác một lần đo cũ, checkout đúng SHA đã ghi.
