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

#[derive(Clone, Debug)]
pub struct InitConfig {
    pub path: PathBuf,
    pub roots: Vec<String>,
    pub default_agent: Agent,
    pub force: bool,
    pub print_only: bool,
}

#[derive(Clone, Debug)]
pub struct InitOutcome {
    pub path: PathBuf,
    pub roots: Vec<String>,
    pub default_agent: Agent,
    pub existed: bool,
    pub written: bool,
    pub contents: String,
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
            roots: default_root_strings()
                .into_iter()
                .map(expand_home)
                .collect::<Result<Vec<_>>>()?,
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

pub fn init(config: InitConfig) -> Result<InitOutcome> {
    let existed = config.path.exists();
    let contents = sample_config(config.default_agent, &config.roots);

    if config.print_only {
        return Ok(InitOutcome {
            path: config.path,
            roots: config.roots,
            default_agent: config.default_agent,
            existed,
            written: false,
            contents,
        });
    }

    if existed && !config.force {
        return Err(SheprdError::Message(format!(
            "config already exists at {}; use --print to inspect or --force to overwrite",
            config.path.display()
        )));
    }

    if let Some(parent) = config.path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&config.path, &contents)?;

    Ok(InitOutcome {
        path: config.path,
        roots: config.roots,
        default_agent: config.default_agent,
        existed,
        written: true,
        contents,
    })
}

pub fn config_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("SHEPRD_CONFIG") {
        return Ok(PathBuf::from(path));
    }
    let legacy = home()?.join(".config/sheprd/config.toml");
    if let Some(dir) = std::env::var_os("HERDR_PLUGIN_CONFIG_DIR") {
        let plugin = PathBuf::from(dir).join("config.toml");
        if plugin.exists() || !legacy.exists() {
            return Ok(plugin);
        }
    }
    Ok(legacy)
}

pub fn default_root_strings() -> Vec<String> {
    vec!["~/Workspace".into(), "~/code".into(), "~/src".into()]
}

fn sample_config(default_agent: Agent, roots: &[String]) -> String {
    let roots = if roots.is_empty() {
        default_root_strings()
    } else {
        roots.to_vec()
    };
    let mut lines = vec![
        "# sheprd config".into(),
        "# Herdr owns runtime state. sheprd owns project discovery and entry.".into(),
        "roots = [".into(),
    ];
    for root in roots {
        lines.push(format!("  \"{}\",", escape_toml_string(&root)));
    }
    lines.extend([
        "]".into(),
        format!("default_agent = \"{default_agent}\""),
        "max_depth = 6".into(),
        "".into(),
        "# Use explicit projects when the public name should differ from the directory name."
            .into(),
        "# [[projects]]".into(),
        "# name = \"my-project\"".into(),
        "# path = \"~/workspace/my-project-main-worktree\"".into(),
        "".into(),
        "ignore = [".into(),
        "  \".git\",".into(),
        "  \".direnv\",".into(),
        "  \".tmp\",".into(),
        "  \"node_modules\",".into(),
        "  \"target\",".into(),
        "  \"vendor\",".into(),
        "]".into(),
    ]);
    format!("{}\n", lines.join("\n"))
}

fn escape_toml_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
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
