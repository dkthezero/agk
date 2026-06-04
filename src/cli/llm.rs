//! Top-level `agk llm` subcommand: list/add/remove/health for LLM providers.

use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Args, Clone)]
pub struct LlmArgs {
    /// Path to the AGK config directory
    #[arg(long, global = true, default_value = ".")]
    pub config_dir: PathBuf,

    #[command(subcommand)]
    pub command: LlmCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum LlmCommand {
    /// List configured LLM providers
    List,
    /// Add a new LLM provider
    Add {
        /// Provider id (e.g., "local-ollama")
        id: String,
        /// Provider kind: ollama, lm-studio, anthropic, openai
        #[arg(long)]
        kind: String,
        /// Endpoint URL (e.g., "http://127.0.0.1:11434")
        #[arg(long)]
        endpoint: String,
        /// Optional API key (for anthropic, openai)
        #[arg(long)]
        api_key: Option<String>,
        /// Optional default model
        #[arg(long)]
        model: Option<String>,
    },
    /// Remove an LLM provider
    Remove {
        /// Provider id
        id: String,
    },
    /// Health check all (or one) configured LLM providers
    Health {
        /// Optional provider id (defaults to all)
        id: Option<String>,
    },
}
