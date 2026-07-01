# Changelog

## Unreleased

### Added

- Started `sheprd` as a fresh Herdr-native smart session manager.
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
