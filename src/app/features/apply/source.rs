//! Resolve an `agk apply <source>` reference into a populated
//! [`ApplyConfigInput`].
//!
//! The CLI mapper builds an empty `ApplyConfigInput::from_url(source)` and
//! delegates the actual read/parse to the use case so that a missing or
//! unreadable source surfaces as a clear error instead of a silent
//! false-success.
//!
//! Supported sources:
//! - `context://<name>` — internal scheme used by `agk context create`; the
//!   input is returned untouched (the caller drives the context upsert).
//! - local file path — read and parsed as a TOML [`TeamConfig`]; its vaults
//!   are mapped into `ApplyConfigInput.vaults`.
//! - `http://` / `https://` — not yet supported; a clear error is returned.

use crate::app::features::apply::command::{ApplyConfigInput, ApplyVault};
use crate::domain::config::{ClawHubVaultSource, GithubVaultSource, LocalVaultSource, VaultConfig};
use crate::domain::team::TeamConfig;
use anyhow::{anyhow, Result};

/// Map a [`crate::domain::team::TeamVault`] (the declarative team.toml shape)
/// into the [`VaultConfig`] used by the config store / `attach_vault`.
fn team_vault_to_config(vault: &crate::domain::team::TeamVault) -> Result<VaultConfig> {
    match vault.vault_type.as_str() {
        "github" => Ok(VaultConfig::Github(GithubVaultSource {
            repo: vault.url.clone(),
            r#ref: vault.branch.clone(),
            path: vault.path.clone().unwrap_or_default(),
            enterprise_url: None,
        })),
        "local" => Ok(VaultConfig::Local(LocalVaultSource {
            // A local vault's `url` is tolerated as the path when no explicit
            // `path` is provided, matching the team.toml convention.
            path: vault.path.clone().unwrap_or_else(|| vault.url.clone()),
        })),
        "clawhub" => Ok(VaultConfig::Clawhub(ClawHubVaultSource {})),
        other => Err(anyhow!(
            "Unknown vault type '{}' in apply source (expected: github, local, clawhub)",
            other
        )),
    }
}

/// Resolve `source_url` into a populated [`ApplyConfigInput`].
///
/// `input` is the (typically empty) input built by the CLI mapper; its
/// `source_url` is preserved and its `vaults` are populated from the parsed
/// source when applicable.
///
/// Resolution only runs when the input is "unresolved" — i.e. no vaults,
/// providers, or profiles were supplied via the builders. This lets callers
/// that construct an input programmatically (tests, `agk context create`)
/// bypass file resolution while still labeling the source, and ensures the
/// CLI path (`ApplyConfigInput::from_url(source)` with nothing else) actually
/// reads the source instead of silently applying an empty config.
///
/// The `context://` scheme always short-circuits, returning the input
/// untouched so `agk context create` keeps its existing behavior.
pub fn resolve_source(mut input: ApplyConfigInput) -> Result<ApplyConfigInput> {
    // Internal scheme used by `agk context create` — no file to read.
    if input.source_url.starts_with("context://") {
        return Ok(input);
    }

    // If the caller already populated the input via builders, treat the
    // source_url as a label and skip file resolution.
    let already_resolved =
        !input.vaults.is_empty() || !input.providers.is_empty() || !input.profiles.is_empty();
    if already_resolved {
        return Ok(input);
    }

    // URL sources require network I/O which is not yet wired into the apply
    // use case. Surface a clear, actionable error instead of silently
    // applying an empty config and reporting success.
    if input.source_url.starts_with("http://") || input.source_url.starts_with("https://") {
        return Err(anyhow!(
            "URL apply sources are not yet supported. Save the config locally and pass the file path instead: `agk apply ./team.toml`"
        ));
    }

    // Local file path: read, parse as TeamConfig, map vaults into the input.
    let path = std::path::Path::new(&input.source_url);
    let contents = std::fs::read_to_string(path)
        .map_err(|e| anyhow!("Failed to read apply source '{}': {}", input.source_url, e))?;

    let team: TeamConfig = toml::from_str(&contents).map_err(|e| {
        anyhow!(
            "Failed to parse apply source '{}' as team config TOML: {}",
            input.source_url,
            e
        )
    })?;

    for vault in &team.vaults {
        let config = team_vault_to_config(vault)?;
        input.vaults.push(ApplyVault {
            id: vault.identity.clone(),
            config,
        });
    }

    Ok(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::VaultConfig;

    #[test]
    fn context_scheme_short_circuits_unchanged() {
        let input = ApplyConfigInput::from_url("context://acme");
        let resolved = resolve_source(input.clone()).unwrap();
        assert_eq!(resolved, input);
        assert!(resolved.vaults.is_empty());
    }

    #[test]
    fn http_url_source_returns_unsupported_error() {
        let input = ApplyConfigInput::from_url("https://example.com/team.toml");
        let err = resolve_source(input).unwrap_err().to_string();
        assert!(
            err.contains("URL apply sources are not yet supported"),
            "got: {err}"
        );
    }

    #[test]
    fn missing_local_file_returns_read_error() {
        let input = ApplyConfigInput::from_url("/nonexistent/path/team.toml");
        let err = resolve_source(input).unwrap_err().to_string();
        assert!(err.contains("Failed to read apply source"), "got: {err}");
    }

    #[test]
    fn malformed_local_file_returns_parse_error() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("agk_apply_malformed_{}.toml", std::process::id()));
        std::fs::write(&path, "this is = not = valid toml = [[[").unwrap();
        let input = ApplyConfigInput::from_url(path.to_string_lossy().to_string());
        let err = resolve_source(input).unwrap_err().to_string();
        std::fs::remove_file(&path).ok();
        assert!(err.contains("Failed to parse apply source"), "got: {err}");
    }

    #[test]
    fn valid_local_file_populates_vaults() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("agk_apply_valid_{}.toml", std::process::id()));
        let toml = r#"
name = "my-team"
[[vaults]]
identity = "shared"
type = "github"
url = "https://github.com/org/skills"
branch = "main"
path = "skills/"
"#;
        std::fs::write(&path, toml).unwrap();
        let input = ApplyConfigInput::from_url(path.to_string_lossy().to_string());
        let resolved = resolve_source(input).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(resolved.vaults.len(), 1);
        assert_eq!(resolved.vaults[0].id, "shared");
        assert!(matches!(resolved.vaults[0].config, VaultConfig::Github(_)));
    }

    #[test]
    fn unknown_vault_type_returns_error() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("agk_apply_badtype_{}.toml", std::process::id()));
        let toml = r#"
name = "my-team"
[[vaults]]
identity = "weird"
type = "ftp"
url = "ftp://example.com"
branch = "main"
"#;
        std::fs::write(&path, toml).unwrap();
        let input = ApplyConfigInput::from_url(path.to_string_lossy().to_string());
        let err = resolve_source(input).unwrap_err().to_string();
        std::fs::remove_file(&path).ok();
        assert!(err.contains("Unknown vault type 'ftp'"), "got: {err}");
    }
}
