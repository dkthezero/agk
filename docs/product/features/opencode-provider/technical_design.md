# Technical Design: OpenCode Provider Support

## Overview

OpenCode is a file-based agent runtime with Claude Code compatibility. The adapter implements `ProviderPort` to copy skills to `.opencode/skills/`. OpenCode auto-discovers skills by scanning this directory; no `skills` configuration array is written.

## Architecture Rules

1. **ProviderPort trait only.** No new traits. Standard `install`/`remove` interface.
2. **JSON merge, not replace.** OpenCode's config system is hierarchical; we must preserve existing keys.
3. **Path isolation.** All OpenCode paths are resolved in one place (`OpenCodeProvider`) for testability.
4. **JSONC support.** Use a JSONC parser (or strip comments before parsing) to avoid corrupting user configs.

## Data Schemas

None required for skills. OpenCode auto-discovers skills from the `.opencode/skills/<name>/` directory layout.

For MCP, entries are written directly into `opencode.json` as raw `serde_json::Value` objects (see `McpProvider` trait).

## Internal Workflows

### Install Workflow
1. Determine scope (Global / Workspace).
2. Compute destination: `provider_root(scope)/skills/{name}/SKILL.md`.
3. Copy the skill directory to the destination using `infra::provider::common::copy_dir`.
4. **Do NOT modify** `opencode.json**. OpenCode discovers skills from the directory layout on startup.
5. **Self-heal:** If a stale `"skills"` array exists in `opencode.json` (from earlier agk versions), strip it. This allows users upgrading from buggy agk versions to get a clean config on their first install.

### Remove Workflow
1. Determine scope.
2. Delete the skill directory.
3. Strip any stale `"skills"` array in `opencode.json` because OpenCode rejects it.
4. If `opencode.json` is now an empty object `{}`, delete it.

### JSONC Handling
- Use `jsonc-parser` crate (or implement a simple comment stripper).
- If we can't preserve comments, document that `opencode.json` may be rewritten as plain JSON.

## Trait Contracts

Uses existing `ProviderPort` and `McpProvider`:
```rust
fn install(&self, pkg: &ScannedPackage, scope: Scope) -> Result<()>;
fn remove(&self, identity: &AssetIdentity, kind: &AssetKind, scope: Scope) -> Result<()>;

// MCP support (McpProvider trait)
fn write_mcp_server(&self, server: &McpServer, scope: Scope) -> Result<()>;
fn remove_mcp_server(&self, name: &str, scope: Scope) -> Result<()>;
```

**MCP Schema (OpenCode):**
- Writes flat `mcp.<name>` entries to `opencode.json` (not nested `mcp.servers`).
- Required fields: `type` ("local" or "remote"), `enabled` (boolean).
- Local: `{ "type": "local", "command": "...", "args": [...], "env": {...}, "enabled": true }`
- Remote (SSE): `{ "type": "remote", "url": "...", "enabled": true }`
- On remove: drops the server entry. If `mcp` becomes empty, drops the entire `mcp` key to avoid schema validation errors.
- **Migration:** On write, drops any stale `mcp.servers` key (from earlier schema iteration).

## Module Structure

```
src/infra/provider/
  opencode.rs     # OpenCodeProvider implementation
  mod.rs          # Add `pub(crate) mod opencode;`
```

## Testing Strategy

- **Unit tests:**
  - Install copies files to correct path.
  - Remove deletes files and updates config.
  - JSON merge preserves unrelated keys.
  - JSONC parsing doesn't panic on comments.
- **Integration:**
  - Register in bootstrap, verify TUI shows OpenCode.

---

*End of Technical Design.*
