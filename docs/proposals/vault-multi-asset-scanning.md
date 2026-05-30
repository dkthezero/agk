# Research & Proposal: Vault-Discoverable MCP Servers, Profiles & Multi-Asset Vaults

> **Status:** Research Complete | **Audience:** Core maintainers + AI agents
> **Scope:** Feature enhancement to make vaults the single source of truth for ALL team AI assets

---

## 1. Executive Summary

AGK's vault scanning system currently discovers **only two asset types** from vaults:
- **Skills** (`skills/*/SKILL.md`)
- **Instructions** (`instructions/*/AGENTS.md`)

**MCP servers, profiles, and any future asset types (harnesses, agent definitions) are NOT discoverable from vaults.** They are registered as `StubFeatureSet` placeholders that never scan. This means:

1. Teams cannot distribute MCP server definitions through their vaults — every developer must manually register each MCP server.
2. Teams cannot distribute profile templates through their vaults — every developer must run the interactive wizard.
3. Vaults are incomplete "team AI environment kits" — they only cover half the asset surface.

**This proposal advocates for extending the `FeatureSetPort` pattern to support MCP servers and profiles (and future asset types) as first-class vault-discoverable assets.** This transforms vaults from "skill/instruction repositories" into "complete AI environment blueprints."

---

## 2. Current State — Evidence

### 2.1 Feature Set Registry (Bootstrap)

`src/app/bootstrap/registry.rs` registers feature sets in tab order:

```rust
// Feature sets — order defines tab order
registry.register_feature_set(Box::new(crate::infra::feature::skill::SkillFeatureSet));
registry.register_feature_set(Box::new(crate::infra::feature::stub::StubFeatureSet::new(
    "mcp", "MCP Servers", "",
)));
registry.register_feature_set(Box::new(
    crate::infra::feature::instruction::InstructionFeatureSet,
));
registry.register_feature_set(Box::new(crate::infra::feature::stub::StubFeatureSet::new(
    "provider", "Providers", "",
)));
registry.register_feature_set(Box::new(crate::infra::feature::stub::StubFeatureSet::new(
    "profile", "Profiles", "",
)));
registry.register_feature_set(Box::new(crate::infra::feature::stub::StubFeatureSet::new(
    "vault", "Vaults", "",
)));
```

**Finding:** Only `SkillFeatureSet` and `InstructionFeatureSet` are real scanners. `mcp`, `provider`, `profile`, and `vault` are all `StubFeatureSet` placeholders.

### 2.2 StubFeatureSet Never Scans

`src/infra/feature/stub.rs`:

```rust
impl FeatureSetPort for StubFeatureSet {
    fn asset_kind(&self) -> AssetKind {
        AssetKind::Instruction  // ← BUG: hardcoded, ignores actual kind
    }
    fn is_package(&self, _: &Path) -> bool {
        false  // ← Never discovers anything
    }
    fn hash_files(&self, _: &Path) -> Vec<PathBuf> {
        vec![]  // ← Empty hash
    }
    fn is_stub(&self) -> bool {
        true  // ← scan() skips this entirely
    }
}
```

### 2.3 Scan Loop Skips Stubs

`src/app/bootstrap/scan.rs`:

```rust
pub fn scan(registry: &Registry, vaults: &[Box<dyn VaultPort>]) -> Result<ScanResult> {
    let mut packages_by_tab = Vec::new();
    for feature in &registry.feature_sets {
        let mut tab_packages = Vec::new();
        if !feature.is_stub() {  // ← STUBS ARE SKIPPED
            for vault in vaults {
                match vault.list_packages(feature.as_ref()) { ... }
            }
        }
        packages_by_tab.push(tab_packages);
    }
    Ok(ScanResult { packages_by_tab })
}
```

### 2.4 filter_scan Hardcodes McpServer as Uninstalled

`src/app/bootstrap/scan.rs::filter_scan`:

```rust
let is_global = match pkg.kind {
    AssetKind::Skill => { global_config.is_skill_installed(...) }
    AssetKind::Instruction => { global_config.is_instruction_installed(...) }
    AssetKind::McpServer => false,  // ← ALWAYS FALSE
};
```

This means even if an MCP server *were* discovered from a vault, the filter logic would treat it as "not installed" and potentially drop it from the results.

### 2.5 AssetKind Already Supports McpServer

`src/domain/asset.rs`:

```rust
pub enum AssetKind {
    Skill,
    Instruction,
    McpServer,  // ← Exists but never scanned from vaults
}
```

**Paradox:** The domain model acknowledges `McpServer` as an asset kind, but the infrastructure layer refuses to discover it from vaults. MCP servers are registered via a separate global registry (`~/.config/agk/mcp.toml`) with a completely different code path (`app/features/mcp/register.rs`, `cli/features/mcp.rs`).

### 2.6 Profile Has No AssetKind

Profiles are not even part of the `AssetKind` enum. They live entirely outside the asset scanning/discovery pipeline:
- Stored inline in `ConfigFile.profiles`
- Created via interactive wizard (`tui/features/profile/controller.rs`)
- No `ScannedPackage` representation
- No SHA10 tracking (no change detection)

---

## 3. The User Impact

### Scenario: Team Onboarding
> A platform team maintains a vault `github.com/acme-org/ai-workflows` with the team's standard AI environment. New developer Alice joins.

**Current experience:**
1. Alice attaches the vault → sees skills and instructions
2. Alice installs skills with `Space` ✓
3. Alice must manually register 5 MCP servers one-by-one with `F2` → fill form → confirm security warning × 5
4. Alice must create a profile via interactive wizard (`F2` in Profiles tab) → answer Q&A → select skills → select MCPs
5. Alice finally has a working environment. **Time: ~10 minutes.**

**Desired experience (with this proposal):**
1. Alice attaches the vault → sees **Skills, Instructions, MCP Servers, and Profiles** all listed
2. Alice installs the team's profile with `Space` → it auto-installs all bundled skills, instructions, and MCP servers
3. Alice starts the profile with `agk p web-app-team` → done.
4. **Time: ~30 seconds.**

### Scenario: MCP Server Versioning
> The team updates their `filesystem` MCP server from `v1.0.0` to `v1.1.0` with new args.

**Current experience:**
- No detection. Alice's machine still runs the old v1.0.0 she registered months ago.
- Team must send Slack message: "Hey everyone, delete your filesystem MCP and re-register it with these new args."

**Desired experience:**
- Vault contains `mcps/filesystem/MCP.md` with updated command/args.
- `agk sync` or `F5` detects SHA10 change → shows `[Update Available]` badge.
- Alice presses `Enter` → MCP server definition updates globally.

---

## 4. Target Architecture

### 4.1 New Asset Types in Domain

Extend `AssetKind` to include profile and future harness types:

```rust
pub enum AssetKind {
    Skill,
    Instruction,
    McpServer,
    Profile,      // ← NEW
    // Harness,   // ← Future (from research-vibecode-agk-report.md)
    // Agent,     // ← Future
}
```

### 4.2 New Feature Set Scanners

| Scanner | Scan Root | Package Marker | File | AssetKind |
|---------|-----------|----------------|------|-----------|
| `SkillFeatureSet` | `skills/` | `SKILL.md` | `SKILL.md` | `Skill` |
| `InstructionFeatureSet` | `instructions/` | `AGENTS.md` | `AGENTS.md` | `Instruction` |
| **`McpFeatureSet`** | `mcps/` | `MCP.md` | `MCP.md` | `McpServer` |
| **`ProfileFeatureSet`** | `profiles/` | `PROFILE.md` | `PROFILE.md` | `Profile` |

### 4.3 Vault Structure (Enhanced)

```
my-vault/
├── skills/
│   └── my-skill/
│       └── SKILL.md
├── instructions/
│   └── my-instruction/
│       └── AGENTS.md
├── mcps/                    ← NEW
│   └── filesystem/
│       └── MCP.md           # YAML frontmatter + markdown body
├── profiles/                ← NEW
│   └── web-app-team/
│       └── PROFILE.md       # YAML frontmatter + markdown body
└── README.md
```

### 4.4 MCP.md Format

```markdown
---
name: filesystem
version: 1.0.0
command: npx
args:
  - "-y"
  - "@modelcontextprotocol/server-filesystem"
  - "."
transport: stdio
description: File system access via MCP
---

# Filesystem MCP Server

Provides read/write access to the local filesystem through the Model Context Protocol.
```

### 4.5 PROFILE.md Format

```markdown
---
name: web-app-team
version: 1.2.0
provider: opencode
description: Full-stack web development profile
skills:
  - acme-org/react-skills
  - acme-org/typescript-linter
instructions:
  - acme-org/web-app-guidelines
mcps:
  - filesystem
  - github-api
---

# Web App Team Profile

Pre-configured agent profile for the frontend platform team.
```

---

## 5. Detailed Design Decisions

### 5.1 McpFeatureSet

```rust
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

**Key question:** MCP servers currently live in `~/.config/agk/mcp.toml` (global registry). If we discover them from vaults:
- Option A: Auto-register them into `mcp.toml` when the vault is attached (like skills are "installed" to providers).
- Option B: Treat vault MCPs as "read-only references" — they appear in the TUI but aren't copied to the global registry until explicitly installed.
- **Recommended:** Option B (parallel to skills). The vault MCP appears in the MCP tab with `[ ]`. Pressing `Space` registers it into `~/.config/agk/mcp.toml` and enables it for active providers.

### 5.2 ProfileFeatureSet

Profiles are more complex because they reference other assets (skills, instructions, MCPs) rather than being standalone installable artifacts.

```rust
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

**Profile installation behavior:**
1. Parse `PROFILE.md` frontmatter to get `skills`, `instructions`, `mcps` lists.
2. Install each referenced skill/instruction (existing asset install logic).
3. Register each referenced MCP server into `mcp.toml` if not already present.
4. Create the profile entry in config.
5. For OpenCode provider: run `opencode agent create` with the profile's description.

### 5.3 filter_scan Update

`filter_scan` must handle new `AssetKind` variants:

```rust
let is_global = match pkg.kind {
    AssetKind::Skill => { global_config.is_skill_installed(...) }
    AssetKind::Instruction => { global_config.is_instruction_installed(...) }
    AssetKind::McpServer => { global_config.is_mcp_installed(...) }  // ← NEW
    AssetKind::Profile => { global_config.is_profile_installed(...) }  // ← NEW
};
```

This requires adding `is_mcp_installed` and `is_profile_installed` to `ConfigFile`.

### 5.4 ConfigFile Schema Updates

`src/domain/config.rs` needs:

```rust
pub struct ConfigFile {
    // ... existing fields ...
    
    /// MCP servers installed from vaults (not the global registry)
    pub installed_mcps: Option<Vec<InstalledAsset>>,  // ← NEW
    
    /// Profiles installed from vaults
    pub installed_profiles: Option<Vec<InstalledAsset>>,  // ← NEW
}
```

**Note:** The existing `mcp.toml` is the **global registry** of *registered* MCP servers. `installed_mcps` in `config.toml` tracks which vault-sourced MCPs are "installed" in the current scope. This is analogous to how `installed_skills` tracks which vault skills are installed.

---

## 6. TUI Impact

### 6.1 Tab Restructure

Current tab order (from registry registration order):
1. Skills (Asset)
2. MCP Servers (Mcp) — currently stub, shows nothing from vaults
3. Instructions (Asset)
4. Providers (Provider)
5. Profiles (Profile) — currently stub, shows nothing from vaults
6. Vaults (Vault)

**After implementation:**
- Tab `[1]` Skills — real assets from vaults ✓
- Tab `[2]` MCP Servers — **real assets from vaults** ✓ (replaces stub)
- Tab `[3]` Instructions — real assets from vaults ✓
- Tab `[4]` Providers — stub (unchanged, providers are not vault assets)
- Tab `[5]` Profiles — **real assets from vaults** ✓ (replaces stub)
- Tab `[0]` Vaults — stub (unchanged)

### 6.2 MCP Tab Behavior Changes

Currently the MCP tab lists globally-registered MCP servers from `mcp.toml`. After this change:
- The MCP tab shows **both**:
  - Vault-discovered MCPs (not yet registered → `[ ]` or `[x]` if installed)
  - Globally registered MCPs from `mcp.toml` (existing behavior)
- Vault-discovered MCPs have a "Register" action (`Space`) that copies them to `mcp.toml`.
- Globally registered MCPs have "Enable/Disable" action (`Space`) for active providers.

**UI distinction needed:** Vault-sourced MCPs that are not yet registered should show `[⊘]` (not registered) vs `[ ]` (registered but disabled) vs `[x]` (registered and enabled).

### 6.3 Profile Tab Behavior Changes

Currently the Profile tab lists profiles from `ConfigFile.profiles` with `F2` to create new.
After this change:
- The Profile tab shows **both**:
  - Vault-discovered profiles (not yet installed → can be "installed")
  - Locally created/configured profiles (existing behavior)
- Vault profile installation is a batch operation: install all referenced assets + create the profile config.

---

## 7. CLI Impact

### 7.1 New Commands

```bash
# Install a vault-discovered MCP server
agk install <vault>/filesystem --kind mcp

# Install a vault-discovered profile (and all its referenced assets)
agk install <vault>/web-app-team --kind profile

# Validate MCP definitions in vaults
agk validate --kind mcp

# Pack a vault MCP for distribution
agk pack <vault>/filesystem --kind mcp
```

### 7.2 sync Command Enhancement

`agk sync` currently syncs skills and instructions. After this change:
- `agk sync` also syncs MCP server definitions (registers new ones, updates changed ones).
- `agk sync` also syncs profile definitions (installs new ones, updates changed ones).

---

## 8. Multi-Provider Considerations

### MCP Server Activation

MCP servers discovered from vaults need provider-specific activation:
- When a vault MCP is "installed" (registered into `mcp.toml`), it's **not yet enabled** for any provider.
- The existing `Space` toggle in the MCP tab enables/disables per provider.
- Vault-sourced MCPs should behave identically to manually-registered MCPs once in the registry.

### Profile Provider Targeting

`PROFILE.md` frontmatter specifies a `provider` (e.g., `provider: opencode`).
- A profile can only be installed if its target provider is active.
- Future: support multiple providers per profile (`providers: [opencode, claude-code]`).

---

## 9. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| **MCP registry collision** | Vault MCP with same name as manually-registered MCP | Hash-based deduplication; vault-sourced MCPs get `vault_id/` prefix in registry name or a warning modal |
| **Profile dependency resolution** | Profile references skill/MCP that doesn't exist in active vaults | Validate during install; emit clear error listing missing references |
| **Config schema migration** | Adding `installed_mcps`/`installed_profiles` to `ConfigFile` | serde `default` + backward-compatible parsing; empty = none installed |
| **TUI tab overload** | 6 tabs is already dense; adding more content makes it busier | Keep tabs as-is; the change is "tabs now have real content" not "new tabs added" |
| **Security** | Vault-sourced MCPs execute arbitrary commands | Same security model as manually-registered MCPs: security warning on install, user must confirm |
| **Performance** | Scanning `mcps/` and `profiles/` adds I/O to vault refresh | Negligible — same `std::fs::read_dir` pattern as skills/instructions; all scanned in parallel |

---

## 10. Implementation Roadmap

### Phase 1: MCP Vault Scanning (P8.1)
1. Add `McpFeatureSet` to `src/infra/feature/mcp.rs`
2. Add `MCP.md` frontmatter parsing to `src/infra/feature/mod.rs`
3. Update `filter_scan` to handle `AssetKind::McpServer`
4. Add `installed_mcps` to `ConfigFile`
5. Update TUI MCP tab to show vault-discovered MCPs alongside registered ones
6. Update `agk sync` to sync MCP definitions
7. Tests: `McpFeatureSet` scans `mcps/`, `filter_scan` retains installed MCPs, `agk sync` registers new MCPs

### Phase 2: Profile Vault Scanning (P8.2)
1. Add `Profile` to `AssetKind` enum
2. Add `ProfileFeatureSet` to `src/infra/feature/profile.rs`
3. Add `PROFILE.md` frontmatter parsing
4. Update `filter_scan` to handle `AssetKind::Profile`
5. Add `installed_profiles` to `ConfigFile`
6. Implement profile installation (batch install of referenced assets + profile creation)
7. Update TUI Profile tab to show vault-discovered profiles
8. Update `agk sync` to sync profiles

### Phase 3: Unified Team Environment (P8.3)
1. `agk team init` generates a vault with skills, instructions, MCPs, and profiles
2. New hire runs `agk sync` → entire environment configured
3. Telemetry: track which assets from team vaults are most used

---

## 11. Success Criteria

- [ ] `mcps/` directory in vault is scanned and MCP definitions appear in TUI MCP tab
- [ ] `profiles/` directory in vault is scanned and profiles appear in TUI Profile tab
- [ ] Installing a vault profile installs all referenced skills, instructions, and MCPs
- [ ] `agk sync` detects changes to vault MCPs and profiles via SHA10
- [ ] `agk validate` checks MCP and profile definitions in vaults
- [ ] `StubFeatureSet("mcp")` and `StubFeatureSet("profile")` are deleted from bootstrap
- [ ] `filter_scan` handles `AssetKind::McpServer` and `AssetKind::Profile` correctly
- [ ] No regression: manually-registered MCPs continue to work exactly as before
- [ ] Architecture tests pass: `domain/` remains pure, `infra/feature/` owns scanning

---

## 12. Why This Matters for Higher-Level Features

This enhancement is the **prerequisite backbone** for:

| Higher-Level Feature | Dependency on This Proposal |
|---------------------|----------------------------|
| **P7: Enterprise Policy & Compliance** | Policy engine needs to inspect ALL asset types in vaults, not just skills |
| **P7: Team Config Synchronization** | `.agk/team.toml` must reference MCPs and profiles, which must be discoverable |
| **Harness Orchestrator** | `HARNESS.md`, `AGENT.md`, `PROCESS.md` need their own `FeatureSet` scanners |
| **Intent-Based Skill Activation** | Trigger manifests (`TRIGGERS.md`) need vault scanning |
| **Drift Detection** | Process plans stored in vaults need SHA10 tracking |

**Without vault-discoverable MCPs and profiles, AGK remains a "skill manager." With them, AGK becomes a "complete AI environment manager."**

---

*Research completed 2026-05-30. Based on analysis of AGK commit `4088606` (master).*
