use crate::error::{Result, SheprdError};
use crate::recipe::Agent;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub path: PathBuf,
    pub roots: Vec<PathBuf>,
    pub projects: Vec<ConfigProject>,
    pub ignore: Vec<String>,
    pub max_depth: usize,
    pub default_agent: Agent,
}

#[derive(Clone, Debug)]
pub struct ConfigProject {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    roots: Option<Vec<String>>,
    projects: Option<Vec<RawProject>>,
    ignore: Option<Vec<String>>,
    max_depth: Option<usize>,
    default_agent: Option<Agent>,
}

#[derive(Debug, Deserialize)]
struct RawProject {
    name: String,
    path: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        let defaults = Self::defaults(path.clone())?;
        if !path.exists() {
            return Ok(defaults);
        }

        let raw: RawConfig = toml::from_str(&std::fs::read_to_string(&path)?)?;
        let projects = raw
            .projects
            .unwrap_or_default()
            .into_iter()
            .map(|project| {
                Ok(ConfigProject {
                    name: project.name,
                    path: expand_home(project.path)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            roots: raw
                .roots
                .unwrap_or_else(|| {
                    defaults
                        .roots
                        .iter()
                        .map(|path| path.display().to_string())
                        .collect()
                })
                .into_iter()
                .map(expand_home)
                .collect::<Result<Vec<_>>>()?,
            projects,
            ignore: raw.ignore.unwrap_or(defaults.ignore),
            max_depth: raw.max_depth.unwrap_or(defaults.max_depth),
            default_agent: raw.default_agent.unwrap_or(defaults.default_agent),
            path,
        })
    }

    fn defaults(path: PathBuf) -> Result<Self> {
        Ok(Self {
            path,
            roots: vec![
                expand_home("~/Workspace")?,
                expand_home("~/code")?,
                expand_home("~/src")?,
            ],
            projects: Vec::new(),
            ignore: vec![
                ".git".into(),
                ".direnv".into(),
                ".tmp".into(),
                "node_modules".into(),
                "target".into(),
                "vendor".into(),
            ],
            max_depth: 6,
            default_agent: Agent::Codex,
        })
    }
}

fn config_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("SHEPRD_CONFIG") {
        return Ok(PathBuf::from(path));
    }
    Ok(home()?.join(".config/sheprd/config.toml"))
}

pub fn expand_home(value: impl AsRef<str>) -> Result<PathBuf> {
    let value = value.as_ref();
    if value == "~" {
        return home();
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return Ok(home()?.join(rest));
    }
    Ok(PathBuf::from(value))
}

fn home() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or(SheprdError::MissingHome)
}
