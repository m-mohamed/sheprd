# Public Launch Checklist

Use this only after Mohamed gives the explicit final ship gate and `sheprd` is
ready to leave private checkpoint mode.

## Local Proof

Run:

```bash
just check
ruby -ryaml -e 'ARGV.each { |path| YAML.load_file(path); puts "ok #{path}" }' .github/workflows/*.yml .github/ISSUE_TEMPLATE/*.yml .github/DISCUSSION_TEMPLATE/*.yml .github/dependabot.yml
SHEPRD_INSTALL_DIR=/tmp/sheprd-install scripts/install-local.sh
/tmp/sheprd-install/sheprd --version
```

Then run the live Herdr checks in `docs/prelaunch-chaos.md`.

## Public Hygiene

Confirm:

- no private machine paths in docs, website, tests, or workflow files;
- README, website, CLI help, changelog, and release docs use the same language;
- `agent-dev` is presented as a sample recipe, not default product policy;
- `sheprd connect <project>` does not force panes, tabs, or commands;
- `sheprd connect <project> --json` reports a non-interactive structured
  project/workspace/action result;
- recipes only shape newly created workspaces and do not rewrite existing live
  Herdr panes;
- `SKILL.md` tells agents to use `--no-attach` inside Herdr;
- issue templates are bug-only and discussion templates handle ideas/Q&A;
- no approval-gate workflow exists until public contributor volume justifies it.

## Public Root Commit

Before pushing a shared remote, rewrite the private checkpoint history into one
clean root commit:

```bash
git reset --soft "$(git rev-list --max-parents=0 HEAD)"
git commit --amend -m "Initial commit"
```

Do not rewrite history after the repository has public users.

If the repository was briefly made public during private iteration, verify that
the release, tag, release workflow artifacts, and seed discussions were removed
before this checklist continues.

## Repository Setup

1. Create the public GitHub repository.
2. Add the remote.
3. Push `main`.
4. Enable Discussions.
5. Confirm issue and discussion templates render correctly.
6. Confirm Dependabot is enabled.
7. Confirm Actions are enabled.

## First Release

1. Move relevant `CHANGELOG.md` entries from `Unreleased` into `## v0.1.0`.
2. Run:

   ```bash
   just check
   just release-notes 0.1.0
   ```

3. Commit:

   ```bash
   git commit -m "release: v0.1.0"
   ```

4. Tag:

   ```bash
   git tag -a v0.1.0 -m "v0.1.0"
   git push origin main --tags
   ```

5. Confirm release artifacts exist for Linux x86_64, macOS x86_64
   (`macos-15-intel`), and macOS aarch64.

## Post-Launch

- Open one Q&A discussion explaining the Herdr boundary.
- Open one idea discussion for the future Ratatui picker.
- Do not add Homebrew, mise, Nix, or approval gates until real usage asks for
  them.
