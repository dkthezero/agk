# Technical Design: GHES Vault Support

**Status:** Draft
**Epic:** [v0.3.1 Enterprise Bridge & Profile Portability](../../../epics/v031-enterprise-bridge.md)
**Related PRD:** [GHES Vault PRD](prd.md)

---

## Architecture

The GHES feature is a minimal extension to the existing `GithubVaultAdapter`. It requires:

1. Domain model extension (`VaultConfig` gains `enterprise_url`).
2. Adapter update (`GithubVaultAdapter` uses custom API base URL when present).
3. Token resolution strategy (try `gh auth`, then env vars).
4. Config persistence (`enterprise_url` in TOML).

### Domain Changes

```rust
// domain/config.rs
pub struct VaultConfig {
    pub id: String,
    pub kind: VaultKind,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enterprise_url: Option<String>, // ← NEW
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}
```

### Adapter Changes

```rust
// infra/vault/github.rs
pub struct GithubVaultAdapter {
    id: String,
    repo_url: String,
    api_base_url: String,   // ← "https://api.github.com" or "https://github.acme.internal/api/v3"
    enterprise_host: Option<String>,
    token: String,
    branch: String,
    subfolder: Option<String>,
}

impl GithubVaultAdapter {
    pub fn new(config: &VaultConfig) -> Result<Self> {
        let enterprise_host = config.enterprise_url.as_ref().map(|u| {
            parse_host(u) // e.g., "github.acme.internal"
        });

        let api_base_url = match &enterprise_host {
            Some(host) => format!("https://{}/api/v3", host),
            None => "https://api.github.com".to_string(),
        };

        let token = resolve_token(&enterprise_host)?;

        Ok(Self { ... })
    }
}

fn resolve_token(enterprise_host: &Option<String>) -> Result<String> {
    // 1. Try `gh auth token --hostname <host>`
    if let Some(host) = enterprise_host {
        if let Ok(token) = run_gh_auth_token(host) {
            return Ok(token);
        }
    }
    // 2. GITHUB_TOKEN
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        return Ok(token);
    }
    // 3. GITHUB_ENTERPRISE_TOKEN
    if let Ok(token) = std::env::var("GITHUB_ENTERPRISE_TOKEN") {
        return Ok(token);
    }
    anyhow::bail!("No GitHub token found. Run `gh auth login --hostname <host>` or set GITHUB_TOKEN.")
}
```

### Token Resolution

```rust
// infra/vault/github.rs
fn run_gh_auth_token(host: &str) -> Result<String> {
    let output = std::process::Command::new("gh")
        .args([&"auth", &"token", &"--hostname", host])
        .output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(anyhow::anyhow!("gh auth token failed"))
    }
}
```

### Bootstrap Wiring

```rust
// app/bootstrap.rs
fn build_vaults(config: &ConfigFile) -> Vec<Box<dyn VaultPort>> {
    config.vaults.iter().map(|vc| {
        match vc.kind {
            VaultKind::Local => Box::new(LocalVaultAdapter::new(vc)),
            VaultKind::Github => Box::new(GithubVaultAdapter::new(vc)),
            VaultKind::Clawhub => Box::new(ClawHubVaultAdapter::new(vc)),
        }
    }).collect()
}
```

---

## Testing Strategy

| Layer | What | How |
|-------|------|-----|
| Domain | `VaultConfig` with `enterprise_url` roundtrips via serde | Unit |
| Infra | `resolve_token` order: `gh` → `GITHUB_TOKEN` → `GITHUB_ENTERPRISE_TOKEN` | Unit with mocked env |
| Infra | `GithubVaultAdapter::new` sets correct `api_base_url` | Unit |
| Integration | Mock GHES API server (`wiremock` or `httptest`) returns repo contents | Integration test |
| Integration | `agk vault attach` with GHES URL creates correct `VaultConfig` | CLI contract test |

---

*Technical Design v0.1 — 2026-05-30*
