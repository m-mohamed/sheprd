use crate::recipe::{Agent, RecipeName};
use clap::{Parser, Subcommand};

#[derive(Clone, Debug, Parser)]
#[command(name = "sheprd")]
#[command(version, about = "Smart session manager for Herdr")]
#[command(
    long_about = "sheprd is a smart session manager for Herdr. It finds the project you mean, chooses the agent lane you want, connects to the matching Herdr workspace, and can apply explicit sample recipes when you ask for them."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Override the configured default agent.
    #[arg(long, value_enum, global = true)]
    pub agent: Option<Agent>,

    /// Emit JSON for automation-friendly commands.
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

        /// Optional sample recipe to apply when creating a new Herdr workspace.
        #[arg(long, value_enum)]
        recipe: Option<RecipeName>,
    },

    /// Show sample workspace recipes.
    Recipes,

    /// Check Herdr, config, and required executables.
    Doctor,

    /// Show the active config.
    ShowConfig,
}
