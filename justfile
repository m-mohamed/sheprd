# sheprd task runner

ci: lint test shell-check

check: ci audit chaos-smoke failure-smoke package

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
    cargo test --locked

audit:
    cargo deny check

shell-check:
    bash -n scripts/install-plugin.sh scripts/open-flok-picker.sh scripts/open-flok-cleanup.sh scripts/install-local.sh scripts/extract-release-notes.sh
    shellcheck scripts/*.sh

build:
    cargo build --release --locked

chaos-smoke: build
    target/release/sheprd --help
    target/release/sheprd init --help
    target/release/sheprd init --print
    target/release/sheprd init --print --json
    target/release/sheprd connect --help
    target/release/sheprd flok --help
    target/release/sheprd factory run --help
    target/release/sheprd cleanup --help
    target/release/sheprd recipes
    target/release/sheprd recipes --json
    target/release/sheprd show-config

failure-smoke: build
    rm -f /tmp/sheprd-json-error.out /tmp/sheprd-json-error.err
    if target/release/sheprd connect definitely-not-a-project --json >/tmp/sheprd-json-error.out 2>/tmp/sheprd-json-error.err; then exit 1; fi
    test ! -s /tmp/sheprd-json-error.out
    grep '"ok": false' /tmp/sheprd-json-error.err
    grep '"exit_code": 2' /tmp/sheprd-json-error.err
    grep "definitely-not-a-project" /tmp/sheprd-json-error.err

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

plugin-install-test:
    cargo test --locked --test install_plugin

live-smoke project=".": build
    target/release/sheprd doctor --json
    target/release/sheprd list --json
    target/release/sheprd connect "{{project}}" --json

release-notes version:
    scripts/extract-release-notes.sh "{{version}}" RELEASE_NOTES.md
