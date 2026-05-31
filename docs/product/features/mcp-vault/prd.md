# PRD: MCP Vault Management (v0.3)

**Status:** Draft — v0.3 update  
**Previous:** [v0.2 PRD](https://github.com/dkthezero/agk/blob/4088606/docs/product/features/mcp-vault/prd.md)  
**Epic:** [v0.3 Team-Ready Profiles](../../../epics/v03-team-ready-profiles.md)

---

## Overview

MCP servers are runtime tools that AI agents invoke via the Model Context Protocol. In v0.2, AGK introduced a global MCP registry (`~/.config/agk/mcp.toml`) with manual registration, testing, and per-provider activation. **v0.3 extends this to make MCP servers vault-discoverable assets** — teams can distribute MCP definitions through their vaults alongside skills and instructions.

Additionally, v0.3 **expands MCP provider coverage from 2 to 5 providers** by adding GitHub Copilot CLI, Gemini CLI, and AMP adapters.

---

## User-Facing Behavior

### TUI Tab [2] MCP Servers

- **Tab position:** `[2]` (between Skills `[1]` and Instructions `[3]`).
- **Content:** Two sources displayed together:
  1. **Globally registered MCPs** from `~/.config/agk/mcp.toml` (existing behavior).
  2. **Vault-discovered MCPs** from attached vaults (`mcps/*/MCP.md`) (new in v0.3).

#### Visual Indicators

| State | Badge | Meaning |
|-------|-------|---------|
| `[⊘]` | Not registered | Vault-discovered but not yet copied to global registry |
| `[ ]` | Registered, disabled | In global registry but not enabled for any provider |
| `[x]` | Registered, enabled | In global registry and enabled for active providers in current scope |
| `[✓]` | Tested | Registry entry has passed the MCP `initialize` handshake |

#### Actions

- **On a vault-discovered MCP (`[⊘]`):**
  - `Space` — Registers it into `~/.config/agk/mcp.toml` (copies definition from vault) and runs the test handshake. Now shows `[ ]` or `[✓]`.
- **On a registered MCP (`[ ]` / `[x]`):**
  - `Space` — Toggles enable/disable for all active MCP-capable providers in the current scope (existing behavior).
- **F2** — Opens the registration modal for a *new* manually-defined MCP (existing behavior).
- **Enter** — Opens detail view showing command, args, transport, description, and vault source (if vault-sourced).

### Vault-Discovered MCP Lifecycle

1. **Scan:** When a vault is attached or refreshed, AGK scans `mcps/` for directories containing `MCP.md`.
2. **Display:** Vault-discovered MCPs appear in the MCP tab with `[⊘]` badge.
3. **Register:** User presses `Space` (or `agk install vault/mcp-name --kind mcp`). AGK:
   - Copies the `MCP.md` definition into `~/.config/agk/mcp.toml`.
   - Runs the JSON-RPC `initialize` handshake test.
   - On success, marks `[✓]`.
4. **Enable:** Once registered, the MCP behaves identically to manually-registered MCPs. `Space` toggles per-provider.
5. **Update:** If the vault `MCP.md` changes SHA, `agk sync` or TUI refresh shows `[Update Available]`. Pressing `Enter` updates the global registry entry.

---

### CLI Commands (updated for v0.3)

```bash
# Register a vault-discovered MCP
agk install clawhub/filesystem --kind mcp

# Register with explicit vault (same as above)
agk mcp add --name filesystem --from-vault clawhub

# Enable/disable (unchanged from v0.2)
agk mcp enable filesystem --provider claude-code --scope workspace
agk mcp disable filesystem --provider opencode --scope global

# List (now includes vault-discovered MCPs)
agk mcp list --json
# Output includes: name, command, tested, enabled_providers, vault_source
```

---

## Functional Requirements

### v0.3 New Requirements

1. **Vault-discoverable MCPs:** `mcps/` directories in attached vaults shall be scanned for `MCP.md` files. Discovered MCPs shall appear in the TUI MCP tab and CLI list output.
2. **MCP registration from vault:** Pressing `Space` on a vault-discovered MCP shall copy its definition to the global registry and run the test handshake.
3. **SHA-based change detection:** Vault-discovered MCPs shall participate in the same SHA10 change-detection pipeline as skills and instructions. Stale MCPs show `[Update Available]`.
4. **MCP provider expansion:** GitHub Copilot CLI, Gemini CLI, and AMP shall gain `McpProvider` implementations, raising coverage from 2 to 5 providers.
5. **Provider exclusion:** Letta, Snowflake Cortex, and Firebender shall explicitly return `supports_mcp: false` and be excluded from MCP operations without errors.
6. **Config preservation:** All new provider `McpProvider` implementations shall preserve existing JSON config content (non-destructive merge).
7. **Scope behavior:** Copilot CLI, Gemini CLI, and AMP support Global scope only for MCP (no documented workspace-level config). If a user attempts workspace-scoped enable, emit a clear warning and fall back to global.

### Retained v0.2 Requirements

- `AssetKind::McpServer` exists in the domain model.
- Registration collects name, command, args, transport, description.
- Test phase performs an MCP `initialize` handshake automatically after registration.
- Registry is stored in `~/.config/agk/mcp.toml`.
- `Space` in Tab `[2]` toggles activation for active MCP-capable providers in current scope.
- Security warning shown before executing unknown MCP commands.
- `--json` support for `agk mcp list`.

---

## MCP.md Format (Vault Asset)

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

---

## Provider-Specific MCP Config

| Provider | Config Path | Schema | v0.3 Status |
|----------|-------------|--------|-------------|
| Claude Code | `.claude/mcp.json` | `mcpServers: { name: { command, args, env } }` | ✅ Existing |
| OpenCode | `~/.config/opencode/opencode.json` | Flat `mcp.<name>: { type, enabled, command, args }` | ✅ Existing |
| **GitHub Copilot CLI** | `~/.copilot/mcp-config.json` | `mcpServers: { name: { type, command, args, env, tools } }` | 🆕 NEW |
| **Gemini CLI** | `~/.gemini/settings.json` | `mcpServers: { name: { command, args, env, trust, includeTools } }` | 🆕 NEW |
| **AMP** | `.amp/settings.json` or `~/.config/amp/settings.json` | `amp.mcpServers` nested under settings | 🆕 NEW |
| Letta | N/A | Proprietary `.skills/` directory | ❌ Unsupported |
| Snowflake Cortex | N/A | `CREATE MCP SERVER ...` via SQL | ❌ Unsupported |
| Firebender | N/A | No discoverable documentation | ❌ Unsupported |

---

## Non-Goals

- Hosting an MCP server. AGK registers and configures them; the provider process owns execution.
- Real-time MCP health monitoring after activation. AGK tests at registration time; ongoing health is the provider's responsibility.
- Cross-machine MCP registry sync. (Fast-follow: export/import.)
- Workspace-level MCP config for Copilot, Gemini, or AMP (not documented by providers).

## Security Considerations

- [x] Arbitrary code execution warning before registering.
- [x] Vault-sourced MCPs execute the same command as manually-registered ones; same security model applies.
- [ ] `~/.config/agk/mcp.toml` file permissions `0600` (pending hardening).

---

## v0.3.1: MCP Security Scorecard

### Problem
All MCP servers currently receive the same generic security warning: "This will execute '{command} {args}' on your machine." There is no differentiation between a read-only MCP and one that requests broad filesystem access or network egress.

### Solution
AGK analyzes the MCP command + args at registration time and assigns **security flags** based on heuristic patterns.

#### Flag Definitions

| Flag | Trigger Pattern | Severity | UI |
|------|----------------|----------|-----|
| `broad-filesystem` | Args contain `/`, `~`, or `.` (root or cwd access) | High | `[!]` badge |
| `network-egress` | Command/args contain `http`, `curl`, `wget`, `fetch` | High | `[!]` badge |
| `arbitrary-execution` | Command is `bash`, `sh`, `python` with unverified script | Critical | `[‼]` badge |
| `env-exfiltration` | Args reference env vars (`$HOME`, `$SSH_KEY`, `$TOKEN`) | Medium | `[!]` badge |
| `unspecified-args` | No args provided (command may default to dangerous behavior) | Low | `[i]` badge |

### TUI Behavior
- **MCP list:** High/Critical severity MCPs show `[!]` or `[‼]` next to their name.
- **Detail view (`Enter`):** Security flags are listed in a dedicated "Security Assessment" section with explanations.
- **Registration modal:** If security flags are detected, an additional warning line appears: "⚠️ This MCP requests broad filesystem access. Only enable if you trust the source."

### CLI Behavior
- `agk mcp list --json` includes `security_flags: ["broad-filesystem", "network-egress"]`.
- `agk mcp add` warns about detected flags before the standard security confirmation.

### Heuristic Implementation
```rust
fn assess_mcp_security(command: &str, args: &[Str]) -> Vec<SecurityFlag> {
    let mut flags = Vec::new();
    let full = format!("{} {}", command, args.join(" "));

    if args.iter().any(|a| a == "/" || a == "~" || a == ".") {
        flags.push(SecurityFlag::BroadFilesystem);
    }
    if full.contains("http") || full.contains("curl") || full.contains("wget") {
        flags.push(SecurityFlag::NetworkEgress);
    }
    if (command == "bash" || command == "sh" || command == "python")
        && args.iter().any(|a| a.ends_with(".sh") || a.ends_with(".py")) {
        flags.push(SecurityFlag::ArbitraryExecution);
    }
    if full.contains("$") {
        flags.push(SecurityFlag::EnvExfiltration);
    }
    if args.is_empty() {
        flags.push(SecurityFlag::UnspecifiedArgs);
    }
    flags
}
```

**Important:** Heuristics are **advisory only**. They never block installation. The user always has the final say.

## Acceptance Criteria

- [x] `AssetKind::McpServer` exists in the domain model.
- [x] TUI tab `[2]` lists globally registered + vault-discovered MCPs with correct badges.
- [x] Vault-discovered MCP can be registered into global registry with `Space`.
- [x] `agk sync` detects SHA10 changes to vault MCPs and shows `[Update Available]`.
- [x] Copilot CLI, Gemini CLI, and AMP support `agk mcp add` / `enable` / `disable`.
- [x] Non-MCP-capable providers (Letta, Snowflake, Firebender) excluded without errors.
- [x] TUI Providers tab shows MCP checkbox `[✓]` only for capable providers.
- [x] All new provider configs preserve existing JSON content.
- [x] No regression: manually-registered MCPs continue to work exactly as before.
- [x] v0.3.1: MCP security flags appear in `agk mcp list --json`.
- [x] v0.3.1: TUI MCP tab shows `[!]` badge for high-risk MCPs.
- [x] Architecture tests pass with zero allowlists.

---

*PRD v0.3 — updated 2026-05-30*
*Security Scorecard section added for v0.3.1 — 2026-05-30*
