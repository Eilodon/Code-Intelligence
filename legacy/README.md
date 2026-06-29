# legacy/ — Retired Python Parity Oracle

> **FROZEN. Không phải build/test/CI dependency. Không sửa, không chạy trong vòng lặp dev.**

Xem `docs/adr/0003-retire-python-to-frozen-fixtures.md`.

## Nội dung

- `codeindex/` — package Python (~2.5k dòng) từng đóng vai **parity oracle**: schemas,
  analysis (hotspot, coverage_reader, codeowners, diff_impact), resolver, path_algo, db.
  KHÔNG phải sản phẩm song song — `indexer/__init__.py` rỗng, không có index engine thực.
- `generate_oracle.py` — sinh `expected_*.json` (golden output) cho parity tests.
- `build_synthetic_db.py` — dựng DB fixture synthetic (A→B→C). **Đã được port sang Rust**
  trong `crates/ci-core/tests/parity_test.rs` (`build_synthetic_db()`, in-memory).

## Tại sao giữ

Provenance: ghi lại cách golden fixtures (`crates/ci-core/tests/fixtures/synthetic_project/`)
được sinh ra. Phục vụ **re-baseline có chủ đích** khi contract đổi — không phải để chạy thường.

Để chạy lại (chỉ khi cần re-baseline): cần Python + đặt `legacy/` vào `sys.path`. Paths trong
`generate_oracle.py` trỏ tới layout cũ (`codeindex/` ở repo root) — cần điều chỉnh thủ công.

## Khi nào xóa

Sau khi Index Engine Rust (migration-plan-v3 Phase I) production-verified và parity được tái
xác nhận bằng dữ liệu do Rust-indexer sinh ra. Git history giữ lại toàn bộ.
