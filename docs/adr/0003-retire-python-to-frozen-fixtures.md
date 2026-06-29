# ADR-0003: Retire Python to Frozen Golden Fixtures

- **Status**: Accepted
- **Date**: 2026-06-30
- **Decision makers**: ybao
- **Related**: `docs/migration-plan-v3.md` (Phase 0)

## Context

`codeindex/` (Python, ~2.5k dòng) tồn tại trong repo như một **parity oracle** — sinh golden
JSON output (`expected_*.json`) cho parity tests của Rust. Nó KHÔNG phải sản phẩm song song:
`codeindex/indexer/__init__.py` rỗng 0 dòng, không có index engine thực.

Trước ADR này, parity harness có 3 vấn đề khiến nó không reproduce được từ clean checkout:

1. Toàn bộ `crates/ci-core/tests/fixtures/synthetic_project/` (golden JSON, DB, lcov,
   CODEOWNERS) **chưa được commit** (untracked).
2. Fixture DB `.antigravity/codeindex.db` là binary blob bị `*.db` trong `.gitignore` chặn,
   và được build bởi `build_synthetic_db.py` (Python).
3. `parity_test.rs` không compile (`CoverageData` thiếu `Serialize`) → harness chưa từng chạy.

Mục tiêu (migration-plan-v3): codebase pure-Rust, Python không còn là runtime/test dependency.

## Decision

1. **Đóng băng golden output thành text fixtures committed**: `expected_*.json`, `lcov.info`,
   `.github/CODEOWNERS` được commit. Đây là output đông cứng của oracle — nguồn chân lý cho
   parity, không bao giờ regenerate trừ khi đổi contract có chủ đích.

2. **Port DB builder sang Rust**: `build_synthetic_db.py` (thuần `sqlite3`, không import
   codeindex) → hàm `build_synthetic_db()` trong `parity_test.rs`, dựng DB **in-memory** lúc
   test. Loại bỏ binary blob, rào `*.db`, và Python khỏi test path.

3. **Retire `codeindex/` → `legacy/`**: di chuyển package Python + 2 script generator
   (`generate_oracle.py`, `build_synthetic_db.py`) vào `legacy/`, đánh dấu frozen reference.
   Không có build/test/CI step nào chạy Python.

4. **Fix `CoverageData: Serialize`** để parity harness compile và chạy thật.

## Consequences

- `cargo test --workspace` xanh **không cần Python interpreter** (verified: 111 + 6 tests).
- Parity tests reproduce từ clean checkout (golden text committed, DB dựng in-memory).
- `legacy/` là provenance: cách golden files được sinh ra. Nếu cần re-baseline (đổi contract),
  chạy lại generator trong `legacy/` một cách có chủ đích — không phải vòng lặp dev thường.
- Khi Index Engine Rust hoàn tất, `legacy/` có thể xóa hẳn (git giữ history).
