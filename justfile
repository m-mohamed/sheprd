# sheprd task runner

ci: lint test

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

install-local: build
    install -m 755 target/release/sheprd ~/.local/bin/sheprd
