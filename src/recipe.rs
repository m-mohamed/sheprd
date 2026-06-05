use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Agent {
    Pi,
    Droid,
    Claude,
    Codex,
    Hermes,
    Opencode,
}

impl Agent {
    pub fn executable(self) -> &'static str {
        match self {
            Self::Pi => "pi",
            Self::Droid => "droid",
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Hermes => "hermes",
            Self::Opencode => "opencode",
        }
    }
}

impl fmt::Display for Agent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.executable())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Recipe {
    pub name: String,
    pub description: String,
    pub agent: Agent,
    pub tabs: Vec<RecipeTab>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RecipeTab {
    pub name: String,
    pub panes: Vec<String>,
}

impl Recipe {
    pub fn agent_dev(agent: Agent) -> Self {
        Self {
            name: "agent-dev".into(),
            description: "Editor, selected agent, shell, git, and review shell.".into(),
            agent,
            tabs: vec![
                RecipeTab {
                    name: "code".into(),
                    panes: vec!["nvim".into(), agent.executable().into(), "shell".into()],
                },
                RecipeTab {
                    name: "git".into(),
                    panes: vec!["lazygit".into(), "shell".into()],
                },
            ],
        }
    }
}
