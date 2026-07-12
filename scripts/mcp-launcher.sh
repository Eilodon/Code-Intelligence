#!/usr/bin/env bash
# Universal MCP stdio launcher for the "calm" server — works from any MCP
# client's config (Claude Code, Cursor, VS Code, Windsurf, JetBrains, Codex
# CLI, Antigravity, or anything else that spawns a command over stdio). See
# docs/mcp-client-setup.md for the full rationale and per-client wiring.
#
# Resolution order (first usable binary wins, no exceptions):
#   1. Fast path   — an already-usable binary: $CI_MCP_BIN override, a
#                     cached verified download, or a local dev build
#                     (target/release/calm, target/debug/calm) — the dev
#                     build is only trusted if `is_binary_fresh` says it's
#                     at least as new as every source file (see that
#                     function's comment for the incident this guards
#                     against: a stale target/debug/calm silently served an
#                     entire MCP session because nothing checked it against
#                     the checked-out source before exec'ing it).
#                     $CI_MCP_BIN and the cached download are NOT freshness-
#                     checked here — one is an explicit override (the caller
#                     is asserting "use exactly this"), the other is an
#                     immutable, checksum-verified artifact for an exact
#                     tagged commit (its own consistency check is the tag
#                     match + `--version` check in download_and_verify).
#   1.5. Rolling edge release — a prebuilt binary published to the `edge`
#                     GitHub Release (not committed to the repo at all —
#                     see the 2026-07-12 correction below) on every push to
#                     main that touches crates/Cargo.toml/Cargo.lock.
#                     Closes the exact cold-start race
#                     docs/cloud-environment-setup.md documents for Claude
#                     Code on the web: a Setup Script or SessionStart hook
#                     can only race a cold `cargo build` against the MCP
#                     client's dial attempt, but a binary already fetchable
#                     the instant a fresh clone happens has no race to lose.
#                     Verified by exact source-tree match (`tree_matches_edge_sha`
#                     — a `git diff --quiet` against the published EDGE_SHA
#                     for crates/Cargo.toml/Cargo.lock, not an approximate
#                     version string), so this is safe to leave enabled by
#                     default for every environment — unlike tier 2's
#                     opt-in-only latest-release fallback, there's no
#                     staleness risk to opt out of. Cached locally after
#                     first verification, so steady-state sessions in the
#                     same environment need zero network calls at all. Only
#                     covers x86_64-unknown-linux-musl today.
#
#                     CORRECTION (2026-07-12): this used to be
#                     `.calm-bin/x86_64-unknown-linux-musl/calm`, a binary
#                     committed straight into the repo via Git LFS. That
#                     exhausted the GitHub account's Git LFS bandwidth
#                     budget (confirmed via job logs: "This repository
#                     exceeded its LFS budget", breaking CI, this binary's
#                     own prebuild workflow, and the nightly SCIP overlay
#                     simultaneously), and indirectly caused a prior AI
#                     session to mistake an unresolved LFS pointer stub for
#                     a stale artifact and delete it from main. GitHub
#                     Release assets have no such quota (docs.github.com:
#                     "no limit on the total size of a release, nor
#                     bandwidth usage") — moving to a Release removes this
#                     failure mode entirely rather than narrowing it. See
#                     docs/superskills/specs/2026-07-12-edge-release-binary-distribution.md
#                     for the full audit-design writeup.
#   2. Verified download — Linux x86_64/aarch64 only, and only when HEAD is
#                     exactly a released git tag (never guesses a version).
#                     Downloads the matching GitHub Release asset, verifies
#                     its SHA256 against the published SHA256SUMS, and
#                     sanity-checks `ci --version` before ever caching or
#                     executing it. Any failure at any step falls through
#                     to build-from-source — a failed/mismatched download
#                     never gets executed, but it also never becomes a dead
#                     end (see docs/mcp-client-setup.md for why fallback was
#                     chosen over a hard failure here). Every network call
#                     in this tier and tier 1.5 carries an explicit
#                     `--connect-timeout`/`--max-time` — added 2026-07-12
#                     after audit-design flagged that a hang here (this
#                     script had none before) would be worse than today's
#                     straight-to-compile fallback, especially now that
#                     tier 1.5 makes a network attempt the default path for
#                     every fresh clone rather than an opt-in edge case.
#   3. Build from source — `cargo build -p calm-cli`, always available as
#                     long as a Rust toolchain is present. This is the only
#                     path for non-Linux platforms, untagged dev checkouts,
#                     or offline environments.
#
# stdout carries ONLY the exec'd server's JSON-RPC traffic — every log line
# this script itself prints goes to stderr, in every tier, on every path.
set -uo pipefail

# Captured before the cd below — MCP clients spawn this script with cwd set
# to the caller's project root (their own project, not this one), and "."
# must resolve relative to that, not to wherever mcp-launcher.sh happens to
# live on disk. Without this, an external consumer pointing their own
# project's MCP config at this script would silently index
# CALM itself instead of their own project.
caller_pwd="$PWD"
cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1

REPO="Eilodon/CALM"
BASE_URL="${CI_MCP_LAUNCHER_BASE_URL:-https://github.com/${REPO}}"
CACHE_ROOT="${XDG_CACHE_HOME:-$HOME/.cache}/calm-mcp"

log() { printf '[mcp-launcher] %s\n' "$*" >&2; }

# Default to the caller's original cwd (captured above, before the cd into
# this script's own directory) unless the caller already passed their own
# --project-root, as either "--project-root /path" or "--project-root=/path"
# — both forms count as the same flag to clap, which is a single-value arg
# (not appendable) and rejects it being passed twice. An external consumer
# wiring this script into another project's client config gets that
# project's cwd for free this way; without the override-detection below it
# would collide with the default and `calm serve` always fails with "cannot
# be used multiple times".
serve_args=(serve --project-root "$caller_pwd" "$@")
for arg in "$@"; do
  case "$arg" in
    --project-root|--project-root=*) serve_args=(serve "$@"); break ;;
  esac
done

# `calm connect` (opt-in shared daemon, ADR-0005) collapses multiple MCP
# client sessions on the same repo into one background indexer/watcher
# instead of one process per session — see
# docs/adr/0005-daemon-forwarder-shared-process.md. Defaults to it when
# safe:
#   - Unix only (Commands::Connect is #[cfg(unix)]-gated at the enum level
#     on the Rust side — a non-Unix build doesn't even have the subcommand,
#     clap exits hard rather than falling back on its own).
#   - No extra args passed to this launcher. `calm connect` only
#     understands --project-root/--preset/--db-path (not --listen, or any
#     future serve-only flag) — reliably telling "--foo bar" (space form)
#     apart from a lone positional token without a real arg-parser here
#     isn't worth the risk of a subtly wrong heuristic, so any custom
#     invocation just keeps today's `calm serve` behavior, unchanged.
#   - CI_MCP_LAUNCHER_NO_DAEMON is not set to "1" (explicit opt-out, for
#     the initial rollout or any environment where the daemon path turns
#     out to be the wrong call).
use_connect=0
if [ "$#" -eq 0 ] && [ "${CI_MCP_LAUNCHER_NO_DAEMON:-0}" != "1" ]; then
  case "$(uname -s)" in
    Linux|Darwin) use_connect=1 ;;
  esac
fi
if [ "$use_connect" -eq 1 ]; then
  connect_args=(connect --project-root "$caller_pwd")
  log "using shared daemon mode (calm connect) — set CI_MCP_LAUNCHER_NO_DAEMON=1 to opt out"
fi

try_exec() {
  local bin="$1"
  if [ -x "$bin" ]; then
    if [ "$use_connect" -eq 1 ]; then
      exec "$bin" "${connect_args[@]}"
    fi
    exec "$bin" "${serve_args[@]}"
  fi
}

# True if `bin` is at least as new as every tracked Rust source file in the
# workspace — a cheap mtime check so the fast path never silently execs a
# local dev build that predates the checked-out source. Deliberately mtime-
# based, not git-commit-based: a git-SHA comparison would miss uncommitted
# or unstaged edits (the exact scenario that caused the incident this
# guards against — mid-session source edits with no new commit yet), and
# `cargo build`'s own incremental system already uses mtimes for the same
# reason. `find -newer` needs at least one existing candidate path per
# `-o` branch or it errors, so this only runs when `crates/` exists — true
# for any checkout that could have produced `bin` in the first place.
is_binary_fresh() {
  local bin="$1"
  [ -d crates ] || return 0
  local newer
  # Also checks vendored assets (crates/calm-core/assets/**, currently just the
  # embedding model's tokenizer/config/weights) — not just source files.
  # `include_bytes!` bakes these into the binary at compile time same as any
  # `.rs` change, so a binary built before `git lfs pull` resolved a
  # previously-stub asset must be treated as stale too, or this check would
  # keep serving a binary with the old (possibly LFS-pointer-stub) bytes
  # baked in even after the asset on disk is fixed — exactly the gap that let
  # a resolved LFS pull go unnoticed until a manual rebuild.
  newer=$(find crates Cargo.toml Cargo.lock -type f \
    \( -name '*.rs' -o -name 'Cargo.toml' -o -name 'Cargo.lock' -o -path '*/calm-core/assets/*' \) \
    -newer "$bin" 2>/dev/null | head -1)
  [ -z "$newer" ]
}

# True if `bin` looks like an unresolved Git LFS pointer stub rather than
# real binary content — happens when `git lfs pull`/the smudge filter never
# ran during checkout (e.g. git-lfs not installed in the environment). A
# real `ci` binary is tens of MB; an LFS pointer is a ~130-byte text file
# starting with this exact line. This matters because `exec`-ing one does
# NOT fail gracefully: the kernel's ENOEXEC (no shebang, not an ELF) makes
# bash fall back to interpreting the file's *text content* as a new shell
# script, which runs (and errors on "version: command not found") INSTEAD
# OF returning control to this script — verified directly against a
# synthetic pointer stub, not assumed. Without this check, an unresolved
# LFS pointer sitting where a real binary was expected would crash the
# whole launcher instead of falling through to the next tier — matters
# less now that tier 1.5 no longer uses LFS at all (see the 2026-07-12
# correction above), but target/release and target/debug are still local
# artifacts checked here, so this stays as defense-in-depth.
is_lfs_pointer() {
  [ "$(head -c 7 -- "$1" 2>/dev/null)" = "version" ]
}

try_exec_if_fresh() {
  local bin="$1"
  if [ -x "$bin" ] && ! is_lfs_pointer "$bin"; then
    if is_binary_fresh "$bin"; then
      if [ "$use_connect" -eq 1 ]; then
        exec "$bin" "${connect_args[@]}"
      fi
      exec "$bin" "${serve_args[@]}"
    else
      log "found $bin but it predates the current source tree — rebuilding instead of using it stale"
    fi
  fi
}

workspace_version=$(grep -m1 '^version = ' Cargo.toml 2>/dev/null | sed -E 's/version = "(.*)"/\1/')
resolved_tag=$(git describe --tags --exact-match 2>/dev/null || true)
cache_key="${resolved_tag:-$workspace_version}"

# ---- Tier 1: fast path — already-usable binary ----
[ -n "${CI_MCP_BIN:-}" ] && try_exec "$CI_MCP_BIN"
[ -n "$cache_key" ] && try_exec "${CACHE_ROOT}/${cache_key}/calm"
try_exec_if_fresh "target/release/calm"
try_exec_if_fresh "target/debug/calm"

# ---- Tier 1.5: rolling edge release (see header) ----
# Only the one platform .github/workflows/prebuild-mcp-binary.yml builds.
EDGE_CACHE_DIR="${CACHE_ROOT}/edge"

# True if a binary verified against $1 (a commit SHA) is still safe to use
# for the current checkout — i.e. no commit between $1 and HEAD touched
# anything that could change the built binary. Uses `git diff` against the
# actual tree instead of independently re-deriving "the last commit that
# touched these paths" on both the publish and check sides, so a
# docs/config-only commit stacked on top of $1 still counts as fresh
# without those two derivations ever risking disagreeing with each other.
# Object-existence is checked first so a shallow clone that never fetched
# $1 degrades to "can't verify" (falls through to the next tier) instead of
# an ambiguous git error.
tree_matches_edge_sha() {
  local edge_sha="$1"
  [ -n "$edge_sha" ] || return 1
  git cat-file -e "${edge_sha}^{commit}" 2>/dev/null || return 1
  git diff --quiet "$edge_sha" HEAD -- crates Cargo.toml Cargo.lock 2>/dev/null
}

if [ "$(uname -s)" = "Linux" ] && [ "$(uname -m)" = "x86_64" ] && [ -d crates ]; then
  cached_edge_sha=""
  [ -f "${EDGE_CACHE_DIR}/verified_sha" ] && cached_edge_sha=$(cat "${EDGE_CACHE_DIR}/verified_sha" 2>/dev/null)
  if [ -x "${EDGE_CACHE_DIR}/calm" ] && tree_matches_edge_sha "$cached_edge_sha"; then
    # Already verified in a previous session against a tree state that
    # still matches — zero network calls needed.
    try_exec "${EDGE_CACHE_DIR}/calm"
  elif command -v curl >/dev/null 2>&1; then
    edge_url="${BASE_URL}/releases/download/edge"
    edge_tmp=$(mktemp -d) || edge_tmp=""
    if [ -n "$edge_tmp" ]; then
      if curl -fsSL --connect-timeout 5 --max-time 30 -o "${edge_tmp}/EDGE_SHA" "${edge_url}/EDGE_SHA" \
        && curl -fsSL --connect-timeout 5 --max-time 30 -o "${edge_tmp}/SHA256SUMS" "${edge_url}/SHA256SUMS" \
        && curl -fsSL --connect-timeout 5 --max-time 120 -o "${edge_tmp}/calm-x86_64-unknown-linux-musl.tar.gz" "${edge_url}/calm-x86_64-unknown-linux-musl.tar.gz"; then
        downloaded_edge_sha=$(tr -d '[:space:]' <"${edge_tmp}/EDGE_SHA")
        if tree_matches_edge_sha "$downloaded_edge_sha" \
          && (cd "$edge_tmp" && grep ' calm-x86_64-unknown-linux-musl.tar.gz$' SHA256SUMS | sha256sum -c - >/dev/null 2>&1) \
          && tar -xzf "${edge_tmp}/calm-x86_64-unknown-linux-musl.tar.gz" -C "$edge_tmp" calm; then
          mkdir -p "$EDGE_CACHE_DIR"
          mv "${edge_tmp}/calm" "${EDGE_CACHE_DIR}/calm"
          chmod +x "${EDGE_CACHE_DIR}/calm"
          printf '%s' "$downloaded_edge_sha" >"${EDGE_CACHE_DIR}/verified_sha"
          rm -rf "$edge_tmp"
          try_exec "${EDGE_CACHE_DIR}/calm"
        else
          log "downloaded edge release didn't verify against the current source tree — building from source"
          rm -rf "$edge_tmp"
        fi
      else
        log "edge release download failed or timed out — building from source"
        rm -rf "$edge_tmp"
      fi
    fi
  fi
fi

# ---- Tier 2: verified download (Linux only, tagged commit only unless opted in) ----
download_and_verify() {
  local os arch target_triple tag asset_name asset_url sums_url tmp_dir downloaded_version cache_dir

  os=$(uname -s)
  if [ "$os" != "Linux" ]; then
    log "no prebuilt binary for OS '${os}' — building from source"
    return 1
  fi

  arch=$(uname -m)
  case "$arch" in
    x86_64)  target_triple="x86_64-unknown-linux-musl" ;;
    aarch64) target_triple="aarch64-unknown-linux-musl" ;;
    *)
      log "no prebuilt binary for arch '${arch}' — building from source"
      return 1
      ;;
  esac

  if ! command -v curl >/dev/null 2>&1; then
    log "curl not found — building from source"
    return 1
  fi

  if [ -n "$resolved_tag" ]; then
    tag="$resolved_tag"
  elif [ "${CI_MCP_LAUNCHER_ALLOW_LATEST:-0}" = "1" ]; then
    tag=$(curl -sS --connect-timeout 5 --max-time 15 -o /dev/null -w '%{redirect_url}' "${BASE_URL}/releases/latest" | sed 's#.*/##')
    if [ -z "$tag" ]; then
      log "could not resolve the latest release tag — building from source"
      return 1
    fi
  else
    log "not on a tagged release commit — building from source" \
        "(set CI_MCP_LAUNCHER_ALLOW_LATEST=1 to fetch the latest release instead)"
    return 1
  fi

  asset_name="calm-${target_triple}.tar.gz"
  asset_url="${BASE_URL}/releases/download/${tag}/${asset_name}"
  sums_url="${BASE_URL}/releases/download/${tag}/SHA256SUMS"

  tmp_dir=$(mktemp -d) || return 1
  trap 'rm -rf "$tmp_dir"' RETURN

  if ! curl -sSL --connect-timeout 5 --max-time 120 -o "${tmp_dir}/${asset_name}" "$asset_url"; then
    log "download failed or timed out for ${asset_url} — building from source"
    return 1
  fi
  if ! curl -sSL --connect-timeout 5 --max-time 30 -o "${tmp_dir}/SHA256SUMS" "$sums_url"; then
    log "download failed or timed out for ${sums_url} — building from source"
    return 1
  fi

  if ! grep " ${asset_name}\$" "${tmp_dir}/SHA256SUMS" >"${tmp_dir}/expected.sha256" 2>/dev/null; then
    log "no checksum entry for ${asset_name} in SHA256SUMS — building from source"
    return 1
  fi
  if ! (cd "$tmp_dir" && sha256sum -c expected.sha256 >/dev/null 2>&1); then
    log "SECURITY: checksum mismatch for ${asset_name} — refusing to use this binary, building from source instead"
    return 1
  fi

  if ! tar -xzf "${tmp_dir}/${asset_name}" -C "$tmp_dir" calm; then
    log "extraction failed for ${asset_name} — building from source"
    return 1
  fi
  chmod +x "${tmp_dir}/calm"

  downloaded_version=$("${tmp_dir}/calm" --version 2>/dev/null | awk '{print $NF}')
  if [ -n "$workspace_version" ] && [ "$downloaded_version" != "$workspace_version" ]; then
    log "downloaded binary reports version '${downloaded_version}', expected '${workspace_version}' — building from source instead"
    return 1
  fi

  cache_dir="${CACHE_ROOT}/${tag}"
  mkdir -p "$cache_dir"
  mv "${tmp_dir}/calm" "${cache_dir}/calm"
  rm -rf "$tmp_dir"

  try_exec "${cache_dir}/calm"
  return 1 # unreachable if try_exec succeeded (exec replaces this process)
}

download_and_verify

# ---- Tier 3: build from source (always must work standalone) ----
# `--features embeddings` is explicit here on purpose, not left to whatever
# calm-cli's Cargo.toml `default` happens to be today: a defense-in-depth
# measure so a future default-features change can't silently regress every
# freshly-built dev binary back to the "embeddings always Disabled" state
# (see `bootstrap_embeddings`/`load_embedder_readonly` in
# crates/calm-server/src/lib.rs for the multi-process lock-loser bug that
# state used to mask). `tier0-5`/`scip-overlay` are also named explicitly for
# the same reason, even though all three are currently also the crate's
# defaults.
log "building calm-cli from source (this may take about a minute on a cold cache)"
if ! cargo build --quiet -p calm-cli --features embeddings,tier0-5,scip-overlay 1>&2; then
  log "build failed — cannot start the calm MCP server"
  exit 1
fi
try_exec "target/debug/calm"
log "build succeeded but target/debug/calm is missing or not executable — aborting"
exit 1
