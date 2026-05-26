use std::path::PathBuf;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

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

/// Resolve the contexts file path: `<config_root>/contexts.toml`.
pub fn contexts_file_path() -> PathBuf {
    global_config_root().join("contexts.toml")
}

/// Resolve the current-context file path: `<config_root>/current-context`.
pub fn current_context_path() -> PathBuf {
    global_config_root().join("current-context")
}

/// Resolve the analytics file path: `<config_root>/analytics.toml`.
pub fn analytics_path() -> PathBuf {
    global_config_root().join("analytics.toml")
}

/// Resolve the MCP registry file path: `<config_root>/mcp.toml`.
pub fn mcp_path() -> PathBuf {
    global_config_root().join("mcp.toml")
}

/// Open the given path in the system file manager (Finder on macOS,
/// explorer on Windows, xdg-open on Linux).
pub fn open_file_manager(path: &std::path::Path) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args([path.as_os_str()])
            .spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .args([path.as_os_str()])
            .spawn()?;
    }
    #[cfg(target_os = "windows")]
    {
        let path_str = path.to_string_lossy().into_owned();
        let args = format!(
            "/c start cmd /k \"cd /d \"{}\"\"",
            escape_cmd_arg(&path_str)
        );
        std::process::Command::new("cmd").raw_arg(args).spawn()?;
    }
    Ok(())
}

/// Open a terminal at the given path.
pub fn open_terminal(path: &std::path::Path) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-a", "Terminal", &path.to_string_lossy()])
            .spawn()?;
    }
    #[cfg(target_os = "linux")]
    {
        let path_str = path.to_string_lossy().into_owned();
        let emulators = [
            (
                "gnome-terminal",
                vec!["--working-directory".into(), path_str.clone()],
            ),
            ("konsole", vec!["--workdir".into(), path_str.clone()]),
            (
                "xfce4-terminal",
                vec!["--working-directory".into(), path_str.clone()],
            ),
            (
                "alacritty",
                vec!["--working-directory".into(), path_str.clone()],
            ),
        ];
        for (term, args) in emulators {
            if std::process::Command::new("which")
                .arg(term)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
            {
                std::process::Command::new(term).args(&args).spawn()?;
                return Ok(());
            }
        }
        // xterm fallback via -e
        if std::process::Command::new("which")
            .arg("xterm")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            std::process::Command::new("xterm")
                .args([
                    "-e",
                    "sh",
                    "-lc",
                    &format!("cd \"{}\" && exec $SHELL", shell_escape(&path_str)),
                ])
                .spawn()?;
            return Ok(());
        }
        anyhow::bail!("No suitable terminal emulator found");
    }
    #[cfg(target_os = "windows")]
    {
        let path_str = path.to_string_lossy().into_owned();
        let args = format!(
            "/c start cmd /k \"cd /d \"{}\"\"",
            escape_cmd_arg(&path_str)
        );
        std::process::Command::new("cmd").raw_arg(args).spawn()?;
    }
    Ok(())
}

/// Escape a string for embedding inside a cmd.exe double-quoted argument.
/// Replaces each backslash with `\\`, then replaces each `"` with `\"`.
#[allow(dead_code)]
fn escape_cmd_arg(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Minimal shell-escape for paths inside `sh -lc 'cd "..." && exec $SHELL'`.
#[allow(dead_code)]
fn shell_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

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
