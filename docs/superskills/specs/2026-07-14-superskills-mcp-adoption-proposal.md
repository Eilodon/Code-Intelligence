---
title: DPS-Superskills-MCP zip — adoption proposal for CALM (dev-workflow vs shipped-product split)
date: 2026-07-14
SPEC_APPROVED: true
SPEC_ESCALATION: false
---

## Problem

User uploaded `DPS-Superskills-MCP-main.zip` (the source of the Super Skills
framework this repo already dogfoods: adr-commit, audit-design, vheatm,
pattern-globalize, etc.) and asked for a full analysis of what's worth
adopting for CALM. A first pass produced a 4-tier proposal list, but
conflated two structurally different kinds of change:

- **Layer 1 — internal dev workflow.** Things that only affect how *this
  repo* gets developed (which Claude Code skills the maintainer has
  installed, repo-local scripts like `scripts/check-adr-staleness.sh`).
  Zero effect on anyone who installs CALM from crates.io/npm/GHCR.
- **Layer 2 — shipped OSS product.** Code inside `crates/calm-core` /
  `crates/calm-server` that ships in the CALM binary itself. Every
  third-party installer gets this, and the primary *consumer* of that code
  is not a human but a coding agent (Claude, or any other MCP client)
  calling CALM's tools — same as this session.

The user asked for `audit-design` to run against the full proposal set with
this split made explicit, and to flag anything mis-scoped once the split is
applied.

## Proposal Set (re-split by layer)

### Layer 1 — internal dev workflow only (no shipped-code change)

1. Copy 8 net-new skills into the Claude Code skill dir: `complexity-gate`,
   `context-reanchor`, `domain-alignment`, `epistemic-health-check`,
   `framework-doctor`, `privacy-secrets-gate`, `release-readiness`,
   `dps-init`/`dps-promote` + `shared/*.md` schemas they reference.
2. Version-diff check: installed `using-super-skills` / `specialist-review`
   / `tdd-verified` may predate v5.2.1 mechanisms (complexity-gate as
   top-level router, MIGRATION/PRIVACY-SECRETS lenses, 5 proof-modes).
3. Upgrade `scripts/check-adr-staleness.sh` (currently a text-shape
   advisory nudge — flags Status=Proposed/Deferred + presence of an
   "## Update" section, always exits 0) toward a numeric staleness-window
   gate, inspired by `epistemic_health_check.py`'s VOLATILE/WATCHFUL/STABLE
   cadence-state mechanism.
   **Correction found during this audit's verification pass:** this repo
   *also* has `docs/pattern-debt-registry.yaml` (16KB, separate artifact,
   Vietnamese-language DEBT-NNN entries, no automated staleness checker at
   all today) — the original proposal conflated this with the ADR
   checker. They are two separate targets.
4. (Tier 3, pilot-only) Full DPS spec lifecycle
   (CONTRACTS/BLUEPRINT/ADR/README + `dps.py sync/check/lint`) for CALM's
   own specs; `framework-doctor`/`framework-tests`-style self-audit if the
   maintainer ever forks the skill bundle locally.

### Layer 2 — shipped CALM product (affects every installer + their agent)

(a) Harden `crates/calm-core/src/sanitize.rs` — verified today
    (`sanitize.rs:12-96`) it is credential-regex-only, text-only: no
    Luhn-validated card detection, no structurally-validated SSN, no
    recursive `structuredContent`/JSON redaction walker. SUPER-MCP's
    `output_firewall.ts` has all three.

(b) Apply a "lethal trifecta" tool-safety lens (private-data-read +
    untrusted-content-exposure + network-or-destructive-effect ⇒ block or
    require explicit waiver) as a design-review pass over CALM's own MCP
    tool registrations (`edit_symbol`, `edit_lines`, etc.).

(c) Hand-wire MCP native Tasks (`tasks/get`/`update`/`cancel`) on top of
    `rmcp`'s lower-level primitives ahead of upstream SDK support, so long
    CALM operations (index rebuild, `lsp_refresh`/`scip_refresh`) don't
    block a synchronous tool call the way SUPER-MCP bypassed its own
    alpha TS SDK's Tasks gap.

(d) Add heartbeat/lease-renewal-with-abort semantics to CALM's
    cross-process `edit_lock`/`instance_lock` — verified today
    (`calm-server/src/tools.rs:300`, `calm-core/src/db/instance_lock`) it
    is acquire-once + ~150ms poll, no lease renewal or steal-detection.

### Explicitly rejected (unchanged either layer)

KMS/Vault crypto-erasure, JWT/OIDC/OAuth resource-server auth, rate-limit
+ quota, plugin OS isolation sandboxing, idempotency-key request dedup —
enterprise multi-tenant SaaS/HTTP concerns, don't match CALM's local
single-user stdio threat model in either layer.

---

## Risk Assessment (audit-design)
<!-- audit-design: DO NOT DUPLICATE — update this section, do not append a second one -->
<!-- last-run: 2026-07-14 | trigger: NORMAL -->

```
CONTEXT_MODE:      DESIGN
STAKEHOLDER:       CALM maintainer (Layer 1) + every third-party CALM installer and their coding agent (Layer 2)
GOAL:              pre-mortem before implementing any Tier 1-2 item
AUDIT_TARGET_TIER: 2 (Production) — Layer 2 items touch code shipped to every installer; no PII/payments/multi-tenant scope is being *added* (2a redacts incidental PII patterns found in source, it doesn't collect/store PII — Tier 3 correctly not triggered)
```

**Tier:** 2 | **Date:** 2026-07-14

### Failure Modes

1. **Perf regression on the hottest path in the binary** — `sanitize_source_output` (2a's target) is not a cold path: it's already flagged in this repo's own history (`docs/plans/2026-07-12-upgrade-plan-2-performance-robustness.md` F13: "13 lần `replace_all` full-string trên **mọi** source/preview/signature/docstring") as needing a RegexSet prefilter *specifically because* it runs on every `source`/`search` call for every installer's every symbol. Bolting on a Luhn digit-group scan + an SSN structural check + a separate recursive JSON walker — without the same RegexSet-prefilter discipline F13 already established — risks re-introducing the exact regression class F13 was written to close, except now amplified across every installer's repo size, not just this one. — **HIGH** — mitigation in plan: NO (not yet specified).
2. **Lock heartbeat introduces a new TOCTOU class into a currently-simple lock** — 2c (heartbeat/lease-renewal for `edit_lock`/`instance_lock`) adds a periodic renewal task racing against release/drop. This repo just closed `DEBT-010-hook-state-toctou-race` (`a42b667`, this session) — a read-modify-write race in a *different* piece of concurrent local state. A heartbeat added without equally careful token-matched-release semantics (the exact thing SUPER-MCP's Lua-script "renew only if token still matches" pattern exists to prevent) would add a new instance of the same bug class this repo just spent a session fixing elsewhere. — **HIGH** — mitigation in plan: NO.
3. **Tasks hand-wiring may be solving an already-solved problem, and may not be portable to Rust at all** — SUPER-MCP's trick is reaching into an *alpha TypeScript SDK's* private `_requestHandlers` map — a dynamically-typed escape hatch. `rmcp` (Rust) has a fundamentally different extensibility model (trait-based, statically typed); there is no verified evidence such a hand-wire point exists in `rmcp`'s current API surface — this proposal asserted feasibility by analogy, not by checking `rmcp`'s actual public/private surface. Separately: CALM already has a working async-long-op pattern (daemon + `indexing_status` polling tool, ADR-0005) — the marginal value of also adopting the MCP-native-Tasks protocol on top of that existing, working mechanism was never actually compared against what Tasks would add for CALM's specific agent-consumer, and could be near-zero. — **MED** (value uncertain, not "this breaks something") — mitigation in plan: NO.

### Layer Signals

- **L1 Logic:** any new sanitize.rs pattern (Luhn/SSN) needs the same false-positive fixture discipline this exact file's history already required once for real (`test_secret_key_pattern_still_matches_at_real_token_boundaries`, commit `743b00a` — a real false-positive incident, not hypothetical). No signal that this discipline is planned for the *new* patterns yet.
- **L2 Concurrency:** see Failure Mode 2 — shared state (`instance_lock`/`edit_lock`) now touched by a second concurrent actor (the heartbeat) once 2c lands.
- **L5 Security:** 2b (lethal-trifecta lens) is a discovery/review exercise, not an implementation task — its own output ("does any CALM tool already violate this?") is unknown until run. Scope it as an audit deliverable, not a code change, until a real finding shows up.
- **L6 Observability:** CALM is local-only with no phone-home telemetry (unlike SUPER-MCP's OTel/JSONL trail that lets it defer tuning to "revisit after production data," e.g. its own DEBT-006). Any Layer 2 change here has **no field signal** to catch drift post-release — correctness has to be established by this repo's own test/benchmark suite *before* release, not tuned afterward. This is a real asymmetry with the source project that changes how much pre-release rigor 2a/2c/2d each need.
- **L7 Cross-cutting:** idempotency/rate-limits — correctly out of scope in the "explicitly rejected" bucket; no signal against that call.
- L3 Data, L4 Integration: no signal.

### Assumptions to Verify

- **ASSUMED:** CALM's rmcp-based tool responses populate an MCP `structuredContent`-equivalent field the same way SUPER-MCP's protocol does — not checked. If CALM tools return plain serialized JSON text instead, "recursive `structuredContent` walker" is the wrong shape of fix; the real gap would be "recursively redact the JSON *text* CALM returns," a related but different problem.
- **ASSUMED:** sanitize.rs hardening (2a) won't regress the F13 perf fix — no benchmark run.
- **ASSUMED:** `rmcp` exposes (or can be made to expose) a hand-wire point for raw protocol handlers analogous to the TS SDK's `_requestHandlers` — no verification against `rmcp`'s actual API was done before proposing 2c.
- **ASSUMED:** MCP-native-Tasks adds value CALM's existing daemon+`indexing_status`-polling pattern doesn't already provide — not compared.
- **ASSUMED (correctly hedged already, still unverified):** installed `using-super-skills`/`specialist-review`/`tdd-verified` predate v5.2.1 mechanisms — stated as "may be," not confirmed by an actual diff.

### Abductive Hypotheses

1. **Interaction between two individually-correct changes:** if 2b (lethal-trifecta review) surfaces a real gap in an existing tool and a new warning/error path is added for it, that new message flows back out through whatever sanitize/redaction layer is live *at that time*. If 2b ships before 2a lands, the new warning path inherits the *current*, narrower sanitize.rs (credential-only) rather than the hardened one — a short window where a security-motivated addition (2b) is less protected than intended. Sequencing matters: 2a should land at or before 2b, not after.
2. **Coverage gap only visible with sensitive data outside the symbol layer:** `sanitize_source_output` is wired into the symbol-read path (`source`/`file_overview`-style calls). CALM's own `search` tool documents `kind="grep"` as reading "raw file content read from disk, including files the indexer never parses." If 2a is only wired into the symbol path and not into the raw-grep path, a card number or SSN sitting in a `.env`-adjacent doc, lockfile, or config file that only `grep`/`file` search kinds touch would ship unredacted even after 2a — an incomplete-coverage failure that would only surface when a real installer's repo has sensitive data outside parsed source.

### Gate Result
<!-- PASS | PASS WITH FLAGS | HOLD -->
**PASS WITH FLAGS**

- Layer 1, item 1-2 (skill copy, version-diff check): proceed freely — reversible, no shipped code, no user impact.
- Layer 1, item 3 (staleness gate): proceed, but scope as **two** separate small checkers (pattern-debt-registry.yaml + ADR), not one.
- Layer 1, item 4 (DPS lifecycle): proceed only as a single pilot on the next real C3/C4-tier CALM change, per original framing — unchanged.
- Layer 2 (a)(b)(c)(d): each requires its **Assumptions to Verify** entry resolved before `writing-plans` is invoked for it — none may proceed straight to implementation as originally framed. (c) specifically should not proceed at all until the redundancy-vs-`indexing_status` question and the `rmcp` feasibility question are both answered; it may turn out to be low-value or infeasible as stated.
- Explicitly-rejected bucket: no change, correctly out of scope for both layers.
