# Public Launch Checklist

The Herdr marketplace is an automatic, unreviewed index of public GitHub
repositories with the `herdr-plugin` topic. Publishing therefore has distinct
gates: source, release, install, topic, and marketplace discovery.

## 1. Release commit

- `Cargo.toml` and `herdr-plugin.toml` both declare `0.3.1`.
- `CHANGELOG.md` has a non-empty `## v0.3.1` section.
- README, CLI help, manifest descriptions, docs, security policy, and website
  agree on Sheprd/Flok and the four-agent roster.
- `just check`, the live plugin gate, disposable Flok gate, and real-project
  iteration gate have receipts.
- public-source and secret scans are clean.

## 2. Public repository

- repository visibility is public;
- description is concise and matches the marketplace card;
- homepage points to `https://herdr.dev/plugins/` or the project site;
- topics include `herdr-plugin`, `herdr`, `coding-agents`, `multi-agent`, and
  `rust`;
- Issues, Discussions, private vulnerability reporting, and Actions are
  enabled as intended;
- `main` is pushed and CI/Audit are green.

## 3. Release

```bash
git tag -a v0.3.1 -m "v0.3.1"
git push origin main
git push origin v0.3.1
```

The tag workflow must:

1. reject tag/crate/manifest disagreement or missing changelog notes;
2. create a draft release;
3. build macOS and Linux archives for x86_64 and aarch64;
4. attach a SHA-256 sidecar and provenance attestation for every archive;
5. publish only after every matrix target succeeds.

Verify the release is not draft and contains eight assets (four archives plus
four checksum sidecars). Verify at least one archive attestation.

## 4. Clean public install

Unlink the development checkout, then install from GitHub:

```bash
herdr plugin unlink m-mohamed.sheprd
herdr plugin install m-mohamed/sheprd --ref v0.3.1
herdr plugin list
herdr plugin action list --plugin m-mohamed.sheprd
herdr plugin action invoke m-mohamed.sheprd.doctor
```

Confirm the install preview names `scripts/install-plugin.sh`; the log reports
an exact-version checksum- and provenance-verified binary, not an unexplained
source build.
Reinstall once to verify replacement of the managed checkout.

## 5. Marketplace discovery

Add the `herdr-plugin` topic only after the clean public install succeeds. The
index refreshes about every 30 minutes.

Verify the public card separately:

- owner/repository: `m-mohamed/sheprd`;
- description matches GitHub metadata;
- primary language is Rust;
- repository link opens publicly;
- `herdr plugin install m-mohamed/sheprd` still succeeds from the card's source.

A topic receipt is submitted, not listed. A visible card is listed, not Herdr-
reviewed or endorsed.

## 6. Post-launch

- keep the local managed install enabled;
- preserve release and marketplace URLs;
- monitor CI/Audit and the first install issue;
- do not add Windows, package-manager, or generic-layout claims without their
  own tested gates.
