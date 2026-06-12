# Open Source Readiness

This document tracks the gap between Herdr as a mature open-source project and
`sheprd` as a young companion project.

The goal is not to clone Herdr. The goal is to meet the same standard of clarity:
clear product boundary, reproducible checks, contributor expectations, release
discipline, and documentation that matches the shipped behavior.

## Herdr Baseline

Herdr checks these boxes:

| Area | Herdr standard | sheprd status |
| --- | --- | --- |
| Product boundary | Runtime, workspaces, tabs, panes, persistence, remotes, keybindings, integrations, agent state | Defined: `sheprd` is the entry layer only |
| Public README | Clear install, quick start, concepts, comparison, docs links | Present, source-first, needs real install once packaged |
| Contributor guide | AI-use rule, issue/discussion routing, PR standards, checks | Present, modeled after Herdr |
| Agent guide | Architecture and validation rules for coding agents | Present |
| Changelog | Human-readable release notes | Present |
| License | Public OSS license | Present |
| Local checks | `just ci`, formatting, lint, tests | Present |
| Crate package | `cargo package` should verify cleanly | Present |
| Git hooks | Pre-commit and commit-msg discipline | Present |
| CI | GitHub Actions for fmt, clippy, tests | Present |
| Dependency updates | Dependabot for Cargo and GitHub Actions | Added |
| Issue templates | Repro-first bug reports | Added |
| Discussion templates | Ideas and Q&A outside the bug queue | Added |
| Release process | SemVer tags, release commits, release notes | Documented, not automated |
| Release automation | Multi-platform builds and GitHub releases | Tag workflow added, unproven until public remote |
| Packaging | Install script, Homebrew, mise, Nix | Source installer added; Homebrew/mise/Nix deferred |
| Docs site | Polished site and versioned docs | One-page site only |
| Protocol/client depth | Socket API and agent skill | Agent skill present; socket client deferred |

## Current Public Shape

`sheprd` should launch publicly as:

- a Herdr companion, not a Herdr replacement;
- a project selector and workspace connector;
- a way to choose an agent lane;
- a preflight doctor for Herdr-based development environments;
- a small library of optional sample recipes.

The default command path must stay boring:

```bash
sheprd connect my-project
```

That command should create or focus the matching Herdr workspace and then let
Herdr own the runtime.

Recipes must stay explicit:

```bash
sheprd connect my-project --recipe agent-dev
```

`agent-dev` is a sample recipe, not the product's worldview.

## Closed Now

- Corrected public language from generic "recipes" to optional sample recipes.
- Added issue and discussion templates.
- Added `clippy.toml` for Rust lint discipline.
- Added this readiness scorecard.
- Added a prelaunch chaos checklist.
- Added a public launch checklist.
- Added a source installer and tag-based release workflow.
- Added `SKILL.md` for agent-facing usage boundaries.

## Still Missing Before Public Release

These should be handled before publishing a real public `v0.1.0`:

1. Run the full prelaunch chaos checklist.
2. Confirm README, CLI help, website, changelog, and docs agree.
3. Decide the public GitHub owner and update issue contact links if needed.
4. Rewrite/squash local private history into one polished root commit named
   `Initial commit`.
5. Run the release workflow once against the public remote.
6. Add a real docs site if the one-page site becomes too small.

## Deferred On Purpose

Do not add these just to look mature:

- an approval-gate workflow before there are public contributors;
- custom Herdr socket code while CLI wrappers cover the workflow;
- a Ratatui picker before the shell commands are boringly stable;
- opinionated personal layouts as default behavior;
- Homebrew, mise, or Nix packaging before the first release contract is settled.
