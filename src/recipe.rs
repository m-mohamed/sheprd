use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Agent {
    Pi,
    Claude,
    Codex,
    Opencode,
}

impl Agent {
    pub fn executable(self) -> &'static str {
        match self {
            Self::Pi => "pi",
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Opencode => "opencode",
        }
    }
}

impl fmt::Display for Agent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.executable())
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum RecipeName {
    AgentDev,
}

impl RecipeName {
    pub fn build(self, agent: Agent) -> Recipe {
        match self {
            Self::AgentDev => Recipe::agent_dev(agent),
        }
    }
}

impl fmt::Display for RecipeName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AgentDev => formatter.write_str("agent-dev"),
        }
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
            description: "Sample editor, selected agent, shell, git, and review shell.".into(),
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
