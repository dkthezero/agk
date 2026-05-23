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
    },

    /// Validate installed assets against source vaults
    Validate {
        /// Target scope
        #[arg(short, long, value_enum)]
        scope: Option<ScopeArg>,
    },

    /// Pack a skill into a provider-specific distributable
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
}

#[derive(Subcommand, Debug)]
pub enum McpCommands {
    /// Add/register a new MCP server
    Add {
        /// Server name (unique identifier)
        #[arg(short, long)]
        name: String,

        /// Command to run the MCP server
        #[arg(short, long)]
        command: String,

        /// Arguments for the command
        #[arg(short, long)]
        args: Option<String>,

        /// Environment variables (KEY=VALUE, comma-separated)
        #[arg(short, long)]
        env: Option<String>,

        /// Transport type (stdio or sse)
        #[arg(short, long, default_value = "stdio")]
        transport: String,

        /// Description of the server
        #[arg(short, long)]
        description: Option<String>,

        /// Skip the connection test after registering
        #[arg(long)]
        no_test: bool,
    },

    /// Enable an MCP server for a provider
    Enable {
        /// Server name
        name: String,

        /// Target provider
        #[arg(short, long)]
        provider: String,

        /// Target scope
        #[arg(short, long, value_enum)]
        scope: Option<ScopeArg>,
    },

    /// Disable an MCP server for a provider
    Disable {
        /// Server name
        name: String,

        /// Target provider
        #[arg(short, long)]
        provider: String,

        /// Target scope
        #[arg(short, long, value_enum)]
        scope: Option<ScopeArg>,
    },

    /// List all registered MCP servers
    List {
        /// Filter by enabled provider
        #[arg(short, long)]
        provider: Option<String>,
    },

    /// Test an MCP server connection
    Test {
        /// Server name
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum TelemetryCommands {
    /// Enable telemetry collection
    Enable,

    /// Disable telemetry collection
    Disable,

    /// Show telemetry status
    Status,

    /// Export telemetry data
    Export {
        /// Output format
        #[arg(long, value_enum, default_value = "json")]
        format: ExportFormat,

        /// Output file (default: stdout)
        #[arg(long)]
        output: Option<String>,
    },
}

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
    pub fn to_domain_scope(&self) -> crate::domain::scope::Scope {
        match self {
            ScopeArg::Global => crate::domain::scope::Scope::Global,
            ScopeArg::Workspace => crate::domain::scope::Scope::Workspace,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PackTarget {
    ClaudeDesktop,
    Firebender,
    Tarball,
}

pub fn parse() -> Cli {
    Cli::parse()
}
