# sheprd

Use this skill when working in a repository that uses `sheprd` to enter or
inspect Herdr workspaces.

## Purpose

`sheprd` is a smart session manager for Herdr. It discovers projects, selects an
agent lane, and connects to the matching Herdr workspace.

Herdr owns the runtime. Do not use `sheprd` as a terminal multiplexer, pane
manager, keybinding layer, persistence layer, remote/SSH layer, or replacement
for Herdr.

## First Checks

Run:

```bash
sheprd doctor
sheprd list
```

For automation or troubleshooting a Herdr/runtime mismatch, prefer:

```bash
sheprd doctor --json
```

Use the typed `herdr.protocol_ready`, `herdr.protocol`, `herdr.compatible`, and
`herdr.socket` fields. Do not scrape the human `checks[].detail` string unless
you are displaying it back to a person.

When onboarding a fresh machine or unclear config, inspect the starter config
without writing first:

```bash
sheprd init --print
```

If you are already inside Herdr, prefer:

```bash
sheprd connect <project> --no-attach
```

This avoids nested Herdr clients.

## Connecting

Use:

```bash
sheprd connect <project>
```

`connect`, `open`, and `switch` are aliases for the same baseline behavior:
create or focus the matching Herdr workspace. They should not force panes, tabs,
or commands by default. Non-JSON output reports the workspace action, project,
agent, optional recipe, and attach result.

For automation, prefer:

```bash
sheprd connect <project> --json
```

JSON connect output reports the resolved project, selected agent, Herdr
workspace label/id, whether the workspace was focused or created, recipe use,
and whether an interactive Herdr client was attached. JSON mode does not launch
an interactive Herdr client.

When a command fails after argument parsing, JSON mode emits a structured error
envelope on stderr. Read `error.kind`, `error.message`, and `error.exit_code`
instead of scraping human `error:` lines.

## Sample Recipes

Recipes are optional samples, not product policy.

Use a recipe only when the user explicitly wants a starter layout:

```bash
sheprd connect <project> --recipe agent-dev
```

The `agent-dev` sample creates:

- `code`: `nvim`, selected agent, shell
- `git`: `lazygit`, shell

Do not assume every user wants this layout.

## Configuration

Read config with:

```bash
sheprd show-config
```

Use `sheprd init` to create a starter config when the user asks for bootstrap
help. It refuses to overwrite an existing config unless `--force` is explicit.
Do not edit `~/.config/sheprd/config.toml` directly unless the user asks for a
config change.

## Development

For this repository:

```bash
just check
```

Use `docs/commands.md` as the command contract when changing CLI behavior.
Before public release, also follow `docs/prelaunch-chaos.md`.
