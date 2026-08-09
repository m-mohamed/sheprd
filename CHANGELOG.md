# Changelog

## Unreleased

## v0.3.0 - 2026-08-09

### Added

- Added `sheprd factory run`, a deterministic Pi-plan/Codex-build/Rust-check/Claude-intent-review/OpenCode-adversarial-review workflow with typed sentinel envelopes, two bounded correction turns, explicit path scope, append-only JSONL phase traces, final JSON receipts, and fail-closed acceptance.

### Changed

- Updated the default OpenCode model to `opencode-go/deepseek-v4-flash`; factory workflow code continues to consume configured Flok agents without model IDs.
- Hardened factory turns with fresh nonce-bound envelopes, stale-worker rejection, bounded prompts and checks, ignored-state attribution, immutable review-source snapshots, mutation detection, and owner-only state artifacts.

## v0.2.1 - 2026-07-26

### Fixed

- Keep Flok receipts and worker checkouts under the stable Sheprd state root so
  standalone CLI and managed Herdr plugin actions share the same Flok.
- Read legacy `v0.2.0` plugin-scoped state during focus and cleanup so existing
  Floks remain recoverable after upgrading.

## v0.2.0 - 2026-07-25

### Added

- Added the Herdr-native `m-mohamed.sheprd` manifest with open, picker, doctor,
  cleanup-preview, and typed-confirmation cleanup actions.
- Added the zoomable 2x2 Pi/Codex/Claude Code/OpenCode Flok with exact
  high-effort model defaults and isolated worker branches/worktrees.
- Added per-project operation locking, atomic state receipts, live four-agent
  health checks, partial-failure rollback, and dirty-state preservation.
- Added preview-first cleanup that validates path ownership, rechecks Git state
  after workspace close, preserves branches, and archives receipts.
- Added exact-version prebuilt installation with enforced SHA-256 and GitHub
  provenance verification, locked source fallback, hermetic installer tests,
  four-target releases, and build provenance attestations.
- Added Linux/macOS CI, ShellCheck, weekly advisory/license/source audits, a
  security policy, a pinned Rust toolchain, and commit-pinned workflow actions.

### Changed

- Reframed Sheprd around the tagline “keep every coding agent in frame.”
- Reduced the supported agent set to Pi, Codex CLI, Claude Code, and OpenCode.
- Made Flok the primary product surface while retaining legacy `connect` and
  sample recipes for compatibility.
- Replaced copied Herdr commercial-license language with Sheprd's actual
  AGPL-3.0-or-later notice.

### Fixed

- Prioritize configured project names over same-named local directories when
  resolving `sheprd connect <project>`.
- Use Herdr's injected `HERDR_BIN_PATH` instead of assuming `herdr` is on PATH.
- Reject untrusted release-origin overrides and remove clean worktrees without
  bypassing Git's dirty-state guard.
- Upgraded `crossbeam-epoch` to the line fixed for RUSTSEC-2026-0204.

## v0.1.0 - 2026-07-03

### Added

- Started `sheprd` as a Herdr-native project-to-workspace entry companion.
- Added project discovery, config loading, `connect`, `list`, `recipes`, `doctor`, and `show-config`.
- Added a first bundled sample recipe, `agent-dev`.
- Added `connect --json` for non-interactive agent/script connection outcomes.
- Added `init` to preview or write starter config safely.
- Added prelaunch Just gates, a pull request template, and stronger agent
  maintainer guidance.
- Strengthened CI and release workflow static smoke coverage.
- Added typed Herdr protocol readiness fields to `doctor --json`.
- Added concise human outcome output for non-JSON `connect`.
- Added a dedicated command reference.
- Added structured JSON error envelopes for runtime failures after argument
  parsing.
- Added JSON failure smoke checks to local and GitHub prelaunch gates.
- Added `agent-guide.md` for agents helping humans learn, set up, or
  troubleshoot Sheprd.
- Added a docs index and linked docs surface from the website.
- Added GitHub templates and local hooks to packaged support surfaces.
- Added a website docs landing page for the public docs surface.
