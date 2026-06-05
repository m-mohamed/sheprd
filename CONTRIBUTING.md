# Contributing to sheprd

Thanks for wanting to contribute.

## The Rule

Understand your change. If you cannot explain what your code does, why it
belongs in `sheprd`, and how it respects Herdr's runtime boundary, do not open
the PR yet.

Using AI to write code is fine. Submitting code you do not understand is not.

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

`sheprd` owns the entry layer:

- project discovery
- project selection
- agent selection
- preflight checks
- small startup recipes

If a change turns `sheprd` into a terminal multiplexer, layout engine, or Herdr
replacement, start with a discussion first.

## Checks

```bash
just ci
```

Without `just`:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Commit Style

The root commit may be named `Initial commit`.

After that, use short conventional commit subjects:

```text
feat: add recipe listing
fix: avoid nested herdr attach
docs: clarify launch model
```
