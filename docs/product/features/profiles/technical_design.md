## Profile Feature – Technical Design

## Architecture

Profiles introduce three new pieces to the existing hexagonal architecture:

1. **`Profile` domain model** — pure data, lives in `domain/profile.rs`.
2. **`ProfileProvider` port trait** — provider-specific implementation for how to set up, launch, and tear down a profile session. Lives in `app/ports.rs`.
3. **`ProfileRuntimePort`** — **new in Phase 5** — separates deterministic plan construction (`build_launch_plan`) from side-effectful execution (`run_plan`).  Implemented by provider adapters that support profile sessions (e.g. `OpenCodeProvider`).
4. **Extensible profile wizard** — a `Vec<WizardStep>` stack owned by the `ProfileProvider`. Each provider defines its own step sequence so the TUI stays generic.
5. **`OpenCodeProfileProvider`** — concrete implementation in `infra/provider/opencode.rs` (extends existing `OpenCodeProvider`).

Everything else is TUI/CLI wiring using existing patterns.

### Phase 5: Profile Runtime Port

```rust
/// Port for building and executing profile launch plans.
pub trait ProfileRuntimePort: Send + Sync {
    fn provider_id(&self) -> &str;

    /// Build a deterministic launch plan without modifying filesystem state.
    fn build_launch_plan(
        &self,
        profile: &Profile,
        config: Option<&ConfigFile>,
    ) -> Result<LaunchPlan>;

    /// Execute a previously-built launch plan, returning a handle that
    /// includes a cleanup closure for restoring provider state.
    fn run_plan(&self,
        plan: &LaunchPlan,
    ) -> Result<ProfileSession>;
}
```

#### `LaunchPlan` (app/event.rs)

A concrete, serialisable plan for what a profile session will do:

```rust
pub struct LaunchPlan {
    pub profile_id: ProfileId,
    pub provider_id: ProviderId,
    pub skills: Vec<SkillId>,
    pub mcps: Vec<McpServerId>,
    pub files_to_write: Vec<PathBuf>,
    pub restore_required: bool,
    // Provider-specific concrete plan details
    pub agent_markdown_source: PathBuf,
    pub patched_provider_config: Option<serde_json::Value>,
    pub original_provider_config_bytes: Option<Vec<u8>>,
}
```

- `build_launch_plan()` sets `agent_markdown_source`, `patched_provider_config`, `original_provider_config_bytes`.
- `run_plan()` writes the session agent file, writes the patched config, spawns the process, and returns a `ProfileSession` with a cleanup closure that restores the original config bytes.
- `--dry-run` flag returns `CoreOutcome::LaunchPlan(plan)` without calling `run_plan()`.

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

## Port Extensions

### `ProfileProvider` trait (session lifecycle)

Already implemented in `app/ports.rs`:

```rust
pub trait ProviderPort: Send + Sync {
    ...
    fn supports_profiles(&self) -> bool;
    fn start_profile_session(
        &self,
        profile: &Profile,
        session_key: &str,
        workspace_root: &Path,
    ) -> Result<ProfileSession>;
}
```

### `ProviderPort` extension for wizard steps

`profile_wizard_steps()` is added directly to the `ProviderPort` trait (no separate `ProfileWizard` trait). Default returns empty vec.

```rust
/// Steps that a provider wants the user to walk through when creating a profile.
/// Each step is a static description (what to show). Mutable UI state lives in
/// `WizardState`, not in the step itself.
```

### `WizardStep` enum (TUI-generic)

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum WizardStep {
    TextInput {
        title: String,
        placeholder: String,
    },
    QuestionAnswer {
        question: String,
        placeholder: String,
    },
    Checklist {
        title: String,
        options: Vec<String>,
    },
    Review {
        title: String,
    },
    Interactive {
        title: String,
        command: String,
        args: Vec<String>,
    },
}
```

The `WizardStep` values are **static descriptions** (what to show). Mutable state (cursor position, checked items, typed buffer) lives in `WizardState` inside `AppState`, indexed by `step_index`.

### `WizardState` (AppState field)

```rust
pub struct WizardState {
    pub steps: Vec<WizardStep>,
    pub step_index: usize,
    /// Shared accumulator across steps.
    pub name: String,
    pub description_parts: Vec<(String, String)>, // (question, answer)
    pub skills: Vec<String>,
    pub mcps: Vec<String>,
    pub skill_options: Vec<String>,
    pub mcp_options: Vec<String>,
    /// UI state for the current step.
    pub prompt_buffer: String,
    checked: Vec<bool>,
    selected: usize,
}
```

### Description composition

After the Q&A loop, the TUI joins every `(question, answer)` pair into a single string:

```text
Q: What is the primary task?
A: Write Rust CLI tools.

Q: What tone should it use?
A: Concise, professional.
```

This exact string becomes `--description` for `opencode agent create`.

## OpenCode Provider Extension

`OpenCodeProvider` implements `ProviderPort`, `McpProvider`, and `ProfileWizard`.

### Wizard steps returned by `OpenCodeProvider::wizard_steps()`

1. `TextInput { title: "Profile name", placeholder: "my-agent", min_length: 1 }`
2. `QuestionAnswer { question: "What is the primary task this agent should handle?", placeholder: "e.g. Write Rust CLI tools" }`
3. `QuestionAnswer { question: "What tone or style should the agent use?", placeholder: "e.g. Concise, professional" }`
4. `QuestionAnswer { question: "Are there any specific constraints or rules?", placeholder: "e.g. Always run cargo fmt" }`
5. `Checklist { title: "Select Skills", options: ... }` — populated from active vault scan.
6. `Checklist { title: "Select MCP Servers", options: ... }` — populated from `McpRegistry`.
7. `Review { title: "Review & Confirm" }`
8. `Interactive { title: "Create Agent", command: "opencode", args: vec!["agent","create",...] }`

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

Already implemented:

```rust
pub enum TabKind {
    Asset,
    Vault,
    Provider,
    Mcp,
    Analytics,
    Profile,
}
```

### `ListMode` simplification

**Old (hard-coded per step):**

```rust
ProfileWizardName,
ProfileWizardDescription,
ProfileWizardSelectSkills { ... },
ProfileWizardSelectMcps { ... },
ProfileWizardConfirmCreate,
```

**New (single variant + stack):**

```rust
ProfileWizard,
```

All sub-step behaviour is delegated to `state.wizard_state.step_index` and `state.wizard_state.steps[current]`.

### `AppState` extension

Replace the five old `pending_profile_*` fields with one `wizard_state: Option<WizardState>`.

```rust
pub wizard_state: Option<WizardState>,
```

### Render dispatch (`tui/render.rs`)

When `list_mode == ListMode::ProfileWizard`:

```rust
if let Some(ref ws) = state.wizard_state {
    let step = &ws.steps[ws.step_index];
    match step {
        WizardStep::TextInput { title, .. } => render_text_input_modal(...),
        WizardStep::QuestionAnswer { question, .. } => render_text_input_modal(...),
        WizardStep::Checklist { title, .. } => render_checklist_modal(...),
        WizardStep::Review { .. } => render_review_modal(...),
        _ => {}
    }
}
```

### Event handling (`tui/event.rs`)

When `list_mode == ListMode::ProfileWizard`:

```rust
if let Some(ref mut ws) = state.wizard_state {
    match ws.steps[ws.step_index] {
        WizardStep::TextInput { .. } | WizardStep::QuestionAnswer { .. } => {
            handle_text_input(state, ctx, key_code)?
        }
        WizardStep::Checklist { .. } => handle_checklist(state, ctx, key_code)?,
        WizardStep::Review { .. } => handle_review(state, ctx, key_code)?,
        WizardStep::Interactive { .. } => handle_interactive(state, ctx, key_code)?,
    }
}
```

`Esc` decrements `step_index` if > 0; if == 0, cancels wizard and clears `wizard_state`.

`Enter` either:
- stores the typed buffer into the correct `WizardState` accumulator field, then increments `step_index`;
- or, on the last step (Interactive), emits `AppEvent::RunInteractiveProcess`.

### Profile creation wizard flow

Because `opencode agent create` is an interactive TUI command, we **cannot** embed it inside the AGK TUI directly (nested TUIs break terminal state). The last `WizardStep::Interactive` therefore works like this:

1. AGK TUI saves `wizard_state` data, then suspends its TUI (leave alternate screen).
2. AGK runs `opencode agent create` as a child process, letting it take over the terminal.
3. After `opencode` finishes, AGK TUI re-initializes (`enable_raw_mode`, `EnterAlternateScreen`).
4. AGK locates the newly created `.opencode/agents/*.md` file, moves it to `.agk/profiles/<name>/agent.md`, and updates `config.toml`.

This is analogous to how `opencode agent create` is already used standalone.

## CLI Integration

### Profile subcommand tree (clap)

```rust
pub enum ProfileCommands {
    Start { name: String },
    Create {
        name: String,
        provider: String,
        skills: Vec<String>,
        mcps: Vec<String>,
        description: Option<String>,
        description_file: Option<String>,
        scope: ScopeArg,
    },
}
```

### `profile start`

Already implemented. Loads profile, delegates session setup to `ProviderPort::start_profile_session`, blocks on child process, then runs cleanup.

### `profile create`

Headless creation for providers that support it. OpenCode v1 flow:

1. **Validate** — Provider must be registered and return `supports_profiles() == true`.
2. **Check uniqueness** — `config.find_profile(name)` must be `None` in the target scope.
3. **Write config** — Push a new `Profile` with provided `skills`, `mcps`, `provider_id` into `ConfigFile` and save.
4. **Generate agent** — Spawn `opencode agent create --name <name> --description <desc> --mode primary` and wait for exit.
5. **Relocate markdown** — Scan `.opencode/agents/` for the newest `.md` file (in case exact filename differs), copy it to `.agk/profiles/<name>/agent.md`.
6. **Report** — Print the destination path on success; print a warning if the markdown file wasn't found.

**Error handling:**
- Duplicate profile name → bail before any side effects.
- Provider not active / no profile support → bail before config write.
- `opencode agent create` exits non-zero → print stderr and return `EXIT_GENERAL_FAILURE`.
- Generated markdown missing → warn but don't fail (profile config is already persisted).

## Files to Modify

### Core changes for this refactor

- `src/tui/app.rs`
  - Replace `ListMode` hard-coded variants with `ProfileWizard`.
  - Replace old `pending_profile_*` fields with `wizard_state: Option<WizardState>`.
- `src/tui/event.rs`
  - Replace `handle_profile_wizard_input` hard-coded match with `WizardState` dispatch.
- `src/tui/render.rs`
  - Replace profile wizard render branches with `WizardState` dispatch.
- `src/app/ports.rs`
  - Add `ProfileWizard` trait, `WizardStep` enum, `WizardState` struct.
  - Add `as_profile_wizard()` to `ProviderPort`.
- `src/infra/provider/opencode.rs`
  - Implement `ProfileWizard` for `OpenCodeProvider`.
  - Return the 8-step wizard sequence.
- `docs/product/features/profiles/technical_design.md`
  - This document.

## Testing Strategy

1. **Unit tests:**
   - `Profile` serialization round-trip in `config.rs`.
   - `OpenCodeProfileProvider` computes correct `opencode.json` merge/diff.
   - Session cleanup restores original `opencode.json` state exactly.
   - `WizardState` step navigation (next/back/cancel).

2. **Integration tests:**
   - `agk p test-profile` with a fake `opencode` binary that exits immediately.
   - Assert no leftover `.opencode/agents/test-profile_*.md`.
   - Assert `opencode.json` unchanged or deleted.

3. **Manual tests:**
   - TUI `[5] Profiles` tab renders.
   - F2 wizard prompts name, Q&A, skills, MCPs, review, then invokes `opencode agent create`.
   - Launch and exit a real OpenCode session; verify cleanup.

## Notes

- `WizardStep` is `Clone + PartialEq` so `AppState` can stay `Clone` if needed. It intentionally carries no mutable UI state — that lives in `WizardState`.
- The random 6-digit suffix is intentionally simple (not a UUID) because it only needs to be unique within the current workspace for a single session.
- `opencode.json` is JSON, not JSONC, during profile manipulation; we use `serde_json` directly and preserve any existing JSONC comments via a round-trip parse → strip-comments → modify → write.
- MCP server definitions are read from `McpRegistry` (global), but their enabled state is written into the workspace `opencode.json` by the provider's `ProfileProvider` implementation.
