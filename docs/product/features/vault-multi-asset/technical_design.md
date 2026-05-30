# Technical Design: Vault Multi-Asset Scanning (v0.3)

**Status:** Draft
**Epic:** [v0.3 Team-Ready Profiles](../../../epics/v03-team-ready-profiles.md)
**Related PRD:** [Vault Multi-Asset PRD](prd.md)

---

## Architecture

The `FeatureSetPort` pattern is extended to support two new asset kinds: `McpServer` and `Profile`. This requires:

1. Domain model extension (`AssetKind`).
2. Two new feature set scanners (`McpFeatureSet`, `ProfileFeatureSet`).
3. Updates to `filter_scan` and the bootstrap scan loop.
4. Deletion of `StubFeatureSet("mcp")` and `StubFeatureSet("profile")`.

### Domain Changes

```rust
// domain/asset.rs
pub enum AssetKind {
    Skill,
    Instruction,
    McpServer,
    Profile,      // ← NEW
}
```

### McpFeatureSet

```rust
// infra/feature/mcp.rs
pub struct McpFeatureSet;

impl FeatureSetPort for McpFeatureSet {
    fn kind_name(&self) -> &str { "mcp" }
    fn display_name(&self) -> &str { "MCP Servers" }
    fn scan_root(&self) -> &str { "mcps" }
    fn asset_kind(&self) -> AssetKind { AssetKind::McpServer }

    fn is_package(&self, path: &Path) -> bool {
        path.join("MCP.md").exists()
    }

    fn hash_files(&self, path: &Path) -> Vec<PathBuf> {
        vec![path.join("MCP.md")]
    }

    fn extract_version(&self, path: &Path) -> Option<String> {
        let content = std::fs::read_to_string(path.join("MCP.md")).ok()?;
        super::extract_frontmatter_version(&content)
    }
}
```

### ProfileFeatureSet

```rust
// infra/feature/profile.rs
pub struct ProfileFeatureSet;

impl FeatureSetPort for ProfileFeatureSet {
    fn kind_name(&self) -> &str { "profile" }
    fn display_name(&self) -> &str { "Profiles" }
    fn scan_root(&self) -> &str { "profiles" }
    fn asset_kind(&self) -> AssetKind { AssetKind::Profile }

    fn is_package(&self, path: &Path) -> bool {
        path.join("PROFILE.md").exists()
    }

    fn hash_files(&self, path: &Path) -> Vec<PathBuf> {
        vec![path.join("PROFILE.md")]
    }
}
```

### Bootstrap Registry Updates

```rust
// app/bootstrap/registry.rs
registry.register_feature_set(Box::new(crate::infra::feature::skill::SkillFeatureSet));
registry.register_feature_set(Box::new(crate::infra::feature::mcp::McpFeatureSet));      // ← replaces StubFeatureSet("mcp")
registry.register_feature_set(Box::new(crate::infra::feature::instruction::InstructionFeatureSet));
registry.register_feature_set(Box::new(crate::infra::feature::stub::StubFeatureSet::new("provider", "Providers", "")));
registry.register_feature_set(Box::new(crate::infra::feature::profile::ProfileFeatureSet)); // ← replaces StubFeatureSet("profile")
registry.register_feature_set(Box::new(crate::infra::feature::stub::StubFeatureSet::new("vault", "Vaults", "")));
```

### filter_scan Update

```rust
// app/bootstrap/scan.rs
let is_global = match pkg.kind {
    AssetKind::Skill => { global_config.is_skill_installed(...) }
    AssetKind::Instruction => { global_config.is_instruction_installed(...) }
    AssetKind::McpServer => { global_config.is_mcp_registered(...) }  // ← NEW
    AssetKind::Profile => { global_config.is_profile_installed(...) } // ← NEW
};
```

---

## MCP Registration from Vault

When a user installs a vault-discovered MCP:

1. Read `MCP.md` from vault path.
2. Parse YAML frontmatter into `McpServer` domain struct.
3. Write to global registry (`~/.config/agk/mcp.toml`) via `McpRegistryPort`.
4. Run `McpRegistryPort::test_server(name)` for JSON-RPC handshake.
5. On success, mark `tested = true` in registry.

**Note:** Vault-sourced MCPs do NOT auto-enable for providers. The user must explicitly `Space`-toggle after registration, same as manually-registered MCPs.

---

## Profile Batch Installation

When a user installs a vault-discovered profile:

1. Parse `PROFILE.md` frontmatter.
2. Resolve referenced skills against the same vault (and attached vaults if identity is qualified).
3. For each missing skill: call `AssetInstaller::install(skill, scope)`.
4. For each missing instruction: same as skills.
5. For each missing MCP: register from vault if not already in global registry.
6. Create profile entry in `config.toml` with `ProfileAssetRef` entries pointing to resolved vaults.
7. Generate `.agk/profiles/<name>/agent.md` (OpenCode: via `opencode agent create`; Claude Code: direct write).

**Atomicity:** Collect all operations into a `BatchInstallPlan`. If any operation fails, roll back completed operations and return `CoreEvent::BatchInstallFailed { missing: Vec<String> }`.

---

## ConfigFile Schema

```rust
// domain/config.rs
pub struct ConfigFile {
    // ... existing fields ...

    #[serde(default)]
    pub installed_mcps: Vec<InstalledAsset>,

    #[serde(default)]
    pub installed_profiles: Vec<InstalledAsset>,
}

pub struct InstalledAsset {
    pub identity: AssetIdentity,
    pub vault_id: String,
    pub installed_at: String, // ISO8601
    pub sha10: String,
}
```

---

## Testing Strategy

| Layer | What | How |
|-------|------|-----|
| Domain | `AssetKind::Profile` serialization | Unit: roundtrip via serde |
| Infra | `McpFeatureSet::is_package` | Unit: temp dir with/without `MCP.md` |
| Infra | `ProfileFeatureSet::is_package` | Unit: temp dir with/without `PROFILE.md` |
| Integration | Vault scan finds MCPs + Profiles | `FakeVault` with `mcps/` and `profiles/` → assert scan result |
| Integration | Batch profile install | `FakeStore` + `FakeVault` → assert all assets installed |
| Integration | `filter_scan` retention | `FakeStore` with pre-installed MCP → assert retained in scan |

---

*Technical Design v0.1 — 2026-05-30*
