#![allow(clippy::expect_used)]

use assert_cmd::Command;
use predicates::prelude::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn help_describes_product_boundary() {
    Command::cargo_bin("sheprd")
        .expect("binary")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("smart session manager for Herdr"));
}

#[test]
fn recipes_emit_agent_dev() {
    Command::cargo_bin("sheprd")
        .expect("binary")
        .args(["recipes", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"agent-dev\""))
        .stdout(predicate::str::contains("\"codex\""));
}

#[test]
fn recipes_text_marks_agent_dev_as_sample() {
    Command::cargo_bin("sheprd")
        .expect("binary")
        .arg("recipes")
        .assert()
        .success()
        .stdout(predicate::str::contains("sample recipes:"))
        .stdout(predicate::str::contains("Sample editor"));
}

#[test]
fn list_marks_running_workspace() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    fixture.git_repo("sample-app");
    fixture.fake_tool("nvim");
    fixture.fake_tool("lazygit");
    fixture.fake_tool("codex");
    fixture.fake_herdr(Some("sample-app-codex"));

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .args(["list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"running\": true"));
}

#[test]
fn connect_existing_workspace_focuses_without_create() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    fixture.git_repo("sample-app");
    fixture.fake_tool("nvim");
    fixture.fake_tool("lazygit");
    fixture.fake_tool("codex");
    fixture.fake_herdr(Some("sample-app-codex"));

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .args(["connect", "sample-app", "--no-attach"])
        .assert()
        .success();

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(log.contains("workspace focus w_existing"));
    assert!(!log.contains("workspace create"));
}

#[test]
fn connect_new_workspace_creates_plain_workspace_by_default() {
    let fixture = Fixture::new();
    fixture.write_config("hermes");
    fixture.git_repo("sample-app");
    fixture.fake_tool("hermes");
    fixture.fake_herdr(None);

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .args(["connect", "sample-app", "--no-attach"])
        .assert()
        .success();

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(log.contains("workspace create"));
    assert!(log.contains("workspace focus w_new"));
    assert!(!log.contains("tab rename"));
    assert!(!log.contains("pane run"));
    assert!(!log.contains("tab create"));
}

#[test]
fn connect_new_workspace_applies_agent_dev_recipe_when_requested() {
    let fixture = Fixture::new();
    fixture.write_config("hermes");
    fixture.git_repo("sample-app");
    fixture.fake_tool("nvim");
    fixture.fake_tool("lazygit");
    fixture.fake_tool("hermes");
    fixture.fake_herdr(None);

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .args([
            "connect",
            "sample-app",
            "--recipe",
            "agent-dev",
            "--no-attach",
        ])
        .assert()
        .success();

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(log.contains("workspace create"));
    assert!(log.contains("tab rename w_new:1 code"));
    assert!(log.contains("pane run w_new-1 nvim ."));
    assert!(log.contains("pane run w_new-2 hermes"));
    assert!(log.contains("tab create"));
    assert!(log.contains("pane run w_new-4 lazygit"));
}

struct Fixture {
    home: assert_fs::TempDir,
    root: std::path::PathBuf,
    bin: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let home = assert_fs::TempDir::new().expect("home");
        let root = home.path().join("Workspace");
        let bin = home.path().join("bin");
        std::fs::create_dir_all(&root).expect("root");
        std::fs::create_dir_all(&bin).expect("bin");
        Self { home, root, bin }
    }

    fn env(&self) -> Vec<(String, String)> {
        vec![
            ("HOME".into(), self.home.path().display().to_string()),
            (
                "SHEPRD_CONFIG".into(),
                self.home.path().join("config.toml").display().to_string(),
            ),
            (
                "PATH".into(),
                format!("{}:/usr/bin:/bin", self.bin.display()),
            ),
            ("HERDR_TEST_LOG".into(), self.log().display().to_string()),
        ]
    }

    fn log(&self) -> std::path::PathBuf {
        self.home.path().join("herdr.log")
    }

    fn write_config(&self, agent: &str) {
        std::fs::write(
            self.home.path().join("config.toml"),
            format!(
                "roots = [\"{}\"]\ndefault_agent = \"{}\"\n",
                self.root.display(),
                agent
            ),
        )
        .expect("config");
    }

    fn git_repo(&self, name: &str) {
        std::fs::create_dir_all(self.root.join(name).join(".git")).expect("repo");
    }

    fn fake_tool(&self, name: &str) {
        write_executable(&self.bin.join(name), "#!/bin/sh\nexit 0\n");
    }

    fn fake_herdr(&self, existing_label: Option<&str>) {
        let existing = existing_label.unwrap_or("");
        let script = format!(
            r#"#!/bin/sh
printf '%s\n' "$*" >> "$HERDR_TEST_LOG"
case "$1 $2" in
  "status server")
    printf 'status: running\nversion: 0.6.2\ncompatible: yes\n'
    ;;
  "workspace list")
    if [ -n "{existing}" ]; then
      printf '{{"id":"x","result":{{"workspaces":[{{"workspace_id":"w_existing","label":"{existing}"}}]}}}}'
    else
      printf '{{"id":"x","result":{{"workspaces":[]}}}}'
    fi
    ;;
  "workspace create")
    printf '{{"id":"x","result":{{"workspace":{{"workspace_id":"w_new","label":"sample-app-hermes"}},"root_pane":{{"pane_id":"w_new-1","tab_id":"w_new:1"}}}}}}'
    ;;
  "pane split")
    if [ "$5" = "right" ]; then pane="w_new-2"; elif grep -q 'tab create' "$HERDR_TEST_LOG"; then pane="w_new-5"; else pane="w_new-3"; fi
    printf '{{"id":"x","result":{{"pane":{{"pane_id":"%s","tab_id":"w_new:1"}}}}}}' "$pane"
    ;;
  "tab create")
    printf '{{"id":"x","result":{{"root_pane":{{"pane_id":"w_new-4","tab_id":"w_new:2"}}}}}}'
    ;;
  *)
    printf '{{"id":"x","result":{{"type":"ok"}}}}'
    ;;
esac
"#
        );
        write_executable(&self.bin.join("herdr"), &script);
    }
}

fn write_executable(path: &std::path::Path, contents: &str) {
    std::fs::write(path, contents).expect("write executable");
    let mut permissions = std::fs::metadata(path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("chmod");
}
