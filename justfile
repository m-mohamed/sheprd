# sheprd task runner

ci: lint test

check: ci chaos-smoke package

install-hooks:
    git config core.hooksPath .githooks
    chmod +x .githooks/pre-commit .githooks/commit-msg
    @echo "installed git hooks from .githooks"

lint:
    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings

test:
    cargo test

build:
    cargo build --release --locked

chaos-smoke: build
    target/release/sheprd --help
    target/release/sheprd connect --help
    target/release/sheprd recipes
    target/release/sheprd recipes --json
    target/release/sheprd show-config

package:
    cargo package --allow-dirty

install-local:
    scripts/install-local.sh

release-notes version:
    scripts/extract-release-notes.sh "{{version}}" RELEASE_NOTES.md
