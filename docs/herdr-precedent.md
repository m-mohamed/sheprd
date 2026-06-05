# Herdr Precedent

`sheprd` should feel native to the Herdr ecosystem without pretending to be
Herdr.

What we mirror:

- small conventional commits;
- `vX.Y.Z` tags;
- release commits named `release: vX.Y.Z`;
- `CHANGELOG.md`;
- `CONTRIBUTING.md`;
- `LICENSE`;
- `just ci`;
- GitHub Actions for fmt, clippy, and tests;
- docs that explain product boundaries.

What we do not mirror yet:

- preview/stable update channels;
- multi-platform binary release automation;
- Homebrew formula;
- installer script;
- raw socket event streams;
- Ratatui picker.

Those come after the CLI is stable and there are real public users.

## Herdr Boundary

Use the CLI wrappers first:

```bash
herdr workspace list
herdr workspace create --cwd PATH --label LABEL --focus
herdr workspace focus WORKSPACE_ID
herdr tab create --workspace WORKSPACE_ID --cwd PATH --label LABEL --no-focus
herdr pane split PANE_ID --direction right --cwd PATH --no-focus
herdr pane run PANE_ID COMMAND
```

Only add raw socket code when `sheprd` is doing something the wrappers cannot
express.
