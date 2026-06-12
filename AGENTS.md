# AGENTS.md

Guidance for agents working in this repository.

`sheprd` is a smart session manager for Herdr. Keep the boundary sharp:

- Herdr owns runtime state, panes, tabs, sessions, remotes, keybindings,
  integrations, persistence, and agent status.
- `sheprd` owns project discovery, project selection, agent selection, preflight
  checks, and optional sample recipes.

Sample recipes are examples, not policy. Do not make one user's personal layout
the default behavior for every user.

Use Herdr CLI wrappers before raw socket work. Add raw socket code only when
`sheprd` becomes a real protocol client or event subscriber.

## Validation

```bash
just ci
```

Without `just`:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```
