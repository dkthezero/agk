use crate::domain::telemetry::AnalyticsConfig;
use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// Parsed skill invocation from a single log line.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillInvocation {
    pub skill_name: String,
    pub provider_id: String,
    pub timestamp: DateTime<Utc>,
}

/// Trait for provider-specific log parsers.
pub trait LogParser: Send + Sync {
    fn provider_id(&self) -> &str;
    fn log_directories(&self) -> Vec<PathBuf>;
    fn parse_line(&self, line: &str) -> Option<SkillInvocation>;
}

// Re-export the provider-specific implementations so callers and tests can
// continue to use them via `crate::infra::telemetry::parser::*`.
pub use crate::infra::telemetry::parsers::{
    AmpLogParser, ClaudeCodeLogParser, CopilotCliLogParser, CopilotLogParser, GeminiLogParser,
    OpenCodeLogParser,
};

// ---------------------------------------------------------------------------
// Helpers (shared by all parsers in parsers.rs)
// ---------------------------------------------------------------------------

pub(crate) fn extract_quoted_after<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let start = line.find(prefix)? + prefix.len();
    let end = line[start..].find('\'')?;
    Some(&line[start..start + end])
}

pub(crate) fn extract_after_prefix<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let start = line.find(prefix)? + prefix.len();
    let rest = &line[start..];
    let end = rest
        .find(|c: char| c.is_whitespace() || c == '\'' || c == '`')
        .unwrap_or(rest.len());
    let name = rest[..end].trim();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

// ---------------------------------------------------------------------------
// Built-in parser registry
// ---------------------------------------------------------------------------

pub fn default_parsers() -> Vec<Box<dyn LogParser>> {
    vec![
        Box::new(ClaudeCodeLogParser),
        Box::new(CopilotLogParser),
        Box::new(CopilotCliLogParser),
        Box::new(GeminiLogParser),
        Box::new(AmpLogParser),
        Box::new(OpenCodeLogParser),
    ]
}

// ---------------------------------------------------------------------------
// Scan a single directory tree for new invocations
// ---------------------------------------------------------------------------

/// Scan a single directory tree, processing only new bytes in each file
/// using per-file offset tracking to avoid double-counting.
pub fn scan_directory(parser: &dyn LogParser, config: &mut AnalyticsConfig) {
    for dir in parser.log_directories() {
        if !dir.exists() {
            continue;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let path_key = path.to_string_lossy().into_owned();
                let saved_offset = config.file_offsets.get(&path_key).copied().unwrap_or(0);

                let current_len = match std::fs::metadata(&path) {
                    Ok(meta) => meta.len(),
                    Err(_) => continue,
                };

                // File unchanged since last scan
                if current_len == saved_offset {
                    continue;
                }

                // File shrunk (truncated/rotated) — reset and rescan from beginning
                let offset: usize = if current_len < saved_offset {
                    0
                } else {
                    saved_offset as usize
                };

                if let Ok(content) = std::fs::read_to_string(&path) {
                    // Only process lines starting after the saved offset.
                    // For offset==0 this scans the entire file.
                    let tail = if offset >= content.len() {
                        ""
                    } else {
                        &content[offset..]
                    };
                    for line in tail.lines() {
                        if let Some(inv) = parser.parse_line(line) {
                            config.increment_invocation(&inv.skill_name, &inv.provider_id);
                        }
                    }
                }

                config.file_offsets.insert(path_key, current_len);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_parses_executed_tool() {
        let p = ClaudeCodeLogParser;
        let line = "2026-05-01 14:32:00 INFO executed tool `web-browsing-tool' with args ...";
        let inv = p.parse_line(line).unwrap();
        assert_eq!(inv.skill_name, "web-browsing-tool");
        assert_eq!(inv.provider_id, "claude-code");
    }

    #[test]
    fn claude_parses_skill_invoked() {
        let p = ClaudeCodeLogParser;
        let line = "skill `react-parser' invoked by user";
        let inv = p.parse_line(line).unwrap();
        assert_eq!(inv.skill_name, "react-parser");
    }

    #[test]
    fn claude_no_match() {
        let p = ClaudeCodeLogParser;
        assert!(p.parse_line("some random log line").is_none());
    }

    #[test]
    fn copilot_parses_tool_call() {
        let p = CopilotLogParser;
        let line = "tool call: web-browsing-tool";
        let inv = p.parse_line(line).unwrap();
        assert_eq!(inv.skill_name, "web-browsing-tool");
        assert_eq!(inv.provider_id, "github-copilot");
    }

    #[test]
    fn opencode_parses_executing_skill() {
        let p = OpenCodeLogParser;
        let line = "executing skill `git-helper' ...";
        let inv = p.parse_line(line).unwrap();
        assert_eq!(inv.skill_name, "git-helper");
        assert_eq!(inv.provider_id, "opencode");
    }

    #[test]
    fn scan_directory_updates_config() {
        let dir = tempfile::tempdir().unwrap();
        let log_dir = dir.path().join("logs");
        std::fs::create_dir_all(&log_dir).unwrap();
        std::fs::write(log_dir.join("claude.log"), "executed tool `test-skill'\n").unwrap();

        let log_dir_path = log_dir.clone();
        struct TestParser {
            dir: std::path::PathBuf,
        }
        impl LogParser for TestParser {
            fn provider_id(&self) -> &str {
                "test"
            }
            fn log_directories(&self) -> Vec<PathBuf> {
                vec![self.dir.clone()]
            }
            fn parse_line(&self, line: &str) -> Option<SkillInvocation> {
                extract_after_prefix(line, "executed tool `").map(|name| SkillInvocation {
                    skill_name: name.to_string(),
                    provider_id: "test".to_string(),
                    timestamp: Utc::now(),
                })
            }
        }

        let mut config = AnalyticsConfig::default();
        scan_directory(
            &TestParser {
                dir: log_dir_path.clone(),
            },
            &mut config,
        );
        let skill = config.skills.get("test-skill").unwrap();
        assert_eq!(skill.total_invocations, 1);

        // Second scan with no file changes should NOT double-count
        scan_directory(
            &TestParser {
                dir: log_dir_path.clone(),
            },
            &mut config,
        );
        let skill = config.skills.get("test-skill").unwrap();
        assert_eq!(skill.total_invocations, 1);
    }

    #[test]
    fn scan_directory_appends_only_new_lines() {
        let dir = tempfile::tempdir().unwrap();
        let log_dir = dir.path().join("logs");
        std::fs::create_dir_all(&log_dir).unwrap();
        let log_file = log_dir.join("claude.log");
        std::fs::write(&log_file, "executed tool `alpha'\n").unwrap();

        let mut config = AnalyticsConfig::default();
        let p = TestParserWithDir {
            dir: log_dir.clone(),
        };
        scan_directory(&p, &mut config);
        assert_eq!(config.skills.get("alpha").unwrap().total_invocations, 1);

        // Append new line
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&log_file)
            .unwrap();
        std::io::Write::write_all(&mut file, b"executed tool `beta'\n").unwrap();
        drop(file);

        scan_directory(&p, &mut config);
        assert_eq!(config.skills.get("alpha").unwrap().total_invocations, 1);
        assert_eq!(config.skills.get("beta").unwrap().total_invocations, 1);

        // Truncate/rotate file (size goes down) — should reset offset
        std::fs::write(&log_file, "executed tool `gamma'\n").unwrap();
        scan_directory(&p, &mut config);
        assert_eq!(config.skills.get("alpha").unwrap().total_invocations, 1);
        assert_eq!(config.skills.get("beta").unwrap().total_invocations, 1);
        assert_eq!(config.skills.get("gamma").unwrap().total_invocations, 1);
    }

    struct TestParserWithDir {
        dir: std::path::PathBuf,
    }
    impl LogParser for TestParserWithDir {
        fn provider_id(&self) -> &str {
            "test"
        }
        fn log_directories(&self) -> Vec<PathBuf> {
            vec![self.dir.clone()]
        }
        fn parse_line(&self, line: &str) -> Option<SkillInvocation> {
            extract_after_prefix(line, "executed tool `").map(|name| SkillInvocation {
                skill_name: name.to_string(),
                provider_id: "test".to_string(),
                timestamp: Utc::now(),
            })
        }
    }
}
