---
title: Replace .calm-bin LFS distribution with a rolling GitHub Release
date: 2026-07-12
SPEC_APPROVED: true
SPEC_ESCALATION: false
---

## Problem

`.calm-bin/x86_64-unknown-linux-musl/calm` (prebuilt `calm` binary, lets a fresh
clone skip compiling before the MCP client's ~7s connect-retry budget expires)
was committed via Git LFS, rebuilt/recommitted on every push to `main`.
This exhausted the GitHub account's Git LFS bandwidth budget (2026-07-12),
breaking CI, this prebuild workflow, and the nightly SCIP workflow
simultaneously, and indirectly caused a prior AI session to misdiagnose an
unresolved LFS pointer stub as a stale artifact and delete it from `main`.

Separately, closing the MCP cold-start race for cloud sandboxes today also
requires a manually-configured Cloud environment Setup Script
(`docs/cloud-environment-setup.md`) — a per-environment, UI-only step most
users don't know exists, unlike other MCP servers that need zero setup.

## Design

1. `.calm-bin` distribution moves off Git LFS onto a rolling GitHub Release
   (tag `edge`), republished by `.github/workflows/prebuild-mcp-binary.yml`
   only when `crates/**`/`Cargo.toml`/`Cargo.lock` change (trigger already
   narrowed in commit `bb3347d`). Release contains the binary tarball,
   `SHA256SUMS`, and `EDGE_SHA` (the exact commit the binary was built from —
   specifically the last commit touching build-relevant paths, not raw
   `HEAD`, so docs/config-only commits don't spuriously miss the cache; see
   Risk Assessment abductive hypothesis 1).
2. `scripts/mcp-launcher.sh` gets a new tier between today's tier 1 (local
   build) and tier 2 (tagged-release download): fetch the `edge` release,
   verify `EDGE_SHA` against the last build-relevant commit, verify
   `SHA256SUMS`, exec. Default-on, no env var — exact-SHA verification
   removes the version-skew risk that keeps the existing
   `CI_MCP_LAUNCHER_ALLOW_LATEST` opt-in-only.
3. Delete `.calm-bin/` and its `.gitattributes` LFS entry.
4. `docs/cloud-environment-setup.md`: Setup Script downgraded from "the
   actual fix" to "optional extra defense-in-depth" — the new tier closes
   the race automatically, in every environment, with no manual UI step.

GitHub Releases confirmed (docs.github.com) to have **no bandwidth/storage
limit**, only a 2GiB-per-file cap — unlike LFS, not subject to any billing
wall. This is a $0, permanent removal of the failure mode, not a bigger
version of the same constraint.

Alternatives considered and rejected: npm package (loses local-source
dogfooding fidelity — ships last-published version, not HEAD); container
image via existing `ghcr.io/eilodon/calm-mcp` (Docker-in-sandbox + volume
mount friction, worse for local dev loop); a fast-responding stub binary
while indexing warms up (real architectural change to calm-server itself,
out of scope here).

## Risk Assessment (audit-design)
<!-- audit-design: DO NOT DUPLICATE — update this section, do not append a second one -->
<!-- last-run: 2026-07-12 | trigger: NORMAL (no prior brainstorming spec existed; audited directly from chat-researched design per explicit user request) -->

**Tier:** 2 | **Date:** 2026-07-12

### Failure Modes
1. Fresh commit has no edge release yet (publish lag) → falls back to slow compile — LOW — mitigation in plan: NO (accepted, matches existing `.calm-bin` behavior, not a regression)
2. Two rapid successive pushes race publishing to the same `edge` tag, asset/SHA256SUMS/EDGE_SHA could mismatch across the race window — MEDIUM — mitigation in plan: YES (`concurrency:` group on the workflow; existing checksum+SHA verification already fails safe, not open)
3. `curl` calls in `mcp-launcher.sh` have no `--max-time`/`--connect-timeout` (pre-existing gap in tier-2 code); making this tier default-on for every fresh clone turns a rare exposure into the common path — a hang could be worse than today's straight-to-compile behavior — HIGH — mitigation in plan: YES (add explicit timeouts to every curl call, old and new)

### Layer Signals
- L5 Security: no new trust boundary — anyone who can push to `main` already gets their code auto-built-and-executed on every fresh session today (tier 3 / Setup Script); this change only makes that same trust boundary's outcome faster via a checksum-verified precompiled artifact of the identical commit.
- L4 Integration: GitHub Release CDN (`objects.githubusercontent.com`, not `github.com` itself) — see Assumptions.
- Other layers: no signal.

### Assumptions to Verify
- ASSUMED: GitHub Release bandwidth stays unmetered indefinitely (current 2026 policy, not a contractual guarantee).
- ASSUMED, unverified: cloud sandboxes' network egress allows `objects.githubusercontent.com` (the actual redirect target for release asset downloads), not just `github.com` — a corporate/locked-down proxy allowlisting only the latter would silently break this tier (falls through safely to compile, but the "instant" benefit is lost without visible signal).
- ASSUMED, unverified: a ~103MB download completes in low single-digit seconds on real target sandboxes — not measured, only inferred from typical cloud bandwidth.

### Abductive Hypotheses
1. Interaction between two individually-correct pieces: the narrowed `paths:` trigger (only publishes on `crates/**` changes) combined with exact-`HEAD`-SHA matching would make every docs/config-only commit (very common in this repo) spuriously miss the edge cache even though the actual binary content is unchanged. Fixed in design by matching against "last commit touching build-relevant paths" instead of raw `HEAD`.
2. High-concurrency thundering-herd downloads of the same `edge` asset after a popular commit — GitHub's release CDN is generally described as resilient at this kind of scale (same class of infra serving npm/docker-scale public artifact traffic), but not independently stress-tested here. Low priority, monitor if it becomes relevant.

### Gate Result
<!-- PASS WITH FLAGS -->
PASS WITH FLAGS — proceed to implementation; all three flagged mitigations (curl timeouts, build-relevant-path SHA matching, workflow concurrency group) are required, not optional, before considering this done.

### Implementation refinement (found while implementing, 2026-07-12)

Abductive hypothesis 1's fix ("match against last commit touching
build-relevant paths, not raw HEAD") is implemented via `git diff --quiet
EDGE_SHA..HEAD -- crates Cargo.toml Cargo.lock` on the client side instead
of independently recomputing "last commit touching paths" via `git log`
on both the publish and check sides. Equivalent outcome (docs/config-only
commits on top of EDGE_SHA still count as fresh), but more robust to
shallow clones: `EDGE_SHA` on the publish side is simply `git rev-parse
HEAD` at publish time (no path-filtering needed there — the workflow's own
`paths:` trigger already guarantees that commit is build-relevant); the
client checks object-existence (`git cat-file -e`) before diffing, so an
unreachable `EDGE_SHA` in a shallow clone degrades safely to "can't verify
→ fall through" instead of an ambiguous git error.
