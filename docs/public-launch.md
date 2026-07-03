# Public Launch Checklist

Use this checklist for launch-style release gates where repository visibility,
history shape, tags, or release artifacts are changing.

## Local Proof

Run:

```bash
just check
just metadata-smoke
just install-smoke
just live-smoke "$PWD"
SHEPRD_INSTALL_DIR=/tmp/sheprd-install scripts/install-local.sh
/tmp/sheprd-install/sheprd --version
```

Then run the live Herdr checks in `docs/prelaunch-chaos.md`.

## Public Hygiene

Confirm:

- no private machine paths in docs, website, tests, or workflow files;
- README, command reference, agent guide, website, CLI help, changelog, and
  release docs use the same language;
- `sheprd init --print` previews starter config and `sheprd init` refuses to
  overwrite an existing config without `--force`;
- `agent-dev` is presented as a sample recipe, not default product policy;
- `sheprd connect <project>` does not force panes, tabs, or commands;
- `sheprd connect <project> --json` reports a non-interactive structured
  project/workspace/action result;
- recipes only shape newly created workspaces and do not rewrite existing live
  Herdr panes;
- `SKILL.md` tells agents to use `--no-attach` inside Herdr;
- `AGENTS.md` explains the architecture, test gates, docs discipline,
  contribution surface, and no-ship guardrails;
- the PR template asks for Herdr boundary, proof, docs, and contributor
  understanding;
- issue templates are bug-only and discussion templates handle ideas/Q&A;
- no approval-gate workflow exists until public contributor volume justifies it.

## Public Root Commit

This step only applied before the first public push. Do not rewrite public
history after users exist.

```bash
git reset --soft "$(git rev-list --max-parents=0 HEAD)"
git commit --amend -m "Initial commit"
```

## Repository Setup

1. Confirm the public GitHub repository exists.
2. Confirm `main` is pushed.
3. Confirm Discussions are enabled.
4. Confirm issue and discussion templates render correctly.
5. Confirm Dependabot is enabled.
6. Confirm Actions are enabled.

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
6. Confirm each release archive includes the binary plus README, license,
   changelog, contributor docs, agent docs, agent guide, `justfile`,
   `.github/`, `.githooks/`, `docs/`, `scripts/`, and `website/`.

## Post-Launch

- Open one Q&A discussion explaining the Herdr boundary.
- Open one idea discussion for the future Ratatui picker.
- Do not add Homebrew, mise, Nix, or approval gates until real usage asks for
  them.
