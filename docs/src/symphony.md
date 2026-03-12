# Symphony

Symphony is a repository-driven automation service for running coding-agent
work against tracker issues. In Moltis, the first implementation slice is a
CLI-first foundation built around a repo-owned `WORKFLOW.md`.

Current scope:

- `WORKFLOW.md` discovery from an explicit path or `./WORKFLOW.md`
- YAML front matter parsing plus prompt body extraction
- Typed runtime config with defaults for tracker, polling, workspace, hooks,
  agent, and Codex settings
- Dispatch preflight validation
- Read-only Linear candidate and issue-state polling
- File watching and hot reload with last-known-good config retention
- Workspace path sanitization and safety checks
- A daemon skeleton exposed through `moltis symphony run`, with `--once`
  performing a real Linear poll cycle

What is not implemented yet in this slice:

- Codex app-server execution
- Retry orchestration and reconciliation
- Gateway dashboard or REST/RPC status APIs

## Workflow File

Symphony reads a repository-owned `WORKFLOW.md` file. The file may start with
YAML front matter followed by a Markdown prompt body.

Example:

```md
---
tracker:
  kind: linear
  api_key: $LINEAR_API_KEY
  project_slug: moltis
polling:
  interval_ms: 30000
workspace:
  root: ~/tmp/symphony_workspaces
codex:
  command: codex app-server
---

Work on {{ issue.identifier }}: {{ issue.title }}
```

## CLI

Validate the workflow and print the effective runtime configuration:

```bash
moltis symphony validate
```

Start the daemon skeleton:

```bash
moltis symphony run
```

Run a single startup cycle and exit:

```bash
moltis symphony run --once
```

You can pass an explicit workflow path to either command:

```bash
moltis symphony validate path/to/WORKFLOW.md
```

## Reload Behavior

When `WORKFLOW.md` changes, Moltis re-reads the file and attempts to apply the
new config immediately. If the reload is invalid, the daemon keeps running with
the last known good configuration and logs a warning.

## Safety Model

- Workspace directories are derived from sanitized issue identifiers
- Workspace paths must remain under the configured workspace root
- Hook execution is timeout-bound
- Tracker API keys are resolved without printing secret values

This is the base needed for the later tracker, runner, and observability
layers.
