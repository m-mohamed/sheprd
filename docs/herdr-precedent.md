# Herdr Ecosystem Precedent

Sheprd should feel native to Herdr without copying Herdr or neighboring
plugins. The August 9, 2026 marketplace audit reviewed the complete Herdr index
metadata and then inspected the manifests and source-facing contracts of the
highest-signal overlapping plugins. The index snapshot contained 541 valid
plugin manifests across 537 repositories, nine topic-tagged repositories with
no manifest, zero invalid manifests, and four duplicate manifests.

The catalog is an automatic, unreviewed GitHub topic index. Marketplace
presence is discovery evidence, not a security review. GitHub reported MIT for
389 of 549 active topic repositories, but 141 had no machine-readable license
and the remainder used several other licenses. Sheprd keeps its intentional
AGPL-3.0-or-later license and borrows patterns rather than code.

## Patterns adopted

- **reviewr:** exact-version prebuilt archives, SHA-256 sidecars, draft-until-
  complete releases, provenance attestations, weekly dependency policy.
- **herdr-file-viewer:** manifest comments that explain trust-sensitive
  commands, hermetic installer tests, honest platform declarations, hard
  checksum behavior, cross-platform CI.
- **Herdr Plus:** an end-to-end plugin action smoke surface and clear separation
  between host context and plugin behavior.
- **Workspace Manager:** startup/state maintenance, idempotence, and preview-
  first destructive workflows.
- **Sessionizer and Spreader:** focused project-selection/layout jobs rather
  than runtime ownership.
- **Command Palette:** discover existing plugin actions instead of inventing a
  second registry, while preserving the origin workspace context.
- **Crabbox:** dedicated doctor panes and explicit remote-job boundaries.

Sheprd's corresponding standard is:

- manifest and scripts readable during Herdr's install preview;
- `HERDR_BIN_PATH` for runtime calls;
- exact `min_herdr_version` and platform claims;
- reproducible locked builds plus verified release binaries;
- adversarial tests for rollback, dirty-state preservation, injected Herdr
  path, old runtime rejection, and cleanup confirmation;
- public security, contribution, changelog, release, and operator surfaces.

The complete index also shows why Sheprd must remain narrow. Keyword groups
overlap, but 281 manifests mention agent workflows, 164 workspace or layout
work, 118 navigation, 98 review or files, 90 Git or worktrees, 59 status or
observability, and 45 remote or notification surfaces. Sheprd owns only the
opinionated four-agent Flok, isolated worker worktrees, and evidence receipts.

## Patterns deliberately not copied

- generic fuzzy project/session management;
- arbitrary declarative layouts;
- file review/viewer UI;
- remote/mobile clients;
- automatic worktree deletion;
- raw socket code while Herdr CLI wrappers express the workflow.

## Herdr boundary

Use wrapper commands first:

```bash
herdr workspace list
herdr workspace create --cwd PATH --label LABEL --focus
herdr pane split PANE_ID --direction right --cwd PATH --no-focus
herdr agent start NAME --kind KIND --pane PANE_ID -- ARGS...
herdr agent prompt NAME PROMPT --wait --timeout 120000
herdr agent read NAME --source recent-unwrapped --lines 120 --format text
```

Read `herdr --skill` before controlling a session and require `HERDR_ENV=1`.
Sheprd's minimum remains 0.7.5 because that release introduced the live-agent
facade, global plugin registry, startup hooks, and reliable prompt waits it
uses. Herdr 0.8.0 is the current verified runtime and command authority.

Raw socket work requires a use case the wrappers cannot express, plus protocol
gates and tests that do not require a live session.
