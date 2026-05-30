# PRD: Providers Management (v0.3)

**Status:** Draft — v0.3 update  
**Previous:** [v0.2 PRD](https://github.com/dkthezero/agk/blob/4088606/docs/product/features/providers/prd.md)  
**Epic:** [v0.3 Team-Ready Profiles](../../../epics/v03-team-ready-profiles.md)

---

## Overview

Providers translate AGK-managed logical assets (Skills, Instructions, MCP servers, and now Profiles) into target AI platforms. In v0.3, the Provider layer gains two major capabilities:

1. **MCP Provider Expansion:** MCP server support grows from 2 to 5 providers by adding GitHub Copilot CLI, Gemini CLI, and AMP adapters.
2. **Profile Tool/Permission Selection:** Providers can advertise configurable tools and permission modes, which the profile wizard surfaces as checklist/select steps.

---

## Supported Providers

| Provider | Skills | Instructions | MCP (v0.2) | MCP (v0.3) | Profiles | Tool/Permission Config |
|----------|--------|--------------|------------|------------|----------|----------------------|
| Claude Code | ✅ | ✅ | ✅ | ✅ | ✅ (NEW) | ✅ (NEW) |
| OpenCode | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ (NEW) |
| GitHub Copilot | ✅ | ✅ | ❌ | ✅ (NEW) | ❌ | ❌ |
| Gemini CLI | ✅ | ✅ | ❌ | ✅ (NEW) | ❌ | ❌ |
| AMP | ✅ | ✅ | ❌ | ✅ (NEW) | ❌ | ❌ |
| Letta | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Snowflake Cortex | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Firebender | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |

---

## Functional Requirements

### v0.3 New Requirements

#### 1. MCP Provider Expansion

**GitHub Copilot CLI:**
- Config path: `~/.copilot/mcp-config.json` (Global scope only).
- Schema: Top-level `mcpServers` object with `type`, `command`, `args`, `env`, `tools`.
- Behavior: Similar to Claude Code but targeting `~/.copilot/` directory.

**Gemini CLI:**
- Config path: `~/.gemini/settings.json` (Global scope only).
- Schema: Top-level `mcpServers` object with `command`, `args`, `env`, `trust`, `includeTools`.
- Behavior: Read → merge into `mcpServers` → write back, preserving other settings.

**AMP:**
- Config path: `.amp/settings.json` (Workspace) or `~/.config/amp/settings.json` (Global).
- Schema: Nested under `amp.mcpServers` inside a larger settings file.
- Behavior: Preserve existing settings; only mutate the `amp.mcpServers` key.

**Unsupported Providers:**
- Letta: `supports_mcp() -> false` (proprietary `.skills/` directory, no local MCP config).
- Snowflake Cortex: `supports_mcp() -> false` (server-side SQL configuration).
- Firebender: `supports_mcp() -> false` (no discoverable MCP documentation).

#### 2. Profile Tool/Permission Selection

Add two optional trait methods to `ProviderPort`:

```rust
/// Return configurable tools/permissions this provider supports.
/// Each entry is (id, description, default_state).
fn available_profile_tools(&self) -> Vec<(String, String, bool)> {
    vec![] // default: no configurable tools
}

/// Return permission modes this provider supports.
fn available_permission_modes(&self) -> Vec<(String, String)> {
    vec![] // default: no configurable modes
}
```

**Claude Code implementation:**
- Tools: `Read`, `Glob`, `Grep`, `Bash`, `Write`, `Edit`, `LSP`
- Permission modes: `default`, `acceptEdits`, `auto`, `dontAsk`, `plan`
- Default tool state: all `true` (full access) unless template overrides.

**OpenCode implementation:**
- Tools: If per-agent tool config is exposed by OpenCode CLI, return it. Otherwise return empty (no wizard step injected).
- Permission modes: If OpenCode exposes permission modes, return them. Otherwise empty.

**Wizard Integration:**
- If `available_profile_tools()` returns non-empty, inject a **Checklist** step: "Select Tools / Permissions".
- If `available_permission_modes()` returns non-empty, inject a **Select** step: "Permission Mode".
- Store selections in `config.toml` under `profile.tools` and `profile.permission_mode`.

#### 3. Non-Destructive Config Writes

All provider implementations (existing + new) must preserve existing JSON/TOML config content when writing MCP or profile data. Only mutate the specific keys owned by AGK.

### Retained v0.2 Requirements

- Simultaneous active providers: Users can activate multiple providers at once.
- Scope targeting: Global vs Workspace overrides.
- Tab `[4]` UI: Boolean toggle with active markers and sync status.
- Provider-specific install paths for skills and instructions.

---

## CLI Impact

```bash
# Toggle provider (unchanged)
agk provider toggle claude-code on --scope workspace

# List providers with MCP support indicators (updated)
agk provider list --json
# Output now includes: supports_mcp, supports_profiles, available_tools, available_permission_modes
```

---

## UI/UX Specifications

### Providers Tab `[4]`

Each provider row now shows additional indicators:

```
[✓] Claude Code    [MCP: ✓] [Profiles: ✓] [Tools: 7]
[ ] GitHub Copilot  [MCP: ✓] [Profiles: ✗] [Tools: —]
[ ] Gemini CLI      [MCP: ✓] [Profiles: ✗] [Tools: —]
[ ] AMP             [MCP: ✓] [Profiles: ✗] [Tools: —]
[ ] Letta           [MCP: ✗] [Profiles: ✗] [Tools: —]
```

- `MCP: ✓` = provider has `McpProvider` implementation.
- `Profiles: ✓` = provider has `supports_profiles() == true`.
- `Tools: N` = provider returns N items from `available_profile_tools()`.

---

## Non-Goals

- Per-provider profile support for Copilot, Gemini, AMP (not supported by those platforms).
- Dynamic plugin loading for providers (compile-time trait implementations only).
- Provider-specific AI model selection in the wizard (out of scope; may be provider-specific step in future).

## Security Considerations

- MCP config files for new providers may contain sensitive env vars. All `McpProvider::write_mcp_server` implementations must preserve existing content and not log full config to stdout.

## Acceptance Criteria

- [ ] Copilot CLI `McpProvider` writes correct `~/.copilot/mcp-config.json` schema.
- [ ] Gemini CLI `McpProvider` writes correct `~/.gemini/settings.json` schema.
- [ ] AMP `McpProvider` writes correct `.amp/settings.json` schema, preserving other settings.
- [ ] Letta, Snowflake, Firebender return `supports_mcp: false` and are excluded from MCP operations.
- [ ] TUI Providers tab shows MCP checkbox `[✓]` only for capable providers.
- [ ] All new provider configs preserve existing JSON content (no destructive overwrites).
- [ ] Tests cover write/read roundtrips for all 3 new providers.
- [ ] `available_profile_tools()` returns correct lists for Claude Code and OpenCode.
- [ ] `available_permission_modes()` returns correct lists for Claude Code.
- [ ] Wizard injects tool/permission steps only when provider returns non-empty lists.
- [ ] Architecture tests pass with zero allowlists.

---

*PRD v0.3 — updated 2026-05-30*
