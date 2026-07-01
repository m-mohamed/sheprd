# Command Reference

`sheprd` is a Herdr entry layer. Every command should make project selection,
agent selection, config bootstrap, or readiness easier without taking runtime
ownership away from Herdr.

Herdr owns sessions, workspaces, tabs, panes, remotes, persistence, keybindings,
integrations, attach/detach, and agent state. `sheprd` calls Herdr's documented
CLI wrappers and treats Herdr ids as live runtime state.

## Global Options

```bash
sheprd --agent codex <command>
sheprd --agent opencode <command>
sheprd --json <command>
sheprd --no-attach <command>
```

`--agent` selects the lane used for workspace labels and sample recipes.
Supported agents are `pi`, `droid`, `claude`, `codex`, `hermes`, and
`opencode`.

`--json` is for scripts and agents. JSON mode must not launch an interactive
Herdr client.

When a command fails after argument parsing, `--json` emits a structured error
envelope on stderr:

```json
{
  "ok": false,
  "error": {
    "kind": "message",
    "message": "project 'missing' was not found",
    "exit_code": 2
  }
}
```

Agents should consume `error.kind`, `error.message`, and `error.exit_code`.
Successful JSON responses keep their command-specific shape.

`--no-attach` creates or focuses Herdr state without attaching a Herdr client.
Use it from inside Herdr or from automation.

## `init`

Preview or write a starter config.

```bash
sheprd init --print
sheprd init --root ~/Workspace --root ~/src
sheprd init --force
sheprd init --print --json
```

`init --print` writes nothing. `init` refuses to overwrite an existing config
unless `--force` is explicit.

JSON output reports:

- `path`
- `existed`
- `written`
- `default_agent`
- `roots`
- `contents` when `--print` is used

## `list`

Discover projects and show their matching Herdr workspace labels.

```bash
sheprd list
sheprd list --json
sheprd --agent droid list
```

Text output is for quick scanning. JSON output reports the selected `agent` plus
project rows with `name`, `path`, `workspace`, and `running`.

`running` means a Herdr workspace with the expected label exists right now. It
does not mean the project has no git changes, no active tasks, or no running
agent work.

## `connect`

Create or focus the Herdr workspace for a project name or repository path.

```bash
sheprd connect my-project
sheprd open my-project
sheprd switch my-project
sheprd connect ~/code/my-project --no-attach
sheprd connect my-project --json
```

`open` and `switch` are visible aliases for `connect`.

Plain `connect` is intentionally boring: it creates or focuses the matching
workspace and then lets Herdr own the runtime. It does not force tabs, panes, or
commands.

Human output reports:

- whether Sheprd `created` or `focused` a Herdr workspace
- project name
- agent lane
- optional recipe
- attach result

JSON output reports:

- resolved `project`
- selected `agent`
- `action`
- `workspace`
- `workspace_id`
- optional `recipe`
- `attached`

JSON mode is non-interactive and leaves `attached` false.

## `connect --recipe agent-dev`

Apply an explicit sample layout only when creating a fresh workspace.

```bash
sheprd connect my-project --recipe agent-dev
sheprd connect my-project --recipe agent-dev --no-attach
```

The sample creates:

- `code`: `nvim`, selected agent, shell
- `git`: `lazygit`, shell

If the workspace already exists, Sheprd focuses it and does not reshape live
tabs or panes.

## `recipes`

Show optional sample recipes.

```bash
sheprd recipes
sheprd recipes --json
```

Recipes are examples, not default policy. The first public recipe is
`agent-dev`.

## `doctor`

Check the Herdr runtime path, expected tools, selected agent executable, and
Herdr server protocol readiness.

```bash
sheprd doctor
sheprd doctor --json
```

Human output is a checklist. JSON output reports `ready`, a typed `herdr` block,
and raw `checks`.

The `herdr` block includes:

- `running`
- `version`
- `protocol`
- `compatible`
- `socket`
- `protocol_ready`
- `error`

Agents and scripts should read `herdr.protocol_ready`, `herdr.protocol`,
`herdr.compatible`, and `herdr.socket` instead of scraping human check details.

## `show-config`

Show the active config after defaults, config file loading, path expansion, and
explicit project entries.

```bash
sheprd show-config
sheprd show-config --json
```

Use this command when project discovery is surprising. It shows which roots and
explicit project mappings Sheprd is actually using.

## Failure Behavior

Failures should be understandable and should not mutate Herdr state when the
project cannot be resolved or the target path is not a git repository. Existing
paths must contain `.git`; pass a repository root, not an arbitrary directory.

Examples:

```bash
sheprd connect definitely-not-a-project --no-attach
sheprd connect /tmp/not-a-repo --no-attach
sheprd init
sheprd init
```

The second `init` should fail unless `--force` is explicit.

With `--json`, runtime failures use the structured error envelope above. Clap
argument errors remain Clap's standard human help output.
