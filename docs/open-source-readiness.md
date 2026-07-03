# Open Source Readiness

This document tracks the standard that keeps `sheprd` aligned with Herdr-quality
open-source expectations.

The goal is not to clone Herdr. The goal is to meet the same standard of clarity:
clear product boundary, reproducible checks, contributor expectations, release
discipline, and documentation that matches the shipped behavior.

For Sheprd, Herdr is the closest benchmark. It has the same
terminal-agent audience and roughly the same stack shape, so the useful lesson is
not to copy features; it is to copy public trust signals: direct README, focused
docs, agent instructions, contribution rules, release discipline, install paths,
and discussion channels.

## Herdr Baseline

Herdr checks these boxes:

| Area | Herdr standard | sheprd status |
| --- | --- | --- |
| Product boundary | Runtime, workspaces, tabs, panes, persistence, remotes, keybindings, integrations, agent state | Defined: `sheprd` is the entry layer only |
| Public README | Clear install, quick start, concepts, comparison, docs links | Present, source-first install |
| Command reference | Dedicated command behavior and JSON contract docs | Added |
| First-run setup | User can preview/write starter config safely | `sheprd init --print`, `sheprd init`, `--force` guard |
| Contributor guide | AI-use rule, issue/discussion routing, PR standards, checks | Present, modeled after Herdr |
| Agent guide | Architecture and validation rules for coding agents | `SKILL.md` plus teaching/troubleshooting guide |
| Changelog | Human-readable release notes | Present |
| License | Public OSS license | Present |
| Local checks | `just ci`, formatting, lint, tests | Present |
| Crate package | `cargo package` should verify cleanly | Present |
| Git hooks | Pre-commit and commit-msg discipline | Present |
| CI | GitHub Actions for fmt, clippy, tests, static CLI smoke, package, install, metadata | Strengthened |
| Metadata smoke | Workflow/template YAML validation | `just metadata-smoke` |
| Dependency updates | Dependabot for Cargo and GitHub Actions | Added |
| Issue templates | Repro-first bug reports | Added |
| PR template | Boundary, proof, docs, and contributor understanding | Added |
| Discussion templates | Ideas and Q&A outside the bug queue | Added |
| Release process | SemVer tags, release commits, release notes | Documented |
| Release automation | Multi-platform builds and GitHub releases | Tag workflow added with static smoke/package checks and support-file artifacts |
| Packaging | Install script, Homebrew, mise, Nix | Source installer added; Homebrew/mise/Nix deferred |
| Docs site | Polished site and versioned docs | One-page site plus HTML docs landing and markdown docs index |
| Protocol/client depth | Socket API, protocol readiness, and agent skill | Agent skill present; `doctor` reports protocol/socket; socket client deferred |
| Automation output | Agents can drive the tool without scraping prose | `connect --json` reports project/workspace/action/recipe/attach outcome; `doctor --json` exposes typed Herdr protocol readiness; runtime failures emit a JSON error envelope |

## Current Public Shape

`sheprd` presents publicly as:

- a Herdr companion, not a Herdr replacement;
- a first-run config bootstrapper;
- a project selector and workspace connector;
- a way to choose an agent lane;
- a preflight doctor for Herdr-based development environments;
- a clean JSON result for agent/script connection flows;
- a small library of optional sample recipes.

As a first public open-source project, the launch bar is clarity over
cleverness: no hidden personal workflow assumptions, no extra Herdr clone
surface, no raw protocol client until it has a public reason.

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
- Added a pull request template that checks Herdr boundary, proof, docs, and
  contributor understanding.
- Added `clippy.toml` for Rust lint discipline.
- Added this readiness scorecard.
- Added a docs index that routes humans and agents to command, product,
  launch, and agent-facing surfaces.
- Added a website docs landing page so the public site does not depend on raw
  markdown links as the primary docs experience.
- Added a command reference covering human output, JSON output, Herdr boundary,
  and failure behavior.
- Added a prelaunch chaos checklist.
- Added a public launch checklist.
- Added a source installer and tag-based release workflow.
- Added `SKILL.md` for agent-facing usage boundaries.
- Added `agent-guide.md` for agents helping humans learn, set up, or
  troubleshoot Sheprd.
- Added Herdr protocol/socket readiness to `doctor` so future native API work has
  an observable gate.
- Added `connect --json` so agents and scripts can see whether Sheprd focused or
  created a Herdr workspace without launching an interactive client.
- Added a typed `herdr` block to `doctor --json` so agents and future clients
  can inspect runtime/protocol readiness without scraping human check details.
- Added a JSON error envelope for runtime failures after argument parsing.
- Added contributor support files to the packaged source and release archives,
  including GitHub templates and local hooks referenced by `just install-hooks`.
- Added `just prelaunch-check`, `just metadata-smoke`, `just install-smoke`, and
  `just live-smoke` so release proof is executable instead of only written
  down.
- Added `init` so users and agents can preview or write starter config without
  hand-authoring TOML.
- Strengthened CI and release workflows so static CLI smoke, JSON failure
  smoke, crate packaging, install smoke, metadata validation, and release
  support files are checked before release.

## Release Gate

Before publishing a release:

1. Run the full prelaunch chaos checklist.
2. Confirm README, CLI help, website, changelog, command reference, and docs
   agree.
3. Compare Sheprd's README, AGENTS.md, SKILL.md, contribution guide, issue
   templates, discussions, release notes, and install docs against Herdr's
   public surfaces.
4. Recreate release notes from `Unreleased`, then tag only after the release
   gate.
5. Run the release workflow against the approved public remote.
6. Confirm release artifacts include the support surface.

## Post-Launch Follow-Ups

- Promote the docs index into a fuller docs site if the one-page website becomes
  too small.
- Add Homebrew, mise, or Nix packaging only after the first release contract is
  settled by real use.

## Deferred On Purpose

Do not add these just to look mature:

- an approval-gate workflow before there are public contributors;
- custom Herdr socket code while CLI wrappers cover the workflow and there is no
  public client/event-subscriber use case;
- a Ratatui picker before the shell commands are boringly stable;
- opinionated personal layouts as default behavior;
- Homebrew, mise, or Nix packaging before the first release contract is settled.
