//! Provider-specific `LogParser` implementations.
//!
//! Each parser is a small zero-sized struct that knows where its provider
//! writes logs and how to extract a `SkillInvocation` from a line. They were
//! extracted from `parser.rs` to keep that file under the 300-LOC limit
//! (ADR-001 §6.4 follow-up).

use crate::infra::telemetry::parser::{
    extract_after_prefix, extract_quoted_after, LogParser, SkillInvocation,
};
use chrono::Utc;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Claude Code
// ---------------------------------------------------------------------------

pub struct ClaudeCodeLogParser;

impl LogParser for ClaudeCodeLogParser {
    fn provider_id(&self) -> &str {
        "claude-code"
    }

    fn log_directories(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Some(home) = dirs_next::home_dir() {
            dirs.push(home.join("Library/Logs/Claude")); // macOS
            dirs.push(home.join(".local/share/Claude/logs")); // Linux
        }
        dirs
    }

    fn parse_line(&self, line: &str) -> Option<SkillInvocation> {
        let name = extract_quoted_after(line, "executed tool `")
            .or_else(|| extract_quoted_after(line, "skill `"))
            .or_else(|| extract_after_prefix(line, "running skill: "))?;
        Some(SkillInvocation {
            skill_name: name.to_string(),
            provider_id: self.provider_id().to_string(),
            timestamp: Utc::now(),
        })
    }
}

// ---------------------------------------------------------------------------
// GitHub Copilot
// ---------------------------------------------------------------------------

pub struct CopilotLogParser;

impl LogParser for CopilotLogParser {
    fn provider_id(&self) -> &str {
        "github-copilot"
    }

    fn log_directories(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Some(home) = dirs_next::home_dir() {
            dirs.push(home.join("Library/Logs/GitHub Copilot")); // macOS
            dirs.push(home.join(".local/share/GitHub Copilot/logs")); // Linux
        }
        dirs
    }

    fn parse_line(&self, line: &str) -> Option<SkillInvocation> {
        let name = extract_quoted_after(line, "invoked tool `")
            .or_else(|| extract_after_prefix(line, "tool call: "))?;
        Some(SkillInvocation {
            skill_name: name.to_string(),
            provider_id: self.provider_id().to_string(),
            timestamp: Utc::now(),
        })
    }
}

// ---------------------------------------------------------------------------
// GitHub Copilot CLI (separate parser from VS Code extension Copilot)
// ---------------------------------------------------------------------------

pub struct CopilotCliLogParser;

impl LogParser for CopilotCliLogParser {
    fn provider_id(&self) -> &str {
        "github-copilot-cli"
    }

    fn log_directories(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Some(home) = dirs_next::home_dir() {
            dirs.push(home.join("Library/Logs/Copilot CLI")); // macOS
            dirs.push(home.join(".local/share/Copilot CLI/logs")); // Linux
            dirs.push(home.join(".copilot/logs")); // fallback
        }
        dirs
    }

    fn parse_line(&self, line: &str) -> Option<SkillInvocation> {
        let name = extract_quoted_after(line, "executing tool `")
            .or_else(|| extract_quoted_after(line, "invoked tool `"))
            .or_else(|| extract_after_prefix(line, "tool: "))?;
        Some(SkillInvocation {
            skill_name: name.to_string(),
            provider_id: self.provider_id().to_string(),
            timestamp: Utc::now(),
        })
    }
}

// ---------------------------------------------------------------------------
// Gemini CLI
// ---------------------------------------------------------------------------

pub struct GeminiLogParser;

impl LogParser for GeminiLogParser {
    fn provider_id(&self) -> &str {
        "gemini-cli"
    }

    fn log_directories(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Some(home) = dirs_next::home_dir() {
            dirs.push(home.join("Library/Logs/Gemini CLI")); // macOS
            dirs.push(home.join(".local/share/Gemini CLI/logs")); // Linux
            dirs.push(home.join(".gemini/logs")); // fallback
        }
        dirs
    }

    fn parse_line(&self, line: &str) -> Option<SkillInvocation> {
        let name = extract_quoted_after(line, "executing skill `")
            .or_else(|| extract_quoted_after(line, "skill `"))
            .or_else(|| extract_after_prefix(line, "skill execution: "))?;
        Some(SkillInvocation {
            skill_name: name.to_string(),
            provider_id: self.provider_id().to_string(),
            timestamp: Utc::now(),
        })
    }
}

// ---------------------------------------------------------------------------
// AMP
// ---------------------------------------------------------------------------

pub struct AmpLogParser;

impl LogParser for AmpLogParser {
    fn provider_id(&self) -> &str {
        "amp"
    }

    fn log_directories(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Some(home) = dirs_next::home_dir() {
            dirs.push(home.join("Library/Logs/AMP")); // macOS
            dirs.push(home.join(".local/share/AMP/logs")); // Linux
            dirs.push(home.join(".amp/logs")); // fallback
        }
        dirs
    }

    fn parse_line(&self, line: &str) -> Option<SkillInvocation> {
        let name = extract_quoted_after(line, "executing skill `")
            .or_else(|| extract_quoted_after(line, "skill `"))
            .or_else(|| extract_after_prefix(line, "skill: "))?;
        Some(SkillInvocation {
            skill_name: name.to_string(),
            provider_id: self.provider_id().to_string(),
            timestamp: Utc::now(),
        })
    }
}

// ---------------------------------------------------------------------------
// OpenCode
// ---------------------------------------------------------------------------

pub struct OpenCodeLogParser;

impl LogParser for OpenCodeLogParser {
    fn provider_id(&self) -> &str {
        "opencode"
    }

    fn log_directories(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Some(home) = dirs_next::home_dir() {
            dirs.push(home.join(".config/opencode/logs"));
        }
        dirs
    }

    fn parse_line(&self, line: &str) -> Option<SkillInvocation> {
        let name = extract_quoted_after(line, "executing skill `")
            .or_else(|| extract_after_prefix(line, "skill execution: "))?;
        Some(SkillInvocation {
            skill_name: name.to_string(),
            provider_id: self.provider_id().to_string(),
            timestamp: Utc::now(),
        })
    }
}
