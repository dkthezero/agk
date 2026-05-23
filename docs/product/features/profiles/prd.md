# Profiles Feature – Product Requirements

## Overview

A **Profile** is a named, self-contained agentic context that bundles an AI agent CLI (e.g. OpenCode), a curated set of skills, selected MCP servers, and a custom agent definition. Launching a Profile starts the agent CLI in a fully pre-configured session and cleans up all session artifacts on exit, leaving the workspace pristine.

## User-Facing Behavior

### TUI Tab [5] Profiles

The AGK TUI gains a new tab **Profiles** rendered at position `[5]` (between `[4] Providers` and `[0] Vault`).

- **Navigation:** `[5]` key switches to Profiles tab.
- **List existing profiles:** Displays profiles stored in the active scope config (Workspace or Global).
- **F2 — Add new profile:** Opens a multi-step modal:
  1. **Profile name** — alphanumeric with hyphens (e.g. `opencode-dev`).
  2. **Select agent CLI** — Choose from providers that implement `ProfileProvider`. Initially only OpenCode.
  3. **Select skills** — Reusable checklist of available skills across all active vaults.
  4. **Select MCPs** — Checklist of registered MCP servers.
  5. **Run `opencode agent create`** — Invoke the OpenCode CLI to generate an agent markdown file. The generated file is moved into `.agk/profiles/<profile_name>/agent.md`.
- **Delete profile:** `Delete` key on selected profile removes it from config and deletes its `.agk/profiles/` subdirectory.

### CLI Launch

```
agk p <profile_name>
# alias: agk profile start <profile_name>
```

**Launch flow:**

1. Load profile from config (workspace scope preferred, fallback to global).
2. Generate a random 6-digit session suffix (e.g. `123456`).
3. **Provider-specific session setup** (delegated to the profile's target provider):
   - OpenCode provider:
     - Copy `.agk/profiles/<name>/agent.md` → `.opencode/agents/<name>_<suffix>.md`
     - Set `mode: primary` and name in frontmatter.
     - Merge `permission.skill` into workspace `opencode.json` (allow listed skills, deny `*`).
     - Enable selected MCP servers in workspace `opencode.json`.
4. **Start agent CLI** — `opencode` (or provider-specific command).
5. **Block until exit**.
6. **Cleanup** — Remove session agent file, revert `opencode.json` changes, delete `opencode.json` if empty, prune `.opencode/` if empty.

### Scope

Profiles obey the existing scoped-config system:
- **Workspace profiles** are stored in `.agk/config.toml` and apply only to the current project.
- **Global profiles** are stored in `~/.config/agk/config.toml` and are available everywhere.
- The TUI scope toggle (`Tab` key) switches between workspace and global profiles.

## Functional Requirements

1. Profile shall bundle: `name`, `provider_id`, `skills: Vec<String>`, `mcps: Vec<String>`, `agent_file: PathBuf`.
2. Profile config shall be stored inline in `ConfigFile` under a `profiles` field (serialized via `serde(flatten)`).
3. The profile's base agent file shall live in `.agk/profiles/<profile_name>/agent.md`.
4. Session agent files shall use a unique random suffix to avoid collisions.
5. On session exit, all files created or modified for the session shall be restored to their pre-session state.
6. If `opencode.json` was created by the session and is empty after cleanup, it shall be deleted.
7. If `.opencode/` has no remaining user files after cleanup, it may be removed.
8. Concurrent sessions of the same profile shall be supported via unique session suffixes.
9. If any step of session setup fails, the partial changes shall be rolled back before reporting the error.

## Out of Scope (Future)

- Support for agent CLIs other than OpenCode (Gemini, Claude Code, etc.) — the `ProfileProvider` trait is designed to support this, but only OpenCode is implemented in v1.
- Editing an existing profile after creation (users can delete and recreate).
- Profile import/export.

## Success Criteria

- TUI shows `[5] Profiles` tab with list/add/delete.
- `agk p <name>` starts OpenCode with the correct agent, skills, and MCPs.
- After `opencode` exits, workspace has no leftover session files.
- `cargo test` passes; `cargo fmt --check` passes.
