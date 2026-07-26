# Open-source Readiness

This is the public trust scorecard for the Sheprd Herdr plugin.

| Gate | Contract | Evidence |
| --- | --- | --- |
| Product boundary | Herdr runtime, Sheprd Flok workflow | README, AGENTS, product foundation |
| Install preview | Plain manifest argv and documented scripts | `herdr-plugin.toml`, manifest test |
| Managed install | Exact version, SHA-256, provenance, locked fallback | installer plus four hermetic tests |
| Platform honesty | macOS/Linux; x86_64/aarch64 assets | manifest and release matrix |
| Runtime portability | Host-provided Herdr binary | `HERDR_BIN_PATH` test |
| Mutation safety | Clean preflight, per-project lock, atomic state | Flok integration tests |
| Failure recovery | Close first, remove only clean worktrees | clean and dirty rollback tests |
| Cleanup | Preview/typed confirm, path scope, branch preserve | cleanup integration tests |
| Existing state | Focus only; health/warnings, no repair | degraded-focus test |
| Dependency policy | Advisories, licenses, sources, duplicates | `cargo deny check`, weekly workflow |
| CI | fmt, Clippy, ShellCheck, tests on Linux/macOS, package, smoke | CI workflow |
| Release integrity | pinned actions, version agreement, exact notes, checksums, attestations | release workflow |
| Security disclosure | private advisory path and trust boundaries | SECURITY.md |
| Public hygiene | no maintainer paths, secrets, or private task systems | source scan and docs tests |

## Current intentional limits

- no Windows claim;
- no Homebrew, mise, or Nix packaging before marketplace use justifies it;
- no generic layout language or general sessionizer surface;
- no raw socket client while Herdr CLI wrappers cover the workflow;
- no promise that provider billing or named models remain available;
- legacy `connect` and sample recipes remain supported but secondary.

## Release gates

1. `just check` and `git diff --check` pass.
2. The manifest links and every action resolves in live Herdr `0.7.5`.
3. A disposable clean repository creates, focuses, previews cleanup, and cleans
   without leaked worktrees or workspace state.
4. A real repository is iterated through the visible Pi/worker flow with a
   repository commit and test receipt.
5. Independent review finds no unresolved correctness, security, docs, or
   packaging issue.
6. Public visibility, `v0.2.1`, release assets, checksums, attestations, clean
   public install, `herdr-plugin` topic, and marketplace listing are verified
   as separate receipts.
