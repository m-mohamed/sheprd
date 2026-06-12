# Release Process

`sheprd` follows SemVer with git tags shaped as `v0.1.0`.

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
SHEPRD_INSTALL_DIR=/tmp/sheprd-install scripts/install-local.sh
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
