# Provider Config Root Selection — PRD

**Status:** Partially Implemented (v0.2)

## Overview

Allow each provider to support multiple config folder names (e.g., OpenCode can use `.opencode` or `.agents`). When the user first enables a provider in the TUI, show a floating modal that lets them pick which folder to use for that provider in the current workspace. The choice is persisted in `.agk/config.toml`. When multiple providers select the same folder, they share it (no separate subdirectories).

## Research Summary

| Provider     | Primary Folder | Secondary Folder(s) | Notes |
|-------------|----------------|---------------------|-------|
| OpenCode    | `.opencode`    | `.agents`           | OpenCode also reads `.agents` for Claude Code compatibility |
| Claude Code | `.claude`      | `.agents`           | Claude Code docs reference `.agents` as a shared dir |
| Copilot     | N/A            | —                   | Uses VS Code settings, not a folder |
| Gemini      | `.gemini`      | `.ai`               | Gemini CLI uses `.gemini/`, legacy `.ai/` exists |
| Firebender  | `.firebender`  | —                   | Single folder only |
| Letta       | `.letta`       | —                   | Single folder only |
| Snowflake   | N/A            | —                   | Not folder-based |
| AMP         | `.amp`         | —                   | Single folder only |

> **Shared-folder rule:** If multiple providers pick `.agents`, they all install into the same directory. OpenCode and Claude Code would both see each other's skills.

## Functional Requirements

### Provider Trait Extension (Approach B)
- [x] Add `fn available_config_roots(&self) -> Vec<(String, String)>` to `ProviderPort` with a default empty vec (single root).
- [ ] Add `fn selected_config_root(&self) -> Option<String>` to read the persisted choice.
- [x] Update `fn provider_root(&self, scope: &Scope) -> PathBuf` in each provider to use the selected root from config.

### Config Schema
- [x] `[provider_roots]` table in `.agk/config.toml` for workspace scope.

### TUI Flow
- [x] Floating modal appears only on first enable of multi-root provider.
- [x] Arrow keys select, Enter confirms, Esc cancels.
- [x] Selection saved to workspace `.agk/config.toml`.

### Scope
- [x] Workspace scope only (global uses hardcoded `~/.config/<provider>/`).

## Acceptance Criteria
- [x] `ProviderPort` exposes `available_config_roots()`.
- [x] Config TOML round-trips `provider_roots` without data loss.
- [x] Floating modal renders centered with arrow-key selection.
- [x] Selection is saved on Enter, cancelled on Esc.
- [x] Provider installs use selected root (tested for OpenCode, Claude Code, Gemini).
- [x] Shared folder works: Claude Code + OpenCode both picking `.agents` see the same skills.
- [x] `cargo test` passes all 152 tests.
- [x] `cargo fmt --check` is clean.

## Non-Goals
- Changing root after initial selection via TUI (CLI/config edit only).
- Validating that the selected folder already exists.
- Per-asset-type roots (e.g., skills in `.opencode`, instructions in `.agents`).
- Auto-migration from old installs to new roots.

*End of PRD.*
