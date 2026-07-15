#!/usr/bin/env sh
# Standalone installer for the "calm" MCP server binary — no git clone, no
# Rust toolchain required. Downloads the release asset matching this
# machine's platform, verifies its SHA256 against the published
# SHA256SUMS, and installs it to $CI_INSTALL_DIR. This is the same
# verified-download logic scripts/mcp-launcher.sh's tier 2 uses for an
# in-repo checkout, repackaged as a standalone entrypoint for someone who
# has never cloned CALM and just wants the `calm` binary.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Eilodon/CALM/main/scripts/install.sh | sh
#
# Env overrides:
#   CI_INSTALL_DIR      install location (default: $HOME/.local/bin)
#   CI_INSTALL_VERSION  release tag to install, e.g. "v0.2.0" or "0.2.0"
#                       (default: latest release)
#
# 5 platforms have a prebuilt binary today — the same matrix
# .github/workflows/release.yml builds: x86_64/aarch64 Linux (musl,
# statically linked, so no glibc version to match), aarch64/x86_64
# (Apple Silicon/Intel) macOS, and x86_64 Windows (MSVC). Anything else
# has nothing to fetch here — clone the repo and
# `cargo build --release --bin calm` instead (see README.md's Quick
# Start). Windows note: this is a POSIX `sh` script, so it only runs
# under a shell that provides one (Git Bash, MSYS2, Cygwin, WSL) — native
# PowerShell/cmd can't execute it at all, so `npx @eilodon/calm-mcp` is
# the more direct path for a plain Windows install.
set -eu

REPO="Eilodon/CALM"
BASE_URL="https://github.com/${REPO}"
INSTALL_DIR="${CI_INSTALL_DIR:-$HOME/.local/bin}"

log() { printf '[install] %s\n' "$*" >&2; }
die() { log "$*"; exit 1; }

os=$(uname -s)
arch=$(uname -m)

case "$os" in
  Linux)
    case "$arch" in
      x86_64)  target="x86_64-unknown-linux-musl" ;;
      aarch64) target="aarch64-unknown-linux-musl" ;;
      *) die "no prebuilt binary for Linux/${arch} — clone the repo and run 'cargo build --release --bin calm' instead" ;;
    esac
    ;;
  Darwin)
    case "$arch" in
      arm64)  target="aarch64-apple-darwin" ;;
      x86_64) target="x86_64-apple-darwin" ;;
      *) die "no prebuilt binary for macOS/${arch} — clone the repo and run 'cargo build --release --bin calm' instead" ;;
    esac
    ;;
  MINGW*|MSYS*|CYGWIN*)
    case "$arch" in
      x86_64) target="x86_64-pc-windows-msvc" ;;
      *) die "no prebuilt binary for Windows/${arch} — clone the repo and run 'cargo build --release --bin calm' instead" ;;
    esac
    ;;
  *)
    die "no prebuilt binary for OS '${os}' — clone the repo and run 'cargo build --release --bin calm' instead"
    ;;
esac

command -v curl >/dev/null 2>&1 || die "curl is required"

if [ -n "${CI_INSTALL_VERSION:-}" ]; then
  case "$CI_INSTALL_VERSION" in
    v*) tag="$CI_INSTALL_VERSION" ;;
    *) tag="v${CI_INSTALL_VERSION}" ;;
  esac
else
  tag=$(curl -sS -o /dev/null -w '%{redirect_url}' "${BASE_URL}/releases/latest" | sed 's#.*/##')
  [ -n "$tag" ] || die "could not resolve the latest release tag — pass CI_INSTALL_VERSION to pin one explicitly"
fi

asset_name="calm-${target}.tar.gz"
asset_url="${BASE_URL}/releases/download/${tag}/${asset_name}"
sums_url="${BASE_URL}/releases/download/${tag}/SHA256SUMS"

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT

log "downloading ${tag} (${target})..."
curl -fsSL -o "${tmp_dir}/${asset_name}" "$asset_url" || die "download failed for ${asset_url} — does tag ${tag} exist?"
curl -fsSL -o "${tmp_dir}/SHA256SUMS" "$sums_url" || die "download failed for ${sums_url}"

grep " ${asset_name}\$" "${tmp_dir}/SHA256SUMS" > "${tmp_dir}/expected.sha256" \
  || die "no checksum entry for ${asset_name} in SHA256SUMS"

# macOS ships `shasum`, not `sha256sum`; Linux is the other way around more
# often than not — try both rather than assuming which platform this is.
if command -v sha256sum >/dev/null 2>&1; then
  (cd "$tmp_dir" && sha256sum -c expected.sha256) >/dev/null 2>&1 \
    || die "SECURITY: checksum mismatch for ${asset_name} — refusing to install"
elif command -v shasum >/dev/null 2>&1; then
  (cd "$tmp_dir" && shasum -a 256 -c expected.sha256) >/dev/null 2>&1 \
    || die "SECURITY: checksum mismatch for ${asset_name} — refusing to install"
else
  die "neither sha256sum nor shasum found — cannot verify download integrity"
fi

# Windows binaries are packaged as calm.exe inside the tarball (see
# release.yml's Package step); every other target ships the bare `calm`
# name.
case "$target" in
  *-windows-*) bin_name="calm.exe" ;;
  *) bin_name="calm" ;;
esac

tar -xzf "${tmp_dir}/${asset_name}" -C "$tmp_dir" "$bin_name"
chmod +x "${tmp_dir}/${bin_name}"

"${tmp_dir}/${bin_name}" --version >/dev/null 2>&1 || die "downloaded binary failed to run — aborting install"

mkdir -p "$INSTALL_DIR"
mv "${tmp_dir}/${bin_name}" "${INSTALL_DIR}/${bin_name}"

log "installed calm ${tag} to ${INSTALL_DIR}/${bin_name}"

case ":$PATH:" in
  *":${INSTALL_DIR}:"*) ;;
  *) log "note: ${INSTALL_DIR} is not on your PATH — add it, e.g.: export PATH=\"${INSTALL_DIR}:\$PATH\"" ;;
esac

log ""
log "Next steps, from inside the project you want calm to analyze:"
log "  calm init  --project-root .   # create .calm/config.json"
log "  calm setup                    # wire this binary into Claude Code / Cursor / VS Code MCP config"
log "  calm index --project-root .   # build the index"
