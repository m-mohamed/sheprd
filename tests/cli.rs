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
fn init_prints_starter_config_without_writing() {
    let fixture = Fixture::new();
    let config_path = fixture.home.path().join("custom/config.toml");

    Command::cargo_bin("sheprd")
        .expect("binary")
        .env("HOME", fixture.home.path())
        .env("SHEPRD_CONFIG", &config_path)
        .args(["init", "--print", "--root", "~/work", "--agent", "opencode"])
        .assert()
        .success()
        .stdout(predicate::str::contains("# sheprd config"))
        .stdout(predicate::str::contains("\"~/work\""))
        .stdout(predicate::str::contains("default_agent = \"opencode\""));

    assert!(!config_path.exists());
}

#[test]
fn init_writes_starter_config_and_refuses_existing_without_force() {
    let fixture = Fixture::new();
    let config_path = fixture.home.path().join("custom/config.toml");

    Command::cargo_bin("sheprd")
        .expect("binary")
        .env("HOME", fixture.home.path())
        .env("SHEPRD_CONFIG", &config_path)
        .args(["init", "--root", "~/work"])
        .assert()
        .success()
        .stdout(predicate::str::contains("created config:"));

    let contents = std::fs::read_to_string(&config_path).expect("config");
    assert!(contents.contains("\"~/work\""));
    assert!(contents.contains("default_agent = \"codex\""));

    Command::cargo_bin("sheprd")
        .expect("binary")
        .env("HOME", fixture.home.path())
        .env("SHEPRD_CONFIG", &config_path)
        .arg("init")
        .assert()
        .failure()
        .stderr(predicate::str::contains("config already exists"));
}

#[test]
fn init_json_reports_force_overwrite() {
    let fixture = Fixture::new();
    let config_path = fixture.home.path().join("custom/config.toml");
    std::fs::create_dir_all(config_path.parent().expect("parent")).expect("config dir");
    std::fs::write(&config_path, "default_agent = \"codex\"\n").expect("seed config");

    Command::cargo_bin("sheprd")
        .expect("binary")
        .env("HOME", fixture.home.path())
        .env("SHEPRD_CONFIG", &config_path)
        .args(["init", "--force", "--json", "--agent", "droid"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"existed\": true"))
        .stdout(predicate::str::contains("\"written\": true"))
        .stdout(predicate::str::contains("\"default_agent\": \"droid\""));

    let contents = std::fs::read_to_string(&config_path).expect("config");
    assert!(contents.contains("default_agent = \"droid\""));
}

#[test]
fn doctor_reports_herdr_protocol_and_socket() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    fixture.fake_tool("nvim");
    fixture.fake_tool("lazygit");
    fixture.fake_tool("codex");
    fixture.fake_herdr(None);

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .args(["doctor", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"herdr\": {"))
        .stdout(predicate::str::contains("\"running\": true"))
        .stdout(predicate::str::contains("\"protocol\": \"14\""))
        .stdout(predicate::str::contains("\"socket\": \"/tmp/herdr.sock\""))
        .stdout(predicate::str::contains("\"protocol_ready\": true"))
        .stdout(predicate::str::contains("protocol=14"))
        .stdout(predicate::str::contains("socket=/tmp/herdr.sock"));
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
        .success()
        .stdout(predicate::str::contains(
            "focused Herdr workspace: sample-app-codex",
        ))
        .stdout(predicate::str::contains("project: sample-app"))
        .stdout(predicate::str::contains("agent: codex"))
        .stdout(predicate::str::contains("attached: no"));

    let log = std::fs::read_to_string(fixture.log()).unwrap_or_default();
    assert!(log.contains("workspace focus w_existing"));
    assert!(!log.contains("workspace create"));
}

#[test]
fn connect_json_failure_uses_error_envelope_without_mutating_herdr() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    fixture.fake_tool("codex");
    fixture.fake_herdr(None);

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .args(["connect", "definitely-not-a-project", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("\"ok\": false"))
        .stderr(predicate::str::contains("\"kind\": \"message\""))
        .stderr(predicate::str::contains(
            "project 'definitely-not-a-project' was not found",
        ))
        .stderr(predicate::str::contains("\"exit_code\": 2"));

    let log = std::fs::read_to_string(fixture.log()).unwrap_or_default();
    assert!(!log.contains("workspace create"));
    assert!(!log.contains("workspace focus"));
}

#[test]
fn connect_rejects_existing_non_git_path_before_touching_herdr() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    fixture.fake_tool("codex");
    fixture.fake_herdr(None);
    let not_repo = fixture.root.join("not-a-repo");
    std::fs::create_dir_all(&not_repo).expect("not repo");

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .args(["connect", &not_repo.display().to_string(), "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("\"ok\": false"))
        .stderr(predicate::str::contains(
            "project path is not a git repository",
        ))
        .stderr(predicate::str::contains("\"exit_code\": 2"));

    let log = std::fs::read_to_string(fixture.log()).unwrap_or_default();
    assert!(!log.contains("workspace create"));
    assert!(!log.contains("workspace focus"));
}

#[test]
fn connect_json_reports_existing_workspace() {
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
        .args(["connect", "sample-app", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"action\": \"focused_existing\""))
        .stdout(predicate::str::contains(
            "\"workspace\": \"sample-app-codex\"",
        ))
        .stdout(predicate::str::contains("\"workspace_id\": \"w_existing\""))
        .stdout(predicate::str::contains("\"recipe\": null"))
        .stdout(predicate::str::contains("\"attached\": false"));

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(log.contains("workspace focus w_existing"));
    assert!(!log.contains("workspace create"));
}

#[test]
fn explicit_projects_keep_configured_name_for_custom_path() {
    let fixture = Fixture::new();
    fixture.write_config_with_project("codex", "corvus", "corvus-pride-month-logo");
    fixture.git_repo("corvus-pride-month-logo");
    fixture.fake_tool("nvim");
    fixture.fake_tool("lazygit");
    fixture.fake_tool("codex");
    fixture.fake_herdr(Some("corvus-codex"));

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .args(["list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"corvus\""))
        .stdout(predicate::str::contains("corvus-pride-month-logo"))
        .stdout(predicate::str::contains("\"workspace\": \"corvus-codex\""));

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .args(["connect", "corvus", "--no-attach"])
        .assert()
        .success();

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(log.contains("workspace focus w_existing"));
}

#[test]
fn configured_project_name_wins_over_local_same_named_directory() {
    let fixture = Fixture::new();
    fixture.write_config_with_project("codex", "ghost", "ghost-worktree");
    fixture.git_repo("ghost-worktree");
    let current_repo = fixture.root.join("current-repo");
    std::fs::create_dir_all(current_repo.join(".git")).expect("current repo");
    std::fs::create_dir_all(current_repo.join("ghost")).expect("shadow dir");
    fixture.fake_tool("nvim");
    fixture.fake_tool("lazygit");
    fixture.fake_tool("codex");
    fixture.fake_herdr(Some("ghost-codex"));

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .current_dir(current_repo)
        .args(["connect", "ghost", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"ghost\""))
        .stdout(predicate::str::contains("ghost-worktree"))
        .stdout(predicate::str::contains("\"workspace\": \"ghost-codex\""));

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(log.contains("workspace focus w_existing"));
    assert!(!log.contains("workspace create"));
}

#[test]
fn connect_json_reports_created_workspace_and_recipe() {
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
        .args(["connect", "sample-app", "--recipe", "agent-dev", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"action\": \"created_workspace\"",
        ))
        .stdout(predicate::str::contains(
            "\"workspace\": \"sample-app-hermes\"",
        ))
        .stdout(predicate::str::contains("\"workspace_id\": \"w_new\""))
        .stdout(predicate::str::contains("\"recipe\": \"agent-dev\""))
        .stdout(predicate::str::contains("\"attached\": false"));

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(log.contains("workspace create"));
    assert!(log.contains("pane run w_new-2 hermes"));
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

    fn write_config_with_project(&self, agent: &str, name: &str, path_name: &str) {
        std::fs::write(
            self.home.path().join("config.toml"),
            format!(
                "roots = [\"{}\"]\ndefault_agent = \"{}\"\n[[projects]]\nname = \"{}\"\npath = \"{}\"\n",
                self.root.display(),
                agent,
                name,
                self.root.join(path_name).display()
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
    printf 'status: running\nversion: 0.7.1\nprotocol: 14\ncompatible: yes\nsocket: /tmp/herdr.sock\n'
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
