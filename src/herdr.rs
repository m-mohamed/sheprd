use crate::config::FlokConfig;
use crate::error::{Result, SheprdError};
use crate::project::Project;
use crate::recipe::{Agent, Recipe};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::OpenOptions;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const MIN_HERDR_VERSION: &str = "0.7.5";
const FLOK_STATE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug)]
pub struct ServerStatus {
    pub running: bool,
    pub version: Option<String>,
    pub protocol: Option<String>,
    pub compatible: Option<bool>,
    pub socket: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectAction {
    CreatedWorkspace,
    FocusedExisting,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConnectOutcome {
    pub action: ConnectAction,
    pub workspace_id: String,
    pub workspace_label: String,
    pub recipe: Option<String>,
    pub attached: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlokAction {
    CreatedFlok,
    FocusedExisting,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FlokAgent {
    pub role: String,
    pub kind: String,
    pub name: String,
    pub pane_id: String,
    pub model: String,
    pub effort: String,
    pub cwd: String,
    pub branch: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FlokOutcome {
    #[serde(default = "flok_state_schema_version")]
    pub schema_version: u32,
    pub action: FlokAction,
    pub project: String,
    pub workspace_id: String,
    pub workspace_label: String,
    pub state_path: String,
    pub agents: Vec<FlokAgent>,
    #[serde(default)]
    pub healthy: bool,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FlokCleanupOutcome {
    pub project: String,
    pub workspace_id: Option<String>,
    pub state_path: String,
    pub confirmed: bool,
    pub can_cleanup: bool,
    pub workspace_closed: bool,
    pub state_archived_to: Option<String>,
    pub worktrees: Vec<FlokCleanupWorktree>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FlokCleanupWorktree {
    pub kind: String,
    pub path: String,
    pub branch: String,
    pub exists: bool,
    pub clean: bool,
    pub removed: bool,
}

#[derive(Debug, Deserialize)]
struct HerdrEnvelope<T> {
    result: T,
}

#[derive(Debug, Deserialize)]
struct WorkspaceList {
    workspaces: Vec<Workspace>,
}

#[derive(Clone, Debug, Deserialize)]
struct Workspace {
    workspace_id: String,
    label: String,
}

#[derive(Debug, Deserialize)]
struct WorkspaceCreated {
    workspace: Workspace,
    root_pane: Pane,
}

#[derive(Debug, Deserialize)]
struct TabCreated {
    root_pane: Pane,
}

#[derive(Debug, Deserialize)]
struct PaneInfo {
    pane: Pane,
}

#[derive(Debug, Deserialize)]
struct Pane {
    pane_id: String,
    tab_id: String,
}

#[derive(Debug, Deserialize)]
struct AgentList {
    agents: Vec<LiveAgent>,
}

#[derive(Debug, Deserialize)]
struct LiveAgent {
    agent: String,
    interactive_ready: bool,
    name: String,
    workspace_id: String,
}

pub fn workspace_labels() -> Result<BTreeSet<String>> {
    Ok(workspaces()?
        .into_iter()
        .map(|workspace| workspace.label)
        .collect())
}

pub fn connect(
    project: &Project,
    agent: Agent,
    recipe: Option<&Recipe>,
    attach: bool,
) -> Result<ConnectOutcome> {
    ensure_server()?;
    let label = project.workspace_label(agent);
    if let Some(workspace) = workspaces()?
        .into_iter()
        .find(|workspace| workspace.label == label)
    {
        run_herdr(["workspace", "focus", &workspace.workspace_id])?;
        let attached = maybe_attach(attach)?;
        return Ok(ConnectOutcome {
            action: ConnectAction::FocusedExisting,
            workspace_id: workspace.workspace_id,
            workspace_label: workspace.label,
            recipe: None,
            attached,
        });
    }

    let created = create_workspace(project.path.as_path(), &label)?;
    let recipe_name = recipe.map(|recipe| recipe.name.clone());
    if let Some(recipe) = recipe {
        apply_agent_dev(project.path.as_path(), recipe.agent, &created)?;
    }
    run_herdr(["workspace", "focus", &created.workspace.workspace_id])?;
    let attached = maybe_attach(attach)?;
    Ok(ConnectOutcome {
        action: ConnectAction::CreatedWorkspace,
        workspace_id: created.workspace.workspace_id,
        workspace_label: created.workspace.label,
        recipe: recipe_name,
        attached,
    })
}

pub fn open_flok(project: &Project, config: &FlokConfig) -> Result<FlokOutcome> {
    ensure_server()?;
    ensure_minimum_herdr_version()?;
    let _lock = FlokLock::acquire(project)?;
    let label = format!("{}-flok", project.name);
    let state_path = flok_state_path(project)?;
    if let Some(workspace) = workspaces()?
        .into_iter()
        .find(|workspace| workspace.label == label)
    {
        run_herdr(["workspace", "focus", &workspace.workspace_id])?;
        return focused_flok_outcome(project, workspace, label, state_path);
    }

    validate_flok_config(config)?;
    ensure_flok_tools()?;
    ensure_clean_checkout(&project.path)?;
    let run_id = flok_run_id();
    let mut created_worktrees = Vec::new();
    let mut created_workspace_id = None;

    let result = (|| {
        let codex_worktree = create_worker_worktree(project, "codex", &run_id)?;
        created_worktrees.push(codex_worktree.clone());
        let claude_worktree = create_worker_worktree(project, "claude", &run_id)?;
        created_worktrees.push(claude_worktree.clone());
        let opencode_worktree = create_worker_worktree(project, "opencode", &run_id)?;
        created_worktrees.push(opencode_worktree.clone());

        let created = create_workspace(project.path.as_path(), &label)?;
        created_workspace_id = Some(created.workspace.workspace_id.clone());
        run_herdr(["tab", "rename", &created.root_pane.tab_id, "Flok"])?;
        run_herdr([
            "pane",
            "rename",
            &created.root_pane.pane_id,
            "Pi · conductor",
        ])?;

        let codex_pane = split_pane_with_env(
            &codex_worktree.path,
            &created.root_pane.pane_id,
            "right",
            None,
        )?;
        run_herdr(["pane", "rename", &codex_pane.pane_id, "Codex · Sol"])?;

        let claude_pane = split_pane_with_env(
            &claude_worktree.path,
            &created.root_pane.pane_id,
            "down",
            None,
        )?;
        run_herdr(["pane", "rename", &claude_pane.pane_id, "Claude · Opus"])?;

        let opencode_inline_config = serde_json::json!({
            "model": config.opencode_model,
            "default_agent": "build",
            "agent": {
                "build": {
                    "model": config.opencode_model,
                    "variant": config.effort
                }
            }
        })
        .to_string();
        let opencode_pane = split_pane_with_env(
            &opencode_worktree.path,
            &codex_pane.pane_id,
            "down",
            Some(("OPENCODE_CONFIG_CONTENT", &opencode_inline_config)),
        )?;
        run_herdr([
            "pane",
            "rename",
            &opencode_pane.pane_id,
            "OpenCode · Kimi K3",
        ])?;

        let pi_name = agent_name(project, "pi");
        let codex_name = agent_name(project, "codex");
        let claude_name = agent_name(project, "claude");
        let opencode_name = agent_name(project, "opencode");
        let conductor_prompt = format!(
            "You are the dedicated Sheprd Flok conductor for {}. Keep the codebase clean: do not edit project files yourself. Delegate implementation, review, and test packets to the three visible Herdr workers with `herdr agent prompt`, monitor them with `herdr agent wait` and `herdr agent read`, then synthesize and verify their work. The workers are Codex `{codex_name}` at {}, Claude Code `{claude_name}` at {}, and OpenCode `{opencode_name}` at {}. Each worker already has an isolated git worktree. Never spawn hidden subagents, never add more coding agents, and never claim success without repository checks.",
            project.name,
            codex_worktree.path.display(),
            claude_worktree.path.display(),
            opencode_worktree.path.display(),
        );

        start_agent(
            &created.root_pane.pane_id,
            &pi_name,
            "pi",
            &[
                "--model".into(),
                config.pi_model.clone(),
                "--thinking".into(),
                config.effort.clone(),
                "--tools".into(),
                "read,bash,grep,find,ls".into(),
                "--approve".into(),
                "--name".into(),
                pi_name.clone(),
                "--append-system-prompt".into(),
                conductor_prompt,
            ],
        )?;
        start_agent(
            &codex_pane.pane_id,
            &codex_name,
            "codex",
            &[
                "--model".into(),
                config.codex_model.clone(),
                "--config".into(),
                format!("model_reasoning_effort={}", config.effort),
                "--sandbox".into(),
                "workspace-write".into(),
                "--add-dir".into(),
                project.path.join(".git").to_string_lossy().into_owned(),
                "--ask-for-approval".into(),
                "never".into(),
            ],
        )?;
        start_agent(
            &claude_pane.pane_id,
            &claude_name,
            "claude",
            &[
                "--model".into(),
                config.claude_model.clone(),
                "--effort".into(),
                config.effort.clone(),
                "--permission-mode".into(),
                "auto".into(),
                "--disallowedTools".into(),
                "Agent,Task".into(),
            ],
        )?;
        start_agent(
            &opencode_pane.pane_id,
            &opencode_name,
            "opencode",
            &["--agent".into(), "build".into(), "--mini".into()],
        )?;

        let mut outcome = FlokOutcome {
            schema_version: FLOK_STATE_SCHEMA_VERSION,
            action: FlokAction::CreatedFlok,
            project: project.name.clone(),
            workspace_id: created.workspace.workspace_id.clone(),
            workspace_label: label.clone(),
            state_path: state_path.display().to_string(),
            agents: vec![
                FlokAgent {
                    role: "conductor".into(),
                    kind: "pi".into(),
                    name: pi_name,
                    pane_id: created.root_pane.pane_id,
                    model: config.pi_model.clone(),
                    effort: config.effort.clone(),
                    cwd: project.path.display().to_string(),
                    branch: None,
                },
                worker_agent(
                    "codex",
                    codex_name,
                    codex_pane.pane_id,
                    config.codex_model.clone(),
                    config,
                    codex_worktree,
                ),
                worker_agent(
                    "claude",
                    claude_name,
                    claude_pane.pane_id,
                    config.claude_model.clone(),
                    config,
                    claude_worktree,
                ),
                worker_agent(
                    "opencode",
                    opencode_name,
                    opencode_pane.pane_id,
                    config.opencode_model.clone(),
                    config,
                    opencode_worktree,
                ),
            ],
            healthy: false,
            warnings: Vec::new(),
        };
        let (healthy, warnings) = live_flok_health(&outcome.workspace_id, &outcome.agents);
        outcome.healthy = healthy;
        outcome.warnings = warnings;
        if !outcome.healthy {
            return Err(SheprdError::Message(format!(
                "Herdr did not report a healthy four-agent Flok: {}",
                outcome.warnings.join("; ")
            )));
        }
        write_flok_state(&state_path, &outcome)?;
        run_herdr(["workspace", "focus", &created.workspace.workspace_id])?;
        Ok(outcome)
    })();

    match result {
        Ok(outcome) => Ok(outcome),
        Err(error) => {
            let rollback =
                rollback_partial_flok(project, created_workspace_id.as_deref(), &created_worktrees);
            Err(SheprdError::Message(format!(
                "{error}; rollback: {}",
                rollback.join("; ")
            )))
        }
    }
}

pub fn cleanup_flok(project: &Project, confirm: bool) -> Result<FlokCleanupOutcome> {
    ensure_server()?;
    ensure_minimum_herdr_version()?;
    let _lock = FlokLock::acquire(project)?;
    let state_path = flok_state_path(project)?;
    let contents = std::fs::read_to_string(&state_path).map_err(|error| {
        SheprdError::Message(format!(
            "could not read Flok state at {}: {error}",
            state_path.display()
        ))
    })?;
    let state: FlokOutcome = serde_json::from_str(&contents).map_err(|error| {
        SheprdError::Message(format!(
            "could not parse Flok state at {}: {error}",
            state_path.display()
        ))
    })?;
    let workspace_id = workspaces()?
        .into_iter()
        .find(|workspace| workspace.label == state.workspace_label)
        .map(|workspace| workspace.workspace_id);
    let mut warnings = Vec::new();
    let workers = state
        .agents
        .iter()
        .filter_map(|agent| {
            agent.branch.as_ref().map(|branch| {
                (
                    agent.kind.clone(),
                    PathBuf::from(&agent.cwd),
                    branch.clone(),
                )
            })
        })
        .collect::<Vec<_>>();
    if workers.len() != 3 {
        warnings.push(format!(
            "saved state describes {} worker checkouts instead of 3",
            workers.len()
        ));
    }

    let mut worktrees = workers
        .into_iter()
        .map(|(kind, path, branch)| {
            let owned = worktree_path_is_owned(project, &path)?;
            if !owned {
                warnings.push(format!(
                    "refusing out-of-scope worktree path from saved state: {}",
                    path.display()
                ));
            }
            let exists = path.exists();
            let clean = if exists && owned {
                checkout_is_clean(&path)?
            } else {
                !exists && owned
            };
            Ok(FlokCleanupWorktree {
                kind,
                path: path.display().to_string(),
                branch,
                exists,
                clean,
                removed: false,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    for worktree in &worktrees {
        if worktree.exists && !worktree.clean {
            warnings.push(format!(
                "worker checkout is dirty and will be preserved: {}",
                worktree.path
            ));
        }
    }

    let mut outcome = FlokCleanupOutcome {
        project: project.name.clone(),
        workspace_id,
        state_path: state_path.display().to_string(),
        confirmed: confirm,
        can_cleanup: warnings.is_empty(),
        workspace_closed: false,
        state_archived_to: None,
        worktrees: worktrees.clone(),
        warnings,
    };
    if !confirm || !outcome.can_cleanup {
        return Ok(outcome);
    }

    if let Some(workspace_id) = outcome.workspace_id.as_deref() {
        run_herdr(["workspace", "close", workspace_id]).map_err(|error| {
            SheprdError::Message(format!(
                "could not close Flok workspace {workspace_id}; no worker checkouts were removed: {error}"
            ))
        })?;
        outcome.workspace_closed = true;
    }

    for worktree in &mut worktrees {
        if !worktree.exists {
            worktree.removed = true;
            continue;
        }
        let path = PathBuf::from(&worktree.path);
        worktree.clean = checkout_is_clean(&path)?;
        if !worktree.clean {
            outcome.can_cleanup = false;
            outcome.warnings.push(format!(
                "worker checkout became dirty while closing the Flok and was preserved: {}",
                worktree.path
            ));
        }
    }
    if !outcome.can_cleanup {
        outcome.worktrees = worktrees;
        return Ok(outcome);
    }

    for worktree in &mut worktrees {
        if worktree.removed {
            continue;
        }
        let worker = WorkerWorktree {
            path: PathBuf::from(&worktree.path),
            branch: worktree.branch.clone(),
        };
        match remove_worker_worktree(project, &worker) {
            Ok(()) => worktree.removed = true,
            Err(error) => {
                outcome.can_cleanup = false;
                outcome.warnings.push(format!(
                    "could not remove clean worker checkout {}: {error}",
                    worktree.path
                ));
            }
        }
    }
    outcome.worktrees = worktrees;
    if !outcome.can_cleanup {
        return Ok(outcome);
    }

    let history_dir = plugin_state_root()?.join("history");
    std::fs::create_dir_all(&history_dir)?;
    let archived = history_dir.join(format!(
        "{}-{}.json",
        short_hash(&project.path),
        flok_run_id()
    ));
    std::fs::rename(&state_path, &archived)?;
    outcome.state_archived_to = Some(archived.display().to_string());
    Ok(outcome)
}

fn worktree_path_is_owned(project: &Project, path: &Path) -> Result<bool> {
    if path
        .components()
        .any(|component| component == std::path::Component::ParentDir)
    {
        return Ok(false);
    }
    let expected = plugin_state_root()?
        .join("worktrees")
        .join(short_hash(&project.path));
    if path.exists() && expected.exists() {
        return Ok(path.canonicalize()?.starts_with(expected.canonicalize()?));
    }
    Ok(path.starts_with(expected))
}

fn flok_state_schema_version() -> u32 {
    FLOK_STATE_SCHEMA_VERSION
}

fn focused_flok_outcome(
    project: &Project,
    workspace: Workspace,
    label: String,
    state_path: PathBuf,
) -> Result<FlokOutcome> {
    let mut warnings = Vec::new();
    let mut outcome = match std::fs::read_to_string(&state_path) {
        Ok(contents) => match serde_json::from_str::<FlokOutcome>(&contents) {
            Ok(outcome) => outcome,
            Err(error) => {
                warnings.push(format!("saved Flok state is unreadable: {error}"));
                empty_focused_outcome(project, &workspace, label, &state_path)
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            warnings.push("saved Flok state is missing".into());
            empty_focused_outcome(project, &workspace, label, &state_path)
        }
        Err(error) => {
            warnings.push(format!("saved Flok state could not be read: {error}"));
            empty_focused_outcome(project, &workspace, label, &state_path)
        }
    };
    outcome.schema_version = FLOK_STATE_SCHEMA_VERSION;
    outcome.action = FlokAction::FocusedExisting;
    outcome.workspace_id = workspace.workspace_id;
    let (healthy, live_warnings) = live_flok_health(&outcome.workspace_id, &outcome.agents);
    warnings.extend(live_warnings);
    outcome.healthy = warnings.is_empty() && healthy;
    outcome.warnings = warnings;
    Ok(outcome)
}

fn empty_focused_outcome(
    project: &Project,
    workspace: &Workspace,
    label: String,
    state_path: &Path,
) -> FlokOutcome {
    FlokOutcome {
        schema_version: FLOK_STATE_SCHEMA_VERSION,
        action: FlokAction::FocusedExisting,
        project: project.name.clone(),
        workspace_id: workspace.workspace_id.clone(),
        workspace_label: label,
        state_path: state_path.display().to_string(),
        agents: Vec::new(),
        healthy: false,
        warnings: Vec::new(),
    }
}

fn live_flok_health(workspace_id: &str, expected: &[FlokAgent]) -> (bool, Vec<String>) {
    let output: Result<HerdrEnvelope<AgentList>> = run_herdr_json(["agent", "list"]);
    let live = match output {
        Ok(output) => output.result.agents,
        Err(error) => {
            return (
                false,
                vec![format!("could not inspect live agents: {error}")],
            )
        }
    };
    let live = live
        .into_iter()
        .filter(|agent| agent.workspace_id == workspace_id)
        .collect::<Vec<_>>();
    let mut warnings = Vec::new();
    if expected.len() != 4 {
        warnings.push(format!(
            "saved state describes {} agents instead of 4",
            expected.len()
        ));
    }
    if live.len() != 4 {
        warnings.push(format!(
            "Herdr reports {} live agents in the Flok workspace instead of 4",
            live.len()
        ));
    }
    for expected_agent in expected {
        match live.iter().find(|agent| agent.name == expected_agent.name) {
            Some(agent) if agent.agent != expected_agent.kind => warnings.push(format!(
                "{} is reported as {} instead of {}",
                expected_agent.name, agent.agent, expected_agent.kind
            )),
            Some(agent) if !agent.interactive_ready => {
                warnings.push(format!("{} is not interactive-ready", expected_agent.name))
            }
            Some(_) => {}
            None => warnings.push(format!("{} is not live", expected_agent.name)),
        }
    }
    (warnings.is_empty(), warnings)
}

fn validate_flok_config(config: &FlokConfig) -> Result<()> {
    for (name, value) in [
        ("effort", &config.effort),
        ("pi_model", &config.pi_model),
        ("codex_model", &config.codex_model),
        ("claude_model", &config.claude_model),
        ("opencode_model", &config.opencode_model),
    ] {
        if value.trim().is_empty() {
            return Err(SheprdError::Message(format!(
                "Flok config field `{name}` must not be empty"
            )));
        }
    }
    Ok(())
}

fn ensure_flok_tools() -> Result<()> {
    let missing = ["git", "pi", "codex", "claude", "opencode"]
        .into_iter()
        .filter(|name| !command_on_path(name))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(SheprdError::Message(format!(
            "Flok prerequisites are missing from PATH: {}",
            missing.join(", ")
        )))
    }
}

fn command_on_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths)
            .map(|path| path.join(name))
            .any(|path| path.is_file())
    })
}

struct FlokLock {
    path: PathBuf,
}

impl FlokLock {
    fn acquire(project: &Project) -> Result<Self> {
        let lock_dir = plugin_state_root()?.join("locks");
        std::fs::create_dir_all(&lock_dir)?;
        let lock_path = lock_dir.join(format!("{}.lock", short_hash(&project.path)));
        for attempt in 0..2 {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(mut file) => {
                    writeln!(file, "{}", std::process::id())?;
                    file.sync_all()?;
                    return Ok(Self { path: lock_path });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && attempt == 0 => {
                    if lock_owner_is_alive(&lock_path) {
                        return Err(SheprdError::Message(format!(
                            "another Sheprd Flok operation is already running for {}",
                            project.path.display()
                        )));
                    }
                    std::fs::remove_file(&lock_path)?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    return Err(SheprdError::Message(format!(
                        "another Sheprd Flok operation is already running for {}",
                        project.path.display()
                    )));
                }
                Err(error) => return Err(error.into()),
            }
        }
        Err(SheprdError::Message(
            "could not acquire the Sheprd Flok operation lock".into(),
        ))
    }
}

impl Drop for FlokLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn lock_owner_is_alive(lock_path: &Path) -> bool {
    let Ok(contents) = std::fs::read_to_string(lock_path) else {
        return true;
    };
    let Ok(pid) = contents.trim().parse::<u32>() else {
        return true;
    };
    if pid == std::process::id() {
        return true;
    }
    Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "pid="])
        .output()
        .is_ok_and(|output| output.status.success() && !output.stdout.is_empty())
}

fn rollback_partial_flok(
    project: &Project,
    workspace_id: Option<&str>,
    worktrees: &[WorkerWorktree],
) -> Vec<String> {
    let mut notes = Vec::new();
    let workspace_closed = match workspace_id {
        Some(workspace_id) => match run_herdr(["workspace", "close", workspace_id]) {
            Ok(()) => {
                notes.push(format!("closed partial workspace {workspace_id}"));
                true
            }
            Err(error) => {
                notes.push(format!(
                    "preserved partial workspace {workspace_id} because close failed: {error}"
                ));
                false
            }
        },
        None => true,
    };

    if workspace_closed {
        for worktree in worktrees.iter().rev() {
            match checkout_is_clean(&worktree.path) {
                Ok(true) => match remove_worker_worktree(project, worktree) {
                    Ok(()) => notes.push(format!(
                        "removed clean partial worktree {} (branch preserved: {})",
                        worktree.path.display(),
                        worktree.branch
                    )),
                    Err(error) => notes.push(format!(
                        "preserved partial worktree {} because removal failed: {error}",
                        worktree.path.display()
                    )),
                },
                Ok(false) => notes.push(format!(
                    "preserved dirty partial worktree {} on branch {}",
                    worktree.path.display(),
                    worktree.branch
                )),
                Err(error) => notes.push(format!(
                    "preserved partial worktree {} because cleanliness could not be verified: {error}",
                    worktree.path.display()
                )),
            }
        }
    } else if !worktrees.is_empty() {
        notes.push(
            "preserved worker worktrees because the partial workspace may still use them".into(),
        );
    }

    if notes.is_empty() {
        notes.push("no Herdr workspace or worker worktrees were created".into());
    }
    notes
}

fn remove_worker_worktree(project: &Project, worktree: &WorkerWorktree) -> Result<()> {
    let output = Command::new("git")
        .args(["worktree", "remove", "--force"])
        .arg(&worktree.path)
        .current_dir(&project.path)
        .output()?;
    if !output.status.success() {
        return Err(SheprdError::Message(command_error(output.stderr)));
    }
    let _ = Command::new("git")
        .args(["worktree", "prune"])
        .current_dir(&project.path)
        .output();
    if let Some(parent) = worktree.path.parent() {
        let _ = std::fs::remove_dir(parent);
    }
    Ok(())
}

fn write_flok_state(path: &Path, outcome: &FlokOutcome) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| SheprdError::Message("invalid Flok state path".into()))?;
    std::fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("flok-state"),
        std::process::id()
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        serde_json::to_writer_pretty(&mut file, outcome)?;
        writeln!(file)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

#[derive(Clone)]
struct WorkerWorktree {
    path: PathBuf,
    branch: String,
}

fn worker_agent(
    kind: &str,
    name: String,
    pane_id: String,
    model: String,
    config: &FlokConfig,
    worktree: WorkerWorktree,
) -> FlokAgent {
    FlokAgent {
        role: "worker".into(),
        kind: kind.into(),
        name,
        pane_id,
        model,
        effort: config.effort.clone(),
        cwd: worktree.path.display().to_string(),
        branch: Some(worktree.branch),
    }
}

fn ensure_clean_checkout(cwd: &Path) -> Result<()> {
    if !checkout_is_clean(cwd)? {
        return Err(SheprdError::Message(format!(
            "refusing to open a new Flok from a dirty checkout: {}",
            cwd.display()
        )));
    }
    Ok(())
}

fn checkout_is_clean(cwd: &Path) -> Result<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .current_dir(cwd)
        .output()?;
    if !output.status.success() {
        return Err(SheprdError::Message(command_error(output.stderr)));
    }
    Ok(output.stdout.is_empty())
}

fn create_worker_worktree(project: &Project, role: &str, run_id: &str) -> Result<WorkerWorktree> {
    let state_root = plugin_state_root()?;
    let repo_id = short_hash(&project.path);
    let path = state_root
        .join("worktrees")
        .join(repo_id)
        .join(run_id)
        .join(role);
    let parent = path
        .parent()
        .ok_or_else(|| SheprdError::Message("invalid worktree path".into()))?;
    std::fs::create_dir_all(parent)?;
    let branch = format!("flok/{}/{run_id}-{role}", slug(&project.name, 20));
    let output = Command::new("git")
        .args(["worktree", "add", "-b", &branch])
        .arg(&path)
        .arg("HEAD")
        .current_dir(&project.path)
        .output()?;
    if !output.status.success() {
        return Err(SheprdError::Message(command_error(output.stderr)));
    }
    Ok(WorkerWorktree { path, branch })
}

fn plugin_state_root() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("HERDR_PLUGIN_STATE_DIR") {
        return Ok(PathBuf::from(path));
    }
    let home = std::env::var_os("HOME").ok_or(SheprdError::MissingHome)?;
    Ok(PathBuf::from(home).join(".local/state/sheprd"))
}

fn flok_state_path(project: &Project) -> Result<PathBuf> {
    Ok(plugin_state_root()?
        .join("floks")
        .join(format!("{}.json", short_hash(&project.path))))
}

fn flok_run_id() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{seconds}-{}", std::process::id())
}

fn agent_name(project: &Project, role: &str) -> String {
    format!(
        "{}-{}-{}",
        slug(&project.name, 15),
        role,
        &short_hash(&project.path)[..4]
    )
}

fn slug(value: &str, max: usize) -> String {
    let mut output = String::new();
    let mut hyphen = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            hyphen = false;
        } else if !output.is_empty() && !hyphen {
            output.push('-');
            hyphen = true;
        }
        if output.len() >= max {
            break;
        }
    }
    output.trim_matches('-').to_string()
}

fn short_hash(value: &Path) -> String {
    let digest = Sha256::digest(value.to_string_lossy().as_bytes());
    digest[..4]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn start_agent(pane_id: &str, name: &str, kind: &str, agent_args: &[String]) -> Result<()> {
    let mut args = vec![
        "agent".into(),
        "start".into(),
        name.into(),
        "--kind".into(),
        kind.into(),
        "--pane".into(),
        pane_id.into(),
        "--timeout".into(),
        "120000".into(),
        "--".into(),
    ];
    args.extend_from_slice(agent_args);
    run_herdr_args(&args)
}

fn apply_agent_dev(cwd: &Path, agent: Agent, created: &WorkspaceCreated) -> Result<()> {
    run_herdr(["tab", "rename", &created.root_pane.tab_id, "code"])?;
    run_herdr(["pane", "rename", &created.root_pane.pane_id, "nvim"])?;
    run_herdr(["pane", "run", &created.root_pane.pane_id, "nvim ."])?;

    let agent_pane = split_pane(cwd, &created.root_pane.pane_id, "right")?;
    run_herdr(["pane", "rename", &agent_pane.pane_id, agent.executable()])?;
    run_herdr(["pane", "run", &agent_pane.pane_id, agent.executable()])?;
    run_herdr([
        "pane",
        "report-agent",
        &agent_pane.pane_id,
        "--source",
        "sheprd",
        "--agent",
        agent.executable(),
        "--state",
        "unknown",
    ])?;

    let shell_pane = split_pane(cwd, &created.root_pane.pane_id, "down")?;
    run_herdr(["pane", "rename", &shell_pane.pane_id, "shell"])?;

    let git_tab = create_tab(cwd, &created.workspace.workspace_id, "git")?;
    run_herdr(["pane", "rename", &git_tab.root_pane.pane_id, "lazygit"])?;
    run_herdr(["pane", "run", &git_tab.root_pane.pane_id, "lazygit"])?;
    let git_shell = split_pane(cwd, &git_tab.root_pane.pane_id, "down")?;
    run_herdr(["pane", "rename", &git_shell.pane_id, "shell"])?;

    run_herdr(["tab", "focus", &created.root_pane.tab_id])?;
    Ok(())
}

fn workspaces() -> Result<Vec<Workspace>> {
    let output: HerdrEnvelope<WorkspaceList> = run_herdr_json(["workspace", "list"])?;
    Ok(output.result.workspaces)
}

fn create_workspace(cwd: &Path, label: &str) -> Result<WorkspaceCreated> {
    let output: HerdrEnvelope<WorkspaceCreated> = run_herdr_json([
        "workspace",
        "create",
        "--cwd",
        path_str(cwd)?,
        "--label",
        label,
        "--focus",
    ])?;
    Ok(output.result)
}

fn create_tab(cwd: &Path, workspace_id: &str, label: &str) -> Result<TabCreated> {
    let output: HerdrEnvelope<TabCreated> = run_herdr_json([
        "tab",
        "create",
        "--workspace",
        workspace_id,
        "--cwd",
        path_str(cwd)?,
        "--label",
        label,
        "--no-focus",
    ])?;
    Ok(output.result)
}

fn split_pane(cwd: &Path, pane_id: &str, direction: &str) -> Result<Pane> {
    let output: HerdrEnvelope<PaneInfo> = run_herdr_json([
        "pane",
        "split",
        pane_id,
        "--direction",
        direction,
        "--cwd",
        path_str(cwd)?,
        "--no-focus",
    ])?;
    Ok(output.result.pane)
}

fn split_pane_with_env(
    cwd: &Path,
    pane_id: &str,
    direction: &str,
    env: Option<(&str, &str)>,
) -> Result<Pane> {
    let mut args = vec![
        "pane".into(),
        "split".into(),
        pane_id.into(),
        "--direction".into(),
        direction.into(),
        "--ratio".into(),
        "0.5".into(),
        "--cwd".into(),
        path_str(cwd)?.into(),
        "--no-focus".into(),
    ];
    if let Some((key, value)) = env {
        args.push("--env".into());
        args.push(format!("{key}={value}"));
    }
    run_herdr_json_args(&args).map(|output: HerdrEnvelope<PaneInfo>| output.result.pane)
}

pub fn server_status() -> Result<Option<ServerStatus>> {
    let output = Command::new(herdr_bin())
        .args(["status", "server"])
        .output();
    let output = match output {
        Ok(output) if output.status.success() => output,
        Ok(_) => return Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(Some(parse_server_status(&text)))
}

fn parse_server_status(text: &str) -> ServerStatus {
    let mut status = ServerStatus {
        running: false,
        version: None,
        protocol: None,
        compatible: None,
        socket: None,
    };
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "status" => status.running = value == "running",
            "version" => status.version = Some(value.into()),
            "protocol" => status.protocol = Some(value.into()),
            "compatible" => status.compatible = Some(value == "yes" || value == "true"),
            "socket" => status.socket = Some(value.into()),
            _ => {}
        }
    }
    status
}

fn ensure_server() -> Result<()> {
    if let Some(status) = server_status()? {
        if status.running && status.compatible != Some(false) {
            return Ok(());
        }
    }

    Command::new(herdr_bin())
        .arg("server")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    for _ in 0..40 {
        if let Some(status) = server_status()? {
            if status.running && status.compatible != Some(false) {
                return Ok(());
            }
        }
        thread::sleep(Duration::from_millis(100));
    }

    Err(SheprdError::Message(
        "Herdr server did not become reachable".into(),
    ))
}

fn ensure_minimum_herdr_version() -> Result<()> {
    let status = server_status()?.ok_or_else(|| {
        SheprdError::Message("Herdr server is not reachable after startup".into())
    })?;
    let version = status
        .version
        .ok_or_else(|| SheprdError::Message("Herdr did not report its version".into()))?;
    if version_at_least(&version, MIN_HERDR_VERSION) {
        Ok(())
    } else {
        Err(SheprdError::Message(format!(
            "Sheprd Flok requires Herdr {MIN_HERDR_VERSION} or newer; running {version}"
        )))
    }
}

fn version_at_least(actual: &str, minimum: &str) -> bool {
    fn parts(value: &str) -> Option<(u64, u64, u64)> {
        let mut parts = value.trim_start_matches('v').split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts
            .next()?
            .split(|character: char| !character.is_ascii_digit())
            .next()?
            .parse()
            .ok()?;
        Some((major, minor, patch))
    }
    parts(actual)
        .zip(parts(minimum))
        .is_some_and(|(actual, minimum)| actual >= minimum)
}

fn maybe_attach(attach: bool) -> Result<bool> {
    if !attach || inside_herdr() || !std::io::stdout().is_terminal() {
        return Ok(false);
    }
    let status = Command::new(herdr_bin()).status()?;
    if status.success() {
        Ok(true)
    } else {
        Err(SheprdError::Message(format!(
            "herdr client exited with status {status}"
        )))
    }
}

fn inside_herdr() -> bool {
    ["HERDR_ENV", "HERDR_PANE_ID", "HERDR_SOCKET_PATH"]
        .iter()
        .any(|name| std::env::var_os(name).is_some())
}

fn run_herdr<const N: usize>(args: [&str; N]) -> Result<()> {
    let output = Command::new(herdr_bin()).args(args).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(SheprdError::Message(command_error(output.stderr)))
    }
}

fn run_herdr_args(args: &[String]) -> Result<()> {
    let output = Command::new(herdr_bin()).args(args).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(SheprdError::Message(command_error(output.stderr)))
    }
}

fn run_herdr_json_args<T>(args: &[String]) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let output = Command::new(herdr_bin()).args(args).output()?;
    if !output.status.success() {
        return Err(SheprdError::Message(command_error(output.stderr)));
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn run_herdr_json<T, const N: usize>(args: [&str; N]) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let output = Command::new(herdr_bin()).args(args).output()?;
    if !output.status.success() {
        return Err(SheprdError::Message(command_error(output.stderr)));
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn command_error(stderr: Vec<u8>) -> String {
    let message = String::from_utf8_lossy(&stderr).trim().to_string();
    if message.is_empty() {
        "herdr command failed".into()
    } else {
        message
    }
}

fn path_str(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| SheprdError::Message("path is not valid UTF-8".into()))
}

pub fn herdr_bin() -> std::ffi::OsString {
    std::env::var_os("HERDR_BIN_PATH").unwrap_or_else(|| "herdr".into())
}
