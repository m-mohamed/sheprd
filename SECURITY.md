# Security

Sheprd is a Herdr plugin, so its build and runtime commands execute as the
current user. Review `herdr-plugin.toml` and the scripts it names before
installing any plugin, including this one.

## Install integrity

Managed installs select release assets from the exact version declared in
`herdr-plugin.toml`. `scripts/install-plugin.sh` verifies the published SHA-256
before installing a prebuilt binary. A checksum mismatch is a hard failure. If
an asset or download tool is unavailable, the script says so and performs
`cargo build --release --locked` from the cloned source instead.

Release archives receive GitHub build-provenance attestations. After a release,
you can verify an archive with:

```bash
gh attestation verify sheprd-TARGET.tar.gz --repo m-mohamed/sheprd
```

## Runtime boundaries

- Sheprd calls Herdr through `HERDR_BIN_PATH` when the plugin host provides it.
- A new Flok requires a clean base checkout and creates one isolated worktree
  per worker.
- Partial creation rolls back only checkouts that are still clean. Dirty state
  is preserved and reported.
- The headless cleanup action is preview-only. The interactive Herdr popup
  requires the project name; the CLI requires `--confirm`. Both refuse dirty or
  out-of-scope paths, preserve worker branches, and archive the state receipt.
- Herdr does not sandbox plugins. Sheprd does not claim otherwise.

## Reporting a vulnerability

Do not open a public issue for a suspected vulnerability. Use GitHub's private
security-advisory flow for `m-mohamed/sheprd` and include the affected version,
reproduction steps, impact, and any suggested mitigation. Please avoid sharing
secrets, private repository contents, or agent transcripts in the report.
