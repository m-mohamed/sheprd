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
        .stdout(predicate::str::contains("visible four-agent Flok"));
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

#[test]
fn flok_creates_exactly_four_agents_with_pinned_models_and_worker_worktrees() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    let repo = fixture.real_git_repo("sample-app");
    for tool in ["pi", "codex", "claude", "opencode"] {
        fixture.fake_tool(tool);
    }
    fixture.fake_herdr(None);

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .env("SHEPRD_STATE_DIR", fixture.home.path().join("plugin-state"))
        .args(["flok", &repo.display().to_string(), "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"action\": \"created_flok\""))
        .stdout(predicate::str::contains(
            "\"workspace_label\": \"sample-app-flok\"",
        ))
        .stdout(predicate::str::contains(
            "\"model\": \"openai-codex/gpt-5.6-sol\"",
        ))
        .stdout(predicate::str::contains("\"model\": \"gpt-5.6-sol\""))
        .stdout(predicate::str::contains("\"model\": \"claude-opus-5\""))
        .stdout(predicate::str::contains(
            "\"model\": \"opencode-go/kimi-k3\"",
        ))
        .stdout(predicate::str::contains("\"effort\": \"high\""))
        .stdout(predicate::str::contains("\"healthy\": true"));

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert_eq!(log.matches("agent start").count(), 4);
    assert!(log.contains("--kind pi"));
    assert!(log.contains("--kind codex"));
    assert!(log.contains("--kind claude"));
    assert!(log.contains("--kind opencode"));
    assert!(log.contains("--agent build --model opencode-go/kimi-k3 --mini"));
    assert!(log.contains("--model openai-codex/gpt-5.6-sol --thinking high"));
    assert!(log.contains("--model gpt-5.6-sol --config model_reasoning_effort=high"));
    assert!(log.contains("--sandbox workspace-write --add-dir"));
    assert!(log.contains("sample-app/.git"));
    assert!(log.contains("--model claude-opus-5 --effort high"));
    let pi_start = log
        .lines()
        .find(|line| line.starts_with("agent start") && line.contains("--kind pi"))
        .expect("Pi start command");
    assert!(pi_start.contains("Never spawn hidden or additional agents"));
    assert!(
        pi_start.len() < 1024,
        "Pi startup command must stay below conservative PTY input limits"
    );

    let branches = std::process::Command::new("git")
        .args(["branch", "--format=%(refname:short)"])
        .current_dir(repo)
        .output()
        .expect("git branches");
    let branches = String::from_utf8_lossy(&branches.stdout);
    assert!(branches.contains("-codex"));
    assert!(branches.contains("-claude"));
    assert!(branches.contains("-opencode"));

    let state_root = fixture.home.path().join("plugin-state");
    assert!(std::fs::read_dir(state_root.join("locks"))
        .expect("lock directory")
        .next()
        .is_none());
    let state_files = std::fs::read_dir(state_root.join("floks"))
        .expect("Flok state directory")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("Flok state entries");
    assert_eq!(state_files.len(), 1);
    assert_eq!(
        state_files[0]
            .path()
            .extension()
            .and_then(|value| value.to_str()),
        Some("json")
    );
    let state = std::fs::read_to_string(state_files[0].path()).expect("Flok state");
    assert!(state.contains("\"schema_version\": 1"));
    assert!(state.contains("\"healthy\": true"));
}

#[test]
fn flok_rejects_missing_prerequisites_before_creating_worktrees_or_workspace() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    let repo = fixture.real_git_repo("sample-app");
    for tool in ["pi", "codex", "claude"] {
        fixture.fake_tool(tool);
    }
    fixture.fake_herdr(None);

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .args(["flok", &repo.display().to_string(), "--json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Flok prerequisites are missing from PATH: opencode",
        ));

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(!log.contains("workspace create"));
    assert!(!log.contains("worktree add"));
    let worktrees = git_output(&repo, &["worktree", "list", "--porcelain"]);
    assert_eq!(worktrees.matches("worktree ").count(), 1);
}

#[test]
fn flok_rolls_back_clean_resources_when_agent_start_fails() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    let repo = fixture.real_git_repo("sample-app");
    for tool in ["pi", "codex", "claude", "opencode"] {
        fixture.fake_tool(tool);
    }
    fixture.fake_herdr(None);

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .env("HERDR_FAIL_MATCH", "--kind opencode")
        .args(["flok", &repo.display().to_string(), "--json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("closed partial workspace w_new"))
        .stderr(predicate::str::contains("removed clean partial worktree"));

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(log.contains("workspace close w_new"));
    let worktrees = git_output(&repo, &["worktree", "list", "--porcelain"]);
    assert_eq!(worktrees.matches("worktree ").count(), 1);
}

#[test]
fn flok_retries_a_temporarily_busy_agent_pane() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    let repo = fixture.real_git_repo("sample-app");
    for tool in ["pi", "codex", "claude", "opencode"] {
        fixture.fake_tool(tool);
    }
    fixture.fake_herdr(None);

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .env("HERDR_BUSY_ONCE", fixture.home.path().join("busy-once"))
        .args(["flok", &repo.display().to_string(), "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"healthy\": true"));

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(log.contains("busy-once agent start"));
    assert_eq!(
        log.lines()
            .filter(|line| line.starts_with("agent start"))
            .count(),
        4
    );
}

#[test]
fn flok_preserves_dirty_worker_state_during_rollback() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    let repo = fixture.real_git_repo("sample-app");
    for tool in ["pi", "codex", "claude", "opencode"] {
        fixture.fake_tool(tool);
    }
    fixture.fake_herdr(None);

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .env("HERDR_DIRTY_ON_KIND", "codex")
        .env("HERDR_FAIL_MATCH", "--kind claude")
        .args(["flok", &repo.display().to_string(), "--json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("preserved dirty partial worktree"));

    let worktrees = git_output(&repo, &["worktree", "list", "--porcelain"]);
    assert_eq!(worktrees.matches("worktree ").count(), 2);
    assert!(worktrees.contains("/codex"));
}

#[test]
fn flok_refuses_old_herdr_before_mutating_runtime_state() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    let repo = fixture.real_git_repo("sample-app");
    for tool in ["pi", "codex", "claude", "opencode"] {
        fixture.fake_tool(tool);
    }
    fixture.fake_herdr(None);

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .env("HERDR_TEST_VERSION", "0.7.4")
        .args(["flok", &repo.display().to_string(), "--json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "requires Herdr 0.7.5 or newer; running 0.7.4",
        ));

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(!log.contains("workspace create"));
}

#[test]
fn existing_flok_without_state_focuses_but_reports_degraded_health() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    let repo = fixture.real_git_repo("sample-app");
    fixture.fake_herdr(Some("sample-app-flok"));

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .args(["flok", &repo.display().to_string(), "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"action\": \"focused_existing\""))
        .stdout(predicate::str::contains("\"healthy\": false"))
        .stdout(predicate::str::contains("saved Flok state is missing"));

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(log.contains("workspace focus w_existing"));
    assert!(!log.contains("workspace create"));
}

#[test]
fn cleanup_previews_without_closing_workspace_or_removing_worktrees() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    let repo = fixture.real_git_repo("sample-app");
    for tool in ["pi", "codex", "claude", "opencode"] {
        fixture.fake_tool(tool);
    }
    fixture.fake_herdr(None);
    open_test_flok(&fixture, &repo);
    fixture.fake_herdr(Some("sample-app-flok"));

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .args(["cleanup", &repo.display().to_string(), "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"confirmed\": false"))
        .stdout(predicate::str::contains("\"can_cleanup\": true"))
        .stdout(predicate::str::contains("\"removed\": false"));

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(!log.contains("workspace close w_existing"));
    let worktrees = git_output(&repo, &["worktree", "list", "--porcelain"]);
    assert_eq!(worktrees.matches("worktree ").count(), 4);
}

#[test]
fn cleanup_from_a_worker_context_resolves_the_base_project() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    let repo = fixture.real_git_repo("sample-app");
    for tool in ["pi", "codex", "claude", "opencode"] {
        fixture.fake_tool(tool);
    }
    fixture.fake_herdr(None);
    open_test_flok(&fixture, &repo);
    let worktrees = git_output(&repo, &["worktree", "list", "--porcelain"]);
    let codex = worktrees
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .find(|path| path.ends_with("/codex"))
        .expect("codex worktree");
    fixture.fake_herdr(Some("sample-app-flok"));
    let context = serde_json::json!({ "focused_pane_cwd": codex });

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .env("HERDR_PLUGIN_CONTEXT_JSON", context.to_string())
        .args(["cleanup", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"project\": \"sample-app\""))
        .stdout(predicate::str::contains("\"can_cleanup\": true"));
}

#[test]
fn plugin_host_state_scoping_does_not_hide_a_cli_created_flok() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    let repo = fixture.real_git_repo("sample-app");
    for tool in ["pi", "codex", "claude", "opencode"] {
        fixture.fake_tool(tool);
    }
    fixture.fake_herdr(None);

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .env_remove("SHEPRD_STATE_DIR")
        .env(
            "HERDR_PLUGIN_STATE_DIR",
            fixture.home.path().join("first-plugin-scope"),
        )
        .args(["flok", &repo.display().to_string(), "--json"])
        .assert()
        .success();

    fixture.fake_herdr(Some("sample-app-flok"));
    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .env_remove("SHEPRD_STATE_DIR")
        .env(
            "HERDR_PLUGIN_STATE_DIR",
            fixture.home.path().join("second-plugin-scope"),
        )
        .args(["cleanup", &repo.display().to_string(), "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"can_cleanup\": true"))
        .stdout(predicate::str::contains(".local/state/sheprd/floks"));
}

#[test]
fn cleanup_reads_v020_plugin_scoped_state_for_compatibility() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    let repo = fixture.real_git_repo("sample-app");
    for tool in ["pi", "codex", "claude", "opencode"] {
        fixture.fake_tool(tool);
    }
    fixture.fake_herdr(None);
    let legacy_state = fixture.home.path().join("legacy-plugin-state");

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .env("SHEPRD_STATE_DIR", &legacy_state)
        .args(["flok", &repo.display().to_string(), "--json"])
        .assert()
        .success();

    fixture.fake_herdr(Some("sample-app-flok"));
    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .env_remove("SHEPRD_STATE_DIR")
        .env("HERDR_PLUGIN_STATE_DIR", &legacy_state)
        .args(["cleanup", &repo.display().to_string(), "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"can_cleanup\": true"))
        .stdout(predicate::str::contains("legacy-plugin-state/floks"));
}

#[test]
fn cleanup_confirm_removes_clean_checkouts_preserves_branches_and_archives_state() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    let repo = fixture.real_git_repo("sample-app");
    for tool in ["pi", "codex", "claude", "opencode"] {
        fixture.fake_tool(tool);
    }
    fixture.fake_herdr(None);
    open_test_flok(&fixture, &repo);
    fixture.fake_herdr(Some("sample-app-flok"));

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .args([
            "cleanup",
            &repo.display().to_string(),
            "--confirm",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"confirmed\": true"))
        .stdout(predicate::str::contains("\"workspace_closed\": true"))
        .stdout(predicate::str::contains("\"removed\": true"))
        .stdout(predicate::str::contains("\"state_archived_to\":"));

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(log.contains("workspace close w_existing"));
    let worktrees = git_output(&repo, &["worktree", "list", "--porcelain"]);
    assert_eq!(worktrees.matches("worktree ").count(), 1);
    let branches = git_output(&repo, &["branch", "--format=%(refname:short)"]);
    assert_eq!(
        branches
            .lines()
            .filter(|line| line.contains("flok/"))
            .count(),
        3
    );
    let state_root = fixture.home.path().join("plugin-state");
    assert!(std::fs::read_dir(state_root.join("floks"))
        .expect("state dir")
        .next()
        .is_none());
    assert_eq!(
        std::fs::read_dir(state_root.join("history"))
            .expect("history dir")
            .count(),
        1
    );
}

#[test]
fn cleanup_confirm_refuses_dirty_worker_before_closing_workspace() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    let repo = fixture.real_git_repo("sample-app");
    for tool in ["pi", "codex", "claude", "opencode"] {
        fixture.fake_tool(tool);
    }
    fixture.fake_herdr(None);
    open_test_flok(&fixture, &repo);
    let worktrees = git_output(&repo, &["worktree", "list", "--porcelain"]);
    let codex = worktrees
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .find(|path| path.ends_with("/codex"))
        .expect("codex worktree");
    std::fs::write(
        std::path::Path::new(codex).join("UNCOMMITTED.txt"),
        "keep\n",
    )
    .expect("dirty worker");
    fixture.fake_herdr(Some("sample-app-flok"));

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .args([
            "cleanup",
            &repo.display().to_string(),
            "--confirm",
            "--json",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"can_cleanup\": false"))
        .stdout(predicate::str::contains(
            "worker checkout is dirty and will be preserved",
        ));

    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(!log.contains("workspace close w_existing"));
    let worktrees = git_output(&repo, &["worktree", "list", "--porcelain"]);
    assert_eq!(worktrees.matches("worktree ").count(), 4);
}

#[test]
fn cleanup_prompt_requires_the_active_project_name_before_mutating() {
    let fixture = Fixture::new();
    fixture.write_config("codex");
    let repo = fixture.real_git_repo("sample-app");
    for tool in ["pi", "codex", "claude", "opencode"] {
        fixture.fake_tool(tool);
    }
    fixture.fake_herdr(None);
    open_test_flok(&fixture, &repo);
    fixture.fake_herdr(Some("sample-app-flok"));
    let context = serde_json::json!({ "focused_pane_cwd": repo });

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .env("HERDR_PLUGIN_CONTEXT_JSON", context.to_string())
        .arg("cleanup-prompt")
        .write_stdin("wrong-name\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Cleanup cancelled; nothing changed.",
        ));
    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(!log.contains("workspace close w_existing"));

    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .env("HERDR_PLUGIN_CONTEXT_JSON", context.to_string())
        .arg("cleanup-prompt")
        .write_stdin("sample-app\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Flok cleanup: confirmed"));
    let log = std::fs::read_to_string(fixture.log()).expect("log");
    assert!(log.contains("workspace close w_existing"));
}

fn open_test_flok(fixture: &Fixture, repo: &std::path::Path) {
    Command::cargo_bin("sheprd")
        .expect("binary")
        .envs(fixture.env())
        .args(["flok", &repo.display().to_string(), "--json"])
        .assert()
        .success();
}

fn git_output(repo: &std::path::Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("git command");
    assert!(output.status.success());
    String::from_utf8(output.stdout).expect("utf-8 git output")
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
            (
                "SHEPRD_STATE_DIR".into(),
                self.home.path().join("plugin-state").display().to_string(),
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

    fn real_git_repo(&self, name: &str) -> std::path::PathBuf {
        let repo = self.root.join(name);
        std::fs::create_dir_all(&repo).expect("repo");
        for args in [
            vec!["init", "-q"],
            vec!["config", "user.email", "test@example.com"],
            vec!["config", "user.name", "Sheprd Test"],
        ] {
            let status = std::process::Command::new("git")
                .args(args)
                .current_dir(&repo)
                .status()
                .expect("git setup");
            assert!(status.success());
        }
        std::fs::write(repo.join("README.md"), "# fixture\n").expect("seed");
        let status = std::process::Command::new("git")
            .args(["add", "README.md"])
            .current_dir(&repo)
            .status()
            .expect("git add");
        assert!(status.success());
        let status = std::process::Command::new("git")
            .args(["commit", "-q", "-m", "seed"])
            .current_dir(&repo)
            .status()
            .expect("git commit");
        assert!(status.success());
        repo
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
  "agent start")
    if [ -n "${{HERDR_DIRTY_ON_KIND:-}}" ] && [ "$5" = "$HERDR_DIRTY_ON_KIND" ]; then
      dirty_dir=$(find "$SHEPRD_STATE_DIR/worktrees" -type d -name "$5" -print -quit)
      if [ -n "$dirty_dir" ]; then
        printf 'preserve me\n' > "$dirty_dir/UNCOMMITTED.txt"
      fi
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
