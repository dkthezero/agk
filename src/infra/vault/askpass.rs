use anyhow::Result;
use std::path::{Path, PathBuf};

/// Create a temporary askpass script that provides the auth token.
///
/// The script echoes the token when git requests a password,
/// and returns "x-access-token" for username requests.
/// The script is written to a temp file with restrictive permissions
/// and must be cleaned up by the caller via `cleanup_askpass`.
pub fn create_askpass_script(id: &str, auth_token: &Option<String>) -> Result<Option<PathBuf>> {
    let token = match auth_token {
        Some(t) => t.clone(),
        None => return Ok(None),
    };

    let temp_dir = std::env::temp_dir();
    let unique_id = format!("{}-{}", id, std::process::id());
    let script_path = temp_dir.join(format!("agk-askpass-{}", unique_id));

    // GIT_ASKPASS scripts receive the prompt as $1 (e.g. "Password for ...").
    // We echo the token for password prompts and "x-access-token" for username prompts.
    // Escape single quotes in the token to prevent shell injection.
    let safe_token = token.replace('\'', "'\\''");
    #[cfg(unix)]
    {
        let script_content = format!(
            "#!/bin/sh\nif echo \"$1\" | grep -qi 'password'; then\n  echo '{}'\nelse\n  echo 'x-access-token'\nfi\n",
            safe_token
        );
        std::fs::write(&script_path, script_content)?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o500))?;
    }

    #[cfg(not(unix))]
    {
        let bat_path = script_path.with_extension("bat");
        let script_content = format!(
            "@echo off\nif echo %1 | findstr /i \"password\" >nul (\n  echo {}\n) else (\n  echo x-access-token\n)\n",
            token
        );
        std::fs::write(&bat_path, script_content)?;
    }

    Ok(Some(script_path))
}

/// Remove the temporary askpass script to prevent credential leakage.
pub fn cleanup_askpass(script_path: &PathBuf) {
    // On Unix, remove the shell script. On Windows, remove the .bat file.
    let _ = std::fs::remove_file(script_path);
    #[cfg(not(unix))]
    let _ = std::fs::remove_file(&script_path.with_extension("bat"));
}

/// Get the executable path for the askpass script.
///
/// On Unix, this is the script path itself. On Windows, it's the .bat variant.
pub fn askpass_executable_path(script_path: &Path) -> PathBuf {
    #[cfg(not(unix))]
    {
        script_path.with_extension("bat")
    }
    #[cfg(unix)]
    {
        script_path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_askpass_script_creates_file_with_token() {
        let token = Some("ghp_test_token_123".to_string());

        let script_path =
            create_askpass_script("askpass-test", &token).expect("create_askpass_script failed");
        let path = script_path.expect("expected Some(script_path)");

        // Script should exist
        assert!(path.exists(), "askpass script should be created on disk");

        // Script should contain the token (it's the password source)
        let content = std::fs::read_to_string(&path).expect("failed to read script");
        assert!(
            content.contains("ghp_test_token_123"),
            "script must contain the token"
        );
        assert!(
            content.contains("x-access-token"),
            "script must provide username"
        );

        // Script should be executable and restricted (owner-only on unix)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            // Owner read+execute, no group/other permissions
            assert_eq!(
                mode & 0o777,
                0o500,
                "script must have owner-only r-x permissions"
            );
        }

        // Cleanup
        cleanup_askpass(&path);
        assert!(
            !path.exists(),
            "askpass script must be removed after cleanup"
        );
    }

    #[test]
    fn test_askpass_script_returns_none_without_token() {
        let result =
            create_askpass_script("no-token", &None).expect("create_askpass_script failed");
        assert!(result.is_none(), "no askpass script needed without a token");
    }

    #[test]
    fn test_askpass_script_cleanup_idempotent() {
        let token = Some("ghp_test".to_string());

        let script_path = create_askpass_script("cleanup-test", &token)
            .unwrap()
            .unwrap();
        assert!(script_path.exists());

        cleanup_askpass(&script_path);
        assert!(!script_path.exists());

        // Second cleanup should not panic
        cleanup_askpass(&script_path);
    }

    #[test]
    fn test_askpass_script_escapes_single_quotes() {
        let token = Some("ghp_it's_a_token".to_string());

        let script_path =
            create_askpass_script("quote-test", &token).expect("create_askpass_script failed");
        let path = script_path.expect("expected Some(script_path)");

        let content = std::fs::read_to_string(&path).expect("failed to read script");
        // The script must use the escaped form, not the raw single quote
        assert!(
            content.contains("ghp_it'\\''s_a_token"),
            "script must escape single quotes in the token"
        );

        cleanup_askpass(&path);
    }
}
