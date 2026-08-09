# Sheprd

<p align="center">
  <img src="website/assets/sheprd-mark.svg" alt="Sheprd" width="96" />
</p>

<p align="center"><strong>Keep every coding agent in frame.</strong></p>

<p align="center">
  <a href="https://github.com/m-mohamed/sheprd/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/m-mohamed/sheprd/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://github.com/m-mohamed/sheprd/actions/workflows/audit.yml"><img alt="Audit" src="https://github.com/m-mohamed/sheprd/actions/workflows/audit.yml/badge.svg"></a>
  <a href="https://github.com/m-mohamed/sheprd/releases"><img alt="Release" src="https://img.shields.io/github/v/release/m-mohamed/sheprd"></a>
  <a href="LICENSE"><img alt="License: AGPL-3.0-or-later" src="https://img.shields.io/badge/license-AGPL--3.0--or--later-7aa2f7"></a>
</p>

Sheprd is an opinionated [Herdr](https://herdr.dev/) plugin for multi-agent
coding. Its flagship workflow, **Flok**, opens exactly four visible agents in
one zoomable workspace:

| Pane | Responsibility | Default model and effort |
| --- | --- | --- |
| Pi | conductor in the clean base checkout | `openai-codex/gpt-5.6-sol` · `high` |
| Codex CLI | implementation worker | `gpt-5.6-sol` · `high` |
| Claude Code | implementation/review worker | `claude-opus-5` · `high` |
| OpenCode | open-model worker | `opencode-go/deepseek-v4-flash` · `high` |

```text
Pi · conductor       │ Codex · GPT-5.6 Sol
─────────────────────┼──────────────────────
Claude · Opus 5      │ OpenCode · DeepSeek V4 Flash
```

Pi coordinates through Herdr's supported `agent prompt`, `agent wait`, and
`agent read` commands. Each worker starts on its own branch in an isolated Git
worktree. All four panes remain ordinary Herdr panes, so Herdr still owns focus,
zoom, persistence, detach/reattach, remotes, agent state, and the socket API.

## Why this exists

Herdr's marketplace already has excellent generic project pickers, declarative
layout engines, worktree tools, and remote clients. Sheprd does not try to
replace them. It owns one explicit cross-harness contract:

```text
Git project → visible Flok → isolated workers → evidence-backed result
```

Sheprd is the product name. Flok is the four-agent mode.

## Requirements

- Herdr `0.7.5` or newer on macOS or Linux
- Git
- authenticated `pi`, `codex`, `claude`, and `opencode` CLIs
- access to the configured models through your own accounts/subscriptions
- `curl` plus GitHub CLI for a verified prebuilt, or Rust `1.92+` for the locked
  source-build fallback

`sheprd doctor` verifies executables and the Herdr runtime boundary. It cannot
verify provider credits, billing, rate limits, or model entitlement before an
agent sends a request.

## Install

Herdr previews the manifest and build command before confirmation:

```bash
herdr plugin install m-mohamed/sheprd
herdr plugin action list --plugin m-mohamed.sheprd
herdr plugin action invoke m-mohamed.sheprd.doctor
```

Managed installation downloads a prebuilt binary for the exact manifest
version, verifies its SHA-256 checksum, and uses GitHub CLI to verify its build
provenance. If a matching asset, `curl`, or `gh` is unavailable, the installer
says so and falls back to `cargo build --release --locked`. A checksum or
provenance mismatch is a hard failure. See
[`SECURITY.md`](SECURITY.md) for the complete trust boundary.

Pin a revision when you want a reproducible source checkout:

```bash
herdr plugin install m-mohamed/sheprd --ref v0.2.1
```

## Open a Flok

From a Herdr pane inside a clean Git checkout:

```bash
herdr plugin action invoke m-mohamed.sheprd.open-flok
```

Or choose from configured projects:

```bash
herdr plugin action invoke m-mohamed.sheprd.choose-flok
```

Sheprd refuses to create a new Flok from a dirty base checkout. If a matching
workspace already exists, Sheprd focuses it and reports its health; it never
silently repairs or reshapes live panes.

Recommended project-picker keybinding:

```toml
[[keys.command]]
key = "prefix+p"
type = "plugin_action"
command = "m-mohamed.sheprd.choose-flok"
description = "choose project and open Flok"
```

## Plugin actions

| Action | Behavior |
| --- | --- |
| `m-mohamed.sheprd.open-flok` | Open or focus Flok for the active project |
| `m-mohamed.sheprd.choose-flok` | Open the project picker overlay |
| `m-mohamed.sheprd.doctor` | Log a non-mutating readiness report |
| `m-mohamed.sheprd.cleanup-preview` | Log a non-mutating cleanup preview |
| `m-mohamed.sheprd.cleanup-flok` | Open a typed-confirmation cleanup overlay |

The cleanup overlay refuses dirty or out-of-scope worker paths, closes the Herdr
workspace before removing checkouts, preserves all worker branches, and moves
the state receipt into plugin history.

## Run the factory

From a source checkout, run one bounded task through an existing or newly
created Flok:

```bash
target/release/sheprd factory run my-app --task "add retry metrics" \
  --allow-path src/metrics.rs --check "cargo test metrics" --json
```

Rust owns the sequence: typed Pi plan, Codex implementation, caller-declared checks, at most two Codex corrections, Claude intent review, then OpenCode adversarial review. Each agent turn uses a fresh nonce-bound JSON envelope. Acceptance requires passing checks and approval from both reviewers.

The factory refuses a stale or dirty Codex checkout, commits, base-checkout drift, agent-authored ignored payloads, and changes outside `--allow-path`. Checks run through `/bin/sh -c` with a documented environment allowlist and a 300-second default timeout, configurable with `--check-timeout-seconds`; timeouts kill the check process group, and any check that mutates reviewed source state fails the run. Check-owned ignored build outputs are allowed but excluded from the reviewed patch. It never merges or pushes. Every attempt writes private append-only `trace.jsonl` and final `receipt.json` files below the stable Sheprd state root; rejected runs return a non-zero exit status and preserve all worker changes for inspection.

## Configuration

Get the Herdr-managed config directory:

```bash
herdr plugin config-dir m-mohamed.sheprd
```

Create `config.toml` there:

```toml
roots = ["~/code", "~/work"]

[[projects]]
name = "my-app"
path = "~/code/my-app"

[flok]
effort = "high"
pi_model = "openai-codex/gpt-5.6-sol"
codex_model = "gpt-5.6-sol"
claude_model = "claude-opus-5"
opencode_model = "opencode-go/deepseek-v4-flash"
```

Resolution order is `SHEPRD_CONFIG`, Herdr's plugin config directory, then the
legacy `~/.config/sheprd/config.toml`. Existing dotfiles do not need to move.

## Safety and recovery

- A new Flok validates Herdr `0.7.5+`, config, all four CLIs, and a clean Git
  checkout before it creates runtime state.
- A per-project operation lock prevents concurrent Flok creation/cleanup.
- State records are written atomically under `SHEPRD_STATE_DIR`, or
  `~/.local/state/sheprd` by default, so CLI and managed plugin actions share
  the same Flok. Legacy `v0.2.0` plugin-scoped state remains readable.
- Partial creation closes the partial workspace, removes only clean temporary
  worktrees, and reports every rollback decision.
- Dirty worktrees are never deleted automatically.
- Cleanup preserves branches even after it removes clean checkouts.
- Pi receives read/search/shell tools but no direct editing tools; workers are
  told not to create hidden subagents.

Use structured output when automating or diagnosing from a source checkout:

```bash
target/release/sheprd doctor --json
target/release/sheprd flok my-app --json
target/release/sheprd cleanup my-app --json
target/release/sheprd cleanup my-app --confirm --json
```

Launch JSON includes live workspace/pane IDs, agent names, models, worktree
paths, branches, health, warnings, and the state receipt. Herdr IDs are live
values; never treat them as durable configuration.

## Development

Herdr skips `[[build]]` for linked plugins, so build the binary first:

```bash
git clone https://github.com/m-mohamed/sheprd
cd sheprd
just check
cargo build --release --locked
herdr plugin link .
herdr plugin action list --plugin m-mohamed.sheprd
```

The full local gate runs formatting, Clippy, the complete test suite, dependency/advisory and
license policy, shell syntax, release build, smoke commands, and crate
packaging. CI repeats the tests on Linux and macOS. Release assets cover macOS
and Linux on x86_64 and aarch64 with SHA-256 sidecars and provenance
attestations.

See the [Docs index](docs/README.md), [Command reference](docs/commands.md),
[`agent-guide.md`](agent-guide.md), [security policy](SECURITY.md), and
[contribution guide](CONTRIBUTING.md).

## License

Sheprd is licensed under [AGPL-3.0-or-later](LICENSE).
