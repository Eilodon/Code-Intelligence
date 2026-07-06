//! Stable cache key for a SCIP overlay pass, so we skip re-running rust-analyzer
//! when nothing that affects the index changed.

/// FNV-1a over (RA version, active toolchain fingerprint, Cargo.lock hash,
/// Cargo.toml hash, sorted dirty files). Reuses the same stable hash as the
/// indexer's file hashing (pipeline::hash_content).
pub fn overlay_cache_key(
    ra_version: &str,
    toolchain_fingerprint: &str,
    lockfile_hash: &str,
    cargo_toml_hash: &str,
    dirty: &[String],
) -> String {
    let mut sorted: Vec<&str> = dirty.iter().map(String::as_str).collect();
    sorted.sort_unstable();
    let material = format!(
        "{ra_version}|{toolchain_fingerprint}|{lockfile_hash}|{cargo_toml_hash}|{}",
        sorted.join(",")
    );
    crate::indexer::pipeline::hash_content(&material)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_changes_with_lockfile() {
        let a = overlay_cache_key("1.96.0", "tc1", "hashAAA", "cargoA", &["src/x.rs".into()]);
        let b = overlay_cache_key("1.96.0", "tc1", "hashBBB", "cargoA", &["src/x.rs".into()]);
        assert_ne!(a, b);
    }

    #[test]
    fn cache_key_changes_with_toolchain_fingerprint_alone() {
        let a = overlay_cache_key("1.96.0", "tc1", "hashAAA", "cargoA", &["src/x.rs".into()]);
        let b = overlay_cache_key("1.96.0", "tc2", "hashAAA", "cargoA", &["src/x.rs".into()]);
        assert_ne!(
            a, b,
            "switching active toolchain must invalidate the cache even when the \
             rust-analyzer binary/version, lockfile, and Cargo.toml are unchanged"
        );
    }

    #[test]
    fn cache_key_changes_with_cargo_toml_hash_alone() {
        let a = overlay_cache_key("1.96.0", "tc1", "hashAAA", "cargoA", &["src/x.rs".into()]);
        let b = overlay_cache_key("1.96.0", "tc1", "hashAAA", "cargoB", &["src/x.rs".into()]);
        assert_ne!(
            a, b,
            "editing Cargo.toml (e.g. bumping `edition`) must invalidate the cache \
             even when Cargo.lock and toolchain are unchanged"
        );
    }

    /// Regression: before `dirty` was threaded into `run_overlay`, editing a
    /// Rust file's body with an unchanged `Cargo.lock`/toolchain produced the
    /// exact same key every time, so the overlay silently never re-ran for
    /// that file's new content. `dirty` (per-file content fingerprints) must
    /// change the key on its own, independent of lockfile/version.
    #[test]
    fn cache_key_changes_with_dirty_content_alone() {
        let a = overlay_cache_key(
            "1.96.0",
            "tc1",
            "hashAAA",
            "cargoA",
            &["src/x.rs@hash1".into()],
        );
        let b = overlay_cache_key(
            "1.96.0",
            "tc1",
            "hashAAA",
            "cargoA",
            &["src/x.rs@hash2".into()],
        );
        assert_ne!(
            a, b,
            "same version + lockfile but different file content must not collide"
        );
    }

    #[test]
    fn cache_key_is_order_independent_over_dirty_set() {
        let a = overlay_cache_key(
            "1.96.0",
            "tc1",
            "hashAAA",
            "cargoA",
            &["src/a.rs@h1".into(), "src/b.rs@h2".into()],
        );
        let b = overlay_cache_key(
            "1.96.0",
            "tc1",
            "hashAAA",
            "cargoA",
            &["src/b.rs@h2".into(), "src/a.rs@h1".into()],
        );
        assert_eq!(
            a, b,
            "dirty set is sorted before hashing — order must not matter"
        );
    }
}
