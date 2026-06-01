# Vault Multi-Asset Scanning Feature – Product Requirements

**Status:** Implemented (v0.3)
**Epic:** [v0.3 Team-Ready Profiles](../../../epics/v03-team-ready-profiles.md)
**Related:**
- [MCP Vault PRD](../mcp-vault/prd.md) (MCP-specific behavior)
- [Profiles PRD](../profiles/prd.md) (profile-specific behavior)
- Source Research: ~~Vault Multi-Asset Scanning Proposal~~ (implemented — removed from `docs/proposals/`)

---

## Overview

AGK's vault scanning system currently discovers only two asset types: **Skills** (`skills/*/SKILL.md`) and **Instructions** (`instructions/*/AGENTS.md`). MCP servers, profiles, and any future asset types are registered as `StubFeatureSet` placeholders that never scan. This means teams cannot distribute complete AI environment blueprints through their vaults.

**v0.3 makes vaults the single source of truth for ALL team AI assets** by extending the `FeatureSetPort` pattern to support MCP servers and profiles as first-class vault-discoverable assets.

---

## User-Facing Behavior

### Scenario 1: Team Onboarding (The "30-Second Onboarding")

> A platform team maintains a vault `github.com/acme-org/ai-workflows` with the team's standard AI environment. New developer Alice joins.

**Before v0.3:**
1. Alice attaches the vault → sees skills and instructions.
2. Alice installs skills with `Space`.
3. Alice must manually register 5 MCP servers one-by-one with `F2` → fill form → confirm security warning × 5.
4. Alice must create a profile via interactive wizard → answer Q&A → select skills → select MCPs.
5. **Time: ~10 minutes.**

**With v0.3:**
1. Alice attaches the vault → sees **Skills, Instructions, MCP Servers, and Profiles** all listed.
2. Alice navigates to the Profiles tab, sees `web-app-team` profile with `[Vault]` badge.
3. Alice presses `Space` → profile installs, auto-installing all bundled skills, instructions, and MCP servers.
4. Alice presses `Enter` on the installed profile → `agk p start web-app-team` → done.
5. **Time: ~30 seconds.**

### Scenario 2: MCP Server Versioning

> The team updates their `filesystem` MCP server from `v1.0.0` to `v1.1.0` with new args.

**Before v0.3:**
- No detection. Alice's machine still runs the old v1.0.0 she registered months ago.
- Team must send Slack message: "Hey everyone, delete your filesystem MCP and re-register it."

**With v0.3:**
- Vault contains `mcps/filesystem/MCP.md` with updated command/args.
- `agk sync` or `F5` detects SHA10 change → shows `[Update Available]` badge.
- Alice presses `Enter` → MCP server definition updates globally.

---

## Functional Requirements

### 1. New Asset Types in Domain

- Extend `AssetKind` to include `Profile`:
  ```rust
  pub enum AssetKind {
      Skill,
      Instruction,
      McpServer,
      Profile,      // ← NEW in v0.3
  }
  ```

### 2. New Feature Set Scanners

| Scanner | Scan Root | Package Marker | File | AssetKind |
|---------|-----------|----------------|------|-----------|
| `SkillFeatureSet` | `skills/` | `SKILL.md` | `SKILL.md` | `Skill` |
| `InstructionFeatureSet` | `instructions/` | `AGENTS.md` | `AGENTS.md` | `Instruction` |
| **`McpFeatureSet`** | `mcps/` | `MCP.md` | `MCP.md` | `McpServer` |
| **`ProfileFeatureSet`** | `profiles/` | `PROFILE.md` | `PROFILE.md` | `Profile` |

### 3. Vault Structure (Enhanced)

```
my-vault/
├── skills/
│   └── my-skill/
│       └── SKILL.md
├── instructions/
│   └── my-instruction/
│       └── AGENTS.md
├── mcps/                    ← NEW in v0.3
│   └── filesystem/
│       └── MCP.md
├── profiles/                ← NEW in v0.3
│   └── web-app-team/
│       └── PROFILE.md
└── README.md
```

### 4. MCP.md Format

See [MCP Vault PRD](../mcp-vault/prd.md) §MCP.md Format.

### 5. PROFILE.md Format

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

**Note:** Skills, instructions, and MCPs referenced in `PROFILE.md` are identified by their **vault-relative identity** (not global name). During installation, AGK resolves these identities against the same vault.

### 6. Profile Installation Behavior

Installing a vault-discovered profile is a **batch operation**:

1. Parse `PROFILE.md` frontmatter to get `skills`, `instructions`, `mcps` lists.
2. For each referenced skill/instruction:
   - If not installed in current scope, install from the same vault (existing asset install logic).
3. For each referenced MCP:
   - If not registered in global registry, copy `MCP.md` definition and run test handshake.
4. Create the profile entry in `config.toml` with vault-aware `ProfileAssetRef` entries.
5. For OpenCode provider: compose structured description and write `.agk/profiles/<name>/agent.md`.

**Atomicity:** If any referenced asset fails to install, the entire profile installation fails with a clear error listing which assets are missing and why.

### 7. filter_scan Update

`filter_scan` must handle new `AssetKind` variants:

```rust
let is_global = match pkg.kind {
    AssetKind::Skill => { global_config.is_skill_installed(...) }
    AssetKind::Instruction => { global_config.is_instruction_installed(...) }
    AssetKind::McpServer => { global_config.is_mcp_registered(...) }  // ← NEW
    AssetKind::Profile => { global_config.is_profile_installed(...) } // ← NEW
};
```

### 8. ConfigFile Schema Updates

```rust
pub struct ConfigFile {
    // ... existing fields ...

    /// MCP servers installed from vaults (tracked separately from global registry)
    #[serde(default)]
    pub installed_mcps: Vec<InstalledAsset>,  // ← NEW

    /// Profiles installed from vaults
    #[serde(default)]
    pub installed_profiles: Vec<InstalledAsset>,  // ← NEW
}
```

**Note:** The existing `mcp.toml` is the **global registry** of *registered* MCP servers. `installed_mcps` in `config.toml` tracks which vault-sourced MCPs are "installed" in the current scope (analogous to `installed_skills`).

---

## TUI Impact

### Tab Restructure

Current tab order:
1. Skills (Asset) ✅
2. MCP Servers (Mcp) — currently stub, shows nothing from vaults
3. Instructions (Asset) ✅
4. Providers (Provider)
5. Profiles (Profile) — currently stub, shows nothing from vaults
6. Vaults (Vault)

**After v0.3:**
- Tab `[1]` Skills — real assets from vaults ✅
- Tab `[2]` MCP Servers — **real assets from vaults** ✅ (replaces stub)
- Tab `[3]` Instructions — real assets from vaults ✅
- Tab `[4]` Providers — unchanged
- Tab `[5]` Profiles — **real assets from vaults** ✅ (replaces stub)
- Tab `[0]` Vaults — unchanged

### MCP Tab Behavior

- Shows **both** globally-registered MCPs and vault-discovered MCPs.
- Vault-discovered MCPs have `[⊘]` (not registered) vs `[ ]` (registered but disabled) vs `[x]` (registered and enabled).
- See [MCP Vault PRD](../mcp-vault/prd.md) for full MCP tab behavior.

### Profile Tab Behavior

- Shows **both** locally created profiles and vault-discovered profiles.
- Vault profiles have `[Vault]` badge and `[ ]` checkbox.
- `Space` on a vault profile triggers batch installation.
- After installation, the profile behaves identically to a locally created profile.

---

## CLI Impact

### New Commands

```bash
# Install a vault-discovered MCP server
agk install <vault>/filesystem --kind mcp

# Install a vault-discovered profile (and all its referenced assets)
agk install <vault>/web-app-team --kind profile

# Validate MCP and profile definitions in vaults
agk validate --kind mcp
agk validate --kind profile

# Pack a vault MCP for distribution
agk pack <vault>/filesystem --kind mcp
```

### sync Command Enhancement

`agk sync` currently syncs skills and instructions. After v0.3:
- `agk sync` also syncs MCP server definitions (registers new ones, updates changed ones).
- `agk sync` also syncs profile definitions (installs new ones, updates changed ones).

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| **MCP registry collision** | Vault MCP with same name as manually-registered MCP | Hash-based deduplication; vault-sourced MCPs get `vault_id/` prefix or warning modal |
| **Profile dependency resolution** | Profile references skill/MCP that doesn't exist in active vaults | Validate during install; emit clear error listing missing references |
| **Config schema migration** | Adding `installed_mcps`/`installed_profiles` to `ConfigFile` | `serde(default)` + backward-compatible parsing; empty = none installed |
| **TUI tab overload** | 6 tabs is already dense; adding more content makes it busier | Keep tabs as-is; the change is "tabs now have real content" not "new tabs added" |
| **Security** | Vault-sourced MCPs execute arbitrary commands | Same security model as manually-registered MCPs: security warning on install, user must confirm |
| **Performance** | Scanning `mcps/` and `profiles/` adds I/O to vault refresh | Negligible — same `std::fs::read_dir` pattern as skills/instructions; all scanned in parallel |

---

## Success Criteria

- [ ] `mcps/` directory in vault is scanned and MCP definitions appear in TUI MCP tab.
- [ ] `profiles/` directory in vault is scanned and profiles appear in TUI Profile tab.
- [ ] Installing a vault profile installs all referenced skills, instructions, and MCPs.
- [ ] `agk sync` detects changes to vault MCPs and profiles via SHA10.
- [ ] `agk validate` checks MCP and profile definitions in vaults.
- [ ] `StubFeatureSet("mcp")` and `StubFeatureSet("profile")` are deleted from bootstrap.
- [ ] `filter_scan` handles `AssetKind::McpServer` and `AssetKind::Profile` correctly.
- [ ] No regression: manually-registered MCPs and locally-created profiles continue to work.
- [ ] Architecture tests pass: `domain/` remains pure, `infra/feature/` owns scanning.

---

## Why This Matters for Higher-Level Features

This enhancement is the **prerequisite backbone** for:

| Higher-Level Feature | Dependency on This Feature |
|---------------------|----------------------------|
| **P7: Enterprise Policy & Compliance** | Policy engine needs to inspect ALL asset types in vaults |
| **P7: Team Config Synchronization** | `.agk/team.toml` must reference MCPs and profiles, which must be discoverable |
| **Harness Orchestrator** | `HARNESS.md`, `AGENT.md`, `PROCESS.md` need their own `FeatureSet` scanners |
| **Intent-Based Skill Activation** | Trigger manifests (`TRIGGERS.md`) need vault scanning |
| **Drift Detection** | Process plans stored in vaults need SHA10 tracking |

**Without vault-discoverable MCPs and profiles, AGK remains a "skill manager." With them, AGK becomes a "complete AI environment manager."**

---

*PRD v0.1 — 2026-05-30*
