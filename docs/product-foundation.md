# Product Foundation

`sheprd` is the smart session manager for Herdr.

The project is legitimate if it stays narrow:

- Herdr is the runtime.
- `sheprd` is the way in.

The first public product surface is intentionally small:

```bash
sheprd list
sheprd connect <project-or-path>
sheprd connect <project-or-path> --recipe agent-dev
sheprd recipes
sheprd doctor
sheprd show-config
```

Plain `connect` creates or focuses a workspace. Sample recipes are opt-in so
`sheprd` stays closer to `sesh` than to a personal layout manager.

Future work should add explicit project entries, recent ranking, recipe config,
and an optional Ratatui picker only after the CLI behavior is boringly stable.
