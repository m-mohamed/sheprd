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
        "id=\"install\"",
        "id=\"quick-start\"",
        "id=\"contract\"",
        "Agent-safe",
        "Herdr owns",
        "sheprd owns",
        "init --print",
        "connect my-project --json",
    ] {
        assert!(html.contains(needle), "website is missing {needle}");
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
        "## Global Options",
        "## `init`",
        "## `list`",
        "## `connect`",
        "## `connect --recipe agent-dev`",
        "## `recipes`",
        "## `doctor`",
        "## `show-config`",
        "## Failure Behavior",
        "`herdr.protocol_ready`",
        "`workspace_id`",
        "`attached`",
        "\"ok\": false",
        "\"exit_code\": 2",
        "`error.kind`",
        "does not reshape live",
        "not an arbitrary directory",
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
        "This guide is different from `SKILL.md`",
        "Sheprd is a Herdr companion, not a terminal runtime.",
        "sheprd doctor --json",
        "sheprd connect my-project --no-attach",
        "herdr.protocol_ready",
        "Do not teach Sheprd as",
        "sample recipe",
        "Herdr workspace, tab, or pane ids",
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
        "scripts/**/*.sh",
        "website/index.html",
        "website/assets/sheprd-mark.svg",
        "justfile",
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

    for needle in [
        "target/release/sheprd init --print --json",
        "JSON failure smoke",
        "definitely-not-a-project",
        "cargo package --locked",
        "scripts/install-local.sh",
        "Validate GitHub metadata",
    ] {
        assert!(ci.contains(needle), "CI workflow is missing {needle}");
    }

    for needle in [
        "target/release/sheprd init --print --json",
        "JSON failure smoke",
        "definitely-not-a-project",
        "cargo package --locked",
        "CONTRIBUTING.md AGENTS.md SKILL.md agent-guide.md justfile",
        "cp -R docs scripts website",
    ] {
        assert!(
            release.contains(needle),
            "release workflow is missing {needle}"
        );
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
