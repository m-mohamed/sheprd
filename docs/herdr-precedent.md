# Herdr Precedent

`sheprd` should feel native to the Herdr ecosystem without pretending to be
Herdr.

## Public Benchmark

Use Herdr's public project shape as the benchmark for `sheprd`:

- README: direct promise, quick start, install paths, and clear concepts;
- docs site: task-oriented docs, API docs, plugin docs, and update cadence;
- AGENTS.md: architecture rules that keep agent changes from creating god
  objects;
- SKILL.md: an agent-facing usage contract for the project;
- GitHub Discussions: ideas and Q&A outside the bug queue;
- releases and packaging: SemVer tags, stable install path, Homebrew/Nix
  options, and a clean crate/package story.

The goal is not feature parity. The goal is the same public trust: a stranger
should understand what the tool does, what it refuses to own, how to install it,
how to contribute, and how to verify it.

What we mirror:

- small conventional commits;
- `vX.Y.Z` tags;
- release commits named `release: vX.Y.Z`;
- `CHANGELOG.md`;
- `CONTRIBUTING.md`;
- `LICENSE`;
- `just ci`;
- GitHub Actions for fmt, clippy, and tests;
- source installer script;
- tag-shaped multi-platform release workflow kept behind the final ship gate;
- issue and discussion templates;
- docs that explain product boundaries.
- source-backed public docs that do not depend on maintainer private workflow.

What we do not mirror yet:

- preview/stable update channels;
- Homebrew formula;
- mise or Nix installers;
- raw socket event streams or custom protocol clients;
- Ratatui picker.
- approval-gate automation for outside contributors.

Those come after the CLI is stable and there are real public users.

## Herdr Boundary

Herdr is moving toward native runtime API support for custom clients while the
TUI remains first citizen. That strengthens, rather than weakens, `sheprd`'s
boundary: Herdr is the runtime and protocol owner; `sheprd` is the public,
boring project-to-workspace connector.

Use the CLI wrappers first:

```bash
herdr workspace list
herdr workspace create --cwd PATH --label LABEL --focus
herdr workspace focus WORKSPACE_ID
herdr tab create --workspace WORKSPACE_ID --cwd PATH --label LABEL --no-focus
herdr pane split PANE_ID --direction right --cwd PATH --no-focus
herdr pane run PANE_ID COMMAND
```

Only add raw socket code when `sheprd` is doing something the wrappers cannot
express, such as a dashboard/mobile client bridge or event subscriber. Any raw
socket path must first prove Herdr server status, protocol compatibility, and
socket location through `sheprd doctor`.
