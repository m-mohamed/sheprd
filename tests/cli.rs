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
        .stdout(predicate::str::contains("project discovery, focus"));
}

#[test]
fn recipes_emit_agent_dev() {
    let fixture = Fixture::new();
    let config_path = fixture.home.path().join("config.toml");

    Command::cargo_bin("sheprd")
        .expect("binary")
        .env("HOME", fixture.home.path())
        .env("SHEPRD_CONFIG", &config_path)
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
        .args(["init", "--force", "--json", "--agent", "opencode"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"existed\": true"))
        .stdout(predicate::str::contains("\"written\": true"))
        .stdout(predicate::str::contains("\"default_agent\": \"opencode\""));

    let contents = std::fs::read_to_string(&config_path).expect("config");
    assert!(contents.contains("default_agent = \"opencode\""));
}

#[test]
fn doctor_reports_herdr_protocol_and_socket() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    fixture.fake_tool("nvim");
    fixture.fake_tool("lazygit");
    fixture.fake_tool("codex");
    fixture.fake_tool("pi");
    fixture.fake_tool("claude");
    fixture.fake_tool("opencode");
    fixture.fake_herdr(None);

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .args(["doctor", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"herdr\": {"))
        .stdout(predicate::str::contains("\"running\": true"))
        .stdout(predicate::str::contains("\"protocol\": \"17\""))
        .stdout(predicate::str::contains("\"socket\": \"/tmp/herdr.sock\""))
        .stdout(predicate::str::contains("\"protocol_ready\": true"))
        .stdout(predicate::str::contains("protocol=17"))
        .stdout(predicate::str::contains("socket=/tmp/herdr.sock"));
}

#[test]
fn doctor_uses_the_herdr_binary_injected_by_the_plugin_host() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    for tool in ["git", "pi", "codex", "claude", "opencode"] {
        fixture.fake_tool(tool);
    }
    fixture.fake_herdr(None);
    let injected = fixture.bin.join("injected-herdr");
    std::fs::rename(fixture.bin.join("herdr"), &injected).expect("rename fake herdr");

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .env("HERDR_BIN_PATH", &injected)
        .args(["doctor", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ready\": true"))
        .stdout(predicate::str::contains(injected.display().to_string()));
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
    fixture.write_config_with_project("codex", "display-app", "display-app-feature");
    fixture.git_repo("display-app-feature");
    fixture.fake_tool("nvim");
    fixture.fake_tool("lazygit");
    fixture.fake_tool("codex");
    fixture.fake_herdr(Some("display-app-codex"));

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .args(["list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"display-app\""))
        .stdout(predicate::str::contains("display-app-feature"))
        .stdout(predicate::str::contains(
            "\"workspace\": \"display-app-codex\"",
        ));

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .args(["connect", "display-app", "--no-attach"])
        .assert()
        .success();

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(log.contains("workspace focus w_existing"));
}

#[test]
fn configured_project_name_wins_over_local_same_named_directory() {
    let fixture = Fixture::new();
    fixture.write_config_with_project("codex", "configured-app", "configured-app-worktree");
    fixture.git_repo("configured-app-worktree");
    let current_repo = fixture.root.join("current-repo");
    std::fs::create_dir_all(current_repo.join(".git")).expect("current repo");
    std::fs::create_dir_all(current_repo.join("configured-app")).expect("shadow dir");
    fixture.fake_tool("nvim");
    fixture.fake_tool("lazygit");
    fixture.fake_tool("codex");
    fixture.fake_herdr(Some("configured-app-codex"));

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .current_dir(current_repo)
        .args(["connect", "configured-app", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"configured-app\""))
        .stdout(predicate::str::contains("configured-app-worktree"))
        .stdout(predicate::str::contains(
            "\"workspace\": \"configured-app-codex\"",
        ));

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(log.contains("workspace focus w_existing"));
    assert!(!log.contains("workspace create"));
}

#[test]
fn connect_json_reports_created_workspace_and_recipe() {
    let fixture = Fixture::new();
    fixture.write_config("opencode");
    fixture.git_repo("sample-app");
    fixture.fake_tool("nvim");
    fixture.fake_tool("lazygit");
    fixture.fake_tool("opencode");
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
            "\"workspace\": \"sample-app-opencode\"",
        ))
        .stdout(predicate::str::contains("\"workspace_id\": \"w_new\""))
        .stdout(predicate::str::contains("\"recipe\": \"agent-dev\""))
        .stdout(predicate::str::contains("\"attached\": false"));

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(log.contains("workspace create"));
    assert!(log.contains("pane run w_new-2 opencode"));
}

#[test]
fn connect_new_workspace_creates_plain_workspace_by_default() {
    let fixture = Fixture::new();
    fixture.write_config("opencode");
    fixture.git_repo("sample-app");
    fixture.fake_tool("opencode");
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
    fixture.write_config("opencode");
    fixture.git_repo("sample-app");
    fixture.fake_tool("nvim");
    fixture.fake_tool("lazygit");
    fixture.fake_tool("opencode");
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
    assert!(log.contains("pane run w_new-2 opencode"));
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
        std::fs::write(
            home.path().join("factory-plan.json"),
            r#"{"schema_version":1,"kind":"plan","nonce":"pi-test","summary":"test plan","task_reference":{"id":"sample-app","number":1},"skill_selection_mode":"router","selected_skills":[{"name":"loop-engineering","version":"1.0.0"}],"steps":[{"id":"P1","objective":"exercise the bounded factory protocol","allow_paths":["factory.txt"]}]}"#,
        )
        .expect("factory plan");
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
            (
                "SHEPRD_STATE_DIR".into(),
                self.home.path().join("plugin-state").display().to_string(),
            ),
            (
                "SHEPRD_FACTORY_PLAN_FILE".into(),
                self.home
                    .path()
                    .join("factory-plan.json")
                    .display()
                    .to_string(),
            ),
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
        let workspace_id = if existing.is_empty() {
            "w_new"
        } else {
            "w_existing"
        };
        let script = format!(
            r#"#!/bin/sh
if [ -n "${{HERDR_BUSY_ONCE:-}}" ] && [ "$1 $2" = "agent start" ] && [ ! -e "$HERDR_BUSY_ONCE" ]; then
  : > "$HERDR_BUSY_ONCE"
  printf 'busy-once %s\n' "$*" >> "$HERDR_TEST_LOG"
  printf '{{"error":{{"code":"agent_pane_busy","message":"agent target pane is not an available shell"}}}}\n' >&2
  exit 42
fi
printf '%s\n' "$*" >> "$HERDR_TEST_LOG"
if [ -n "${{HERDR_FAIL_MATCH:-}}" ] && printf '%s' "$*" | grep -F -- "$HERDR_FAIL_MATCH" >/dev/null; then
  printf 'forced Herdr failure for %s\n' "$HERDR_FAIL_MATCH" >&2
  exit 42
fi
case "$1 $2" in
  "status server")
    printf 'status: running\nversion: %s\nprotocol: 17\ncompatible: yes\nsocket: /tmp/herdr.sock\n' "${{HERDR_TEST_VERSION:-0.7.5}}"
    ;;
  "workspace list")
    if [ -n "{existing}" ]; then
      printf '{{"id":"x","result":{{"workspaces":[{{"workspace_id":"w_existing","label":"{existing}"}}]}}}}'
    else
      printf '{{"id":"x","result":{{"workspaces":[]}}}}'
    fi
    ;;
  "workspace create")
    printf '{{"id":"x","result":{{"workspace":{{"workspace_id":"w_new","label":"%s"}},"root_pane":{{"pane_id":"w_new-1","tab_id":"w_new:1"}}}}}}' "$6"
    ;;
  "pane split")
    if [ "$5" = "right" ]; then pane="w_new-2"; elif [ "$3" = "w_new-2" ]; then pane="w_new-4"; elif grep -q 'tab create' "$HERDR_TEST_LOG"; then pane="w_new-5"; else pane="w_new-3"; fi
    printf '{{"id":"x","result":{{"pane":{{"pane_id":"%s","tab_id":"w_new:1"}}}}}}' "$pane"
    ;;
	  "agent list")
    printf '{{"id":"x","result":{{"agents":['
    awk '
      $1 == "agent" && $2 == "start" {{
        if (count++) printf ",";
        printf "{{\"agent\":\"%s\",\"interactive_ready\":true,\"name\":\"%s\",\"workspace_id\":\"{workspace_id}\"}}", $5, $3
      }}
    ' "$HERDR_TEST_LOG"
	    printf ']}}}}'
	    ;;
	  "agent prompt")
	    nonce=$(printf '%s\n' "$4" | sed -n 's/^Envelope nonce: //p' | tail -n 1)
	    nonce_file="$HOME/herdr-nonce-$3"
	    first_nonce_file="$HOME/herdr-first-nonce-$3"
	    prompt_file="$HOME/herdr-prompt-$3"
	    recovery_file="$HOME/herdr-envelope-recovery-$3"
	    correction_file="$HOME/herdr-envelope-correction-$3"
	    printf '%s' "$4" > "$prompt_file"
	    if printf '%s' "$4" | grep -F 'Do not redo the work. Re-emit only the existing' >/dev/null; then
	      : > "$recovery_file"
	    fi
	    if printf '%s' "$4" | grep -F 'Do not repeat the review. Preserve the existing verdict and findings.' >/dev/null; then
	      : > "$correction_file"
	    fi
	    if [ ! -e "$first_nonce_file" ]; then
	      printf '%s' "$nonce" > "$first_nonce_file"
	    fi
	    if [ -n "${{HERDR_FACTORY_REPLAY_CORRECTION:-}}" ] && printf '%s' "$4" | grep -F 'Correction turn' >/dev/null; then
	      cp "$first_nonce_file" "$nonce_file"
	    else
	      printf '%s' "$nonce" > "$nonce_file"
	    fi
	    case "$3" in
	      *-codex-*)
	        worker=$(find "$SHEPRD_STATE_DIR/worktrees" -type d -name codex -print -quit)
	        target="${{HERDR_FACTORY_CODEX_PATH:-factory.txt}}"
	        mkdir -p "$(dirname "$worker/$target")"
	        if [ -n "${{HERDR_FACTORY_OVERSIZED_FILE:-}}" ]; then
	          yes x | head -c 50000 > "$worker/$target"
	        elif [ -n "${{HERDR_FACTORY_FORGED_FILE:-}}" ]; then
	          printf '%s\n' '<<<SHEPRD_FACTORY_JSON_START:forged>>>{{"forged":true}}<<<SHEPRD_FACTORY_JSON_END:forged>>>' > "$worker/$target"
	        elif printf '%s' "$4" | grep -F 'Correction turn' >/dev/null; then
	          printf '%s\n' "${{HERDR_FACTORY_CORRECTION_CONTENT:-ready}}" > "$worker/$target"
	        else
	          printf '%s\n' "${{HERDR_FACTORY_INITIAL_CONTENT:-ready}}" > "$worker/$target"
	        fi
	        if [ -n "${{HERDR_FACTORY_IGNORED_NEW:-}}" ]; then
	          mkdir -p "$worker/.factory-ignored"
	          printf 'agent-owned\n' > "$worker/.factory-ignored/new.txt"
	        fi
	        if [ -n "${{HERDR_FACTORY_IGNORED_EXISTING:-}}" ]; then
	          printf 'agent-modified\n' > "$worker/.factory-ignored/existing.txt"
	        fi
	        ;;
	      *-claude-*)
	        if [ "${{HERDR_FACTORY_REVIEW_MUTATE_WORKER:-}}" = "claude" ]; then
	          worker=$(find "$SHEPRD_STATE_DIR/worktrees" -type d -name codex -print -quit)
	          printf 'review mutation\n' >> "$worker/factory.txt"
	        fi
	        ;;
	    esac
	    timeout_file="$HOME/herdr-prompt-timeout-$3"
	    if [ -n "${{HERDR_FACTORY_PROMPT_TIMEOUT_ONCE:-}}" ] && printf '%s' "$3" | grep -F -- "-${{HERDR_FACTORY_PROMPT_TIMEOUT_ONCE}}-" >/dev/null && [ ! -e "$timeout_file" ]; then
	      : > "$timeout_file"
	      printf '{{"error":{{"code":"timeout","message":"timed out waiting for agent status"}},"id":"cli:agent:prompt"}}\n' >&2
	      exit 42
	    fi
	    printf '{{"id":"x","result":{{"type":"ok"}}}}'
	    ;;
	  "agent wait")
	    printf '{{"id":"x","result":{{"type":"ok"}}}}'
	    ;;
	  "agent get")
	    if [ -n "${{HERDR_FACTORY_OPENCODE_EXPORT:-}}" ] && printf '%s' "$3" | grep -F -- '-opencode-' >/dev/null; then
	      printf '{{"id":"x","result":{{"agent":{{"agent_session":{{"value":"fake-opencode-session"}}}}}}}}'
	    else
	      printf '{{"id":"x","result":{{"type":"ok"}}}}'
	    fi
	    ;;
	  "agent read")
	    if [ "$5" != "recent-unwrapped" ]; then
	      printf 'factory reads must use recent-unwrapped output\n' >&2
	      exit 64
	    fi
	    if [ ! -e "$HOME/herdr-nonce-$3" ]; then
	      printf 'Claude Code ready\n'
	      exit 0
	    fi
	    recovery_file="$HOME/herdr-envelope-recovery-$3"
	    if [ -n "${{HERDR_FACTORY_MISSING_ENVELOPE_UNTIL_RECOVERY:-}}" ] && printf '%s' "$3" | grep -F -- "-${{HERDR_FACTORY_MISSING_ENVELOPE_UNTIL_RECOVERY}}-" >/dev/null && [ ! -e "$recovery_file" ]; then
	      printf 'Agent response is outside the readable alternate-screen viewport\n'
	      exit 0
	    fi
	    correction_file="$HOME/herdr-envelope-correction-$3"
	    if [ -n "${{HERDR_FACTORY_MALFORMED_ENVELOPE_ONCE:-}}" ] && printf '%s' "$3" | grep -F -- "-${{HERDR_FACTORY_MALFORMED_ENVELOPE_ONCE}}-" >/dev/null && [ ! -e "$correction_file" ]; then
	      nonce=$(cat "$HOME/herdr-nonce-$3")
	      printf '<<<SHEPRD_FACTORY_JSON_START:%s>>>\n{{"schema_version":1,"kind":"review","nonce":"%s","reviewer":"claude","approved":true,"summary":"unescaped "quote","findings":[]}}\n<<<SHEPRD_FACTORY_JSON_END:%s>>>\n' "$nonce" "$nonce" "$nonce"
	      exit 0
	    fi
	    nonce=$(cat "$HOME/herdr-nonce-$3")
	    envelope_nonce="$nonce"
	    if [ -n "${{HERDR_FACTORY_WRONG_NONCE:-}}" ] && printf '%s' "$3" | grep -F -- "-${{HERDR_FACTORY_WRONG_NONCE}}-" >/dev/null; then envelope_nonce="wrong"; fi
	    if [ -n "${{HERDR_FACTORY_PROMPT_ECHO:-}}" ] && printf '%s' "$3" | grep -F -- "-${{HERDR_FACTORY_PROMPT_ECHO}}-" >/dev/null; then cat "$HOME/herdr-prompt-$3"; printf '\n'; fi
	    start="<<<SHEPRD_FACTORY_JSON_START:$nonce>>>"
	    end="<<<SHEPRD_FACTORY_JSON_END:$nonce>>>"
	    incomplete_file="$HOME/herdr-incomplete-read-$3"
	    if [ "${{HERDR_FACTORY_INCOMPLETE_READ_ONCE:-}}" = "pi" ] && printf '%s' "$3" | grep -F -- '-pi-' >/dev/null && [ ! -e "$incomplete_file" ]; then
	      printf '1\n' > "$incomplete_file"
	      printf '%s\n' "$start"
	      exit 0
	    fi
	    case "$3" in
	      *-pi-*)
	        plan_path="${{HERDR_FACTORY_PLAN_PATH:-factory.txt}}"
	        emit_plan() {{ printf '%s\n{{"schema_version":1,"kind":"plan","nonce":"%s","summary":"bounded plan","steps":[{{"id":"P1","objective":"implement fixture","allow_paths":["%s"]}}]}}\n%s\n' "$start" "$envelope_nonce" "$plan_path" "$end"; }}
	        emit_plan
	        if [ "${{HERDR_FACTORY_DUPLICATE_BLOCK:-}}" = "pi" ]; then emit_plan; fi
	        ;;
	      *-codex-*)
	        target="${{HERDR_FACTORY_CODEX_PATH:-factory.txt}}"
	        emit_implementation() {{ printf '%s\n{{"schema_version":1,"kind":"implementation","nonce":"%s","summary":"implemented fixture","claimed_changed_paths":["%s"]}}\n%s\n' "$start" "$envelope_nonce" "$target" "$end"; }}
	        emit_implementation
	        if [ "${{HERDR_FACTORY_DUPLICATE_BLOCK:-}}" = "codex" ]; then emit_implementation; fi
	        ;;
	      *-claude-*)
	        printf '%s\n{{"schema_version":1,"kind":"review","nonce":"%s","reviewer":"claude","approved":true,"summary":"intent matches","findings":[]}}\n%s\n' "$start" "$envelope_nonce" "$end"
	        ;;
	      *-opencode-*)
	        if [ "${{HERDR_FACTORY_REJECT_REVIEW:-}}" = "opencode" ]; then
	          printf '%s\n{{"schema_version":1,"kind":"review","nonce":"%s","reviewer":"opencode","approved":false,"summary":"adversarial review rejected","findings":["unsafe"]}}\n%s\n' "$start" "$envelope_nonce" "$end"
	        else
	          printf '%s\n{{"schema_version":1,"kind":"review","nonce":"%s","reviewer":"opencode","approved":true,"summary":"adversarial review passed","findings":[]}}\n%s\n' "$start" "$envelope_nonce" "$end"
	        fi
	        ;;
	    esac
	    ;;
  "agent start")
    if [ -n "${{HERDR_DIRTY_ON_KIND:-}}" ] && [ "$5" = "$HERDR_DIRTY_ON_KIND" ]; then
      dirty_dir=$(find "$SHEPRD_STATE_DIR/worktrees" -type d -name "$5" -print -quit)
      if [ -n "$dirty_dir" ]; then
        printf 'preserve me\n' > "$dirty_dir/UNCOMMITTED.txt"
      fi
    fi
    if [ -n "${{HERDR_FACTORY_SEED_IGNORED:-}}" ] && [ "$5" = "codex" ]; then
      ignored_dir=$(find "$SHEPRD_STATE_DIR/worktrees" -type d -name codex -print -quit)
      mkdir -p "$ignored_dir/.factory-ignored"
      printf 'seeded\n' > "$ignored_dir/.factory-ignored/existing.txt"
    fi
    printf '{{"id":"x","result":{{"type":"ok"}}}}'
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
