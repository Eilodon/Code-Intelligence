//! Detect a `rust-analyzer` binary and drive its batch `scip` subcommand.
//! Detect-once, fail-silent (ADR-0004 §2): any failure returns None/Err and the
//! caller keeps the syntactic graph untouched.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::provider::ScipProvider;

/// Total wall-clock budget for one `rust-analyzer scip` pass. Measured cost on
/// the `ci` workspace itself (~44k occurrences) was 21.5s; ripgrep 20s. 120s
/// leaves generous headroom; overrun → kill and keep whatever the syntactic
/// tier already produced.
pub const SCIP_TIMEOUT: Duration = Duration::from_secs(120);

/// Resolve a usable rust-analyzer binary path. Tries, in order: an explicit
/// override, `PATH`, `rustup which`, and the VS Code extension bundle.
/// `root` scopes the `rustup which` probe to the project directory so a
/// `rust-toolchain.toml` override there is honored instead of whatever
/// toolchain happens to be active in the server process's own cwd.
pub fn resolve_binary(override_bin: Option<&str>, root: &Path) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(b) = override_bin {
        candidates.push(PathBuf::from(b));
    }
    candidates.push(PathBuf::from("rust-analyzer")); // PATH lookup via which-style probe
    if let Some(path) = rustup_which(root) {
        candidates.push(path);
    }
    if let Some(home) = dirs_home()
        && let Some(p) = newest_vscode_ra(&home)
    {
        candidates.push(p);
    }
    candidates.into_iter().find(|c| binary_runs(c))
}

/// Run `<bin> scip <root> --output <out>` under the time budget. Returns the
/// output path on success. Never propagates a panic; a non-zero exit or timeout
/// is an `Err` the caller swallows.
pub fn rust_build_command(bin: &Path, root: &Path, out: &Path) -> Command {
    let mut cmd = Command::new(bin);
    cmd.arg("scip")
        .arg(root)
        .arg("--output")
        .arg(out)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    cmd
}

/// Spawn `provider.build_command(bin, root, out)` and poll it to completion,
/// killing on overrun. Generic over every provider — this is the one place
/// that owns "run an external indexer binary, bounded", so a Java/clang
/// provider with a much larger `timeout` doesn't need its own copy of this
/// poll loop.
pub fn run_indexer(
    provider: &ScipProvider,
    bin: &Path,
    root: &Path,
    out: &Path,
) -> anyhow::Result<()> {
    let mut child = (provider.build_command)(bin, root, out).spawn()?;
    // Poll with a deadline; kill on overrun (Command has no built-in timeout —
    // same pattern as analysis/diff_impact.rs's bounded wait).
    let deadline = std::time::Instant::now() + provider.timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            if status.success() {
                return Ok(());
            }
            anyhow::bail!("{} indexer exited with {status}", provider.lang);
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            anyhow::bail!(
                "{} indexer exceeded {}s budget",
                provider.lang,
                provider.timeout.as_secs()
            );
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Total wall-clock budget for one `scip-go index` pass. Go's compiler-driven
/// analysis (`go/packages` loading + typechecking) is typically slower per
/// LOC than rust-analyzer's incremental LSP pass, so this gets a larger
/// budget than `SCIP_TIMEOUT`; still bounded so a pathological module graph
/// can't hang the watcher/CLI indefinitely.
pub const GO_SCIP_TIMEOUT: Duration = Duration::from_secs(180);

/// Resolve a usable `scip-go` binary: an explicit override, then `PATH`, then
/// `$HOME/go/bin` and `$GOBIN` (where `go install .../scip-go@latest` lands
/// when neither is already on `PATH` — the common case for a freshly
/// installed toolchain).
pub fn go_resolve_binary(override_bin: Option<&str>, _root: &Path) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(b) = override_bin {
        candidates.push(PathBuf::from(b));
    }
    candidates.push(PathBuf::from("scip-go")); // PATH lookup via which-style probe
    if let Some(gobin) = std::env::var_os("GOBIN") {
        candidates.push(PathBuf::from(gobin).join("scip-go"));
    }
    if let Some(home) = dirs_home() {
        candidates.push(home.join("go").join("bin").join("scip-go"));
    }
    candidates.into_iter().find(|c| binary_runs(c))
}

/// `scip-go index --module-root <root> --output <out> --quiet`. `module-root`
/// (rather than relying on cwd) is what lets `run_indexer`'s spawn work
/// regardless of the calling process's own working directory.
pub fn go_build_command(bin: &Path, root: &Path, out: &Path) -> Command {
    let mut cmd = Command::new(bin);
    cmd.arg("index")
        .arg("--module-root")
        .arg(root)
        .arg("--output")
        .arg(out)
        .arg("--quiet")
        .current_dir(root)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    cmd
}

/// `go version`, trimmed, or `""` if it can't be run. Part of the Go overlay
/// cache key alongside `binary_version` (the scip-go binary itself) — a
/// different active Go toolchain can change typechecking results even with
/// the same scip-go version.
pub fn go_toolchain_fingerprint(root: &Path) -> String {
    Command::new("go")
        .arg("version")
        .current_dir(root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Total wall-clock budget for one `scip-python index` pass. Pyright's
/// (the type checker scip-python wraps) full-project inference is typically
/// slower than Go's compiler-driven pass, so this gets the largest budget of
/// the three providers; still bounded so a pathological dependency graph
/// can't hang the watcher/CLI indefinitely.
pub const PYTHON_SCIP_TIMEOUT: Duration = Duration::from_secs(240);

/// Resolve a way to run `scip-python`: an explicit override, then a
/// standalone binary on `PATH` (e.g. `npm install -g @sourcegraph/scip-python`),
/// then `npx` itself as a proxy for "run the npm package on demand" — the
/// plan's own "npm package (cần node) — probe cả binary lẫn npx" requirement,
/// since most checkouts won't have it installed globally. When the returned
/// path's filename is `npx` (not a real `scip-python` binary),
/// `python_build_command`/`python_binary_version` know to prepend the
/// package name themselves.
pub fn python_resolve_binary(override_bin: Option<&str>, _root: &Path) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(b) = override_bin {
        candidates.push(PathBuf::from(b));
    }
    candidates.push(PathBuf::from("scip-python")); // PATH lookup via which-style probe
    if let Some(c) = candidates.into_iter().find(|c| binary_runs(c)) {
        return Some(c);
    }
    let npx = PathBuf::from("npx");
    npx_can_run_scip_python(&npx).then_some(npx)
}

/// Whether `npx --yes @sourcegraph/scip-python --version` succeeds — `--yes`
/// auto-confirms the on-demand download (npx would otherwise prompt
/// interactively on a first run, hanging a non-interactive agent/CI
/// process). Real network/npm-cache round trip, not a cheap check, but this
/// is exactly the "does this actually work" probe the plan calls for.
fn npx_can_run_scip_python(npx: &Path) -> bool {
    Command::new(npx)
        .args(["--yes", "@sourcegraph/scip-python", "--version"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// `scip-python index --cwd <root> --project-name <name> --project-version
/// 0.0.0 --output <out> --quiet`. `--project-name`/`--project-version` are
/// NOT optional in practice — confirmed experimentally against this exact
/// package version: omitting either one crashes indexing entirely
/// (`TypeError: Cannot read properties of undefined (reading 'indexOf')` in
/// `normalizeNameOrVersion`) whenever `root` isn't a git repository scip-python
/// can infer a version from. The actual name/version values don't matter for
/// call-graph purposes, only their presence — `--project-version` is a fixed
/// placeholder rather than trying to infer a real one.
pub fn python_build_command(bin: &Path, root: &Path, out: &Path) -> Command {
    let mut cmd = Command::new(bin);
    if is_npx(bin) {
        cmd.args(["--yes", "@sourcegraph/scip-python"]);
    }
    let project_name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");
    cmd.arg("index")
        .arg("--cwd")
        .arg(root)
        .arg("--project-name")
        .arg(project_name)
        .arg("--project-version")
        .arg("0.0.0")
        .arg("--output")
        .arg(out)
        .arg("--quiet")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    cmd
}

/// scip-python's own `--version` output, trimmed, or `""` if it can't be
/// run — routed through `npx` (same package-name prefix as
/// `python_build_command`) when `bin` is the `npx` proxy, so the cache key
/// reflects the actual indexer version, not npx's own unrelated version.
pub fn python_binary_version(bin: &Path) -> String {
    let mut cmd = Command::new(bin);
    if is_npx(bin) {
        cmd.args(["--yes", "@sourcegraph/scip-python", "--version"]);
    } else {
        cmd.arg("--version");
    }
    cmd.output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

fn is_npx(bin: &Path) -> bool {
    bin.file_name().and_then(|f| f.to_str()) == Some("npx")
}

/// `python3 --version` (falling back to `python --version`), trimmed, or
/// `""` if neither can be run. Part of the Python overlay cache key
/// alongside `python_binary_version` — a different active interpreter/
/// installed packages can change pyright's inference even with the same
/// scip-python version.
pub fn python_toolchain_fingerprint(root: &Path) -> String {
    for interpreter in ["python3", "python"] {
        if let Some(out) = Command::new(interpreter)
            .arg("--version")
            .current_dir(root)
            .output()
            .ok()
            .filter(|o| o.status.success())
        {
            return String::from_utf8_lossy(&out.stdout).trim().to_string();
        }
    }
    String::new()
}

fn binary_runs(path: &Path) -> bool {
    Command::new(path)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// `<bin> --version` output, trimmed, or `""` if it can't be run. Used as part
/// of the overlay cache key — any version change invalidates the cache.
pub fn binary_version(bin: &Path) -> String {
    Command::new(bin)
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// `rustc --version --verbose` run from `root`, trimmed, or `""` if it can't
/// be run. Used alongside `binary_version` in the overlay cache key:
/// `binary_version` fingerprints the rust-analyzer binary doing the
/// analysis, this fingerprints the toolchain/edition semantics of the
/// project being analyzed — switching active toolchain (`rustup default`,
/// `rust-toolchain.toml`) without changing which rust-analyzer binary
/// resolves must still invalidate the cache. `current_dir(root)` is what
/// makes rustup's shim resolve the project's local override instead of
/// whatever's active for the server process's own cwd.
pub fn active_toolchain_fingerprint(root: &Path) -> String {
    Command::new("rustc")
        .args(["--version", "--verbose"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// `rustup which rust-analyzer` — resolves the toolchain-managed binary when
/// it's not directly on `PATH`. Run from `root` (not the server process's own
/// cwd) and without a hardcoded `--toolchain`, so rustup's own override
/// resolution (`rust-toolchain.toml`, `RUSTUP_TOOLCHAIN`, `rustup override`)
/// picks the project's actual pinned toolchain instead of always `stable`.
/// Absent/failing `rustup` is not an error here; `binary_runs` is the real gate.
fn rustup_which(root: &Path) -> Option<PathBuf> {
    let out = Command::new("rustup")
        .args(["which", "rust-analyzer"])
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8(out.stdout).ok()?;
    let path = path.trim();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn newest_vscode_ra(home: &Path) -> Option<PathBuf> {
    let ext_dir = home.join(".vscode/extensions");
    let mut hits: Vec<PathBuf> = std::fs::read_dir(&ext_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("rust-lang.rust-analyzer-"))
        })
        .map(|p| p.join("server/rust-analyzer"))
        .filter(|p| p.exists())
        .collect();
    hits.sort();
    hits.pop()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_none_when_binary_absent() {
        // `resolve_binary` cascades an override -> PATH -> rustup -> VS Code
        // fallback, so a fake override alone doesn't prove absence: on any dev
        // box with rust-analyzer actually installed (as this one is), a fake
        // override would still resolve via one of those fallbacks, making the
        // end-to-end assertion flaky based on what's installed. Pin down the
        // real invariant — an absent/non-executable binary path yields a
        // negative, non-panicking result — at the underlying probe instead.
        assert!(!binary_runs(Path::new(
            "definitely-not-a-real-ra-binary-xyz"
        )));
    }

    /// Same invariant as `detect_returns_none_when_binary_absent`, pinned at
    /// the underlying probe for the same reason: this sandbox has a real
    /// `scip-go` on `PATH` (P2.1 was verified against it), so asserting
    /// `go_resolve_binary(...).is_none()` end-to-end would be flaky based on
    /// what's installed rather than testing the actual invariant — an
    /// absent/non-executable path is rejected.
    #[test]
    fn go_binary_runs_rejects_a_nonexistent_path() {
        assert!(!binary_runs(Path::new(
            "definitely-not-a-real-scip-go-binary-xyz"
        )));
    }

    /// `is_npx` gates whether `python_build_command`/`python_binary_version`
    /// prepend the `@sourcegraph/scip-python` package name — must key off
    /// the resolved binary's filename only (`npx` vs a real `scip-python`
    /// binary), not assume one or the other.
    #[test]
    fn is_npx_true_only_for_a_path_named_npx() {
        assert!(is_npx(Path::new("npx")));
        assert!(is_npx(Path::new("/usr/local/bin/npx")));
        assert!(!is_npx(Path::new("scip-python")));
        assert!(!is_npx(Path::new("/usr/local/bin/scip-python")));
    }
}
