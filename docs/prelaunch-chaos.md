# Prelaunch Chaos Checklist

Run this before treating the repository as publish-ready.

The point is to prove the actual user flow, not just compile the code.

## Static Gates

```bash
just check
just metadata-smoke
just install-smoke
target/release/sheprd --help
target/release/sheprd init --print
target/release/sheprd init --print --json
target/release/sheprd connect --help
target/release/sheprd recipes
target/release/sheprd recipes --json
target/release/sheprd show-config
target/release/sheprd connect "$PWD" --json
target/release/sheprd connect definitely-not-a-project --json
SHEPRD_INSTALL_DIR=/tmp/sheprd-install scripts/install-local.sh
/tmp/sheprd-install/sheprd --version
```

Expected result: commands succeed, help text says sample recipes, `init --print`
emits valid starter config without writing, and output matches README, command
reference, website, and changelog language. `cargo package` should also verify
the crate package without warnings. `connect --json` must report the project,
selected agent, Herdr workspace label/id, action, recipe status, and
`attached: false`. Runtime failures after argument parsing must emit a JSON
error envelope on stderr.

The shortcut for the full local, install, and live smoke pass is:

```bash
just prelaunch-check
```

## Herdr Runtime Gates

With Herdr running:

```bash
herdr status
target/release/sheprd doctor
target/release/sheprd list
target/release/sheprd connect "$PWD" --no-attach
target/release/sheprd connect "$PWD" --json
target/release/sheprd connect "$PWD" --recipe agent-dev --no-attach
tmp_repo="$(mktemp -d /tmp/sheprd-recipe.XXXXXX)"
git -C "$tmp_repo" init
target/release/sheprd connect "$tmp_repo" --recipe agent-dev --json
```

Or run:

```bash
just live-smoke "$PWD"
```

Expected result:

- `doctor` reports Herdr, Neovim, Lazygit, and the selected agent.
- plain `connect` creates or focuses a workspace without forcing a layout.
- `--recipe agent-dev` applies the sample layout only when it creates a fresh
  workspace.
- repeated connects reuse the existing workspace instead of duplicating state or
  reshaping live panes.
- JSON mode is non-interactive and reports whether it focused an existing
  workspace or created a new one.

## Failure Gates

```bash
target/release/sheprd connect /tmp/not-a-repo --no-attach
target/release/sheprd connect definitely-not-a-project --no-attach
target/release/sheprd --agent missing-agent doctor
tmp_home="$(mktemp -d /tmp/sheprd-home.XXXXXX)"
SHEPRD_CONFIG="$tmp_home/config.toml" target/release/sheprd init
SHEPRD_CONFIG="$tmp_home/config.toml" target/release/sheprd init
```

Expected result: failures are understandable and do not mutate Herdr state.
The second `init` must fail unless `--force` is explicit.
With `--json`, runtime failures should use the structured error envelope.

## Terminal Gates

Manually verify in Ghostty:

1. Open a normal shell.
2. Run `sheprd connect "$PWD"` from this repository.
3. Confirm it attaches to Herdr once.
4. From inside Herdr, run `sheprd connect "$PWD" --no-attach`.
5. Confirm no nested Herdr attach is attempted.
6. Detach and reattach using Herdr's own flow.

## Docs Gates

Before the public root commit:

```bash
rg -n --glob '!docs/prelaunch-chaos.md' "built-in|coding layout|small startup|recipe optional" README.md CONTRIBUTING.md AGENTS.md CHANGELOG.md docs website src tests
rg -n --glob '!docs/prelaunch-chaos.md' "sample recipe|agent-dev|Herdr owns|sheprd owns" README.md CONTRIBUTING.md AGENTS.md docs website src
```

Expected result: public language says `sample recipe`, not default layout policy.
The command list in `docs/commands.md` should match the CLI help and README.

## History Gate

This historical first-launch step only applied before the first public push.
Do not rewrite public history after users exist.

```bash
git reset --soft "$(git rev-list --max-parents=0 HEAD)"
git commit --amend -m "Initial commit"
```
