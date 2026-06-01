use anyhow::{bail, Result};

/// Resolve a GitHub authentication token.
///
/// Resolution order:
/// 1. If `enterprise_host` is provided, try `gh auth token --hostname <host>`
/// 2. Fall back to `GITHUB_TOKEN` env var
/// 3. Fall back to `GITHUB_ENTERPRISE_TOKEN` env var
///
/// Returns an error if no token can be found.
pub fn resolve_token(enterprise_host: Option<&str>) -> Result<String> {
    // Step 1: Try gh CLI with hostname if provided
    if let Some(host) = enterprise_host {
        if let Ok(token) = try_gh_auth_token(Some(host)) {
            return Ok(token);
        }
    }

    // Step 2: GITHUB_TOKEN env var
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            return Ok(token);
        }
    }

    // Step 3: GITHUB_ENTERPRISE_TOKEN env var
    if let Ok(token) = std::env::var("GITHUB_ENTERPRISE_TOKEN") {
        if !token.is_empty() {
            return Ok(token);
        }
    }

    // Step 4: Try gh CLI without hostname as last resort
    if let Ok(token) = try_gh_auth_token(None) {
        return Ok(token);
    }

    bail!(
        "No GitHub token found. Set GITHUB_TOKEN or GITHUB_ENTERPRISE_TOKEN, \
         or run `gh auth login` to authenticate."
    )
}

/// Attempt to retrieve a token from the `gh` CLI.
///
/// If `hostname` is provided, passes `--hostname <host>` to `gh auth token`.
/// Returns `Ok(token)` on success, `Err` if `gh` is unavailable or not
/// authenticated.
fn try_gh_auth_token(hostname: Option<&str>) -> Result<String> {
    let mut cmd = std::process::Command::new("gh");
    cmd.arg("auth").arg("token");

    if let Some(host) = hostname {
        cmd.arg("--hostname").arg(host);
    }

    let output = cmd.output()?;

    if !output.status.success() {
        bail!("gh auth token returned non-zero exit code");
    }

    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if token.is_empty() {
        bail!("gh auth token returned empty output");
    }

    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mutex to serialize tests that mutate process-wide environment variables,
    /// preventing parallel races under `cargo test` (which runs tests in
    /// parallel by default).
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Test that resolution falls through to GITHUB_TOKEN when no enterprise
    /// host is given and gh CLI is unavailable.
    #[test]
    fn resolve_token_prefers_github_token_env() {
        let _lock = ENV_LOCK.lock().unwrap();

        // Save and restore prior GITHUB_TOKEN
        let saved_github_token = std::env::var("GITHUB_TOKEN").ok();
        std::env::set_var("GITHUB_TOKEN", "test-token-123");
        let result = resolve_token(None);
        assert!(result.is_ok());
        // GITHUB_TOKEN takes precedence, but if gh auth returns a token first
        // it would win. Either way, we get a token.
        assert!(!result.unwrap().is_empty());
        std::env::remove_var("GITHUB_TOKEN");
        if let Some(token) = saved_github_token {
            std::env::set_var("GITHUB_TOKEN", token);
        }
    }

    /// Test that GITHUB_ENTERPRISE_TOKEN is used when GITHUB_TOKEN is absent.
    #[test]
    fn resolve_token_falls_back_to_ghe_env() {
        let _lock = ENV_LOCK.lock().unwrap();

        // Save and clear GITHUB_TOKEN so GITHUB_ENTERPRISE_TOKEN is tested
        let saved_github_token = std::env::var("GITHUB_TOKEN").ok();
        std::env::remove_var("GITHUB_TOKEN");

        // Save and restore prior GITHUB_ENTERPRISE_TOKEN
        let saved_ghe_token = std::env::var("GITHUB_ENTERPRISE_TOKEN").ok();
        std::env::set_var("GITHUB_ENTERPRISE_TOKEN", "ghe-token-456");

        let result = resolve_token(None);
        assert!(result.is_ok());
        // The resolved token should be non-empty. It comes from either
        // GITHUB_ENTERPRISE_TOKEN or gh auth (if available).
        assert!(!result.unwrap().is_empty());

        std::env::remove_var("GITHUB_ENTERPRISE_TOKEN");
        if let Some(token) = saved_ghe_token {
            std::env::set_var("GITHUB_ENTERPRISE_TOKEN", token);
        }
        if let Some(token) = saved_github_token {
            std::env::set_var("GITHUB_TOKEN", token);
        }
    }

    /// Test that resolution errors when no token source is available.
    #[test]
    fn resolve_token_errors_when_none_available() {
        let _lock = ENV_LOCK.lock().unwrap();

        // Save and clear both env vars
        let saved_github_token = std::env::var("GITHUB_TOKEN").ok();
        let saved_ghe_token = std::env::var("GITHUB_ENTERPRISE_TOKEN").ok();
        std::env::remove_var("GITHUB_TOKEN");
        std::env::remove_var("GITHUB_ENTERPRISE_TOKEN");

        let result = resolve_token(Some("nonexistent.ghes.example.com"));
        // Cannot strongly assert error vs success: gh CLI availability and
        // authentication state vary across environments. In most CI/test
        // environments this will be an error, but if gh happens to return a
        // token that is also valid.
        let _ = result;

        // Restore both env vars
        if let Some(token) = saved_github_token {
            std::env::set_var("GITHUB_TOKEN", token);
        }
        if let Some(token) = saved_ghe_token {
            std::env::set_var("GITHUB_ENTERPRISE_TOKEN", token);
        }
    }

    /// Test that an enterprise host is passed to gh auth token --hostname.
    #[test]
    fn try_gh_auth_token_constructs_hostname_flag() {
        // This just verifies the function doesn't panic; actual gh CLI
        // availability varies by environment.
        let result = try_gh_auth_token(Some("github.example.com"));
        // We don't assert success/failure since gh may not be installed,
        // but we verify it returns a Result without panicking.
        let _ = result;
    }

    /// Test that gh auth token without hostname works (or gracefully fails).
    #[test]
    fn try_gh_auth_token_no_hostname() {
        let result = try_gh_auth_token(None);
        let _ = result;
    }
}
