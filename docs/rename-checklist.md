# Rename checklist — `ci` / Code-Intelligence → CALM

Scope note (2026-07-05): the user asked for a step-by-step rename, starting with the README only.
This checklist is the "everything else" — nothing on it has been executed. It exists so a full
rename can be picked up in whatever order/scope makes sense later, without re-deriving the blast
radius from scratch. Grouped by how contained vs. externally-visible each change is, since that's
what determines how carefully each one needs to be reviewed before touching it.

## Tier 1 — internal, reversible, no external impact

Safe to do in one sitting; nothing outside this repo depends on these names.

- **Crate names** — `crates/ci-core`, `crates/ci-server`, `crates/ci-cli` → e.g. `calm-core`,
  `calm-server`, `calm-cli` in each crate's `Cargo.toml` `[package] name`, and the root
  `Cargo.toml`'s `[workspace.dependencies]` path-dependency aliases.
- **Every internal `use ci_core::…` / `ci_core::…` / `ci_server::…` qualified path** — this is the
  big one by volume: essentially every `.rs` file in `ci-server` and `ci-cli` qualifies calls into
  `ci_core::` (dozens of files, hundreds of call sites per the audit that produced this checklist).
  A crate rename means `cargo build`/`clippy` will point at every single one — mechanical but not
  small. Consider `sed`/`bulk_replace`-driven rather than hand-editing.
- **Directory names** (optional, separate decision from the crate/package rename above) —
  `crates/ci-core/` → `crates/calm-core/` etc. Not required for the crate rename to work (Cargo
  doesn't care that a directory name matches the package name), but leaving them mismatched forever
  would read as an oversight rather than an in-progress rename.
- **Binary name** — `crates/ci-cli/Cargo.toml`'s `[[bin]] name = "ci"` → `"calm"`. Cascades into:
  - `scripts/mcp-launcher.sh` — binary name and every `target/{debug,release}/ci` path lookup.
  - `scripts/install.sh` — hardcoded binary name, install path.
  - `.claude/hooks/ci-nudge.sh` — filename and internal tool-name-prefix checks.
  - CLI help text and every doc/example showing `ci index`, `ci serve`, etc. (this README already
    does, `AGENTS.md`, all of `docs/`).
- **MCP server registration key** — `.mcp.json` / `.cursor/mcp.json` / `.vscode/mcp.json`'s `"ci"`
  server name → `"calm"`. This is the one with the widest *documentation* blast radius even though
  it's a config change: every tool becomes `mcp__calm__*` instead of `mcp__ci__*`, which means every
  reference to `mcp__ci__*` across `AGENTS.md`, hooks, and anyone's saved memory/notes about this
  project goes stale the moment this flips. Coordinate with the crate/binary rename above — doing
  them in the same pass avoids a window where the binary is renamed but tools still answer to the
  old prefix (or vice versa).
- **`.codeindex/` directory name** — could become `.calm/` for full consistency, or stay as-is (an
  implementation detail most users never look at directly). If renamed: every existing checkout's
  index needs to either migrate or re-index from scratch — treat this as a breaking change for
  anyone who already has a `.codeindex/` locally, not a free rename.

## Tier 2 — published artifacts, need a coordinated release

Each of these has already been distributed to whoever installed this tool before today. A rename
here means "the old thing keeps existing whether you touch it or not" — the only choice is whether
to add a deprecation pointer or just let it go stale silently.

- **npm packages** — `npm/ci-mcp/`, `npm/ci-mcp-darwin-arm64/`, `npm/ci-mcp-linux-arm64/`,
  `npm/ci-mcp-linux-x64/` (all four `package.json`'s `name` field is `@eilodon/ci-mcp*`, already
  published and live per prior session notes). A rename means publishing **new** packages under
  `@eilodon/calm-mcp*` — the old ones don't get renamed retroactively on npm, they just sit there
  unless explicitly deprecated (`npm deprecate`) with a pointer to the new name.
- **Docker/GHCR image** — currently `ghcr.io/eilodon/code-intelligence`. A rename means pushing to a
  **new** GHCR repository; existing tags at the old name stay put unless you also delete them
  (a separate, more destructive decision). Update `Containerfile` and any binary-name references
  inside it, plus `compose.yaml`'s service/image name.
- **Release binaries** — `.github/workflows/release.yml`'s artifact naming (`ci-${target}.tar.gz`)
  and the binary inside each tarball; `scripts/install.sh`'s download-URL pattern and
  `SHA256SUMS` naming convention all assume the current binary/artifact name.

## Tier 3 — outside this repo's control, needs the user directly

Not something an agent can safely do unattended — genuinely destructive/irreversible if rushed, or
literally requires access this session doesn't have.

- **GitHub repository rename** — `Eilodon/Code-Intelligence` → e.g. `Eilodon/CALM`. GitHub redirects
  the old URL for a while after a rename, but every hardcoded reference to
  `github.com/Eilodon/Code-Intelligence` across `scripts/install.sh` (the `curl | sh` URL),
  README badges/links, and `docs/mcp-client-setup.md` should still be updated rather than relying on
  the redirect indefinitely. This is a `gh repo rename` / GitHub-UI action on a shared, external
  resource — needs the repo owner to actually do it, not something to script speculatively.

## Suggested order, if/when this moves forward

1. Tier 1 in one pass (crate names + binary name + MCP server key + all internal
   `ci_core::`/`ci_server::` references + doc updates) — self-contained, fully reversible via git,
   nothing external notices until a new release goes out.
2. Tier 2 as part of a deliberate version bump / release, with the old npm packages/GHCR tags
   explicitly deprecated (not silently abandoned) so existing installs get a clear signal to
   migrate instead of just quietly going stale.
3. Tier 3 last, and only with the user driving it directly — it's the one step that touches a
   resource other people may have bookmarked, forked, or scripted against.
