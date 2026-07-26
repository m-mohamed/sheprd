# Prelaunch Chaos Checklist

Run this against the exact release commit. Keep static, disposable-runtime, real
project, public-release, and marketplace truth separate.

## Static gate

```bash
just check
just metadata-smoke
git diff --check
cargo package --locked
cargo deny check
bash -n scripts/*.sh
```

Expected: the complete test suite plus format/Clippy/package green, no advisory/license/source
failure, and only the documented `winnow` duplicate-version warning.

## Public hygiene gate

```bash
rg -n --hidden --glob '!target/**' --glob '!.git/**' \
  '(Users/[^/]+|BEGIN .*PRIVATE KEY|gh[pousr]_[A-Za-z0-9]{20,}|sk-[A-Za-z0-9_-]{10,})' .
rg -n --hidden --glob '!target/**' --glob '!.git/**' --glob '!docs/prelaunch-chaos.md' \
  '(smart session manager|Herdr is dual-licensed|hey@herdr.dev)' .
```

Expected: no maintainer machine path, token/key, copied Herdr licensing, or old
product framing.

## Live plugin gate

```bash
cargo build --release --locked
herdr plugin unlink m-mohamed.sheprd 2>/dev/null || true
herdr plugin link .
herdr plugin action list --plugin m-mohamed.sheprd
herdr plugin action invoke m-mohamed.sheprd.doctor
herdr plugin log list --plugin m-mohamed.sheprd
```

Expected: Herdr accepts the manifest, lists five actions, and doctor reports
Herdr `0.7.5`, protocol `17`, Git, and four agent CLIs ready.

## Disposable Flok gate

Use an exact temporary path and keep the cleanup receipt:

```bash
tmp_repo="$(mktemp -d /tmp/sheprd-flok.XXXXXX)"
git -C "$tmp_repo" init -q
git -C "$tmp_repo" config user.name "Sheprd Release Test"
git -C "$tmp_repo" config user.email "release-test@example.invalid"
printf '# release fixture\n' > "$tmp_repo/README.md"
git -C "$tmp_repo" add README.md
git -C "$tmp_repo" commit -q -m seed
target/release/sheprd flok "$tmp_repo" --json
target/release/sheprd flok "$tmp_repo" --json
target/release/sheprd cleanup "$tmp_repo" --json
target/release/sheprd cleanup "$tmp_repo" --confirm --json
git -C "$tmp_repo" worktree list --porcelain
```

Expected:

- first launch returns `created_flok`, four agents, `healthy: true`;
- second returns `focused_existing` without new worktrees or pane reshaping;
- preview changes nothing;
- confirmed cleanup closes the workspace, removes three clean checkouts,
  preserves three branches, and archives state;
- the base repository is the only remaining worktree.

Move the exact temporary directory to Trash after the receipt is captured.

## Failure gate

The automated suite covers missing CLIs, Herdr `0.7.4`, forced agent-start
failure, clean rollback, dirty rollback preservation, missing state, injected
`HERDR_BIN_PATH`, checksum mismatch, dirty cleanup refusal, and typed overlay
confirmation. Re-run the named tests when changing those paths:

```bash
cargo test --locked --test cli flok_
cargo test --locked --test cli cleanup_
cargo test --locked --test install_plugin
cargo test --locked --test manifest
```

## Real project iteration gate

Use one clean, non-release-critical repository:

1. Open its Flok through the plugin action.
2. Have Pi give one bounded implementation packet to a visible worker.
3. Require another visible worker to review the diff.
4. Require repository tests and a commit in the worker branch.
5. Inspect the actual branch, diff, test receipt, and base-checkout cleanliness.
6. Preview cleanup; do not remove worktrees containing useful unmerged work.

This proves the operating loop. It does not by itself prove release assets or
the public marketplace path.

## Release and marketplace gates

Follow [public-launch.md](public-launch.md). Do not call the plugin published
until public install succeeds. Do not call it listed until the marketplace card
is visible after the automatic index refresh.
