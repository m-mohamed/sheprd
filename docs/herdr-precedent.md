# Herdr Ecosystem Precedent

Sheprd should feel native to Herdr without copying Herdr or neighboring
plugins. The July 2026 marketplace review used high-signal community plugins as
implementation benchmarks, not feature shopping lists.

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

Sheprd's corresponding standard is:

- manifest and scripts readable during Herdr's install preview;
- `HERDR_BIN_PATH` for runtime calls;
- exact `min_herdr_version` and platform claims;
- reproducible locked builds plus verified release binaries;
- adversarial tests for rollback, dirty-state preservation, injected Herdr
  path, old runtime rejection, and cleanup confirmation;
- public security, contribution, changelog, release, and operator surfaces.

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
herdr agent prompt NAME PROMPT
herdr agent wait NAME --until idle --timeout 120000
herdr agent read NAME --source recent --lines 120 --format text
```

Raw socket work requires a use case the wrappers cannot express, plus protocol
gates and tests that do not require a live session.
