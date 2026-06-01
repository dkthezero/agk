# Profiles Feature – Product Requirements (v0.3)

**Status:** Implemented (v0.3 + v0.3.1 + v0.3.2)
**Previous:** [v0.2 PRD](https://github.com/dkthezero/agk/blob/4088606/docs/product/features/profiles/prd.md)
**Epic:** [v0.3 Team-Ready Profiles](../../../epics/v03-team-ready-profiles.md)

---

## Overview

A **Profile** is a named, self-contained agentic context that bundles an AI agent CLI (e.g. OpenCode, Claude Code), a curated set of skills, selected MCP servers, and a custom agent definition. In v0.3, profiles become **portable, versioned, team-distributable environment blueprints** that can be discovered from vaults, self-heal their dependencies on launch, and store vault provenance for every referenced asset.

---

## User-Facing Behavior

### TUI Tab [5] Profiles

The AGK TUI retains the **Profiles** tab at position `[5]` (between `[4] Providers` and `[0] Vault`).

- **Navigation:** `[5]` key switches to Profiles tab.
- **List existing profiles:** Displays profiles stored in the active scope config (Workspace or Global).
- **Vault-discovered profiles:** Profiles found in attached vaults (`profiles/*/PROFILE.md`) appear in the list with a `[Vault]` badge and an `[ ]` checkbox. Pressing `Space` installs them (batch-installs all referenced skills, instructions, and MCPs).
- **F2 — Add new profile:** Opens the enhanced provider-specific multi-step modal wizard. See [Profile Wizard PRD](../profile-wizard/prd.md).
- **F3 — Edit profile:** Opens the Profile Editor for the selected profile. See §Profile Editor below.
- **Delete profile:** `Delete` key on selected profile opens a confirmation modal, then removes it from config and deletes its `.agk/profiles/` subdirectory.
- **Enter — Start profile:** Launches the profile session. If dependencies are missing, a dependency-resolution overlay shows progress; missing skills/MCPs are auto-installed from their specified vaults before the provider starts.

#### Profile Editor (F3)

The editor is a tabbed modal:

1. **Overview** — Profile name, provider, scope, estimated token count of the composed prompt.
2. **Skills** — Checklist of available skills across vaults. Each skill shows its originating vault (e.g., `rust-patterns [clawhub]`). `Space` toggles attachment.
3. **MCPs** — Checklist of registered + vault-discovered MCPs. Each shows vault if vault-sourced. `Space` toggles attachment.
4. **Tools / Permissions** — If the provider advertises configurable tools, a checklist appears here. `Space` toggles.
5. **Raw Markdown** — Editable text area showing the composed `agent.md` content. Live token count updates as the user types.

**Save:** `Ctrl+S` or `Enter` writes changes back to `config.toml` and `.agk/profiles/<name>/agent.md`.

---

### CLI Commands

#### Launch a profile session (updated for v0.3)

```
agk p <profile_name>
# alias: agk profile start <profile_name>
```

**Launch flow (v0.3 enhancements in bold):**

1. Load profile from config (workspace scope preferred, fallback to global).
2. **Resolve missing dependencies:**
   - Read `profile.skill_refs` + `skill_vault_refs`.
   - For any skill not installed in the current scope, resolve its vault.
   - If vault is `"auto"`, scan all attached vaults for the skill name (warn if ambiguous).
   - Auto-install missing skills from the identified vault.
   - Repeat for MCP servers: check global registry; if missing, resolve vault and auto-register.
   - Emit clear error if a specified vault is unavailable or the asset is not found.
3. Generate a random 6-digit session suffix (e.g. `123456`).
4. **Provider-specific session setup:**
   - **OpenCode provider:**
     - Copy `.agk/profiles/<name>/agent.md` → `.opencode/agents/<name>_<suffix>.md`.
     - If `agent.md` is missing, use the structured markdown composed by the wizard.
     - Set `mode: primary` and name in frontmatter.
     - Under `agent.<name>_<suffix>` in workspace `opencode.json`, set:
       - `permission.skill` — allow listed skills, deny `*`.
       - `mcp` — enable selected MCP servers (`enabled: true`).
       - **NEW:** `tools` — if `tool_refs` present, restrict tool access.
   - **Claude Code provider (NEW in v0.3):**
     - Copy `.agk/profiles/<name>/agent.md` → `.claude/agents/<name>.md`.
     - `agent.md` contains full YAML frontmatter (`name`, `description` with `<example>` blocks, `tools`, `model`, `color`, `memory`) + structured body.
5. **Start agent CLI** — provider-specific command.
6. **Block until exit**.
7. **Cleanup** — Remove session agent file, remove the provider-config entry, prune if empty.

#### Create a profile headlessly (updated for v0.3)

```bash
agk profile create <name> \
  --provider opencode \
  --skills skill1:vault1,skill2:vault2 \
  --mcps mcp1:vault1,mcp2 \
  --description-file ./my-agent.md \
  --scope workspace
```

**v0.3 changes:**
- `--skills` now accepts `name:vault` syntax. If vault omitted, defaults to `"auto"`.
- `--mcps` now accepts `name:vault` syntax.
- `--description-file` allows supplying a custom `agent.md` instead of wizard-generated.
- `--tools` and `--permission-mode` flags added for providers that support them.

---

### Scope

Profiles obey the existing scoped-config system:
- **Workspace profiles** are stored in `.agk/config.toml` and apply only to the current project.
- **Global profiles** are stored in `~/.config/agk/config.toml` and are available everywhere.
- The TUI scope toggle (`Tab` key) switches between workspace and global profiles.

---

## Functional Requirements

### v0.3 New Requirements

1. **Vault-aware dependency storage:** Profile shall store skills and MCPs as structured objects with `name` + `vault` fields. See §Config Schema.
2. **Auto-install on start:** `agk p start` shall resolve missing skills/MCPs and install them from their specified vaults before launching the provider.
3. **Vault-discoverable profiles:** `profiles/` directories in attached vaults shall be scanned; discovered profiles shall appear in the TUI Profile tab with `[Vault]` badge.
4. **Batch profile installation:** Installing a vault profile shall atomically install all referenced skills, instructions, and MCPs, then create the profile config entry.
5. **Profile editor (F3):** Post-creation editing of skills (with vault), MCPs, tools, permissions, and raw markdown with live token count.
6. **Custom prompt overlay:** `prompt_overlay_path` shall be supported; if present, AGK uses the user-supplied file instead of wizard-generated markdown.
7. **Tool/permission projection:** If a profile has `tool_refs` or `permission_mode`, the provider's `build_launch_plan()` shall project them into the provider-specific config.
8. **Claude Code projection:** For the Claude Code provider, `agk p start` shall write `.agk/profiles/<name>/agent.md` to `.claude/agents/<name>.md` with full frontmatter.
9. **Launch simulation overlay:** The TUI shall show a visual progress panel during `agk p start`: dependency resolution → install → projection → provider runtime.
10. **Backward-compatible config migration:** Old flat `skills = ["name"]` shall deserialize into `ProfileAssetRef { name, vault: "auto" }` and rewrite to structured format on next save.

### Retained v0.2 Requirements

- Profile shall bundle: `name`, `provider_id`, skills, MCPs, instructions, agent file.
- Profile config shall be stored in `ConfigFile` under a `profiles` field.
- Session agent files shall use a unique random suffix to avoid collisions.
- On session exit, all files created or modified for the session shall be restored to pre-session state.
- Concurrent sessions of the same profile shall be supported via unique session suffixes.
- If any step of session setup fails, partial changes shall be rolled back before reporting the error.
- Wizard extensibility: The profile creation wizard shall be a stack of `WizardStep` values produced by the active provider.

---

## Config Schema (v0.3)

### Structured Profile Format (Canonical)

```toml
[[profiles]]
name = "web-app-team"
provider_id = "opencode"
scope = "workspace"

[[profiles.skills]]
name = "rust-patterns"
vault = "clawhub"

[[profiles.skills]]
name = "docker"
vault = "ecc"

[[profiles.mcps]]
name = "filesystem"
vault = "workspace"

[[profiles.mcps]]
name = "github-api"
vault = "auto"        # resolved at runtime

[profiles.tools]
refs = ["Read", "Glob", "Grep"]
permission_mode = "acceptEdits"

prompt_overlay_path = ".agk/profiles/web-app-team/custom.md"
```

### Backward-Compatible Flat Format (Read-Only Legacy)

```toml
[[profiles]]
name = "legacy"
provider_id = "opencode"
skills = ["rust-patterns", "docker"]     # deserializes to vault = "auto"
mcps = ["filesystem"]
```

On first write, AGK re-serializes to the structured format.

---

## Out of Scope (Future)

- ~~Profile import/export across machines~~ (shipped in v0.3.1: [Profile Portability PRD](../profile-portability/prd.md)).
- ~~Profile diff (compare local vs vault)~~ (shipped in v0.3.2: `agk profile diff`).
- Profile versioning / rollback (depends on git integration).
- Multi-provider profiles (one profile targeting both OpenCode and Claude Code).

## Success Criteria

- [x] TUI shows `[5] Profiles` tab with list + vault-discovered profiles.
- [x] `agk p <name>` starts the provider with the correct agent, skills, and MCPs.
- **v0.3 additions:**
  - [x] `agk p start` auto-installs missing dependencies from specified vaults.
  - [x] Installing a vault profile installs all referenced assets atomically.
  - [x] F3 Editor allows editing skills, MCPs, tools, and raw markdown.
  - [x] Claude Code provider writes `.claude/agents/<name>.md` with frontmatter.
  - [x] Old flat-string profiles continue to work and migrate on save (write-migration shipped in v0.3.2).
- [x] `agk profile diff <name>` shows which skills/MCPs/tools differ between local and vault (v0.3.2).
- [x] `cargo test` passes; `cargo fmt --check` passes.
- [x] Architecture tests pass with zero allowlists.

---

*PRD v0.3 — updated 2026-06-01*
