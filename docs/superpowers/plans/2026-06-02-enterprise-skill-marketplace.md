# Enterprise Skill Marketplace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the Enterprise Skill Marketplace (P7) — Team Sync, Policy & Compliance, and Telemetry & Reporting — across three phased releases.

**Architecture:** Hexagonal/clean architecture following existing AGK patterns. Domain models in `src/domain/`, use cases in `src/app/features/`, infrastructure in `src/infra/`, CLI in `src/cli/`, TUI in `src/tui/`. New port traits (`TeamConfigStorePort`, `VaultManifestStorePort`, `PolicyStorePort`, `AuditLogPort`) follow the existing `ConfigStorePort` pattern. New `CoreCommand` variants dispatched through `AgkCore::execute()`.

**Tech Stack:** Rust, tokio (async), serde (TOML/JSON), clap (CLI), ratatui (TUI), glob (pattern matching for policy), reqwest (HTTP for telemetry upload).

**Spec:** `docs/superpowers/specs/2026-06-02-enterprise-skill-marketplace-design.md`

---

## Phase 1: Skill Marketplace & Team Sync (v0.4.0)

### Task 1: Domain Models — TeamConfig, VaultManifest, AssetSource

**Files:**
- Create: `src/domain/team.rs`
- Create: `src/domain/vault_manifest.rs`
- Modify: `src/domain/mod.rs` — add `mod team` and `mod vault_manifest`
- Modify: `src/domain/config/vault_section.rs` — add `AssetSource` to `AssetBucket`
- Test: `src/domain/team.rs` (inline `#[cfg(test)]`)
- Test: `src/domain/vault_manifest.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write failing tests for TeamConfig serialization**

Add to `src/domain/team.rs`:

```rust
use serde::{Deserialize, Serialize};
use crate::domain::asset::AssetKind;

fn default_branch() -> String {
    "main".to_string()
}

fn default_kind() -> AssetKind {
    AssetKind::Skill
}

/// Team membership configuration stored in .agk/team.toml
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TeamConfig {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default)]
    pub vaults: Vec<TeamVault>,
    #[serde(default)]
    pub requirements: Vec<TeamRequirement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamVault {
    pub identity: String,
    #[serde(rename = "type")]
    pub vault_type: String,
    pub url: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamRequirement {
    pub identity: String,
    pub vault: String,
    #[serde(default = "default_kind")]
    pub kind: AssetKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_constraint: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_config_round_trip() {
        let config = TeamConfig {
            name: "frontend-platform".to_string(),
            source: Some("github.com/acme-org/frontend-app".to_string()),
            branch: Some("main".to_string()),
            vaults: vec![TeamVault {
                identity: "acme-platform".to_string(),
                vault_type: "github".to_string(),
                url: "https://github.com/acme-org/platform-skills".to_string(),
                branch: "main".to_string(),
                path: None,
            }],
            requirements: vec![TeamRequirement {
                identity: "acme-org/react-conventions".to_string(),
                vault: "acme-platform".to_string(),
                kind: AssetKind::Skill,
                version_constraint: Some(">= 2.0.0".to_string()),
            }],
        };
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: TeamConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed, config);
    }

    #[test]
    fn team_config_defaults() {
        let config: TeamConfig = toml::from_str("[team]\nname = \"test\"\n").unwrap();
        assert_eq!(config.name, "test");
        assert!(config.vaults.is_empty());
        assert!(config.requirements.is_empty());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test domain::team --no-run 2>&1 || true`
Expected: Compilation error — `team` module doesn't exist yet.

- [ ] **Step 3: Create `src/domain/team.rs` with the full TeamConfig model**

Create the file with the full model shown in Step 1.

- [ ] **Step 4: Add `mod team` to `src/domain/mod.rs`**

Add `pub mod team;` to `src/domain/mod.rs`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test domain::team -- --nocapture`
Expected: 2 tests pass — `team_config_round_trip` and `team_config_defaults`.

- [ ] **Step 6: Write failing tests for VaultManifest**

Add to `src/domain/vault_manifest.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Vault metadata stored inside vault repos (.agk/vault.toml)
/// Generated by `agk vault init`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VaultManifest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<VaultDependency>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VaultDependency {
    pub identity: String,
    #[serde(rename = "type")]
    pub dep_type: String,
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_manifest_round_trip() {
        let manifest = VaultManifest {
            name: "platform-skills".to_string(),
            description: Some("Acme Frontend Platform Skills".to_string()),
            version: Some("1.0.0".to_string()),
            dependencies: vec![VaultDependency {
                identity: "clawhub-public".to_string(),
                dep_type: "clawhub".to_string(),
                url: "https://clawhub.ai".to_string(),
            }],
        };
        let toml_str = toml::to_string_pretty(&manifest).unwrap();
        let parsed: VaultManifest = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed, manifest);
    }

    #[test]
    fn vault_manifest_minimal() {
        let toml_str = "[vault]\nname = \"my-vault\"\n";
        let parsed: VaultManifest = toml::from_str(toml_str).unwrap();
        assert_eq!(parsed.name, "my-vault");
        assert!(parsed.description.is_none());
        assert!(parsed.dependencies.is_empty());
    }
}
```

- [ ] **Step 7: Create `src/domain/vault_manifest.rs` and add `mod vault_manifest` to `src/domain/mod.rs`**

- [ ] **Step 8: Run tests to verify VaultManifest passes**

Run: `cargo test domain::vault_manifest -- --nocapture`
Expected: 2 tests pass.

- [ ] **Step 9: Add `AssetSource` enum to `src/domain/config/vault_section.rs`**

Add the `AssetSource` enum to `vault_section.rs` and add a `source` field to `AssetBucket` items:

```rust
/// Tag indicating whether an installed asset is team-mandated or personal.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum AssetSource {
    #[default]
    #[serde(rename = "personal")]
    Personal,
    #[serde(rename = "team")]
    Team,
}
```

Update `AssetBucket` items to carry source info. Since `AssetBucket.items` is currently `Vec<String>`, we need to change this to carry source. The simplest backward-compatible approach: keep items as `Vec<String>` for identity, add a parallel `Vec<AssetSource>` that maps by index. When the source vec is shorter or missing, default to `Personal`.

Add to `AssetBucket`:

```rust
#[serde(default)]
pub sources: Vec<AssetSource>,
```

And update serialization to handle the case where `sources` is empty (backward compat: missing sources = all personal).

- [ ] **Step 10: Write failing tests for AssetSource**

```rust
#[test]
fn asset_source_default_is_personal() {
    assert_eq!(AssetSource::default(), AssetSource::Personal);
}

#[test]
fn asset_source_serializes_to_string() {
    assert_eq!(serde_json::to_string(&AssetSource::Team).unwrap(), "\"team\"");
    assert_eq!(serde_json::to_string(&AssetSource::Personal).unwrap(), "\"personal\"");
}
```

- [ ] **Step 11: Run tests to verify AssetSource passes**

Run: `cargo test domain::config -- --nocapture`
Expected: All existing config tests pass plus new AssetSource tests.

- [ ] **Step 12: Commit**

```bash
git add src/domain/team.rs src/domain/vault_manifest.rs src/domain/mod.rs src/domain/config/vault_section.rs
git commit -m "feat(domain): add TeamConfig, VaultManifest, and AssetSource models for enterprise skill marketplace"
```

---

### Task 2: Infrastructure — TeamConfigStorePort, VaultManifestStorePort, GitignoreManager

**Files:**
- Create: `src/app/ports/team_config_store.rs`
- Create: `src/app/ports/vault_manifest_store.rs`
- Create: `src/infra/config/team_store.rs`
- Create: `src/infra/config/vault_manifest_store.rs`
- Create: `src/infra/config/gitignore.rs`
- Modify: `src/app/ports/mod.rs` — add new port modules
- Modify: `src/infra/config/mod.rs` — add new infra modules
- Test: Inline tests in each file

- [ ] **Step 1: Write failing test for TeamConfigStorePort**

Create `src/app/ports/team_config_store.rs`:

```rust
use crate::domain::team::TeamConfig;
use anyhow::Result;
use crate::domain::scope::Scope;

/// Port for reading/writing team configuration.
/// Concrete implementation: `TeamTomlStore` in `infra/config/team_store.rs`.
pub trait TeamConfigStorePort: Send + Sync {
    fn load(&self, scope: Scope) -> Result<TeamConfig>;
    fn save(&self, scope: Scope, config: &TeamConfig) -> Result<()>;
    fn exists(&self, scope: Scope) -> bool;
}
```

- [ ] **Step 2: Write failing test for VaultManifestStorePort**

Create `src/app/ports/vault_manifest_store.rs`:

```rust
use crate::domain::vault_manifest::VaultManifest;
use anyhow::Result;
use std::path::PathBuf;

/// Port for reading/writing vault manifest (.agk/vault.toml).
/// Concrete implementation: `VaultManifestTomlStore` in `infra/config/vault_manifest_store.rs`.
pub trait VaultManifestStorePort: Send + Sync {
    fn load(&self, path: &PathBuf) -> Result<VaultManifest>;
    fn save(&self, path: &PathBuf, manifest: &VaultManifest) -> Result<()>;
}
```

- [ ] **Step 3: Create infra implementations — TeamTomlStore**

Create `src/infra/config/team_store.rs`:

```rust
use crate::app::ports::TeamConfigStorePort;
use crate::domain::scope::Scope;
use crate::domain::team::TeamConfig;
use anyhow::Result;
use std::path::PathBuf;

pub struct TeamTomlStore {
    workspace_root: PathBuf,
}

impl TeamTomlStore {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    fn team_toml_path(&self, scope: &Scope) -> PathBuf {
        match scope {
            Scope::Workspace => self.workspace_root.join(".agk").join("team.toml"),
            Scope::Global => dirs_next::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
                .join("agk")
                .join("team.toml"),
        }
    }
}

impl TeamConfigStorePort for TeamTomlStore {
    fn load(&self, scope: Scope) -> Result<TeamConfig> {
        let path = self.team_toml_path(&scope);
        if !path.exists() {
            return Ok(TeamConfig::default());
        }
        let content = std::fs::read_to_string(&path)?;
        let config: TeamConfig = toml::from_str(&content)?;
        Ok(config)
    }

    fn save(&self, scope: Scope, config: &TeamConfig) -> Result<()> {
        let path = self.team_toml_path(&scope);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(config)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    fn exists(&self, scope: Scope) -> bool {
        self.team_toml_path(&scope).exists()
    }
}
```

- [ ] **Step 4: Create infra implementation — VaultManifestTomlStore**

Create `src/infra/config/vault_manifest_store.rs` with similar load/save pattern.

- [ ] **Step 5: Create GitignoreManager**

Create `src/infra/config/gitignore.rs`:

```rust
use anyhow::Result;
use std::path::PathBuf;

/// Manages the .agk/.gitignore file that keeps personal config out of git
/// when team.toml is present.
pub struct GitignoreManager;

impl GitignoreManager {
    /// Ensure .agk/.gitignore contains the entry to ignore config.toml.
    /// Called when team.toml is present to keep personal config out of git.
    pub fn ensure_config_gitignore(workspace_root: &PathBuf) -> Result<()> {
        let agk_dir = workspace_root.join(".agk");
        std::fs::create_dir_all(&agk_dir)?;

        let gitignore_path = agk_dir.join(".gitignore");
        let entry = "config.toml";

        if gitignore_path.exists() {
            let content = std::fs::read_to_string(&gitignore_path)?;
            if content.lines().any(|line| line.trim() == entry) {
                return Ok(()); // Already present
            }
            // Append the entry
            let new_content = format!("{}\n{}\n", content.trim_end(), entry);
            std::fs::write(&gitignore_path, new_content)?;
        } else {
            let content = format!("# Personal workspace config — not committed\n{}\n", entry);
            std::fs::write(&gitignore_path, content)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_gitignore_when_missing() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        GitignoreManager::ensure_config_gitignore(&root).unwrap();
        let content = std::fs::read_to_string(root.join(".agk").join(".gitignore")).unwrap();
        assert!(content.contains("config.toml"));
    }

    #[test]
    fn appends_entry_when_gitignore_exists_without_it() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        std::fs::create_dir_all(root.join(".agk")).unwrap();
        std::fs::write(root.join(".agk").join(".gitignore"), "# other ignores\n*.log\n").unwrap();
        GitignoreManager::ensure_config_gitignore(&root).unwrap();
        let content = std::fs::read_to_string(root.join(".agk").join(".gitignore")).unwrap();
        assert!(content.contains("config.toml"));
        assert!(content.contains("*.log"));
    }

    #[test]
    fn does_not_duplicate_entry() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        GitignoreManager::ensure_config_gitignore(&root).unwrap();
        GitignoreManager::ensure_config_gitignore(&root).unwrap();
        let content = std::fs::read_to_string(root.join(".agk").join(".gitignore")).unwrap();
        let count = content.matches("config.toml").count();
        assert_eq!(count, 1);
    }
}
```

- [ ] **Step 6: Add module declarations to mod.rs files**

Update `src/app/ports/mod.rs` to add `pub mod team_config_store;` and `pub mod vault_manifest_store;`.
Update `src/infra/config/mod.rs` to add `pub mod team_store;`, `pub mod vault_manifest_store;`, and `pub mod gitignore;`.

- [ ] **Step 7: Run all tests**

Run: `cargo test -- --nocapture`
Expected: All existing and new tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/app/ports/ src/infra/config/
git commit -m "feat(infra): add TeamConfigStorePort, VaultManifestStorePort, GitignoreManager"
```

---

### Task 3: CLI — `agk vault init` Command

**Files:**
- Modify: `src/cli/entry.rs` — add `VaultInit` subcommand
- Modify: `src/cli/entry_subcommands.rs` — add vault init args
- Modify: `src/cli/core_dispatcher.rs` — add `VaultInit` to `CoreCommand`
- Modify: `src/app/core.rs` — add `VaultInit` dispatch
- Create: `src/app/features/vault/init.rs` — `vault_init()` use case
- Modify: `src/app/features/vault/mod.rs` — add `mod init`
- Modify: `src/app/command.rs` — add `VaultInit` command variant

- [ ] **Step 1: Write failing test for vault_init use case**

Create `src/app/features/vault/init.rs` with test:

```rust
use anyhow::Result;
use std::path::PathBuf;

/// Initialize a vault repo with .agk/vault.toml and standard asset folders.
pub fn vault_init(workspace_root: &PathBuf, name: Option<String>, dry_run: bool) -> Result<VaultInitResult> {
    let vault_name = name.unwrap_or_else(|| {
        workspace_root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "vault".to_string())
    });

    let agk_dir = workspace_root.join(".agk");
    let vault_toml_path = agk_dir.join("vault.toml");

    // Check if vault.toml already exists
    if vault_toml_path.exists() {
        return Ok(VaultInitResult {
            name: vault_name,
            created: false,
            message: "Vault already initialized. Use --force to overwrite.".to_string(),
        });
    }

    if dry_run {
        return Ok(VaultInitResult {
            name: vault_name,
            created: false,
            message: format!("Would initialize vault '{}' with standard folders.", vault_name),
        });
    }

    // Create standard asset folders
    let folders = ["skills", "instructions", "mcps", "profiles"];
    for folder in &folders {
        std::fs::create_dir_all(workspace_root.join(folder))?;
    }

    // Create .agk directory
    std::fs::create_dir_all(&agk_dir)?;

    // Write vault.toml
    let manifest = crate::domain::vault_manifest::VaultManifest {
        name: vault_name.clone(),
        description: None,
        version: Some("1.0.0".to_string()),
        dependencies: vec![],
    };
    let content = toml::to_string_pretty(&manifest)?;
    std::fs::write(&vault_toml_path, content)?;

    Ok(VaultInitResult {
        name: vault_name,
        created: true,
        message: format!("Initialized vault '{}' with standard folders.", vault_name),
    })
}

pub struct VaultInitResult {
    pub name: String,
    pub created: bool,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn vault_init_creates_folders_and_manifest() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let result = vault_init(&root, Some("my-vault".to_string()), false).unwrap();
        assert!(result.created);
        assert_eq!(result.name, "my-vault");
        assert!(root.join(".agk").join("vault.toml").exists());
        assert!(root.join("skills").exists());
        assert!(root.join("instructions").exists());
        assert!(root.join("mcps").exists());
        assert!(root.join("profiles").exists());
    }

    #[test]
    fn vault_init_defaults_to_folder_name() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("platform-skills");
        std::fs::create_dir_all(&root).unwrap();
        let result = vault_init(&root, None, false).unwrap();
        assert_eq!(result.name, "platform-skills");
    }

    #[test]
    fn vault_init_dry_run_does_not_create_files() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let result = vault_init(&root, Some("test".to_string()), true).unwrap();
        assert!(!result.created);
        assert!(!root.join(".agk").join("vault.toml").exists());
    }

    #[test]
    fn vault_init_idempotent() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        vault_init(&root, Some("test".to_string()), false).unwrap();
        let result = vault_init(&root, Some("test".to_string()), false).unwrap();
        assert!(!result.created);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail (missing module declaration)**

- [ ] **Step 3: Add `VaultInit` to CoreCommand, entry.rs, and core dispatcher**

Follow the existing pattern from `ProfileCreate` / `VaultAttach`:

1. Add `VaultInit` variant to `src/app/command.rs` CoreCommand enum
2. Add `VaultInit` subcommand to `src/cli/entry.rs` or `entry_subcommands.rs`
3. Add `vault_init` dispatch in `src/cli/core_dispatcher.rs`
4. Add `vault_init` execution in `src/app/core.rs` AgkCore::execute()

- [ ] **Step 4: Run tests to verify vault_init passes**

Run: `cargo test vault_init -- --nocapture`
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/app/features/vault/init.rs src/cli/ src/app/core.rs src/app/command.rs
git commit -m "feat(cli): add `agk vault init` command to create vault.toml and standard folders"
```

---

### Task 4: CLI — `agk team init/add/add-vault/remove/diff/status/update` Commands

**Files:**
- Create: `src/app/features/team/init.rs`
- Create: `src/app/features/team/add.rs`
- Create: `src/app/features/team/remove.rs`
- Create: `src/app/features/team/diff.rs`
- Create: `src/app/features/team/status.rs`
- Create: `src/app/features/team/update.rs`
- Create: `src/app/features/team/mod.rs`
- Modify: `src/cli/entry.rs` — add `Team` subcommand group
- Modify: `src/cli/entry_subcommands.rs` — add `TeamCommands`
- Modify: `src/cli/core_dispatcher.rs` — add team commands
- Modify: `src/app/core.rs` — add team command dispatch

This is the largest task. Each subcommand follows the same pattern as existing commands (profile, vault, mcp). Implement one at a time with TDD.

- [ ] **Step 1: Write failing tests for team_init**

Create `src/app/features/team/init.rs`:

```rust
use crate::domain::team::TeamConfig;
use anyhow::Result;
use std::path::PathBuf;

/// Initialize team config in the workspace.
pub fn team_init(workspace_root: &PathBuf, name: &str, dry_run: bool) -> Result<TeamInitResult> {
    let agk_dir = workspace_root.join(".agk");
    let team_toml_path = agk_dir.join("team.toml");

    if team_toml_path.exists() {
        return Ok(TeamInitResult {
            name: name.to_string(),
            created: false,
            message: "team.toml already exists. Remove it first or edit directly.".to_string(),
        });
    }

    if dry_run {
        return Ok(TeamInitResult {
            name: name.to_string(),
            created: false,
            message: format!("Would create team.toml with name '{}'.", name),
        });
    }

    std::fs::create_dir_all(&agk_dir)?;

    let config = TeamConfig {
        name: name.to_string(),
        source: None,
        branch: Some("main".to_string()),
        vaults: vec![],
        requirements: vec![],
    };
    let content = toml::to_string_pretty(&config)?;
    std::fs::write(&team_toml_path, content)?;

    // Ensure .agk/.gitignore contains config.toml
    crate::infra::config::gitignore::GitignoreManager::ensure_config_gitignore(workspace_root)?;

    Ok(TeamInitResult {
        name: name.to_string(),
        created: true,
        message: format!("Team '{}' initialized. Edit .agk/team.toml to add vaults and requirements.", name),
    })
}

pub struct TeamInitResult {
    pub name: String,
    pub created: bool,
    pub message: String,
}
```

- [ ] **Step 2: Write tests for team_add_vault and team_add**

Create `src/app/features/team/add.rs` with `team_add_vault()` and `team_add_requirement()` functions. Each loads `team.toml`, mutates the `TeamConfig`, saves back.

- [ ] **Step 3: Write tests for team_remove**

Create `src/app/features/team/remove.rs` with `team_remove_requirement()`.

- [ ] **Step 4: Write tests for team_diff**

Create `src/app/features/team/diff.rs` with `team_diff()` comparing `TeamConfig.requirements` against installed assets in `config.toml`.

- [ ] **Step 5: Write tests for team_status**

Create `src/app/features/team/status.rs` with `team_status()` counting team requirements installed vs missing.

- [ ] **Step 6: Write tests for team_update**

Create `src/app/features/team/update.rs` with `team_update()` pulling latest `team.toml` from source git repo.

- [ ] **Step 7: Add TeamCommands to CLI entry**

Add `Team` subcommand group to `src/cli/entry_subcommands.rs` following the pattern of `ProfileCommands`:

```rust
#[derive(Subcommand, Debug)]
pub enum TeamCommands {
    /// Initialize team configuration
    Init {
        /// Team name
        #[arg(short, long)]
        name: String,
    },
    /// Add a vault to the team marketplace
    AddVault {
        /// Vault identity
        identity: String,
        /// Vault type (github, local, clawhub)
        #[arg(short, long, default_value = "github")]
        vault_type: String,
        /// Vault URL
        #[arg(short, long)]
        url: String,
        /// Branch
        #[arg(short, long, default_value = "main")]
        branch: String,
    },
    /// Add a skill requirement to the team
    Add {
        /// Skill identity (e.g., acme-org/react-conventions)
        identity: String,
        /// Vault to install from
        #[arg(short, long)]
        vault: String,
        /// Asset kind
        #[arg(short, long, default_value = "skill")]
        kind: String,
        /// Version constraint (e.g., >= 2.0.0)
        #[arg(short, long)]
        version_constraint: Option<String>,
    },
    /// Remove a skill requirement from the team
    Remove {
        /// Skill identity to remove
        identity: String,
    },
    /// Show diff between team requirements and installed state
    Diff,
    /// Show team status (installed vs missing)
    Status,
    /// Update team.toml from source repository
    Update,
}
```

- [ ] **Step 8: Add CoreCommand variants and dispatcher**

Add `TeamInit`, `TeamAddVault`, `TeamAddRequirement`, `TeamRemove`, `TeamDiff`, `TeamStatus`, `TeamUpdate` to `CoreCommand` in `src/app/command.rs` and wire up in `core_dispatcher.rs`.

- [ ] **Step 9: Run all tests**

Run: `cargo test -- --nocapture`
Expected: All tests pass.

- [ ] **Step 10: Commit**

```bash
git add src/app/features/team/ src/cli/ src/app/core.rs src/app/command.rs
git commit -m "feat(team): add team init/add-vault/add/remove/diff/status/update commands"
```

---

### Task 5: Core Logic — Team-Aware Sync

**Files:**
- Create: `src/app/features/asset/sync_team.rs`
- Modify: `src/app/features/asset/sync.rs` — integrate team sync into existing sync flow
- Modify: `src/app/features/asset/mod.rs` — add `mod sync_team`

- [ ] **Step 1: Write failing test for team-aware sync**

Create `src/app/features/asset/sync_team.rs`:

```rust
use crate::domain::team::TeamConfig;
use crate::domain::scope::Scope;
use anyhow::Result;

/// Result of a team sync operation.
pub struct TeamSyncResult {
    pub vaults_attached: Vec<String>,
    pub skills_installed: Vec<String>,
    pub skills_updated: Vec<String>,
    pub skills_removed_from_team: Vec<String>,
    pub personal_opt_outs: Vec<String>,
    pub errors: Vec<String>,
}

/// Check if team.toml exists and perform team-aware sync.
/// Returns None if no team.toml is present (regular sync behavior).
pub fn sync_team_config(
    team_config: &TeamConfig,
    config_store: &dyn crate::app::ports::ConfigStorePort,
    registry: &crate::app::registry::Registry,
    scope: Scope,
    provider_filter: Option<&str>,
    dry_run: bool,
) -> Result<TeamSyncResult> {
    let mut result = TeamSyncResult {
        vaults_attached: vec![],
        skills_installed: vec![],
        skills_updated: vec![],
        skills_removed_from_team: vec![],
        personal_opt_outs: vec![],
        errors: vec![],
    };

    // 1. Attach missing team vaults
    for team_vault in &team_config.vaults {
        let config = config_store.load(scope)?;
        if !config.vaults.contains(&team_vault.identity) {
            if !dry_run {
                // Auto-attach vault (delegate to existing attach_vault logic)
                // For now, record what would be attached
            }
            result.vaults_attached.push(team_vault.identity.clone());
        }
    }

    // 2. Install missing team requirements
    for req in &team_config.requirements {
        // Check if already installed
        // If not, install and tag as [Team]
        result.skills_installed.push(req.identity.clone());
    }

    // 3. Check for previously-[Team] skills no longer in team.toml
    // (requires reading config.toml to find Team-sourced assets)

    Ok(result)
}
```

Write tests verifying:
- Auto-attach of missing team vaults
- Install of missing team requirements
- Tagging of team assets as `AssetSource::Team`
- Detection of removed team requirements

- [ ] **Step 2: Integrate team sync into existing `agk sync` flow**

Modify `src/app/features/asset/sync.rs` to check for `team.toml` presence. If present, run `sync_team_config()` before/after regular sync.

- [ ] **Step 3: Add `--provider` flag to sync command**

Modify `src/cli/entry.rs` Sync variant to include `--provider` flag.

- [ ] **Step 4: Run tests**

Run: `cargo test sync_team -- --nocapture`

- [ ] **Step 5: Commit**

```bash
git add src/app/features/asset/sync_team.rs src/app/features/asset/sync.rs src/cli/entry.rs
git commit -m "feat(sync): add team-aware sync with auto-attach and --provider flag"
```

---

### Task 6: TUI — `[Team]` Badge and F3 Toggle

**Files:**
- Create: `src/tui/widgets/team_badge.rs`
- Modify: `src/tui/widgets/mod.rs` — add `mod team_badge`
- Modify: `src/tui/features/assets/controller.rs` — add F3 handler
- Modify: `src/tui/widgets/list_entity.rs` — render `[Team]` badge
- Modify: `src/tui/render/status.rs` — add team status bar

- [ ] **Step 1: Create team badge widget**

Create `src/tui/widgets/team_badge.rs`:

```rust
use ratatui::text::Span;

/// Render a [Team] or [You] badge for the TUI.
pub fn team_badge(source: &crate::domain::config::vault_section::AssetSource) -> Span<'static> {
    match source {
        AssetSource::Team => Span::styled(
            " [Team] ",
            ratatui::style::Style::default().fg(ratatui::style::Color::Cyan),
        ),
        AssetSource::Personal => Span::raw(""),
    }
}

/// Render the team status bar line: "[Team] 15/15 ✓ | 3 personal"
pub fn team_status_line(installed: usize, required: usize, personal: usize) -> String {
    let check = if installed == required { "✓" } else { "✗" };
    format!("[Team] {}/{} {} | {} personal", installed, required, check, personal)
}
```

- [ ] **Step 2: Add F3 key binding for team toggle**

Modify `src/tui/features/assets/controller.rs` to handle F3 key: toggle the selected skill's `AssetSource` between `Team` and `Personal`. This updates `team.toml` (add/remove requirement) and re-tags in `config.toml`.

- [ ] **Step 3: Render `[Team]` badge in skill list**

Modify `src/tui/widgets/list_entity.rs` to check `AssetSource` for each listed skill and append the `[Team]` badge.

- [ ] **Step 4: Add team status to status bar**

Modify the TUI status bar rendering to show `[Team] X/Y ✓ | Z personal` when `team.toml` is present.

- [ ] **Step 5: Run TUI tests (if available) or manual QA**

- [ ] **Step 6: Commit**

```bash
git add src/tui/widgets/team_badge.rs src/tui/features/assets/ src/tui/widgets/list_entity.rs src/tui/render/
git commit -m "feat(tui): add [Team] badge, F3 toggle, and team status bar"
```

---

### Task 7: Architecture Tests and Integration

**Files:**
- Modify: `tests/architecture/` — add tests for new domain models
- Create: `tests/integration/team_sync.rs` — full team sync flow test

- [ ] **Step 1: Add architecture tests for new domain models**

Verify that `domain/team.rs`, `domain/vault_manifest.rs`, and `domain/policy.rs` (when added) follow hexagonal architecture rules (no I/O in domain).

- [ ] **Step 2: Write integration test for full team sync flow**

Create `tests/integration/team_sync.rs`:

```rust
use assert_cmd::Command;

#[test]
fn team_init_creates_team_toml() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("agk").unwrap();
    cmd.args(["team", "init", "--name", "test-team"])
        .env("AGK_WORKSPACE", dir.path())
        .assert()
        .success();

    assert!(dir.path().join(".agk/team.toml").exists());
    assert!(dir.path().join(".agk/.gitignore").exists());
}

#[test]
fn vault_init_creates_vault_toml_and_folders() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("agk").unwrap();
    cmd.args(["vault", "init", "--name", "my-vault"])
        .env("AGK_WORKSPACE", dir.path())
        .assert()
        .success();

    assert!(dir.path().join(".agk/vault.toml").exists());
    assert!(dir.path().join("skills").exists());
    assert!(dir.path().join("instructions").exists());
    assert!(dir.path().join("mcps").exists());
    assert!(dir.path().join("profiles").exists());
}
```

- [ ] **Step 3: Run full test suite**

Run: `cargo test -- --nocapture && cargo test --test architecture -- --ignored`
Expected: All unit, integration, and architecture tests pass.

- [ ] **Step 4: Commit**

```bash
git add tests/
git commit -m "test: add architecture and integration tests for team sync and vault init"
```

---

### Task 8: End-to-End Manual QA

- [ ] **Step 1: `agk vault init` in a test directory** — creates `.agk/vault.toml` with folder name, `skills/`, `instructions/`, `mcps/`, `profiles/` folders
- [ ] **Step 2: `agk team init --name test-team`** — creates `.agk/team.toml`, creates `.agk/.gitignore` with `config.toml`
- [ ] **Step 3: `agk team add-vault`** — adds a vault to team.toml
- [ ] **Step 4: `agk team add`** — adds a skill requirement to team.toml
- [ ] **Step 5: `agk team status`** — shows X/Y installed
- [ ] **Step 6: `agk team diff`** — shows missing/extra/outdated
- [ ] **Step 7: `agk sync`** — auto-attaches team vaults, installs team skills
- [ ] **Step 8: TUI: `[Team]` badge visible** on team-mandated skills
- [ ] **Step 9: TUI: F3 toggles team membership**
- [ ] **Step 10: TUI: Status bar shows `[Team] X/Y ✓ | Z personal`**

---

## Phase 2: Policy & Compliance (v0.4.1)

*Outline only — full TDD plan to be written after Phase 1 ships.*

### Task 9: Domain Models — PolicyConfig, PolicyViolation, PolicyAction

- Create `src/domain/policy.rs` with `PolicyConfig`, `PolicyViolation`, `PolicyAction`
- Add `PolicyStorePort` trait to `src/app/ports/`
- Create `src/infra/config/policy_store.rs` (load/save `policy.toml`)
- TOML round-trip tests

### Task 10: Policy Engine — check_install_policy, check_vault_policy

- Create `src/app/features/policy/check.rs` with glob pattern matching
- Create `src/app/features/policy/status.rs`
- Policy merge logic (global → workspace)
- Unit tests for each policy rule

### Task 11: Audit Logger

- Create `src/infra/audit.rs` with `AuditLogger` (append-only JSONL writer)
- Rotate at 10MB, keep last 3 files
- Integration test: write/read audit log

### Task 12: Policy Enforcement in Install/Sync Flow

- Hook `check_install_policy()` before every `install_asset()` call
- Hook `check_vault_policy()` before every vault attach
- Exit code 4 for `POLICY_VIOLATION`, exit code 6 for `TEAM_REQUIREMENT`
- TUI modal for policy violations

### Task 13: CLI — `agk policy status/check/audit-log`

- Add `Policy` subcommand group to CLI entry
- Add `PolicyCheck`, `PolicyStatus`, `PolicyAuditLog` to `CoreCommand`
- `--json` output for all commands

---

## Phase 3: Telemetry & Reporting (v0.4.2)

*Outline only — full TDD plan to be written after Phase 2 ships.*

### Task 14: Domain Models — TeamReportConfig, TeamReport, SkillUsage, McpUsage

- Extend `src/domain/telemetry.rs` with team reporting models
- TOML serialization for `TeamReportConfig`

### Task 15: Team Report Generation

- Create `src/app/features/telemetry/team_report.rs`
- Create `src/app/features/telemetry/stale_report.rs`
- CSV and JSON export

### Task 16: Team Reporter Infrastructure

- Create `src/infra/telemetry/reporter.rs` with `TeamReporter`
- Anonymization logic (strip usernames, machine names)
- HTTP POST to company endpoint (opt-in)

### Task 17: CLI — `agk telemetry team-report/stale-report`

- Add subcommands to CLI entry
- `--json` output for CI integration

---

## Cross-Cutting Tasks (Apply to All Phases)

### Task: Update Documentation

- Update `docs/product/architecture.md` with new config files and modules
- Create `docs/product/features/team-sync/prd.md`
- Create `docs/product/features/team-sync/technical_design.md`
- Update `docs/SUPPORT.md` with new CLI commands

### Task: Version Bump

- Update `Cargo.toml` version to `0.4.0` (Phase 1), `0.4.1` (Phase 2), `0.4.2` (Phase 3)

---

*Implementation Plan v0.1 — 2026-06-02*