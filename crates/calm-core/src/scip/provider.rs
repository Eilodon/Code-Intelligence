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

/// One row of the provider table.
pub struct ScipProvider {
    /// `file_index.language` value this provider's edges are primarily
    /// attributed to, e.g. `"rust"`. Used to label log lines and to name the
    /// cache file so a second provider's output isn't ambiguous with this
    /// one's. NOT used to scope `source_dirty_keys` — see `dirty_langs`,
    /// which exists precisely because `lang` alone can't represent a
    /// provider covering more than one `file_index.language` value.
    pub lang: &'static str,
    /// `file_index.language` values whose dirty (changed-since-last-run)
    /// files should invalidate this provider's cache key. A single-language
    /// provider (Rust/Go/Python) sets this to `&[lang]`; `TYPESCRIPT` needs
    /// both `"javascript"` and `"typescript"` because one `scip-typescript
    /// index` run covers both extensions in the same project — added
    /// alongside that provider rather than guessed up front, per P0.4's own
    /// "widen when a real 2nd case proves the shape" rule.
    pub dirty_langs: &'static [&'static str],
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
    dirty_langs: &["rust"],
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
    dirty_langs: &["go"],
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
    dirty_langs: &["python"],
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

/// The 4th entry in the table (Phase 3 / P3.2) — one `scip-typescript index`
/// pass covers both `.js`/`.jsx` and `.ts`/`.tsx` (it infers a `tsconfig.json`
/// for plain-JS projects via `--infer-tsconfig` when none exists), hence
/// `dirty_langs` carrying both `file_index.language` values while `lang`
/// stays a single display string for logs/cache-file naming. Also the exit
/// ramp from the archived `stack-graphs` JS/TS formal tier (P1.1/pre-existing
/// TS support): this provider runs after the base pipeline, and `scip`
/// provenance is allowed to override a prior `stack_graphs` verdict via the
/// same P0.3 mechanism Python's provider already relies on — no new code
/// needed here for that part either.
pub const TYPESCRIPT: ScipProvider = ScipProvider {
    lang: "javascript",
    dirty_langs: &["javascript", "typescript"],
    resolve_binary: super::runner::js_resolve_binary,
    build_command: super::runner::js_build_command,
    timeout: super::runner::JS_SCIP_TIMEOUT,
    cache_key: js_cache_key,
    cache_file_name: "scip-ts.cache",
};

fn js_cache_key(bin: &Path, root: &Path, dirty: &[String]) -> String {
    super::cache::overlay_cache_key(
        &super::runner::js_binary_version(bin),
        &super::runner::js_toolchain_fingerprint(root),
        &js_lockfile_hash(root),
        &js_manifest_hash(root),
        dirty,
    )
}

fn js_manifest_hash(root: &Path) -> String {
    std::fs::read_to_string(root.join("package.json"))
        .map(|s| crate::indexer::pipeline::hash_content(&s))
        .unwrap_or_default()
}

/// First lockfile that actually exists wins (a checkout has at most one) —
/// npm/yarn/pnpm are mutually exclusive in practice, so there's no need to
/// hash all three and no ambiguity from checking in this fixed order.
fn js_lockfile_hash(root: &Path) -> String {
    for name in ["package-lock.json", "yarn.lock", "pnpm-lock.yaml"] {
        if let Ok(s) = std::fs::read_to_string(root.join(name)) {
            return crate::indexer::pipeline::hash_content(&s);
        }
    }
    String::new()
}

/// The 5th entry in the table (Phase 2 / P2.2) — unlike Go/Python/TS,
/// `scip-java` isn't a standalone binary a package manager installs; it's a
/// Maven Central artifact (`com.sourcegraph:scip-java_2.13`) meant to be run
/// via `cs bootstrap ... -o scip-java` (creating a launcher script on
/// `PATH`) or the `sourcegraph/scip-java` Docker image — confirmed by
/// actually running it that way (Maven-resolved classpath, no `coursier`
/// binary needed: `mvn dependency:build-classpath` against a throwaway pom
/// pulls the exact same jars `cs launch` would, since both just resolve
/// Maven Central). `java_resolve_binary` therefore only ever looks for a
/// `scip-java` launcher already on `PATH` — same shape as `GO`'s
/// `go_resolve_binary`, deliberately simpler than Python/TS's `npx` fallback
/// (there's no equivalently ubiquitous "run this artifact on demand" tool
/// bundled with a JDK the way `npx` ships with `npm`).
pub const JAVA: ScipProvider = ScipProvider {
    lang: "java",
    dirty_langs: &["java"],
    resolve_binary: super::runner::java_resolve_binary,
    build_command: super::runner::java_build_command,
    timeout: super::runner::JAVA_SCIP_TIMEOUT,
    cache_key: java_cache_key,
    cache_file_name: "scip-java.cache",
};

fn java_cache_key(bin: &Path, root: &Path, dirty: &[String]) -> String {
    super::cache::overlay_cache_key(
        &super::runner::binary_version(bin),
        &super::runner::java_toolchain_fingerprint(root),
        "", // no reliable Java lockfile equivalent (V1 simplification, see java_build_file_hash)
        &java_build_file_hash(root),
        dirty,
    )
}

/// First build descriptor that actually exists wins — Maven (`pom.xml`) and
/// Gradle (`build.gradle[.kts]`) are mutually exclusive build systems in
/// practice, mirroring `js_lockfile_hash`'s same reasoning. V1
/// simplification: a multi-module Maven/Gradle aggregator's child-module
/// build files aren't hashed individually (matches Python's V1 cut of only
/// hashing `requirements.txt`, not every possible dependency-pinning file).
fn java_build_file_hash(root: &Path) -> String {
    for name in ["pom.xml", "build.gradle.kts", "build.gradle"] {
        if let Ok(s) = std::fs::read_to_string(root.join(name)) {
            return crate::indexer::pipeline::hash_content(&s);
        }
    }
    String::new()
}
