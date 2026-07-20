# Docker MCP Catalog submission (prepared 2026-07-20, pending auth)

`server.yaml` here is the ready-to-submit entry for
[docker/mcp-registry](https://github.com/docker/mcp-registry) — the curated
catalog behind Docker Desktop's MCP Toolkit (hub.docker.com/mcp). It is
pinned to the `v0.3.4` tag/commit and validated against the catalog's real
schema (modeled on their `ast-grep`/`filesystem` entries: single rw project
volume, `disableNetwork: true`, `mcp/calm` Docker-built image namespace).

Submission was **blocked only by GitHub auth** (gh keyring token locked in
the working session). To submit once `gh auth status` is green:

```bash
gh repo fork docker/mcp-registry --clone=false
git clone --depth 1 https://github.com/docker/mcp-registry.git /tmp/mcp-registry
cd /tmp/mcp-registry
git checkout -b add-calm-server
mkdir -p servers/calm
cp <CALM-repo>/docs/distribution/docker-mcp-catalog/server.yaml servers/calm/server.yaml
git add servers/calm && git commit -m "Add CALM (Coding Agent Liveness Map) MCP server"
git remote add fork https://github.com/Eilodon/mcp-registry.git
git push fork add-calm-server
gh pr create --repo docker/mcp-registry --head Eilodon:add-calm-server \
  --title "Add CALM (Coding Agent Liveness Map) MCP server" \
  --body "Adds CALM — call-graph-aware code intelligence with hash-verified, safety-gated edits for coding agents. Local server, Dockerfile at Containerfile, MIT-licensed, image builds green in our own CI (ghcr.io/eilodon/calm-mcp, cosign-signed). Docs: https://github.com/Eilodon/CALM"
```

Their CI (`task validate` + a real containerized `tools/list` probe) runs on
the PR; the Docker team review follows. Keep `source.branch`/`commit` pinned
to the latest release tag when resubmitting after future releases.
