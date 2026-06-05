# Product Foundation

`sheprd` is the smart session manager for Herdr.

The project is legitimate if it stays narrow:

- Herdr is the runtime.
- `sheprd` is the way in.

The first public product surface is intentionally small:

```bash
sheprd list
sheprd connect <project-or-path>
sheprd recipes
sheprd doctor
sheprd show-config
```

Future work should add explicit project entries, recent ranking, and an optional
Ratatui picker only after the CLI behavior is boringly stable.
