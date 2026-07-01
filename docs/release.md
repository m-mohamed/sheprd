# Release Process

`sheprd` follows SemVer with git tags shaped as `v0.1.0`.

The current state is private prelaunch. Do not create release tags, publish
GitHub releases, or change repository visibility until the explicit final ship
gate.

## Pre-Public Root Commit

Before the project is published, the local checkpoint history can be rewritten.
After the prelaunch chaos checklist passes, collapse the private history into one
intentional root commit:

```bash
git reset --soft "$(git rev-list --max-parents=0 HEAD)"
git commit --amend -m "Initial commit"
```

Do not do this after the repository has public consumers.

## Check

```bash
just check
just metadata-smoke
just install-smoke
SHEPRD_INSTALL_DIR=/tmp/sheprd-install scripts/install-local.sh
```

Confirm there are no accidental release tags before the final gate:

```bash
git tag --list 'v*'
gh release list -R m-mohamed/sheprd
```

## Prepare

1. Update `CHANGELOG.md`.
2. Bump `Cargo.toml`.
3. Confirm the Cargo version and intended tag match.
4. Confirm release notes can be extracted:

   ```bash
   just release-notes 0.1.0
   ```

5. Commit with:

   ```bash
   git commit -m "release: v0.1.0"
   ```

6. Tag with:

   ```bash
   git tag -a v0.1.0 -m "v0.1.0"
   ```

## Standard

Do not publish a release where CLI help, README, website, and changelog
disagree.

Release artifacts should include the binary plus `README.md`, `LICENSE`,
`CHANGELOG.md`, `CONTRIBUTING.md`, `AGENTS.md`, `SKILL.md`, `justfile`,
`docs/`, `scripts/`, and `website/` so users can inspect the same support
surface that the README points at.

Do not publish a release unless the repository visibility, release tag, release
notes, downloadable artifacts, discussions, and history cleanup have all been
approved in the final ship gate.
