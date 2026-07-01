# sheprd task runner

ci: lint test

check: ci chaos-smoke package

prelaunch-check project=".":
    just check
    just metadata-smoke
    just install-smoke
    just live-smoke "{{project}}"

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
    target/release/sheprd init --help
    target/release/sheprd init --print
    target/release/sheprd init --print --json
    target/release/sheprd connect --help
    target/release/sheprd recipes
    target/release/sheprd recipes --json
    target/release/sheprd show-config

package:
    cargo package --allow-dirty

install-local:
    scripts/install-local.sh

metadata-smoke:
    ruby -ryaml -e 'ARGV.each { |path| YAML.load_file(path); puts "ok #{path}" }' .github/workflows/*.yml .github/ISSUE_TEMPLATE/*.yml .github/DISCUSSION_TEMPLATE/*.yml .github/dependabot.yml

install-smoke: build
    rm -rf /tmp/sheprd-install-smoke
    SHEPRD_INSTALL_DIR=/tmp/sheprd-install-smoke scripts/install-local.sh
    /tmp/sheprd-install-smoke/sheprd --version

live-smoke project=".": build
    target/release/sheprd doctor --json
    target/release/sheprd list --json
    target/release/sheprd connect "{{project}}" --json

release-notes version:
    scripts/extract-release-notes.sh "{{version}}" RELEASE_NOTES.md
