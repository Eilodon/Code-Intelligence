//! Data-driven description of one SCIP-capable language, so adding a new
//! provider (Go, Java, ...) means adding one `ScipProvider` value instead of
//! copying this whole `scip/` module — see the plan doc
//! `docs/superskills/plans/2026-07-07-eight-lang-formal-tier.md` §3 (P0.4).
//!
//! `RUST`, `GO` (P2.1), and `PYTHON` (P2.4) exist today. Fields sketched in
//! the plan's P0.4 design for multi-root marker-file discovery, prereq
//! gating, and refresh policy are still deliberately NOT here — neither Go's
//! nor Python's single-project case needed them (Go's `go.work`
//! multi-module handling is a documented upstream `scip-go` limitation, not
//! something this table papers over yet; scip-python indexes one `--cwd`
//! tree per invocation). Add them when a provider actually needs them.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// One row of the (currently one-row) provider table.
pub struct ScipProvider {
    /// `file_index.language` value this provider's edges are attributed to,
    /// e.g. `"rust"`. Also used to scope `source_dirty_keys` and to label
    /// log lines so a future second provider's output isn't ambiguous with
    /// this one's.
    pub lang: &'static str,
    /// Locate a usable indexer binary: explicit override first, then this
    /// language's own probe (Rust's is rustup/VS Code aware; a future
    /// simpler provider might just be a `PATH` lookup).
    pub resolve_binary: fn(Option<&str>, &Path) -> Option<PathBuf>,
    /// Build the (not-yet-spawned) command that writes a `.scip` index to
    /// `out` when run against `root` using binary `bin`.
    pub build_command: fn(bin: &Path, root: &Path, out: &Path) -> Command,
    /// Bounded wait before killing an overrunning indexer run (Java/clang
    /// providers will need a higher budget than Rust's).
    pub timeout: Duration,
    /// Full cache-key material for this language: binary version, toolchain
    /// fingerprint, manifest/lockfile hashes, and `dirty`, all folded into
    /// one opaque string. Owns its own probing so the generic pipeline
    /// never needs to know how many manifest files a given language has.
    pub cache_key: fn(bin: &Path, root: &Path, dirty: &[String]) -> String,
    /// `.calm/<this>` cache filename — kept distinct per provider so a
    /// future second language's overlay can't collide with Rust's existing
    /// `scip.cache`.
    pub cache_file_name: &'static str,
}
/// The only entry in the table today. `run_overlay`/`overlay_status` in
/// `mod.rs` are thin Rust-specific wrappers around the generic
/// `run_overlay_for`/`overlay_status_for` built against this value — see
/// those wrappers for why their signatures still take `RustConfig` directly
/// instead of `&ScipProvider` (backward compat for the 3 existing production
/// call sites, zero behavior change per P0.4's own DoD).
pub const RUST: ScipProvider = ScipProvider {
    lang: "rust",
    resolve_binary: super::runner::resolve_binary,
    build_command: super::runner::rust_build_command,
    timeout: super::runner::SCIP_TIMEOUT,
    cache_key: rust_cache_key,
    cache_file_name: "scip.cache",
};

fn rust_cache_key(bin: &Path, root: &Path, dirty: &[String]) -> String {
    super::cache::overlay_cache_key(
        &super::runner::binary_version(bin),
        &super::runner::active_toolchain_fingerprint(root),
        &lockfile_hash(root),
        &cargo_toml_hash(root),
        dirty,
    )
}

fn lockfile_hash(root: &Path) -> String {
    std::fs::read_to_string(root.join("Cargo.lock"))
        .map(|s| crate::indexer::pipeline::hash_content(&s))
        .unwrap_or_default()
}

fn cargo_toml_hash(root: &Path) -> String {
    std::fs::read_to_string(root.join("Cargo.toml"))
        .map(|s| crate::indexer::pipeline::hash_content(&s))
        .unwrap_or_default()
}

/// The 2nd entry in the table (Phase 2 / P2.1) — validates that P0.4's
/// `ScipProvider` shape actually generalizes past Rust, per the plan doc's
/// own note that widening the struct should wait for a real 2nd provider
/// instead of being guessed up front.
pub const GO: ScipProvider = ScipProvider {
    lang: "go",
    resolve_binary: super::runner::go_resolve_binary,
    build_command: super::runner::go_build_command,
    timeout: super::runner::GO_SCIP_TIMEOUT,
    cache_key: go_cache_key,
    cache_file_name: "scip-go.cache",
};

fn go_cache_key(bin: &Path, root: &Path, dirty: &[String]) -> String {
    super::cache::overlay_cache_key(
        &super::runner::binary_version(bin),
        &super::runner::go_toolchain_fingerprint(root),
        &go_sum_hash(root),
        &go_mod_hash(root),
        dirty,
    )
}

fn go_mod_hash(root: &Path) -> String {
    std::fs::read_to_string(root.join("go.mod"))
        .map(|s| crate::indexer::pipeline::hash_content(&s))
        .unwrap_or_default()
}

fn go_sum_hash(root: &Path) -> String {
    std::fs::read_to_string(root.join("go.sum"))
        .map(|s| crate::indexer::pipeline::hash_content(&s))
        .unwrap_or_default()
}

/// The 3rd entry in the table (Phase 2 / P2.4) — upgrades Python from
/// stack-graphs (archived upstream, per-file name-set matching only) to real
/// exact-(file,line) SCIP resolution. The two coexist via the provenance
/// mechanism P0.3 already built for exactly this: `ingest_occurrences` is
/// allowed to override a `formal_source = 'stack_graphs'` edge but never its
/// own prior `'scip'` verdict — no new code needed here for that part.
pub const PYTHON: ScipProvider = ScipProvider {
    lang: "python",
    resolve_binary: super::runner::python_resolve_binary,
    build_command: super::runner::python_build_command,
    timeout: super::runner::PYTHON_SCIP_TIMEOUT,
    cache_key: python_cache_key,
    cache_file_name: "scip-python.cache",
};

fn python_cache_key(bin: &Path, root: &Path, dirty: &[String]) -> String {
    super::cache::overlay_cache_key(
        &super::runner::python_binary_version(bin),
        &super::runner::python_toolchain_fingerprint(root),
        &python_requirements_hash(root),
        &python_pyproject_hash(root),
        dirty,
    )
}

/// V1 simplification (noted in the plan): only `requirements.txt` is hashed
/// as "the lockfile" — `poetry.lock`/`Pipfile.lock` aren't consulted yet.
/// Missing entirely is not an error (plenty of real Python projects have
/// neither) — an absent file just contributes an empty, stable hash input.
fn python_requirements_hash(root: &Path) -> String {
    std::fs::read_to_string(root.join("requirements.txt"))
        .map(|s| crate::indexer::pipeline::hash_content(&s))
        .unwrap_or_default()
}

fn python_pyproject_hash(root: &Path) -> String {
    std::fs::read_to_string(root.join("pyproject.toml"))
        .map(|s| crate::indexer::pipeline::hash_content(&s))
        .unwrap_or_default()
}
