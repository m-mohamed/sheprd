# Command reference

Sheprd is intentionally small. It routes projects into Herdr workspaces and
checks readiness. The `factory` Ratatui cockpit and HQ Sol/Luna scripts own
operator control and bounded execution.

## Global options

```text
--agent <pi|claude|codex|opencode>
--json
--no-attach
```

JSON commands emit structured output. JSON failures go to stderr as an error
envelope with `ok: false`, `kind`, and `exit_code`.

## `init`

```bash
sheprd init --print
sheprd init --root ~/workspace/current/work
sheprd init --force
```

Creates a small project-discovery configuration. It does not contain model or
worker-fleet settings.

## `list`

```bash
sheprd list
sheprd list --json
```

Discovers configured Git projects and reports whether their Herdr workspace is
running. It does not mutate runtime state.

## `connect`

```bash
sheprd connect <project>
sheprd connect <project> --recipe agent-dev --no-attach --json
```

Resolves a project by configured name or Git path, then focuses an existing
workspace or creates one. `agent-dev` is the explicit sample recipe. Existing
workspaces are never silently reshaped.

## `recipes`

```bash
sheprd recipes
sheprd recipes --json
```

Shows available sample workspace recipes.

## `doctor`

```bash
sheprd doctor
sheprd doctor --json
```

Checks Herdr, Git, the configured project roots, and required editor/runtime
executables. Claude is optional. Retired peer-agent and fleet integrations are
not checked.

## `show-config`

```bash
sheprd show-config
sheprd show-config --json
```

Prints the active project-discovery configuration.

## Sol/Luna workflow

Use HQ, not Sheprd, for bounded parallel work:

```bash
~/workspace/hq/workflows/sol-luna-launch.sh \
  --project <project> \
  --task-id <tuxedo-project-id> \
  --task-number <line> \
  --task "<bounded outcome>" \
  --allow-path <path> \
  --check "<deterministic check>"
```

The launcher requires `HERDR_ENV=1`, a clean base checkout, explicit scope,
checks, fixed budgets, and human acceptance. It creates one Sol conductor and
three visible Luna workers and writes a private receipt.
