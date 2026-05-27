# PRD: OpenCode Provider Support

> **Product Mindset:** `agk` is the agent kit for teams to share the way they work with AI agents together. OpenCode is a file-based, Claude-compatible agent runtime; supporting it extends agk's polyglot coverage with minimal adapter complexity.

---

## Overview

Add [OpenCode](https://github.com/anomalyco/opencode) as a first-class deployment target. OpenCode uses a hierarchical, merge-based configuration system and stores skills as markdown files with YAML frontmatter — making it structurally similar to Claude Code. This adapter follows the existing `ProviderPort` trait pattern and is implementable in 3–5 days.

---

## Research Summary

**Configuration Storage:**
- Global config: `~/.config/opencode/opencode.json`
- Project config: `opencode.json` in project root
- Skills directory: `.opencode/skills/<name>/SKILL.md`
- Format: JSON / JSONC (JSON with Comments)
- Merge semantics: Configuration files are **merged** (not replaced), allowing layered settings.
- Claude Code compatibility: OpenCode supports Claude Code paths for migration.

**MCP Support:**
- OpenCode reads MCP servers from a flat `mcp` key in `opencode.json`, not a nested `mcp.servers` key.
- Each server entry requires: `type` ("local" or "remote"), `enabled` (boolean).
- **Local:** `{ "type": "local", "command": "...", "args": [...], "env": {...}, "enabled": true }`
- **Remote (SSE):** `{ "type": "remote", "url": "...", "enabled": true }`
- When removing a server, the entire `mcp` key is dropped if empty to avoid schema validation errors.

**Skills Support:**
- Location: `.opencode/skills/<name>/SKILL.md`
- Format: Markdown with YAML frontmatter (fields: `name`, `description`)
- Search locations: project, global, and Claude-compatible paths

**Installation Mechanism:**
- File-based (not API-based).
- Skills loaded from filesystem directories.
- Supports `package.json` in `.opencode/` directory that triggers `bun install` for dependencies.

---

## Functional Requirements

### Provider Adapter
- [x] Create `infra/provider/opencode.rs` implementing `ProviderPort`.
- [x] **Global install path:** `~/.config/opencode/skills/<name>/SKILL.md`
- [x] **Workspace install path:** `.opencode/skills/<name>/SKILL.md`
- [x] **No skill config merge.** OpenCode auto-discovers skills from the `.opencode/skills/` directory. `agk` does **not** add a `skills` array to `opencode.json`.

### Config Merge Semantics
- [x] `opencode.json` is **never modified** for skill installation or removal.
- [x] On skill removal, any stale `"skills"` array (from earlier agk versions) is quietly stripped so OpenCode does not throw schema errors.
- [x] On skill install, any stale `"skills"` array is also stripped as a self-heal for users upgrading from buggy agk versions.

### Scope Targeting
- [x] **Global scope:** Installs skills to `~/.config/opencode/skills/`. `opencode.json` is modified only for MCP configuration.
- [x] **Workspace scope:** Installs skills to `.opencode/skills/`. `opencode.json` is modified only for MCP configuration (and to strip stale `skills` keys during self-heal).

### MCP Support
- [x] `McpProvider` trait implemented (`write_mcp_server`, `remove_mcp_server`).
- [x] Writes flat `mcp.<name> = { type, command, args, env, enabled }` schema (not nested `mcp.servers`).
- [x] Drops stale `mcp.servers` key on write (migration from earlier schema).
- [x] Drops empty `mcp` key on remove to avoid schema validation errors.

### TUI Integration
- [x] Add "OpenCode" to the Providers tab (Tab 3).
- [x] Space toggles active/inactive.
- [x] Active marker shows when synchronizing.

### Headless CLI Integration
- [x] `agk sync` installs to OpenCode if it is an active provider.
- [x] `agk install opencode/<skill>` or `agk install <skill> --provider opencode` targets OpenCode explicitly.

---

## User Personas & Expected UX

### 👤 Human User

| Scenario | Expected UX |
|----------|-------------|
| Enable OpenCode as a target | Tab 3 (Providers) → navigate to "OpenCode" → press `Space`. A checkmark appears. |
| Install a skill to OpenCode | Tab 1 (Skills) → select skill → press `Space`. Skill is copied to `.opencode/skills/<name>/SKILL.md` (workspace) or `~/.config/opencode/skills/<name>/SKILL.md` (global). `opencode.json` is updated with the skill reference. |
| Enable MCP for OpenCode | Tab 2 (MCP) → select server → `Space`. OpenCode writes flat `mcp.<name>` entry to `opencode.json`. |
| Verify install | OpenCode CLI can now use the skill natively because it scans the same directories. |

### 🤖 AI Agent User

| Scenario | Expected UX |
|----------|-------------|
| Agent targets OpenCode explicitly | `agk install clawhub/react-builder --provider opencode --json` returns `{"installed": true, "path": ".opencode/skills/react-builder/SKILL.md", "config_updated": "opencode.json"}` |
| Agent enables MCP for OpenCode | `agk mcp enable fs --provider opencode --scope workspace` writes flat `mcp.fs = { "type": "local", ... }` to `opencode.json`. |
| Agent checks provider status | `agk sync --dry-run --json` shows OpenCode as an active target and lists which skills would be installed there. |

### 🏭 CI/CD User

| Scenario | Expected UX |
|----------|-------------|
| Standardize OpenCode skills in CI | `agk sync --global --quiet` ensures `~/.config/opencode/skills/` contains exactly the skills declared in `~/.config/agk/config.toml`. |
| Project-specific OpenCode setup | `agk sync` (workspace scope) in a repo ensures `.opencode/skills/` and `opencode.json` are correct. |
| Config merge safety | If `opencode.json` already contains unrelated settings (other plugins, custom commands), `agk` only adds/removes the skill array and flat `mcp` entries; all other keys are preserved. |

---

## Non-Goals
- OpenCode plugin management (`bun install` for `.opencode/package.json`). agk manages skills, not runtime dependencies.
- Migration from Claude Code to OpenCode. OpenCode already supports Claude-compatible paths; agk simply writes to the native OpenCode paths.
- Remote OpenCode API configuration. This adapter is file-system only.

---

## Acceptance Criteria
- [x] `infra/provider/opencode.rs` implements `ProviderPort` (`id`, `name`, `install`, `remove`).
- [x] Global path: `~/.config/opencode/skills/<name>/SKILL.md`.
- [x] Workspace path: `.opencode/skills/<name>/SKILL.md`.
- [x] `opencode.json` is merged (not replaced) when skills are added or removed.
- [x] MCP writes flat `mcp.<name>` schema with `type` and `enabled` fields.
- [x] TUI Providers tab shows OpenCode with toggle support.
- [x] Headless `agk sync` and `agk install` support `--provider opencode`.
- [x] Unit tests for JSON/JSONC merge logic.
- [x] `cargo test` and `cargo clippy -- -D warnings` pass.
- [ ] Comment preservation in JSONC (basic stripping works; true preservation requires AST-level parser — future enhancement).

---

*End of PRD.*
