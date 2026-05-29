use std::path::PathBuf;

/// Resolve the global configuration root according to OS standards and user preference.
/// - **macOS**: `~/.config/agk` (overriding default Library/Application Support)
/// - **Linux**: `~/.config/agk` (standard XDG path via dirs_next)
/// - **Windows**: `AppData/Roaming/agk` (standard via dirs_next)
pub fn global_config_root() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        // Force ~/.config/agk on macOS instead of ~/Library/Application Support/agk
        dirs_next::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".config")
            .join("agk")
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Use default config_dir (Linux: ~/.config, Windows: AppData/Roaming)
        dirs_next::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("agk")
    }
}

/// Resolve the global vaults directory: `<config_root>/vaults`.
pub fn global_vaults_dir() -> PathBuf {
    global_config_root().join("vaults")
}

/// Resolve the ClawHub cache directory: `<config_root>/clawhub`.
pub fn clawhub_cache_dir() -> PathBuf {
    global_config_root().join("clawhub")
}

/// Resolve the contexts directory: `<config_root>/contexts`.
pub fn contexts_dir() -> PathBuf {
    global_config_root().join("contexts")
}

/// Resolve the analytics file path: `<config_root>/analytics.toml`.
pub fn analytics_path() -> PathBuf {
    global_config_root().join("analytics.toml")
}

/// Resolve the MCP registry file path: `<config_root>/mcp.toml`.
pub fn mcp_path() -> PathBuf {
    global_config_root().join("mcp.toml")
}

// `open_file_manager` and `open_terminal` were moved to `infra/process/opener.rs`
// (concrete `OsFileOpener` implementing `FileOpenerPort`) by ADR-001 Commit 1.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clawhub_cache_dir() {
        let dir = clawhub_cache_dir();
        assert!(dir.to_string_lossy().contains("agk"));
        assert!(dir.to_string_lossy().ends_with("clawhub"));
    }

    #[test]
    fn test_global_config_root() {
        let root = global_config_root();
        #[cfg(target_os = "macos")]
        assert!(root.to_string_lossy().contains(".config/agk"));
        #[cfg(all(unix, not(target_os = "macos")))]
        assert!(root.to_string_lossy().contains(".config/agk"));
        #[cfg(target_os = "windows")]
        assert!(root.to_string_lossy().contains("AppData"));
    }
}
