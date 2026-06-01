use serde::{Deserialize, Serialize};

/// Security flags for MCP servers based on heuristic analysis of command + args.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityFlag {
    /// Args contain "/", "~", or "." — broad filesystem access
    BroadFilesystem,
    /// Command/args contain "http", "curl", "wget", "fetch" — network egress
    NetworkEgress,
    /// Command is bash/sh/python with unverified script — arbitrary execution
    ArbitraryExecution,
    /// Args reference env vars ($HOME, $SSH_KEY, etc.) — env exfiltration
    EnvExfiltration,
    /// No args provided — command may default to dangerous behavior
    UnspecifiedArgs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl SecurityFlag {
    pub fn severity(&self) -> SecuritySeverity {
        match self {
            Self::UnspecifiedArgs => SecuritySeverity::Low,
            Self::EnvExfiltration => SecuritySeverity::Medium,
            Self::BroadFilesystem => SecuritySeverity::High,
            Self::NetworkEgress => SecuritySeverity::High,
            Self::ArbitraryExecution => SecuritySeverity::Critical,
        }
    }

    pub fn badge(&self) -> &'static str {
        match self.severity() {
            SecuritySeverity::Low => "[i]",
            SecuritySeverity::Medium => "[!]",
            SecuritySeverity::High => "[!]",
            SecuritySeverity::Critical => "[!!]",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::BroadFilesystem => "Requests broad filesystem access",
            Self::NetworkEgress => "May make network requests",
            Self::ArbitraryExecution => "May execute arbitrary scripts",
            Self::EnvExfiltration => "References environment variables",
            Self::UnspecifiedArgs => "No arguments specified — may default to dangerous behavior",
        }
    }
}

/// Pure heuristic function that assesses MCP security based on command and args.
/// Advisory only — never blocks installation.
pub fn assess_mcp_security(command: &str, args: &[String]) -> Vec<SecurityFlag> {
    let mut flags = Vec::new();

    // broad-filesystem: args contain "/", "~", or "."
    if args.iter().any(|a| a == "/" || a == "~" || a == ".") {
        flags.push(SecurityFlag::BroadFilesystem);
    }

    // network-egress: command or args contain http, curl, wget, fetch
    let full = format!("{} {}", command, args.join(" "));
    if full.contains("http")
        || full.contains("curl")
        || full.contains("wget")
        || full.contains("fetch")
    {
        flags.push(SecurityFlag::NetworkEgress);
    }

    // arbitrary-execution: command is bash/sh/python with script args
    if (command == "bash" || command == "sh" || command == "python" || command == "python3")
        && args
            .iter()
            .any(|a| a.ends_with(".sh") || a.ends_with(".py"))
    {
        flags.push(SecurityFlag::ArbitraryExecution);
    }

    // env-exfiltration: args contain "$" (env var references)
    if args.iter().any(|a| a.contains('$')) {
        flags.push(SecurityFlag::EnvExfiltration);
    }

    // unspecified-args: no args provided
    if args.is_empty() {
        flags.push(SecurityFlag::UnspecifiedArgs);
    }

    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broad_filesystem_slash() {
        let flags = assess_mcp_security(
            "npx",
            &[
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
                "/".to_string(),
            ],
        );
        assert!(flags.contains(&SecurityFlag::BroadFilesystem));
    }

    #[test]
    fn broad_filesystem_dot() {
        let flags = assess_mcp_security(
            "npx",
            &[
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
                ".".to_string(),
            ],
        );
        assert!(flags.contains(&SecurityFlag::BroadFilesystem));
    }

    #[test]
    fn network_egress_curl() {
        let flags = assess_mcp_security("curl", &["https://example.com".to_string()]);
        assert!(flags.contains(&SecurityFlag::NetworkEgress));
    }

    #[test]
    fn arbitrary_execution_bash_script() {
        let flags = assess_mcp_security("bash", &["script.sh".to_string()]);
        assert!(flags.contains(&SecurityFlag::ArbitraryExecution));
    }

    #[test]
    fn env_exfiltration_home() {
        let flags = assess_mcp_security("npx", &["$HOME/path".to_string()]);
        assert!(flags.contains(&SecurityFlag::EnvExfiltration));
    }

    #[test]
    fn unspecified_args_empty() {
        let flags = assess_mcp_security("npx", &[]);
        assert!(flags.contains(&SecurityFlag::UnspecifiedArgs));
    }

    #[test]
    fn multiple_flags_returned() {
        let flags = assess_mcp_security("bash", &["$HOME/script.sh".to_string()]);
        // Should flag env exfiltration ($HOME) and arbitrary execution (bash + .sh)
        assert!(flags.contains(&SecurityFlag::ArbitraryExecution));
        assert!(flags.contains(&SecurityFlag::EnvExfiltration));
    }

    #[test]
    fn severity_levels() {
        assert_eq!(
            SecurityFlag::UnspecifiedArgs.severity(),
            SecuritySeverity::Low
        );
        assert_eq!(
            SecurityFlag::EnvExfiltration.severity(),
            SecuritySeverity::Medium
        );
        assert_eq!(
            SecurityFlag::BroadFilesystem.severity(),
            SecuritySeverity::High
        );
        assert_eq!(
            SecurityFlag::NetworkEgress.severity(),
            SecuritySeverity::High
        );
        assert_eq!(
            SecurityFlag::ArbitraryExecution.severity(),
            SecuritySeverity::Critical
        );
    }

    #[test]
    fn badge_display_strings() {
        assert_eq!(SecurityFlag::UnspecifiedArgs.badge(), "[i]");
        assert_eq!(SecurityFlag::EnvExfiltration.badge(), "[!]");
        assert_eq!(SecurityFlag::BroadFilesystem.badge(), "[!]");
        assert_eq!(SecurityFlag::NetworkEgress.badge(), "[!]");
        assert_eq!(SecurityFlag::ArbitraryExecution.badge(), "[!!]");
    }

    #[test]
    fn no_flags_for_safe_config() {
        let flags =
            assess_mcp_security("npx", &["-y".to_string(), "some-safe-package".to_string()]);
        assert!(flags.is_empty());
    }
}
