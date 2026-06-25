use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "agk",
    about = "Agent skill and instruction manager CLI & TUI",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Suppress all non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Verbose debug output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Output structured JSON
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Remove agk configuration files from the active scope
    Clean {
        /// Recursively clean from global folder instead of workspace folder
        #[arg(short, long)]
        global: bool,
    },

    /// Apply a declarative team configuration from a URL or local path
    Apply {
        /// Source URL or file path to the configuration
        source: String,
        /// Target scope
        #[arg(short, long, value_enum, default_value = "workspace")]
        scope: ScopeArg,
        /// Target context (if omitted, applies to current context)
        #[arg(short, long)]
        context: Option<String>,
        /// Target environment (local, dev, staging, prod)
        #[arg(short, long, value_enum)]
        environment: Option<EnvironmentArg>,
        /// Only show what would change
        #[arg(long)]
        dry_run: bool,
    },

    /// Manage contexts (company / team separation)
    Context {
        #[command(subcommand)]
        command: ContextCommands,
    },

    /// Synchronize installed assets with config (install missing, update outdated)
    Sync {
        /// Force global scope
        #[arg(short, long)]
        global: bool,

        /// Only show what would change, without modifying anything
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Install a specific asset by identity
    Install {
        /// Asset identity: [vault/]name[:version]
        identity: String,

        /// Target scope
        #[arg(short, long, value_enum)]
        scope: Option<ScopeArg>,

        /// Only show what would change
        #[arg(short, long)]
        dry_run: bool,

        /// Limit to a specific provider
        #[arg(short, long)]
        provider: Option<String>,

        /// Include the `evals/` subfolder in the installation
        #[arg(long)]
        evals: bool,
    },

    /// Validate installed assets against source vaults
    Validate {
        /// Target scope
        #[arg(short, long, value_enum)]
        scope: Option<ScopeArg>,
    },

    /// Pack a skill into a provider-specific distributable
    #[cfg(feature = "pack")]
    Pack {
        /// Asset identity
        identity: String,

        /// Target provider format
        #[arg(short, long, value_enum, default_value = "claude-desktop")]
        target: PackTarget,

        /// Write to stdout instead of file
        #[arg(long)]
        stdout: bool,
    },

    /// Manage MCP servers
    Mcp {
        #[command(subcommand)]
        command: McpCommands,
    },

    /// Manage telemetry and usage analytics
    Telemetry {
        #[command(subcommand)]
        command: TelemetryCommands,
    },

    /// Manage profiles
    #[command(visible_alias = "p")]
    Profile {
        #[command(subcommand)]
        command: ProfileCommands,
    },

    /// Manage vaults
    Vault {
        #[command(subcommand)]
        command: VaultCommands,
    },

    /// Manage agent providers (activate / list capabilities)
    Provider {
        #[command(subcommand)]
        command: ProviderCommands,
    },

    /// Manage team configuration
    Team {
        #[command(subcommand)]
        command: TeamCommands,
    },

    /// Manage LLM providers
    Llm {
        #[command(subcommand)]
        command: crate::cli::llm::LlmCommand,
    },

    /// Debug / observability commands (hidden in help)
    #[command(hide = true)]
    Debug {
        #[command(subcommand)]
        command: DebugCommands,
    },
}

// Subcommand enums (ProfileCommands, McpCommands, TelemetryCommands,
// ContextCommands) were moved to `entry_subcommands.rs` for ADR-001 §6.4
// file-size compliance and are re-exported here so caller paths
// (`crate::cli::entry::ProfileCommands`, …) continue to resolve.
pub use crate::cli::entry_subcommands::ToggleState;
pub use crate::cli::entry_subcommands::{
    ContextCommands, DebugCommands, McpCommands, ProfileCommands, ProviderCommands,
    TelemetryCommands, VaultCommands,
};
pub use crate::cli::entry_team_subcommands::TeamCommands;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ExportFormat {
    Json,
    Csv,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ScopeArg {
    Global,
    Workspace,
}

impl ScopeArg {
    pub fn into_domain_scope(self) -> crate::domain::scope::Scope {
        match self {
            ScopeArg::Global => crate::domain::scope::Scope::Global,
            ScopeArg::Workspace => crate::domain::scope::Scope::Workspace,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum EnvironmentArg {
    Local,
    Dev,
    Staging,
    Prod,
}

impl From<EnvironmentArg> for crate::domain::context::Environment {
    fn from(arg: EnvironmentArg) -> Self {
        match arg {
            EnvironmentArg::Local => crate::domain::context::Environment::Local,
            EnvironmentArg::Dev => crate::domain::context::Environment::Dev,
            EnvironmentArg::Staging => crate::domain::context::Environment::Staging,
            EnvironmentArg::Prod => crate::domain::context::Environment::Prod,
        }
    }
}

#[cfg(feature = "pack")]
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PackTarget {
    ClaudeDesktop,
    Firebender,
    Tarball,
}

pub fn parse() -> Cli {
    Cli::parse()
}
