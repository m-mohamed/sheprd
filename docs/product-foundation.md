# Product Foundation

`sheprd` is the smart session manager for Herdr.

The project is legitimate if it stays narrow:

- Herdr is the runtime.
- `sheprd` is the way in.

The first public product surface is intentionally small:

```bash
sheprd init --print
sheprd init
sheprd list
sheprd connect <project-or-path>
sheprd connect <project-or-path> --json
sheprd connect <project-or-path> --recipe agent-dev
sheprd recipes
sheprd doctor
sheprd doctor --json
sheprd show-config
```

Plain `connect` creates or focuses a workspace. Sample recipes are opt-in so
`sheprd` stays closer to `sesh` than to a personal layout manager.

Human `connect` output should be concise and factual: workspace action, project,
agent, optional recipe, and attach result. Machine consumers should use
`connect --json`.

`connect --json` is the automation surface: it should report the resolved
project, selected agent, Herdr workspace label/id, whether the workspace was
created or focused, recipe use, and attach status without launching an
interactive Herdr client.

`doctor --json` is the Herdr runtime readiness surface: it reports typed server
running state, version, protocol, compatibility, socket path, and
`protocol_ready` so agents can decide whether Herdr protocol automation is safe
without scraping human details.

JSON failure output is also part of the automation surface. Runtime failures
after argument parsing should emit `ok: false`, `error.kind`, `error.message`,
and `error.exit_code` on stderr.

`init` is the first-run bootstrap surface. It prints or writes a starter config,
uses repeated `--root` values for discovery roots, and refuses to overwrite an
existing config unless `--force` is explicit.

Future work should add explicit project entries, recent ranking, recipe config,
and an optional Ratatui picker only after the CLI behavior is boringly stable.
