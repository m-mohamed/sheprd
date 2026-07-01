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
fn cargo_package_keeps_public_support_files() {
    let manifest = fs::read_to_string(repo_root().join("Cargo.toml")).expect("cargo manifest");

    for needle in [
        "AGENTS.md",
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

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn doc_files(root: &Path) -> Vec<PathBuf> {
    let mut files = vec![
        root.join("README.md"),
        root.join("CONTRIBUTING.md"),
        root.join("SKILL.md"),
        root.join("AGENTS.md"),
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
