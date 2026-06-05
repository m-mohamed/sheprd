use crate::recipe::Agent;
use clap::{Parser, Subcommand};

#[derive(Clone, Debug, Parser)]
#[command(name = "sheprd")]
#[command(version, about = "Smart session manager for Herdr")]
#[command(
    long_about = "sheprd is a smart session manager for Herdr. It finds the project you mean, chooses the agent you want, applies a small recipe, and lets Herdr own the runtime."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Override the configured default agent.
    #[arg(long, value_enum, global = true)]
    pub agent: Option<Agent>,

    /// Emit JSON for read-only commands.
    #[arg(long, global = true)]
    pub json: bool,

    /// Create or focus Herdr state without attaching a Herdr client.
    #[arg(long, global = true)]
    pub no_attach: bool,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    /// List discovered projects and their Herdr workspace state.
    List,

    /// Connect to a project by discovered name or filesystem path.
    #[command(visible_alias = "open", visible_alias = "switch")]
    Connect {
        /// Project name from `sheprd list`, or a path to a Git repository.
        project: String,
    },

    /// Show built-in workspace recipes.
    Recipes,

    /// Check Herdr, config, and required executables.
    Doctor,

    /// Show the active config.
    ShowConfig,
}
