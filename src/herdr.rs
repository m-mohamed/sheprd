use crate::error::{Result, SheprdError};
use crate::project::Project;
use crate::recipe::{Agent, Recipe};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::io::IsTerminal;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct ServerStatus {
    pub running: bool,
    pub version: Option<String>,
    pub compatible: Option<bool>,
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

pub fn workspace_labels() -> Result<BTreeSet<String>> {
    Ok(workspaces()?
        .into_iter()
        .map(|workspace| workspace.label)
        .collect())
}

pub fn connect(project: &Project, recipe: &Recipe, attach: bool) -> Result<()> {
    ensure_server()?;
    let label = project.workspace_label(recipe.agent);
    if let Some(workspace) = workspaces()?
        .into_iter()
        .find(|workspace| workspace.label == label)
    {
        run_herdr(["workspace", "focus", &workspace.workspace_id])?;
        maybe_attach(attach)?;
        return Ok(());
    }

    let created = create_workspace(project.path.as_path(), &label)?;
    apply_agent_dev(project.path.as_path(), recipe.agent, &created)?;
    run_herdr(["workspace", "focus", &created.workspace.workspace_id])?;
    maybe_attach(attach)?;
    Ok(())
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

pub fn server_status() -> Result<Option<ServerStatus>> {
    let output = Command::new("herdr").args(["status", "server"]).output();
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
        compatible: None,
    };
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "status" => status.running = value == "running",
            "version" => status.version = Some(value.into()),
            "compatible" => status.compatible = Some(value == "yes" || value == "true"),
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

    Command::new("herdr")
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

fn maybe_attach(attach: bool) -> Result<()> {
    if !attach || inside_herdr() || !std::io::stdout().is_terminal() {
        return Ok(());
    }
    let status = Command::new("herdr").status()?;
    if status.success() {
        Ok(())
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
    let output = Command::new("herdr").args(args).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(SheprdError::Message(command_error(output.stderr)))
    }
}

fn run_herdr_json<T, const N: usize>(args: [&str; N]) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let output = Command::new("herdr").args(args).output()?;
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
