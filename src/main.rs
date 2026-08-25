mod cli;
mod config;
mod error;
mod herdr;
mod project;
mod recipe;

use clap::{CommandFactory, Parser};
use cli::{Cli, Command};
use config::Config;
use error::{Result, SheprdError};
use recipe::{Agent, Recipe, RecipeName};
use serde::Serialize;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json = cli.json;

    match run(cli) {
        Ok(code) => code,
        Err(error) => {
            if json {
                print_json_error(&error);
            } else {
                eprintln!("error: {error}");
            }
            ExitCode::from(error.exit_code())
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode> {
    let Some(command) = cli.command.clone() else {
        let mut command = Cli::command();
        command.print_help()?;
        println!();
        return Ok(ExitCode::SUCCESS);
    };

    if let Command::Init {
        print,
        force,
        roots,
    } = &command
    {
        return init(
            cli.agent.unwrap_or(Agent::Codex),
            roots.clone(),
            *print,
            *force,
            cli.json,
        );
    }

    let mut config = Config::load()?;
    if let Some(agent) = cli.agent {
        config.default_agent = agent;
    }

    match command {
        Command::Init { .. } => unreachable!("init returns before config load"),
        Command::List => list(&config, cli.json),
        Command::Connect { project, recipe } => {
            connect(&config, &project, recipe, cli.no_attach, cli.json)
        }
        Command::Recipes => recipes(&config, cli.json),
        Command::Doctor => doctor(&config, cli.json),
        Command::ShowConfig => show_config(&config, cli.json),
    }
}

#[derive(Serialize)]
struct InitOutput {
    path: String,
    existed: bool,
    written: bool,
    default_agent: Agent,
    roots: Vec<String>,
    contents: Option<String>,
}

fn init(
    agent: Agent,
    roots: Vec<String>,
    print_only: bool,
    force: bool,
    json: bool,
) -> Result<ExitCode> {
    let outcome = config::init(config::InitConfig {
        path: config::config_path()?,
        roots: if roots.is_empty() {
            config::default_root_strings()
        } else {
            roots
        },
        default_agent: agent,
        force,
        print_only,
    })?;

    if json {
        print_json(&InitOutput {
            path: outcome.path.display().to_string(),
            existed: outcome.existed,
            written: outcome.written,
            default_agent: outcome.default_agent,
            roots: outcome.roots,
            contents: if print_only {
                Some(outcome.contents)
            } else {
                None
            },
        })?;
    } else if print_only {
        print!("{}", outcome.contents);
    } else {
        println!(
            "{} config: {}",
            if outcome.existed {
                "updated"
            } else {
                "created"
            },
            outcome.path.display()
        );
    }

    Ok(ExitCode::SUCCESS)
}

#[derive(Serialize)]
struct ListOutput {
    agent: Agent,
    projects: Vec<ProjectRow>,
}

#[derive(Serialize)]
struct ProjectRow {
    name: String,
    path: String,
    workspace: String,
    running: bool,
}

fn list(config: &Config, json: bool) -> Result<ExitCode> {
    let running = herdr::workspace_labels().unwrap_or_default();
    let projects = project::discover(config)?;
    let rows = projects
        .into_iter()
        .map(|project| {
            let workspace = project.workspace_label(config.default_agent);
            ProjectRow {
                name: project.name,
                path: project.path.display().to_string(),
                running: running.contains(&workspace),
                workspace,
            }
        })
        .collect::<Vec<_>>();
    let output = ListOutput {
        agent: config.default_agent,
        projects: rows,
    };

    if json {
        print_json(&output)?;
    } else {
        println!("projects for {}:", output.agent);
        for project in &output.projects {
            let marker = if project.running { "*" } else { " " };
            println!("{marker} {:<28} {}", project.name, project.path);
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn connect(
    config: &Config,
    selector: &str,
    recipe_name: Option<RecipeName>,
    no_attach: bool,
    json: bool,
) -> Result<ExitCode> {
    let project = project::resolve(config, selector)?;
    let recipe = recipe_name.map(|name| name.build(config.default_agent));
    let outcome = herdr::connect(
        &project,
        config.default_agent,
        recipe.as_ref(),
        !no_attach && !json,
    )?;
    if json {
        print_json(&ConnectOutput {
            project: ProjectConfigRow {
                name: project.name,
                path: project.path.display().to_string(),
            },
            agent: config.default_agent,
            action: outcome.action,
            workspace: outcome.workspace_label,
            workspace_id: outcome.workspace_id,
            recipe: outcome.recipe,
            attached: outcome.attached,
        })?;
    } else {
        let action = match outcome.action {
            herdr::ConnectAction::CreatedWorkspace => "created",
            herdr::ConnectAction::FocusedExisting => "focused",
        };
        println!("{action} Herdr workspace: {}", outcome.workspace_label);
        println!("project: {}", project.name);
        println!("agent: {}", config.default_agent);
        if let Some(recipe) = outcome.recipe {
            println!("recipe: {recipe}");
        }
        println!("attached: {}", if outcome.attached { "yes" } else { "no" });
    }
    Ok(ExitCode::SUCCESS)
}

#[derive(Serialize)]
struct ConnectOutput {
    project: ProjectConfigRow,
    agent: Agent,
    action: herdr::ConnectAction,
    workspace: String,
    workspace_id: String,
    recipe: Option<String>,
    attached: bool,
}

#[derive(Serialize)]
struct RecipesOutput {
    default: Option<String>,
    recipes: Vec<Recipe>,
}

fn recipes(config: &Config, json: bool) -> Result<ExitCode> {
    let output = RecipesOutput {
        default: None,
        recipes: vec![Recipe::agent_dev(config.default_agent)],
    };
    if json {
        print_json(&output)?;
    } else {
        println!("sample recipes:");
        for recipe in &output.recipes {
            let marker = if output.default.as_deref() == Some(&recipe.name) {
                "*"
            } else {
                "-"
            };
            println!("{marker} {} - {}", recipe.name, recipe.description);
            for tab in &recipe.tabs {
                println!("    {} -> {}", tab.name, tab.panes.join(" | "));
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

#[derive(Serialize)]
struct DoctorOutput {
    ready: bool,
    herdr: DoctorHerdr,
    checks: Vec<Check>,
}

#[derive(Serialize)]
struct DoctorHerdr {
    running: bool,
    version: Option<String>,
    protocol: Option<String>,
    compatible: Option<bool>,
    socket: Option<String>,
    protocol_ready: bool,
    error: Option<String>,
}

impl DoctorHerdr {
    fn from_status(status: &herdr::ServerStatus) -> Self {
        let protocol_ready =
            status.running && status.compatible != Some(false) && status.socket.is_some();
        Self {
            running: status.running,
            version: status.version.clone(),
            protocol: status.protocol.clone(),
            compatible: status.compatible,
            socket: status.socket.clone(),
            protocol_ready,
            error: None,
        }
    }

    fn unavailable(error: impl Into<String>) -> Self {
        Self {
            running: false,
            version: None,
            protocol: None,
            compatible: None,
            socket: None,
            protocol_ready: false,
            error: Some(error.into()),
        }
    }
}

#[derive(Serialize)]
struct Check {
    name: String,
    ok: bool,
    detail: String,
}

fn doctor(_config: &Config, json: bool) -> Result<ExitCode> {
    let mut checks = vec![
        herdr_path_check(),
        path_check("git"),
        path_check("pi"),
        path_check("codex"),
        path_check("opencode"),
    ];

    let herdr = match herdr::server_status() {
        Ok(Some(status)) => {
            let herdr = DoctorHerdr::from_status(&status);
            let status_label = if status.running { "running" } else { "stopped" };
            let version = status.version.unwrap_or_else(|| "unknown".into());
            let protocol = status.protocol.unwrap_or_else(|| "unknown".into());
            let compatible = status
                .compatible
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".into());
            let socket = status.socket.unwrap_or_else(|| "unknown".into());
            checks.push(Check {
                name: "herdr_server".into(),
                ok: herdr.protocol_ready,
                detail: format!(
                    "status={status_label} version={version} protocol={protocol} compatible={compatible} socket={socket}"
                ),
            });
            herdr
        }
        Ok(None) => {
            checks.push(Check {
                name: "herdr_server".into(),
                ok: false,
                detail: "not reachable".into(),
            });
            DoctorHerdr::unavailable("not reachable")
        }
        Err(error) => {
            let message = error.to_string();
            checks.push(Check {
                name: "herdr_server".into(),
                ok: false,
                detail: message.clone(),
            });
            DoctorHerdr::unavailable(message)
        }
    };

    let ready = checks.iter().all(|check| check.ok);
    let output = DoctorOutput {
        ready,
        herdr,
        checks,
    };
    if json {
        print_json(&output)?;
    } else {
        println!("doctor: {}", if output.ready { "ready" } else { "blocked" });
        for check in &output.checks {
            println!(
                "  [{}] {} - {}",
                if check.ok { "ok" } else { "!!" },
                check.name,
                check.detail
            );
        }
    }

    Ok(if ready {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

#[derive(Serialize)]
struct ConfigOutput {
    path: String,
    exists: bool,
    roots: Vec<String>,
    projects: Vec<ProjectConfigRow>,
    ignore: Vec<String>,
    max_depth: usize,
    default_agent: Agent,
}

#[derive(Serialize)]
struct ProjectConfigRow {
    name: String,
    path: String,
}

fn show_config(config: &Config, json: bool) -> Result<ExitCode> {
    let output = ConfigOutput {
        path: config.path.display().to_string(),
        exists: config.path.exists(),
        roots: config
            .roots
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        projects: config
            .projects
            .iter()
            .map(|project| ProjectConfigRow {
                name: project.name.clone(),
                path: project.path.display().to_string(),
            })
            .collect(),
        ignore: config.ignore.clone(),
        max_depth: config.max_depth,
        default_agent: config.default_agent,
    };
    if json {
        print_json(&output)?;
    } else {
        println!("config: {}", output.path);
        println!(
            "status: {}",
            if output.exists { "exists" } else { "defaults" }
        );
        println!("default agent: {}", output.default_agent);
        println!("roots:");
        for root in &output.roots {
            println!("  - {root}");
        }
        if !output.projects.is_empty() {
            println!("projects:");
            for project in &output.projects {
                println!("  - {} -> {}", project.name, project.path);
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn herdr_path_check() -> Check {
    let configured = std::path::PathBuf::from(herdr::herdr_bin());
    let resolved = if configured.components().count() > 1 {
        configured.is_file().then_some(configured)
    } else {
        configured.to_str().and_then(find_on_path)
    };
    match resolved {
        Some(path) => Check {
            name: "path:herdr".into(),
            ok: true,
            detail: path.display().to_string(),
        },
        None => Check {
            name: "path:herdr".into(),
            ok: false,
            detail: "HERDR_BIN_PATH is unusable and herdr was not found on PATH".into(),
        },
    }
}

fn path_check(name: &str) -> Check {
    match find_on_path(name) {
        Some(path) => Check {
            name: format!("path:{name}"),
            ok: true,
            detail: path.display().to_string(),
        },
        None => Check {
            name: format!("path:{name}"),
            ok: false,
            detail: "not found on PATH".into(),
        },
    }
}

fn find_on_path(name: &str) -> Option<std::path::PathBuf> {
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|path| path.join(name))
        .find(|path| path.is_file())
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

#[derive(Serialize)]
struct ErrorOutput {
    ok: bool,
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    kind: &'static str,
    message: String,
    exit_code: u8,
}

fn print_json_error(error: &SheprdError) {
    let output = ErrorOutput {
        ok: false,
        error: ErrorBody {
            kind: error.kind(),
            message: error.to_string(),
            exit_code: error.exit_code(),
        },
    };

    match serde_json::to_string_pretty(&output) {
        Ok(json) => eprintln!("{json}"),
        Err(json_error) => eprintln!("error: failed to serialize error response: {json_error}"),
    }
}
