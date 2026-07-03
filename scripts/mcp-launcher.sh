#!/usr/bin/env bash
# Universal MCP stdio launcher for the "ci" server — works from any MCP
# client's config (Claude Code, Cursor, VS Code, Windsurf, JetBrains, or
# anything else that spawns a command over stdio). See
# docs/mcp-client-setup.md for the full rationale and per-client wiring.
#
# Resolution order (first usable binary wins, no exceptions):
#   1. Fast path   — an already-usable binary: $CI_MCP_BIN override, a
#                     cached verified download, or a local dev build
#                     (target/release/ci, target/debug/ci) — the dev build
#                     candidates are only trusted if `is_binary_fresh` says
#                     they're at least as new as every source file (see that
#                     function's comment for the incident this guards
#                     against: a stale target/debug/ci silently served an
#                     entire MCP session because nothing checked it against
#                     the checked-out source before exec'ing it).
#                     $CI_MCP_BIN and the cached download are NOT freshness-
#                     checked here — one is an explicit override (the caller
#                     is asserting "use exactly this"), the other is an
#                     immutable, checksum-verified artifact for an exact
#                     tagged commit (its own consistency check is the tag
#                     match + `--version` check in download_and_verify).
#   2. Verified download — Linux x86_64/aarch64 only, and only when HEAD is
#                     exactly a released git tag (never guesses a version).
#                     Downloads the matching GitHub Release asset, verifies
#                     its SHA256 against the published SHA256SUMS, and
#                     sanity-checks `ci --version` before ever caching or
#                     executing it. Any failure at any step falls through
#                     to build-from-source — a failed/mismatched download
#                     never gets executed, but it also never becomes a dead
#                     end (see docs/mcp-client-setup.md for why fallback was
#                     chosen over a hard failure here).
#   3. Build from source — `cargo build -p ci-cli`, always available as
#                     long as a Rust toolchain is present. This is the only
#                     path for non-Linux platforms, untagged dev checkouts,
#                     or offline environments.
#
# stdout carries ONLY the exec'd server's JSON-RPC traffic — every log line
# this script itself prints goes to stderr, in every tier, on every path.
set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1

REPO="Eilodon/Code-Intelligence"
BASE_URL="${CI_MCP_LAUNCHER_BASE_URL:-https://github.com/${REPO}}"
CACHE_ROOT="${XDG_CACHE_HOME:-$HOME/.cache}/ci-mcp"

log() { printf '[mcp-launcher] %s\n' "$*" >&2; }

# Default to "." (this repo) unless the caller already passed their own
# --project-root, as either "--project-root /path" or "--project-root=/path"
# — both forms count as the same flag to clap, which is a single-value arg
# (not appendable) and rejects it being passed twice. An external consumer
# wiring this script into another project's client config supplies their
# own --project-root; without this check it collides with the hardcoded
# default below and `ci serve` always fails with "cannot be used multiple
# times".
serve_args=(serve --project-root . "$@")
for arg in "$@"; do
  case "$arg" in
    --project-root|--project-root=*) serve_args=(serve "$@"); break ;;
  esac
done

try_exec() {
  local bin="$1"
  if [ -x "$bin" ]; then
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
  newer=$(find crates Cargo.toml Cargo.lock -type f \
    \( -name '*.rs' -o -name 'Cargo.toml' -o -name 'Cargo.lock' \) \
    -newer "$bin" 2>/dev/null | head -1)
  [ -z "$newer" ]
}

try_exec_if_fresh() {
  local bin="$1"
  if [ -x "$bin" ]; then
    if is_binary_fresh "$bin"; then
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
[ -n "$cache_key" ] && try_exec "${CACHE_ROOT}/${cache_key}/ci"
try_exec_if_fresh "target/release/ci"
try_exec_if_fresh "target/debug/ci"

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
    tag=$(curl -sS -o /dev/null -w '%{redirect_url}' "${BASE_URL}/releases/latest" | sed 's#.*/##')
    if [ -z "$tag" ]; then
      log "could not resolve the latest release tag — building from source"
      return 1
    fi
  else
    log "not on a tagged release commit — building from source" \
        "(set CI_MCP_LAUNCHER_ALLOW_LATEST=1 to fetch the latest release instead)"
    return 1
  fi

  asset_name="ci-${target_triple}.tar.gz"
  asset_url="${BASE_URL}/releases/download/${tag}/${asset_name}"
  sums_url="${BASE_URL}/releases/download/${tag}/SHA256SUMS"

  tmp_dir=$(mktemp -d) || return 1
  trap 'rm -rf "$tmp_dir"' RETURN

  if ! curl -sSL -o "${tmp_dir}/${asset_name}" "$asset_url"; then
    log "download failed for ${asset_url} — building from source"
    return 1
  fi
  if ! curl -sSL -o "${tmp_dir}/SHA256SUMS" "$sums_url"; then
    log "download failed for ${sums_url} — building from source"
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

  if ! tar -xzf "${tmp_dir}/${asset_name}" -C "$tmp_dir" ci; then
    log "extraction failed for ${asset_name} — building from source"
    return 1
  fi
  chmod +x "${tmp_dir}/ci"

  downloaded_version=$("${tmp_dir}/ci" --version 2>/dev/null | awk '{print $NF}')
  if [ -n "$workspace_version" ] && [ "$downloaded_version" != "$workspace_version" ]; then
    log "downloaded binary reports version '${downloaded_version}', expected '${workspace_version}' — building from source instead"
    return 1
  fi

  cache_dir="${CACHE_ROOT}/${tag}"
  mkdir -p "$cache_dir"
  mv "${tmp_dir}/ci" "${cache_dir}/ci"
  rm -rf "$tmp_dir"

  try_exec "${cache_dir}/ci"
  return 1 # unreachable if try_exec succeeded (exec replaces this process)
}

download_and_verify

# ---- Tier 3: build from source (always must work standalone) ----
log "building ci-cli from source (this may take about a minute on a cold cache)"
if ! cargo build --quiet -p ci-cli 1>&2; then
  log "build failed — cannot start the ci MCP server"
  exit 1
fi
try_exec "target/debug/ci"
log "build succeeded but target/debug/ci is missing or not executable — aborting"
exit 1
