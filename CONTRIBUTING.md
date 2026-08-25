# Contributing to sheprd

Thanks for wanting to contribute.

## The Rule

Understand your change. If you cannot explain what your code does, why it
belongs in `sheprd`, and how it respects Herdr's runtime boundary, do not open
the PR yet.

Using AI to write code is fine. Submitting code you do not understand is not.

By submitting a contribution, you agree that it is licensed under the
repository's `AGPL-3.0-or-later` license.

## Product Boundary

Herdr owns runtime state:

- sessions
- workspaces
- tabs
- panes
- persistence
- attach and detach
- remotes and SSH
- keybindings
- agent state
- integrations

`sheprd` owns the project-router layer:

- project discovery and canonical checkout resolution
- Herdr workspace focus and the explicit editor-first recipe
- readiness checks and structured JSON outcomes

If a change turns `sheprd` into a terminal multiplexer, layout engine, or Herdr
replacement, start with a discussion first.

## Issues And Discussions

Use issues for reproducible bugs with a clear current behavior, expected
behavior, reproduction, environment, and impact.

Use discussions for ideas, product direction, workflow questions, integration
requests, and anything that would expand `sheprd` beyond the project-router boundary.

## Checks

Install the repo hooks once in your clone:

```bash
just install-hooks
```

The pre-commit hook runs `cargo fmt --check`. The commit-msg hook keeps commit
subjects conventional and lowercase so history stays easy to review.

```bash
just ci
```

Without `just`:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo deny check
```

## Commit Style

The root commit may be named `Initial commit`.

After that, use short conventional commit subjects:

```text
feat: add project readiness output
fix: preserve dirty rollback state
docs: clarify Herdr ownership
```

Do not tag, publish a release, or change repository visibility from a normal
contribution. Public release is a maintainer gate.
