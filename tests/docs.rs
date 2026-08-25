#![allow(clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn local_doc_links_resolve() {
    let root = repo_root();
    let mut failures = Vec::new();

    for file in doc_files(&root) {
        let text = fs::read_to_string(&file).expect("doc file");
        let parent = file.parent().expect("doc parent");

        for href in references(&text) {
            let target = href.split('#').next().unwrap_or_default();
            if target.is_empty() || is_external(target) {
                continue;
            }

            if !parent.join(target).exists() {
                failures.push(format!(
                    "{} -> {}",
                    file.strip_prefix(&root).expect("relative path").display(),
                    href
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "missing local documentation targets:\n{}",
        failures.join("\n")
    );
}

#[test]
fn website_keeps_the_public_surface() {
    let html = fs::read_to_string(repo_root().join("website/index.html")).expect("website index");

    for needle in [
        "project router",
        "Ratatui",
        "Sol/Luna",
        "Luna-Max",
        "docs.html",
        "Daily entry",
    ] {
        assert!(html.contains(needle), "website is missing {needle}");
    }
}

#[test]
fn website_docs_page_keeps_public_docs_surface() {
    let html = fs::read_to_string(repo_root().join("website/docs.html")).expect("website docs");

    for needle in [
        "Sheprd docs.",
        "herdr plugin install m-mohamed/sheprd",
        "factory --json",
        "sol-luna-launch.sh",
        "../docs/commands.md",
        "../SKILL.md",
        "../agent-guide.md",
        "../SECURITY.md",
        "../docs/prelaunch-chaos.md",
        "../CONTRIBUTING.md",
    ] {
        assert!(html.contains(needle), "website docs is missing {needle}");
    }
}

#[test]
fn docs_index_routes_public_surfaces() {
    let root = repo_root();
    let docs_index = fs::read_to_string(root.join("docs/README.md")).expect("docs index");
    let readme = fs::read_to_string(root.join("README.md")).expect("readme");

    assert!(
        readme.contains("[Docs index](docs/README.md)"),
        "README must link to the docs index"
    );

    for needle in [
        "[Command reference](commands.md)",
        "[Product foundation](product-foundation.md)",
        "[Herdr precedent](herdr-precedent.md)",
        "[Skill contract](../SKILL.md)",
        "[Agent guide](../agent-guide.md)",
        "[Open-source readiness](open-source-readiness.md)",
        "[Prelaunch checks](prelaunch-chaos.md)",
        "[Public launch checklist](public-launch.md)",
        "[Release process](release.md)",
    ] {
        assert!(
            docs_index.contains(needle),
            "docs index is missing {needle}"
        );
    }
}

#[test]
fn command_reference_covers_public_cli_surface() {
    let root = repo_root();
    let reference = fs::read_to_string(root.join("docs/commands.md")).expect("command reference");
    let readme = fs::read_to_string(root.join("README.md")).expect("readme");
    let skill = fs::read_to_string(root.join("SKILL.md")).expect("skill");

    assert!(
        readme.contains("[Command reference](docs/commands.md)"),
        "README must link to the command reference"
    );
    assert!(
        skill.contains("docs/commands.md"),
        "SKILL.md must point agents at the command contract"
    );

    for needle in [
        "## Global options",
        "## `init`",
        "## `list`",
        "## `connect`",
        "## `recipes`",
        "## `doctor`",
        "## `show-config`",
        "## Sol/Luna workflow",
        "JSON failures go to stderr",
        "never silently reshaped",
    ] {
        assert!(
            reference.contains(needle),
            "command reference is missing {needle}"
        );
    }
}

#[test]
fn agent_guide_covers_teaching_surface() {
    let root = repo_root();
    let guide = fs::read_to_string(root.join("agent-guide.md")).expect("agent guide");
    let readme = fs::read_to_string(root.join("README.md")).expect("readme");
    let skill = fs::read_to_string(root.join("SKILL.md")).expect("skill");

    assert!(
        readme.contains("[`agent-guide.md`](agent-guide.md)"),
        "README must link to the agent guide"
    );
    assert!(
        skill.contains("agent-guide.md"),
        "SKILL.md must point agents at the teaching guide"
    );

    for needle in [
        "Sheprd is a thin Herdr project router",
        "factory --json",
        "Sol-Hi / Pi conductor",
        "OpenCode",
        "No hidden agents are allowed",
        "Herdr IDs",
    ] {
        assert!(guide.contains(needle), "agent guide is missing {needle}");
    }
}

#[test]
fn cargo_package_keeps_public_support_files() {
    let manifest = fs::read_to_string(repo_root().join("Cargo.toml")).expect("cargo manifest");

    for needle in [
        "AGENTS.md",
        "agent-guide.md",
        "herdr-plugin.toml",
        "scripts/install-plugin.sh",
        "scripts/**/*.sh",
        "website/**/*.html",
        "website/assets/sheprd-mark.svg",
        "justfile",
        "SECURITY.md",
        "deny.toml",
        "rust-toolchain.toml",
        "publish = false",
    ] {
        assert!(
            manifest.contains(needle),
            "package include is missing {needle}"
        );
    }
}

#[test]
fn github_workflows_enforce_public_smoke_surface() {
    let root = repo_root();
    let ci = fs::read_to_string(root.join(".github/workflows/ci.yml")).expect("ci workflow");
    let release =
        fs::read_to_string(root.join(".github/workflows/release.yml")).expect("release workflow");
    let audit =
        fs::read_to_string(root.join(".github/workflows/audit.yml")).expect("audit workflow");

    for needle in [
        "target/release/sheprd init --print --json",
        "target/release/sheprd list --help",
        "target/release/sheprd doctor --help",
        "JSON failure smoke",
        "definitely-not-a-project",
        "cargo package --locked",
        "bash -n scripts/install-plugin.sh",
        "shellcheck scripts/*.sh",
        "Validate GitHub metadata",
        "os: [ubuntu-latest, macos-latest]",
    ] {
        assert!(ci.contains(needle), "CI workflow is missing {needle}");
    }

    for needle in [
        "Verify tag, crate, and plugin versions agree",
        "Verify changelog section exists",
        "--draft",
        "taiki-e/upload-rust-binary-action@f0d45ae91ee7b8ee928de7a9d04d893a08bcbec6",
        "aarch64-apple-darwin",
        "x86_64-unknown-linux-musl",
        "aarch64-unknown-linux-musl",
        "checksum: sha256",
        "actions/attest-build-provenance@0f67c3f4856b2e3261c31976d6725780e5e4c373",
        "--draft=false",
    ] {
        assert!(
            release.contains(needle),
            "release workflow is missing {needle}"
        );
    }

    for needle in [
        "schedule:",
        "cron:",
        "EmbarkStudios/cargo-deny-action@3c6349835b2b7b196a839186cb8b78e02f7b5f25",
        "command: check",
    ] {
        assert!(audit.contains(needle), "audit workflow is missing {needle}");
    }
}

#[test]
fn github_actions_are_pinned_by_full_commit_sha() {
    let workflows = repo_root().join(".github/workflows");
    for entry in fs::read_dir(workflows).expect("workflows") {
        let path = entry.expect("workflow entry").path();
        if path.extension().and_then(|value| value.to_str()) != Some("yml") {
            continue;
        }
        let contents = fs::read_to_string(&path).expect("workflow contents");
        for line in contents.lines() {
            let Some(action) = line.trim_start().strip_prefix("- uses: ") else {
                continue;
            };
            let reference = action
                .split_once('@')
                .map(|(_, value)| value.split_whitespace().next().unwrap_or_default())
                .unwrap_or_default();
            assert!(
                reference.len() == 40 && reference.chars().all(|value| value.is_ascii_hexdigit()),
                "{} contains an unpinned action: {action}",
                path.display()
            );
        }
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn doc_files(root: &Path) -> Vec<PathBuf> {
    let mut files = vec![
        root.join("README.md"),
        root.join("CONTRIBUTING.md"),
        root.join("SKILL.md"),
        root.join("AGENTS.md"),
        root.join("agent-guide.md"),
        root.join(".github/pull_request_template.md"),
        root.join("website/index.html"),
        root.join("website/docs.html"),
    ];

    for entry in fs::read_dir(root.join("docs")).expect("docs directory") {
        let path = entry.expect("docs entry").path();
        if path.extension().is_some_and(|extension| extension == "md") {
            files.push(path);
        }
    }

    files
}

fn references(text: &str) -> Vec<String> {
    let mut refs = markdown_links(text);
    refs.extend(html_attrs(text, "href=\""));
    refs.extend(html_attrs(text, "src=\""));
    refs
}

fn markdown_links(text: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut rest = text;

    while let Some(start) = rest.find("](") {
        rest = &rest[start + 2..];
        if let Some(end) = rest.find(')') {
            refs.push(rest[..end].to_string());
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }

    refs
}

fn html_attrs(text: &str, attr: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut rest = text;

    while let Some(start) = rest.find(attr) {
        rest = &rest[start + attr.len()..];
        if let Some(end) = rest.find('"') {
            refs.push(rest[..end].to_string());
            rest = &rest[end + 1..];
        } else {
            break;
        }
    }

    refs
}

fn is_external(target: &str) -> bool {
    target.starts_with("http://") || target.starts_with("https://") || target.starts_with("mailto:")
}
