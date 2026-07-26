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
- A new Flok requires a clean base checkout and creates one isolated worktree
  per worker.
- Worker files are isolated by checkout, but Git worktrees intentionally share
  the base repository's object store and refs. A worker with Git write access
  can affect shared branches; review commits before integrating them.
- Pi, Codex, and Claude are launched in non-interactive approval modes suited
  to their assigned checkout. OpenCode keeps its native permission prompts;
  external-path work may block until a human approves it.
- Partial creation rolls back only checkouts that are still clean. Dirty state
  is preserved and reported.
- Cleanup asks Git itself to refuse dirty worktree removal; it does not pass
  `--force`.
- The headless cleanup action is preview-only. The interactive Herdr overlay
  requires the project name; the CLI requires `--confirm`. Both refuse dirty or
  out-of-scope paths, preserve worker branches, and archive the state receipt.
- Herdr does not sandbox plugins. Sheprd does not claim otherwise.

## Reporting a vulnerability

Do not open a public issue for a suspected vulnerability. Use GitHub's private
security-advisory flow for `m-mohamed/sheprd` and include the affected version,
reproduction steps, impact, and any suggested mitigation. Please avoid sharing
secrets, private repository contents, or agent transcripts in the report.
