# Release Process

`sheprd` follows SemVer with git tags shaped as `v0.1.0`.

## Check

```bash
just ci
```

## Prepare

1. Update `CHANGELOG.md`.
2. Bump `Cargo.toml`.
3. Confirm the Cargo version and intended tag match.
4. Commit with:

   ```bash
   git commit -m "release: v0.1.0"
   ```

5. Tag with:

   ```bash
   git tag -a v0.1.0 -m "v0.1.0"
   ```

## Standard

Do not publish a release where CLI help, README, and changelog disagree.
