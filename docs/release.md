# Release Process

Sheprd follows SemVer with annotated `vX.Y.Z` tags. The current plugin release
line is `v0.2.1`.

## Prepare

1. Move release entries from `Unreleased` into `## vX.Y.Z - YYYY-MM-DD`.
2. Set the same version in `Cargo.toml` and `herdr-plugin.toml`.
3. Run:

   ```bash
   just check
   just metadata-smoke
   just release-notes X.Y.Z
   cargo package --locked
   git diff --check
   ```

4. Complete the live, disposable, real-project, and public-hygiene gates in
   [prelaunch-chaos.md](prelaunch-chaos.md).
5. Commit with `release: vX.Y.Z` only when the tree is clean and review is
   resolved.

## Tag and publish

```bash
git tag -a vX.Y.Z -m "vX.Y.Z"
git push origin main
git push origin vX.Y.Z
```

The workflow validates version agreement and exact changelog notes, creates a
draft release, builds four target archives, adds SHA-256 sidecars and GitHub
provenance attestations, then publishes only after every target succeeds. Every
workflow action is pinned to a full commit SHA. A failed target leaves a draft
for inspection rather than a partial public release.

Supported release targets:

- `aarch64-apple-darwin`
- `x86_64-apple-darwin`
- `aarch64-unknown-linux-musl`
- `x86_64-unknown-linux-musl`

## Verify

```bash
gh release view vX.Y.Z -R m-mohamed/sheprd
gh release download vX.Y.Z -R m-mohamed/sheprd --pattern 'sheprd-*.tar.gz'
gh attestation verify sheprd-TARGET.tar.gz --repo m-mohamed/sheprd
```

Then perform a clean Herdr install pinned to the tag. Release success is not
marketplace success; add/verify the `herdr-plugin` topic and visible marketplace
card as separate gates.

Never rewrite public history, reuse a release tag, publish mismatched versions,
or treat a draft/partial workflow as shipped.
