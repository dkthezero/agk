# Profiles Feature – Technical Design

## Architecture

Profiles introduce two new pieces to the existing hexagonal architecture:

1. **`Profile` domain model** — pure data, lives in `domain/profile.rs`.
2. **`ProfileProvider` port trait** — provider-specific implementation for how to set up, launch, and tear down a profile session. Lives in `app/ports.rs`.
3. **`OpenCodeProfileProvider`** — concrete implementation in `infra/provider/opencode.rs` (extends existing `OpenCodeProvider`).

Everything else is TUI/CLI wiring using existing patterns.

## Data Model

### `Profile` (domain)

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Profile {
    pub name: String,
    pub provider_id: String,
    pub skills: Vec<String>,       // skill names (directory names under .opencode/skills/)
    pub mcps: Vec<String>,         // MCP server names from McpRegistry
}
```

### `ConfigFile` extension

Add to `ConfigFile`:

```rust
#[serde(default)]
pub profiles: Vec<Profile>,
```

This serializes as `[[profiles]]` array-of-tables in TOML. The base agent markdown is located at a fixed path relative to the profile name (`.agk/profiles/<name>/agent.md`) so it is **not** stored in the config struct.

## Port Trait: `ProfileProvider`

```rust
pub trait ProfileProvider: Send + Sync {
    fn supports_profiles(&self) -> bool;

    /// Prepare the workspace for a profile session.
    /// Returns a `ProfileSession` handle that can be used for cleanup.
    fn start_session(
        &self,
        profile: &Profile,
        session_key: &str,
        workspace_root: &Path,
    ) -> Result<ProfileSession>;
}

pub struct ProfileSession {
    pub process: std::process::Child,
    pub cleanup: Box<dyn FnOnce() -> Result<()> + Send>,
}
```

**Why a trait?** Each agent CLI has different config files, directory layouts, and launch commands. OpenCode uses `.opencode/agents/` and `opencode.json`; Gemini CLI might use `.gemini/agents/` and `gemini.json`. The trait keeps provider-specific logic out of the app layer.

## OpenCode Provider Extension

`OpenCodeProvider` already implements `ProviderPort` and `McpProvider`. We add `ProfileProvider`.

### Session Setup

1. Read `.agk/profiles/<name>/agent.md`.
2. Write `.opencode/agents/<name>_<key>.md`:
   - Update frontmatter `name` to `<name>_<key>`.
   - Ensure `mode: primary`.
3. Read/create workspace `opencode.json`:
   - Under `"agent"`, insert `{"<name>_<key>": {"mode": "primary", ...}}`.
   - Under `"permission" -> "skill"`, set `"*": "deny"` and each selected skill to `"allow"`.
   - Under `"mcp"`, set each selected MCP to `"enabled": true`.
   - Remember original state for rollback.

### Launch

Spawn `opencode` in the workspace root (it auto-discovers the new primary agent).

### Cleanup

1. Delete `.opencode/agents/<name>_<key>.md`.
2. Revert `opencode.json`:
   - Remove the agent entry.
   - Remove skill permission entries we added (if none remain, drop `"permission" -> "skill"`).
   - Remove MCP entries we added (if `"mcp"` becomes empty, drop the key).
3. If `opencode.json` is now `{}`, delete it.
4. If `.opencode/` is empty, remove it.

**Rollback on failure:** If any setup step fails, immediately run cleanup logic before returning the error.

## TUI Integration

### `TabKind` extension

```rust
pub enum TabKind {
    Asset,      // Skills / Instructions
    Vault,
    Provider,
    Mcp,
    Analytics,
    Profile,    // NEW
}
```

### `AppState` extension

```rust
pub profile_entries: Vec<Profile>,
```

### `bootstrap.rs` extension

Register a new `StubFeatureSet` for "profiles" → `TabKind::Profile`, inserted at data index 4 (rendered as `[5]`). This follows the exact pattern used for MCP, Providers, and Vaults.

```rust
registry.register_feature_set(Box::new(StubFeatureSet::new("profile", "Profiles", "")));
```

### Render dispatch (`tui/render.rs`)

Add `TabKind::Profile` arm:
- Left list: profile names from `state.profile_entries`.
- Right detail: profile details (provider, skills, MCPs).
- Footer keybinds: `[↑/↓] Move  [F2] Add Profile  [Delete] Remove  [Esc]x2 Quit`

If `list_mode` is in profile creation substates, show the appropriate input modals.

### Event handling (`tui/event.rs`)

Add `ListMode` variants for profile creation wizards:

```rust
ProfileWizardName,
ProfileWizardSelectSkills,
ProfileWizardSelectMcps,
ProfileWizardConfirmCreate,
```

Handle `F2` on Profile tab to start wizard. Handle `Delete` to show confirm-remove modal.

**Skill/MCP selection UI:** Reuse existing list rendering but in modal form. Because AGK already has list + detail panes, the simplest modal is a full-screen overlay: a scrollable checklist with `[Space]` to toggle items and `[Enter]` to confirm.

### Profile creation wizard

Because `opencode agent create` is an interactive TUI command, we **cannot** embed it inside the AGK TUI directly (nested TUIs break terminal state). The wizard therefore works like this:

1. AGK TUI quits to alternate screen.
2. AGK runs `opencode agent create` as a child process, letting it take over the terminal.
3. After `opencode` finishes, AGK TUI re-initializes (`enable_raw_mode`, `EnterAlternateScreen`).
4. AGK locates the newly created `.opencode/agents/*.md` file, moves it to `.agk/profiles/<name>/agent.md`, and updates `config.toml`.

This is analogous to how `opencode agent create` is already used standalone.

## CLI Integration

### `cli/entry.rs`

Add subcommand:

```rust
/// Launch a profile session
Profile {
    #[command(subcommand)]
    command: ProfileCommands,
}
```

```rust
pub enum ProfileCommands {
    /// Start a profile session
    Start {
        /// Profile name
        name: String,
    },
}
```

Add alias: `agk p <name>` → `agk profile start <name>` at the parser level (clap alias).

### `cli/commands.rs`

Add handler:

```rust
pub fn run_profile_start(name: &str, workspace: &Path) -> Result<i32> {
    let (registry, _scan, store) = bootstrap::build(workspace)?;
    let config = store.load(Scope::Workspace)?;
    let profile = config.profiles.get(name)
        .or_else(|| store.load(Scope::Global).ok()?.profiles.get(name))
        .ok_or_else(|| anyhow!("Profile '{}' not found", name))?;

    let provider = registry.providers.iter()
        .find(|p| p.id() == profile.provider_id)
        .and_then(|p| p.as_profile_provider())
        .ok_or_else(|| anyhow!("Provider '{}' does not support profiles", profile.provider_id))?;

    let session_key = format!("{:06}", rand::random::<u32>() % 1_000_000);
    let mut session = provider.start_session(profile, &session_key, workspace)?;

    let exit_status = session.process.wait()?;

    (session.cleanup)()?;

    Ok(if exit_status.success() { 0 } else { 1 })
}
```

## Files to Modify / Add

### New files

- `src/domain/profile.rs` — `Profile`, `ProfileConfig`, `ProfilesBucket`
- `docs/product/features/profiles/prd.md` — this PRD
- `docs/product/features/profiles/technical_design.md` — this document

### Modified files

- `src/domain/config.rs` — add `profiles` field to `ConfigFile`
- `src/domain/mod.rs` — re-export `profile`
- `src/app/ports.rs` — add `ProfileProvider` trait; add `as_profile_provider()` to `ProviderPort` or keep as separate trait
- `src/app/registry.rs` — add `get_profile_provider()` helper
- `src/app/bootstrap.rs` — register `StubFeatureSet` for profiles
- `src/infra/provider/opencode.rs` — implement `ProfileProvider`
- `src/infra/provider/mod.rs` — export profile provider helpers
- `src/tui/app.rs` — add `TabKind::Profile`, `profile_entries`, `ProfileWizard*` list modes
- `src/tui/render.rs` — render Profile tab and wizard modals
- `src/tui/event.rs` — handle Profile tab keys and wizard input
- `src/tui/widgets/mod.rs` — add profile list/detail widgets (or reuse existing list/detail)
- `src/cli/entry.rs` — add `Profile` subcommand with `Start`
- `src/cli/commands.rs` — add `run_profile_start`

## Testing Strategy

1. **Unit tests:**
   - `Profile` serialization round-trip in `config.rs`.
   - `OpenCodeProfileProvider` computes correct `opencode.json` merge/diff.
   - Session cleanup restores original `opencode.json` state exactly.

2. **Integration tests:**
   - `agk p test-profile` with a fake `opencode` binary that exits immediately.
   - Assert no leftover `.opencode/agents/test-profile_*.md`.
   - Assert `opencode.json` unchanged or deleted.

3. **Manual tests:**
   - TUI `[5] Profiles` tab renders.
   - F2 wizard prompts name, lists skills/MCPs, invokes `opencode agent create`.
   - Launch and exit a real OpenCode session; verify cleanup.

## Rollout Plan

1. Merge domain + infra changes (OpenCode profile provider).
2. Merge TUI tab rendering + event handling.
3. Merge CLI `agk p` command.
4. Add tests and docs.
5. Final review and merge to master.

## Notes

- The random 6-digit suffix is intentionally simple (not a UUID) because it only needs to be unique within the current workspace for a single session.
- `opencode.json` is JSON, not JSONC, during profile manipulation; we use `serde_json` directly and preserve any existing JSONC comments via a round-trip parse → strip-comments → modify → write. The existing `strip_jsonc_comments` helper in `opencode.rs` can be extracted to a shared utility if needed.
- MCP server definitions are read from `McpRegistry` (global), but their enabled state is written into the workspace `opencode.json` by the provider's `ProfileProvider` implementation.
