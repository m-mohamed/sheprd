# Prelaunch Chaos Checklist

Run this before treating the repository as publish-ready.

The point is to prove the actual user flow, not just compile the code.

## Static Gates

```bash
just check
target/release/sheprd --help
target/release/sheprd connect --help
target/release/sheprd recipes
target/release/sheprd recipes --json
target/release/sheprd show-config
SHEPRD_INSTALL_DIR=/tmp/sheprd-install scripts/install-local.sh
/tmp/sheprd-install/sheprd --version
```

Expected result: commands succeed, help text says sample recipes, and output
matches README, website, and changelog language. `cargo package` should also
verify the crate package without warnings.

## Herdr Runtime Gates

With Herdr running:

```bash
herdr status
target/release/sheprd doctor
target/release/sheprd list
target/release/sheprd connect "$PWD" --no-attach
target/release/sheprd connect "$PWD" --recipe agent-dev --no-attach
tmp_repo="$(mktemp -d /tmp/sheprd-recipe.XXXXXX)"
git -C "$tmp_repo" init
target/release/sheprd connect "$tmp_repo" --recipe agent-dev --no-attach
```

Expected result:

- `doctor` reports Herdr, Neovim, Lazygit, and the selected agent.
- plain `connect` creates or focuses a workspace without forcing a layout.
- `--recipe agent-dev` applies the sample layout only when it creates a fresh
  workspace.
- repeated connects reuse the existing workspace instead of duplicating state or
  reshaping live panes.

## Failure Gates

```bash
target/release/sheprd connect /tmp/not-a-repo --no-attach
target/release/sheprd connect definitely-not-a-project --no-attach
target/release/sheprd --agent missing-agent doctor
```

Expected result: failures are understandable and do not mutate Herdr state.

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

## History Gate

Do not treat the current private checkpoint history as public history. After the
chaos checklist passes, rebuild the public root commit intentionally:

```bash
git reset --soft "$(git rev-list --max-parents=0 HEAD)"
git commit --amend -m "Initial commit"
```

Only do this before the repository is public or before any shared remote depends
on the local commit history.
