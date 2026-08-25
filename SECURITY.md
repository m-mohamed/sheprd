# Security

Sheprd is a Herdr plugin, so its build and runtime commands execute as the
current user. Review `herdr-plugin.toml` and the scripts it names before
installing any plugin, including this one.

## Install integrity

Managed installs select release assets from the exact version declared in
`herdr-plugin.toml`. `scripts/install-plugin.sh` verifies the published SHA-256
and GitHub build provenance before installing a prebuilt binary. A verification
mismatch is a hard failure. If an asset, `curl`, or GitHub CLI is unavailable,
the script says so and performs `cargo build --release --locked` from the cloned
source instead. The release origin is fixed to this repository's HTTPS URL.

Release archives receive GitHub build-provenance attestations. The installer
enforces them when choosing a prebuilt. You can also verify an archive manually:

```bash
gh attestation verify sheprd-TARGET.tar.gz --repo m-mohamed/sheprd
```

## Runtime boundaries

- Sheprd calls Herdr through `HERDR_BIN_PATH` when the plugin host provides it.
- Sheprd only creates or focuses a workspace and applies the explicit
  editor-first recipe; it does not launch a fleet.
- The HQ Sol/Luna launcher creates isolated worker worktrees, requires a clean
  base, explicit allow-paths and checks, and preserves dirty state.
- Worker files are isolated by checkout, but Git worktrees intentionally share
  the base repository's object store and refs; review branches before
  integrating them.
- Herdr does not sandbox plugins. Sheprd does not claim otherwise.

## Reporting a vulnerability

Do not open a public issue for a suspected vulnerability. Use GitHub's private
security-advisory flow for `m-mohamed/sheprd` and include the affected version,
reproduction steps, impact, and any suggested mitigation. Please avoid sharing
secrets, private repository contents, or agent transcripts in the report.
