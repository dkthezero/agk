//! OS file-manager and terminal-emulator launcher.
//!
//! Concrete `FileOpenerPort` implementation. Logic was previously inlined in
//! `domain/paths.rs` (ADR-001 Commit 1).

use crate::app::ports::FileOpenerPort;
use anyhow::Result;
use std::path::Path;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Default, Clone, Copy)]
pub struct OsFileOpener;

impl FileOpenerPort for OsFileOpener {
    fn open_file_manager(&self, path: &Path) -> Result<()> {
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
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            let _ = path;
            return Err(anyhow::anyhow!(
                "Opening file manager is not supported on this platform"
            ));
        }
        Ok(())
    }

    fn open_terminal(&self, path: &Path) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .args(["-a", "Terminal", &path.to_string_lossy()])
                .spawn()?;
            Ok(())
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
            Err(anyhow::anyhow!("No suitable terminal emulator found"))
        }
        #[cfg(target_os = "windows")]
        {
            let path_str = path.to_string_lossy().into_owned();
            let args = format!(
                "/c start cmd /k \"cd /d \"{}\"\"",
                escape_cmd_arg(&path_str)
            );
            std::process::Command::new("cmd").raw_arg(args).spawn()?;
            return Ok(());
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            let _ = path;
            return Err(anyhow::anyhow!(
                "Opening terminal is not supported on this platform"
            ));
        }
    }
}

#[cfg(target_os = "windows")]
fn escape_cmd_arg(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(target_os = "linux")]
fn shell_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
