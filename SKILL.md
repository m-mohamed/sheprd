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
or commands by default.

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

Do not edit `~/.config/sheprd/config.toml` unless the user asks for a config
change.

## Development

For this repository:

```bash
just check
```

Before public release, also follow `docs/prelaunch-chaos.md`.
