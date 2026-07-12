#!/usr/bin/env bash
# Cursor Cloud/Background Agent "install" step (.cursor/environment.json).
# Cached after the first run per Cursor's own docs, so this is the closest
# equivalent Cursor has to Claude Code on the web's Setup Script — see
# docs/cloud-environment-setup.md for the full writeup of the race this
# guards against: `.cursor/mcp.json` points at `scripts/mcp-launcher.sh`,
# which only builds the calm-cli binary inline as a last resort. Without
# this, the very first Background Agent session on this repo hits a cold
# `cargo build -p calm-cli` (~59s measured) that the MCP client's initial
# dial attempt can lose, marking the "calm" server failed for that session.
# Prebuilding here means the launcher's fast path (already-built
# target/release/calm) wins instead.
#
# UPDATE (2026-07-12): `mcp-launcher.sh`'s tier 1.5 now fetches a
# checksum-and-SHA-verified prebuilt binary from the rolling `edge` GitHub
# Release by default — no compile needed even on a completely fresh
# checkout with nothing cached yet. This script is kept as defense-in-depth
# (guarantees a warm `target/release/calm` before the very first dial, no
# network dependency at all), same as the Claude Code Setup Script — see
# docs/cloud-environment-setup.md — but is no longer the only thing
# standing between a fresh Background Agent and a failed first connection.
#
# No LFS-pull step needed any more (2026-07-12): this repo has zero Git
# LFS-tracked content — the vendored embedding model weights are fetched
# and checksum-verified by crates/calm-core/build.rs::ensure_embedding_weights
# at compile time instead, degrading to a placeholder (never failing the
# build) if that fetch doesn't succeed. See
# docs/superskills/specs/2026-07-12-edge-release-binary-distribution.md.

# Release, not debug: unlike the Claude Code hook (which targets
# target/debug/calm to match a fast SessionStart-hook rebuild every
# session), this only runs once per cached Cursor environment snapshot, so
# the slower release build is worth it — it's also what
# `scripts/mcp-launcher.sh`'s fast path checks for first.
cargo build --release -p calm-cli 2>&1 || true
