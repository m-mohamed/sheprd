## Summary

What changed, and why does it belong in `sheprd`?

## Herdr Boundary

- [ ] This keeps Herdr as the runtime owner.
- [ ] This does not add terminal multiplexer, keybinding, persistence, remote,
      or agent-status ownership to `sheprd`.
- [ ] This keeps the visible roster at exactly Pi, Codex, Claude Code, and
      OpenCode; no hidden subagents were added.
- [ ] Herdr workspaces are focused/inspected, not silently reshaped or repaired.
- [ ] Cleanup cannot delete unrelated dirty worktrees or repository WIP.
      Sheprd's owned state root.

## Proof

Paste the checks you ran:

```bash
just ci
```

For user-facing changes, also include the relevant output from:

```bash
just check
target/release/sheprd connect <project> --no-attach --json
target/release/sheprd doctor --json
```

## Docs

- [ ] CLI help, README, `SKILL.md`, website, and docs agree.
- [ ] `CHANGELOG.md` is updated when behavior changes.
- [ ] No private machine paths, task-system notes, or personal workflow
      assumptions leaked into public files.

## Contributor Check

- [ ] I understand the code I am submitting.
- [ ] If this is a feature or product-direction change, it was discussed first.
