# Command Reference

The `sheprd` binary implements the Herdr plugin. Daily human use should prefer
the manifest actions; the binary remains the testable JSON and recovery
surface. Managed plugin installs do not add it to the global `PATH`.

Herdr owns sessions, workspaces, tabs, panes, remotes, persistence, keybindings,
integrations, attach/detach, and agent state. Sheprd calls Herdr through the
injected `HERDR_BIN_PATH` and treats every Herdr ID as live runtime state.

## Global options

```text
--agent <pi|claude|codex|opencode>
--json
--no-attach
```

`--agent` applies to the legacy single-lane workspace commands and sample
recipes. It does not change the four fixed Flok roles.

Runtime failures after argument parsing emit a JSON envelope on stderr when
`--json` is set:

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

Clap argument errors retain Clap's normal help output.
Automation should consume `error.kind`, `error.message`, and `error.exit_code`
instead of parsing the rendered message.

## `flok`

```bash
sheprd flok
sheprd flok my-app
sheprd flok /path/to/repository --json
```

With no selector, Sheprd resolves `focused_pane_cwd` or `workspace_cwd` from
`HERDR_PLUGIN_CONTEXT_JSON`, then falls back to the current directory.

A new Flok performs these gates and mutations in order:

1. verify Herdr `0.7.5+` and acquire a per-project operation lock;
2. validate non-empty model/effort config and all four CLIs;
3. refuse a dirty base checkout;
4. create three worker branches and worktrees under plugin state;
5. create the Herdr 2x2 workspace and start exactly four named agents;
6. verify the live roster is exactly four and interactive-ready;
7. atomically write the state receipt and focus the workspace.

Partial failure closes a created workspace first, then removes only worker
checkouts still verified clean. Dirty state and branches are preserved. The
error reports every rollback decision.

If the labeled workspace already exists, Sheprd only focuses it. It loads the
saved receipt, compares it with `herdr agent list`, and returns `healthy` plus
`warnings`; it never repairs or reshapes the live workspace implicitly.

Successful JSON includes:

- `schema_version`
- `action`: `created_flok` or `focused_existing`
- `project`, `workspace_id`, and `workspace_label`
- `state_path`
- four `agents` with role, kind, name, pane, model, effort, cwd, and branch
- `healthy` and `warnings`

## `factory run`

```bash
sheprd factory run my-app --task "add retry metrics" \
  --plan-file plan.json --allow-path src/metrics.rs \
  --check "cargo test metrics" --json
```

Pi must create the typed JSON plan before this command starts. The command opens
or focuses the project Flok, then runs this safety-owned execution protocol:

1. Sheprd validates the supplied plan against the caller's allow paths;
2. Codex implements in its existing isolated worker checkout and returns an
   `implementation` envelope;
3. Rust runs every caller-declared check command in that checkout;
4. failed checks may cause at most two Codex correction turns;
5. Claude returns an intent-review `review` envelope;
6. OpenCode returns an adversarial-review `review` envelope;
7. Rust accepts only if checks pass and both reviews approve.

`--plan-file`, at least one `--allow-path`, and at least one `--check` are
required. The plan is a schema-1 `plan` envelope with a non-empty summary and
one or more bounded steps. Allow paths are
repository-relative files or directories. The Codex worker's initial HEAD must
equal the base checkout's initial HEAD. Before every transition, Sheprd verifies
that the base checkout and Codex worker HEAD have not changed and that actual
changed paths remain in scope. Review checkouts must also remain unchanged. The
Codex checkout must be clean before the run, making attribution unambiguous.

Each check uses `/bin/sh -c`, not a login shell. Sheprd clears the environment,
then inherits only `CARGO_HOME`, `HOME`, `LANG`, `LC_ALL`, `LOGNAME`, `PATH`,
`RUSTC_WRAPPER`, `RUSTUP_HOME`, `TERM`, `TMPDIR`, and `USER`, and sets
`SHEPRD_FACTORY_CHECK=1`; if `PATH` is absent it uses `/usr/bin:/bin`. The default
per-check timeout is 300 seconds and can be changed with
`--check-timeout-seconds`. A timeout kills the check process group. Sheprd
snapshots tracked and non-ignored untracked source state around every check and
fails if the check mutates it.

Sheprd also snapshots bounded ignored-path metadata around every agent turn.
Codex, Claude, and OpenCode may not create or modify ignored payloads.
The snapshot fails closed above 100,000 entries, 16 MiB of enumerated path
data, or 64 GiB of apparent ignored-file size.
Caller-declared checks may create ignored build outputs; Sheprd adopts their
post-check ignored state as the baseline before a correction. Those check-owned
ignored outputs are excluded from the reviewed source patch. The exact worker
source snapshot used for that patch must remain unchanged through both reviews,
and claimed paths are checked against Git again afterward.

The runner never commits, merges, pushes, or deletes worker state. Failed and
rejected runs return a non-zero status and preserve changes. Each run creates an
append-only `trace.jsonl` plus an atomic `receipt.json` under
`$SHEPRD_STATE_DIR/factory`, or `~/.local/state/sheprd/factory`. The receipt
includes a version plus started/finished Unix-millisecond timestamps, monotonic
elapsed milliseconds, implementation and check-attempt counts, envelopes,
actual paths, integrity verdicts, explicit review and acceptance outcomes,
failure stage, failure reason, cost availability, and both state paths. Factory state
directories are owner-only; trace, receipt, temporary, and lock files are mode
`0600` on Unix. Review patches are capped at 48 KiB and every agent prompt is
capped at 60 KiB before Herdr execution. Workflow code uses the agents already
configured in the Flok and contains no model IDs.

## `factory stats`

```bash
sheprd factory stats
sheprd factory stats my-app --json
```

This command resolves the project like `factory run` and reads its receipts
without creating directories, locks, repairs, or other state. Human and JSON
output aggregate total, accepted, and rejected runs; acceptance as
`accepted / total`; corrections as `runs with more than one implementation
turn / runs that reached implementation`; total check attempts; failure stages;
and elapsed-runtime coverage and total.

`total_runs` and all metric denominators include only completed, validated
receipts. A stable private run directory without `receipt.json` is reported
separately as `incomplete_runs`; it is never inferred accepted or rejected.

Receipt schema 2 supplies all observability fields. Schema-1 receipts written
by v0.3.1 remain valid: their acceptance and attempt data are counted, their
rejected failure stage is grouped as `legacy_unknown`, and their runtime is not
included in runtime coverage. Zero receipts produce zero denominators and
`unavailable` coverage rather than a synthetic rate.

Cost coverage is `unavailable`, `partial`, or `complete`. A cost enters totals
only when a receipt declares typed authoritative data with a non-empty provider
source, currency, integer minor-unit amount, and decimal scale. Sheprd does not
infer token usage, convert currencies, or estimate dollar cost. Totals remain
separate by currency and scale.

Statistics require stable 0700 state/run directories and 0600 regular receipt
and trace files under one owner. Malformed JSON, unsupported versions, missing
artifacts, symlinks, unsafe permissions, inconsistent duplicated fields,
active temporary/lock state, arithmetic overflow, or metadata changes during a
read fail the entire command; no suspect receipt is silently omitted. A 0600
regular `factory.lock` with a stable, valid PID is checked read-only: a live PID
blocks statistics, a PID proven dead is treated as stale without deleting the
lock, and malformed, symlinked, unsafe, changing, or unverifiable locks fail.

## `cleanup`

Preview is the default:

```bash
sheprd cleanup
sheprd cleanup my-app --json
```

Mutation requires an explicit flag:

```bash
sheprd cleanup my-app --confirm
sheprd cleanup /path/to/repository --confirm --json
```

Cleanup reads the project receipt, verifies every worker path belongs under
Sheprd's plugin state root, and checks Git cleanliness. If any path is dirty or
out of scope, `can_cleanup` is false and the workspace is not closed.

After confirmation, Sheprd closes the matching workspace, checks cleanliness
again, removes clean worker checkouts, preserves their branches, and moves the
state receipt into plugin history. If a checkout becomes dirty during shutdown,
it is preserved.

The Herdr actions are:

- `m-mohamed.sheprd.cleanup-preview`: headless JSON preview, never confirms;
- `m-mohamed.sheprd.cleanup-flok`: overlay preview requiring the project name.

JSON includes `confirmed`, `can_cleanup`, `workspace_closed`,
`state_archived_to`, per-worktree `exists`/`clean`/`removed` fields, and
`warnings`.

## `doctor`

```bash
sheprd doctor
sheprd doctor --json
```

Checks the effective Herdr binary, Git, Pi, Codex, Claude Code, OpenCode, and
the Herdr server. JSON returns `ready`, `checks`, and a typed `herdr` block:

- `running`
- `version`
- `protocol`
- `compatible`
- `socket`
- `protocol_ready`
- `error`

Read `herdr.protocol_ready` rather than scraping the human checklist. Doctor
cannot verify model billing, credits, entitlement, or a provider's availability.

## `init`

```bash
sheprd init --print
sheprd init --root ~/code --root ~/work
sheprd init --force
sheprd init --print --json
```

`init --print` writes nothing. `init` refuses to overwrite an existing config
unless `--force` is explicit. JSON reports `path`, `existed`, `written`,
`default_agent`, `roots`, and `contents` for a print operation.

For managed plugins, prefer editing `config.toml` under:

```bash
herdr plugin config-dir m-mohamed.sheprd
```

## `list`

```bash
sheprd list
sheprd list --json
sheprd --agent claude list
```

Lists discovered projects and legacy single-lane workspace labels. `running`
only means a matching Herdr label exists; it is not repository or agent health.

## `show-config`

```bash
sheprd show-config
sheprd show-config --json
```

Shows the effective config path, roots, explicit projects, ignore list,
discovery depth, default legacy lane, and Flok model defaults. Use this when
project resolution is surprising.

## Legacy `connect`

The pre-Flok single-workspace entry path remains for compatibility:

```bash
sheprd connect my-app
sheprd open my-app
sheprd switch my-app
sheprd connect my-app --json
```

It creates or focuses a labeled Herdr workspace without forcing layout. JSON
reports project, lane, action, `workspace_id`, optional recipe, and `attached`.
JSON mode never launches a client.

## Legacy `connect --recipe agent-dev`

```bash
sheprd connect my-app --recipe agent-dev --no-attach
```

This opt-in sample shapes only a newly created workspace with editor, selected
agent, shell, and Git panes. Reconnecting focuses an existing workspace and
does not reshape live panes.

## `recipes`

```bash
sheprd recipes
sheprd recipes --json
```

Recipes are compatibility examples, not Flok policy.

## Failure behavior

- An unresolved selector or existing non-repository path fails before Herdr
  mutation. A repository path must contain `.git`; it cannot be an arbitrary
  directory.
- New Flok creation fails before worktree creation when the base is dirty,
  Herdr is too old, a required CLI is missing, or model config is empty.
- Concurrent Flok operations for one project fail with an operation-lock error.
- Rollback and cleanup never delete a worktree that cannot be proven clean.
- `--json` reports machine-readable runtime failures; it does not convert Clap
  usage errors into JSON.
