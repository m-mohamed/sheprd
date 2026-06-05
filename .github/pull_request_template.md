## Summary

What changed, and why does it belong in `sheprd`?

## Herdr Boundary

- [ ] This keeps Herdr as the runtime owner.
- [ ] This does not add terminal multiplexer, keybinding, persistence, remote,
      or agent-status ownership to `sheprd`.
- [ ] Any recipe behavior is explicit and does not reshape existing live
      workspaces.

## Proof

Paste the checks you ran:

```bash
just ci
```

For user-facing changes, also include the relevant output from:

```bash
just check
sheprd connect <project> --json
```

## Docs

- [ ] CLI help, README, `SKILL.md`, website, and docs agree.
- [ ] `CHANGELOG.md` is updated when behavior changes.
- [ ] No private machine paths, task-system notes, or personal workflow
      assumptions leaked into public files.

## Contributor Check

- [ ] I understand the code I am submitting.
- [ ] If this is a feature or product-direction change, it was discussed first.
