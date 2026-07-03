//! Stable cache key for a SCIP overlay pass, so we skip re-running rust-analyzer
//! when nothing that affects the index changed.

/// FNV-1a over (RA version, Cargo.lock hash, sorted dirty files). Reuses the
/// same stable hash as the indexer's file hashing (pipeline::hash_content).
pub fn overlay_cache_key(ra_version: &str, lockfile_hash: &str, dirty: &[String]) -> String {
    let mut sorted: Vec<&str> = dirty.iter().map(String::as_str).collect();
    sorted.sort_unstable();
    let material = format!("{ra_version}|{lockfile_hash}|{}", sorted.join(","));
    crate::indexer::pipeline::hash_content(&material)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_changes_with_lockfile() {
        let a = overlay_cache_key("1.96.0", "hashAAA", &["src/x.rs".into()]);
        let b = overlay_cache_key("1.96.0", "hashBBB", &["src/x.rs".into()]);
        assert_ne!(a, b);
    }
}
