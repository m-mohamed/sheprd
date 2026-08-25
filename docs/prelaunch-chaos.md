# Prelaunch checks

Sheprd's prelaunch gate is intentionally small and deterministic.

## Static checks

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --locked
cargo package --locked --allow-dirty
bash -n scripts/*.sh
shellcheck scripts/*.sh
```

## CLI smoke

```bash
target/release/sheprd --help
target/release/sheprd init --print --json
target/release/sheprd list --help
target/release/sheprd connect --help
target/release/sheprd doctor --help
target/release/sheprd recipes --json
target/release/sheprd show-config --json
```

## Failure smoke

```bash
if target/release/sheprd connect definitely-not-a-project --json; then
  exit 1
fi
```

The command must emit no successful JSON on stdout and must return a structured
error on stderr. No runtime state may be mutated.

## Sol/Luna pilot gate

The separate HQ launcher requires a Herdr-managed pane, a clean base checkout,
explicit Tuxedo identity, allow paths, and deterministic checks. The first
pilot is run from `~/workspace/hq/workflows/sol-luna-max.md`; inspect its private
receipt before cleanup or acceptance.
