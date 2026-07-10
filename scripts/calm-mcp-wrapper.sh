#!/usr/bin/env bash
# Wrapper script to redirect calm logs to file, keeping only JSON-RPC on stdout.
#
# Dogfooding the daemon (ADR-0005, v1/M6, 2026-07-10): `calm connect` instead
# of `calm serve` — a thin forwarder that connects to (or spawns) one shared
# daemon per project instead of a fresh full `calm serve` process per MCP
# client session. Fixes the measured N-process problem this repo hit
# directly (up to 4 concurrent `calm serve --project-root /home/ybao/B.1/CALM`
# processes seen live). Does NOT retroactively affect any already-running
# `calm serve` process from before this change — the collapse to one shared
# daemon happens progressively as new MCP client sessions connect through
# this script. `scripts/mcp-launcher.sh` (the external-consumer-facing
# launcher) and the npm package's default args are deliberately untouched
# until this dogfood run proves the daemon out on real usage.
exec /home/ybao/B.1/CALM/target/release/calm connect --project-root /home/ybao/B.1/CALM 2>/tmp/calm-mcp.log
