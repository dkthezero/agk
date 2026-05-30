# Profiles Feature – Product Requirements

## Overview

A **Profile** is a named, self-contained agentic context that bundles an AI agent CLI (e.g. OpenCode), a curated set of skills, selected MCP servers, and a custom agent definition. Launching a Profile starts the agent CLI in a fully pre-configured session and cleans up all session artifacts on exit, leaving the workspace pristine.

## User-Facing Behavior

### TUI Tab [5] Profiles

The AGK TUI gains a new tab **Profiles** rendered at position `[5]` (between `[4] Providers` and `[0] Vault`).

- **Navigation:** `[5]` key switches to Profiles tab.
- **List existing profiles:** Displays profiles stored in the active scope config (Workspace or Global).
- **F2 — Add new profile:** Opens a provider-specific multi-step modal wizard.

#### Profile Creation Wizard (OpenCode Provider)

The wizard is **owned by the provider** — each `ProfileProvider` implementation declares its own step sequence via a `Vec<WizardStep>` stack. This lets future providers (Gemini, Claude Code, etc.) define entirely different on-boarding flows.

OpenCode provider steps (current default):

1. **Profile name** — Enter a unique alphanumeric name with hyphens.
2. **Tailor the agent** (Q&A loop) — Answer a short questionnaire so the final agent description matches the user's actual need. Each question is shown one at a time:
   - *What is the primary task this agent should handle?*
   - *What tone or style should the agent use?*
   - *Are there any specific constraints or rules?*
   Answers are accumulated and later joined into a single `--description` string passed to `opencode agent create`.
3. **Select skills** — Scrollable checklist of all available skills across active vaults (`Space` toggles, `Enter` confirms).
4. **Select MCP servers** — Scrollable checklist of registered MCP servers.
5. **Review & confirm** — Read-only overview pane showing: profile name, generated description, selected skills count, selected MCPs count. `Enter` proceeds; `Esc` goes back to step 4.
6. **Interactive agent creation** — AGK suspends its TUI, yields the terminal to `opencode agent create`, waits for the user to finish the interactive OpenCode wizard, then resumes AGK. The resulting agent markdown is moved to `.agk/profiles/<profile_name>/agent.md`.

- **Delete profile:** `Delete` key on selected profile opens a confirmation modal, then removes it from config and deletes its `.agk/profiles/` subdirectory.

### CLI Commands

#### Launch a profile session

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
     - Under `agent.<name>_<suffix>` in workspace `opencode.json`, set:
       - `permission.skill` — allow listed skills, deny `*`.
       - `mcp` — enable selected MCP servers (`enabled: true`).
4. **Start agent CLI** — `opencode` (or provider-specific command).
5. **Block until exit**.
6. **Cleanup** — Remove session agent file, remove the `agent.<name>_<suffix>` entry from `opencode.json`, delete `opencode.json` if empty, prune `.opencode/` if empty.

#### Create a profile headlessly

```
agk profile create <name> \
  --provider opencode \
  --skills skill1,skill2 \
  --mcps mcp1,mcp2 \
  --description "A Rust coding assistant" \
  --scope workspace
```

**Args:**

| Flag | Short | Description | Default |
|------|-------|-------------|---------|
| `--provider` | `-p` | Provider ID (only `opencode` supported in v1) | `opencode` |
| `--skills` | `-s` | Comma-separated list of skill names to bundle | (empty) |
| `--mcps` | `-m` | Comma-separated list of MCP server names to enable | (empty) |
| `--description` | `-d` | Raw description string passed to `opencode agent create` | (none) |
| `--description-file` | | Path to a markdown file whose contents are used as description | (none) |
| `--scope` | | `global` or `workspace` | `workspace` |

**Flow:**

1. Validate the provider is active and supports profiles.
2. Ensure no duplicate profile name exists in the chosen scope.
3. Write the new profile entry to `config.toml`.
4. Run `opencode agent create --name <name> --description <desc>` headlessly.
5. On success, copy the generated `.opencode/agents/<name>.md` → `.agk/profiles/<name>/agent.md`.

### Scope

Profiles obey the existing scoped-config system:
- **Workspace profiles** are stored in `.agk/config.toml` and apply only to the current project.
- **Global profiles** are stored in `~/.config/agk/config.toml` and are available everywhere.
- The TUI scope toggle (`Tab` key) switches between workspace and global profiles.

### Dry-run mode (Phase 5)

```
agk profile start <name> --dry-run
agk profile create <name> ... --dry-run
```

When `--dry-run` is passed:
1. The profile config is loaded and validated.
2. A `LaunchPlan` is built via `ProfileRuntimePort::build_launch_plan()`.
3. The plan is returned (and optionally emitted as JSON with `--json`) without executing.
4. No filesystem modifications, no process spawning.
5. The plan includes: `agent_markdown_source`, `patched_provider_config`, `original_provider_config_bytes`, `skills`, `mcps`.

This allows CI pipelines and user scripts to preview exactly what a profile session will do before committing side effects.

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
10. **Wizard extensibility:** The profile creation wizard shall be a stack of `WizardStep` values produced by the active `ProfileProvider`. New providers can inject their own steps (e.g. API-key prompts, model selection) without modifying `ListMode` or TUI dispatch code.
11. **Q&A description:** The tailor step shall concatenate question+answer pairs into a single description string. This string is passed as `--description` to `opencode agent create` so the generated agent markdown reflects the user's intent.

## Out of Scope (Future)

- Editing an existing profile after creation (users can delete and recreate).
- Profile import/export.

## Success Criteria

- TUI shows `[5] Profiles` tab with list/add/delete.
- `agk p <name>` starts OpenCode with the correct agent, skills, and MCPs.
- After `opencode` exits, workspace has no leftover session files.
- `cargo test` passes; `cargo fmt --check` passes.
