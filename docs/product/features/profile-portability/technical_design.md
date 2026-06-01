# Technical Design: Profile Export / Import

**Status:** Draft
**Epic:** [v0.3.1 Enterprise Bridge & Profile Portability](../../../epics/v031-enterprise-bridge.md)
**Related PRD:** [Profile Portability PRD](prd.md)

---

## Architecture

Export/Import adds two new use cases to the `profile` feature slice. They are thin wrappers around existing domain models and config store operations.

### Domain Model

```rust
// domain/profile.rs

/// Portable serialization of a profile for cross-machine sharing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedProfile {
    pub agk_version: String,
    pub exported_at: String,
    pub profile: ExportPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPayload {
    pub name: String,
    pub provider_id: String,
    pub scope: Scope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_answers: Option<HashMap<String, String>>,
    #[serde(default)]
    pub skills: Vec<ProfileAssetRef>,
    #[serde(default)]
    pub mcps: Vec<ProfileAssetRef>,
    #[serde(default)]
    pub instructions: Vec<ProfileAssetRef>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    pub agent_markdown: String,
}
```

### Use Cases

```rust
// app/features/profile/export.rs
pub fn run(
    profile_id: ProfileId,
    scope: Scope,
    resolve_vaults: bool,
    store: &dyn ConfigStorePort,
) -> Result<ExportedProfile> {
    let config = store.read(scope)?;
    let profile = config.profiles
        .iter()
        .find(|p| p.name == profile_id.as_str())
        .ok_or_else(|| anyhow!("Profile not found"))?;

    let mut export = ExportPayload {
        name: profile.name.clone(),
        provider_id: profile.provider_id.clone(),
        scope: profile.scope.clone(),
        structured_answers: profile.structured_answers.clone(),
        skills: profile.skills.clone(),
        mcps: profile.mcps.clone(),
        instructions: profile.instructions.clone(),
        tools: profile.tools.clone(),
        permission_mode: profile.permission_mode.clone(),
        agent_markdown: read_agent_markdown(&profile.name)?,
    };

    if resolve_vaults {
        // Replace "auto" vaults with best-guess resolution
        for skill in &mut export.skills {
            if skill.vault == "auto" {
                skill.vault = resolve_vault(&skill.name, store)?;
            }
        }
    }

    Ok(ExportedProfile {
        agk_version: env!("CARGO_PKG_VERSION").to_string(),
        exported_at: Utc::now().to_rfc3339(),
        profile: export,
    })
}
```

```rust
// app/features/profile/import.rs
pub fn run(
    export: ExportedProfile,
    target_name: Option<String>,
    target_scope: Scope,
    store: &dyn ConfigStorePort,
) -> Result<()> {
    // Version compatibility check
    let version = Version::parse(&export.agk_version)?;
    let current = Version::parse(env!("CARGO_PKG_VERSION"))?;
    if version.major > current.major {
        bail!("Export was created with a newer major version of AGK. Please upgrade.");
    }

    let name = target_name.unwrap_or_else(|| export.profile.name.clone());

    // Check collision
    let config = store.read(target_scope)?;
    if config.profiles.iter().any(|p| p.name == name) {
        bail!("Profile '{}' already exists in scope {:?}", name, target_scope);
    }

    // Replace missing vaults with "auto"
    let mut skills = export.profile.skills.clone();
    let mut mcps = export.profile.mcps.clone();
    let mut instructions = export.profile.instructions.clone();
    for asset in skills.iter_mut().chain(mcps.iter_mut()).chain(instructions.iter_mut()) {
        if !vault_is_attached(&asset.vault, &config) {
            asset.vault = "auto".to_string();
        }
    }

    // Create profile
    let profile = Profile {
        name: name.clone(),
        provider_id: export.profile.provider_id.clone(),
        scope: target_scope,
        skills,
        mcps,
        instructions,
        tools: export.profile.tools.clone(),
        permission_mode: export.profile.permission_mode.clone(),
        // ... other fields
    };

    // Write config + agent.md
    let mut new_config = config.clone();
    new_config.profiles.push(profile);
    store.write(target_scope, &new_config)?;
    write_agent_markdown(&name, &export.profile.agent_markdown)?;

    Ok(())
}
```

### TUI Integration

- **Controller:** `tui/features/profile/controller.rs` gains `handle_export_input()` and `handle_import_input()`.
- **Modal:** Reuses existing modal infrastructure (`modal_long.rs` for file path input, scrollable preview).
- **Event:** `AppEvent::ExecuteCommand(CoreCommand::ExportProfile { ... })` / `ImportProfile { ... }`.

---

## Testing Strategy

| Layer | What | How |
|-------|------|-----|
| Domain | `ExportedProfile` serde roundtrip | Unit |
| App | Export use case with `FakeStore` | Unit |
| App | Import use case: version mismatch, collision, missing vaults | Unit |
| Integration | Full export → import roundtrip | `TestBackend` + `FakeStore` |
| Contract | `agk profile export --json` schema validation | `assert_cmd` + JSON schema |

---

*Technical Design v0.1 — 2026-05-30*
