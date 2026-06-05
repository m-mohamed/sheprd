mod cli;
mod config;
mod error;
mod herdr;
mod project;
mod recipe;

use clap::{CommandFactory, Parser};
use cli::{Cli, Command};
use config::Config;
use error::Result;
use recipe::{Agent, Recipe, RecipeName};
use serde::Serialize;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();
    let Some(command) = cli.command.clone() else {
        let mut command = Cli::command();
        command.print_help()?;
        println!();
        return Ok(ExitCode::SUCCESS);
    };

    let mut config = Config::load()?;
    if let Some(agent) = cli.agent {
        config.default_agent = agent;
    }

    match command {
        Command::List => list(&config, cli.json),
        Command::Connect { project, recipe } => connect(&config, &project, recipe, cli.no_attach),
        Command::Recipes => recipes(&config, cli.json),
        Command::Doctor => doctor(&config, cli.json),
        Command::ShowConfig => show_config(&config, cli.json),
    }
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
) -> Result<ExitCode> {
    let project = project::resolve(config, selector)?;
    let recipe = recipe_name.map(|name| name.build(config.default_agent));
    herdr::connect(&project, config.default_agent, recipe.as_ref(), !no_attach)?;
    Ok(ExitCode::SUCCESS)
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
        println!("recipes:");
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
    checks: Vec<Check>,
}

#[derive(Serialize)]
struct Check {
    name: String,
    ok: bool,
    detail: String,
}

fn doctor(config: &Config, json: bool) -> Result<ExitCode> {
    let mut checks = vec![
        path_check("herdr"),
        path_check("nvim"),
        path_check("lazygit"),
        path_check(config.default_agent.executable()),
    ];

    match herdr::server_status() {
        Ok(Some(status)) => checks.push(Check {
            name: "herdr_server".into(),
            ok: status.running && status.compatible != Some(false),
            detail: format!(
                "status={} version={} compatible={}",
                if status.running { "running" } else { "stopped" },
                status.version.unwrap_or_else(|| "unknown".into()),
                status
                    .compatible
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".into())
            ),
        }),
        Ok(None) => checks.push(Check {
            name: "herdr_server".into(),
            ok: false,
            detail: "not reachable".into(),
        }),
        Err(error) => checks.push(Check {
            name: "herdr_server".into(),
            ok: false,
            detail: error.to_string(),
        }),
    }

    let ready = checks.iter().all(|check| check.ok);
    let output = DoctorOutput { ready, checks };
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
    ignore: Vec<String>,
    max_depth: usize,
    default_agent: Agent,
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
    }
    Ok(ExitCode::SUCCESS)
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
