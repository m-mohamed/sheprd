use crate::recipe::{Agent, RecipeName};
use clap::{Parser, Subcommand};

#[derive(Clone, Debug, Parser)]
#[command(name = "sheprd")]
#[command(version, about = "Keep every coding agent in frame with Herdr")]
#[command(
    long_about = "Sheprd is a Herdr plugin for opening a visible four-agent Flok: Pi conducts while Codex, Claude Code, and OpenCode work in isolated git worktrees."
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
    /// Open or focus a four-agent Flok for a project.
    Flok {
        /// Project name or git checkout path. Defaults to the active Herdr project.
        project: Option<String>,
    },

    /// Run the deterministic, receipt-backed factory workflow in a Flok.
    Factory {
        #[command(subcommand)]
        command: FactoryCommand,
    },

    /// Preview or perform safe cleanup of a Flok workspace and worker checkouts.
    Cleanup {
        /// Project name or git checkout path. Defaults to the active Herdr project.
        project: Option<String>,

        /// Close the Flok and remove only verified-clean worker checkouts.
        #[arg(long)]
        confirm: bool,
    },

    /// Interactively confirm cleanup inside a Herdr overlay.
    #[command(hide = true)]
    CleanupPrompt,

    /// Pick a project interactively and open its Flok.
    #[command(hide = true)]
    Pick,

    /// Print or write a starter config file.
    Init {
        /// Print the starter config instead of writing it.
        #[arg(long)]
        print: bool,

        /// Overwrite an existing config file.
        #[arg(long)]
        force: bool,

        /// Root directory to scan for Git repositories. Can be repeated.
        #[arg(long = "root")]
        roots: Vec<String>,
    },

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

#[derive(Clone, Debug, Subcommand)]
pub enum FactoryCommand {
    /// Plan, implement, check, review, and emit an acceptance receipt.
    Run {
        /// Project name or git checkout path. Defaults to the active Herdr project.
        project: Option<String>,

        /// Bounded task for the four-agent factory.
        #[arg(long)]
        task: String,

        /// Repository-relative file or directory the Codex worker may change.
        #[arg(long = "allow-path", required = true)]
        allow_paths: Vec<String>,

        /// Check command Rust runs in the Codex worker checkout. May be repeated.
        #[arg(long = "check", required = true)]
        checks: Vec<String>,

        /// Per-check timeout in seconds.
        #[arg(long = "check-timeout-seconds", default_value_t = 300)]
        check_timeout_seconds: u64,
    },

    /// Aggregate private factory receipts without changing state.
    Stats {
        /// Project name or git checkout path. Defaults to the active Herdr project.
        project: Option<String>,
    },
}
