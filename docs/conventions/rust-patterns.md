# Rust Patterns (AGK Edition)

> Adapted for AGK from `ECC/rules/rust/patterns.md`. Patterns reframed to match our ports + feature-slice architecture; SQL/HTTP-specific patterns dropped.

## Port Pattern (our take on Repository + Service)

Encapsulate every external capability behind a port trait in `app/ports/`. This is AGK's central architectural pattern.

```rust
// app/ports/config_store.rs
pub trait ConfigStorePort: Send + Sync {
    fn load(&self, scope: Scope) -> anyhow::Result<ConfigFile>;
    fn save(&self, scope: Scope, config: &ConfigFile) -> anyhow::Result<()>;
}

// infra/config/toml_store.rs — concrete implementation
pub struct TomlConfigStore { /* ... */ }
impl ConfigStorePort for TomlConfigStore { /* ... */ }

// app/features/profile/create.rs — use case depends only on the trait
pub fn run(
    input: &CreateProfileInput,
    store: &dyn ConfigStorePort,
    sink: &mut dyn CoreEventSink,
) -> CoreResult { /* ... */ }
```

**Rules:**
- Every port trait is `Send + Sync` (so `Arc<dyn Port>` works across `tokio::spawn_blocking`).
- One trait per file in `app/ports/`. Re-export from `app/ports/mod.rs`.
- Concrete implementations live in `infra/<area>/`. The name describes the backing tech (`TomlConfigStore`, `StdProcessRunner`, `OsFileOpener`, `GithubVaultAdapter`).
- Tests use hand-written fakes (in-memory, recording) — see `rust-testing.md`. We do not use `mockall` because our ports are small and explicit.

## Feature Dispatch Pattern (our take on Service Layer)

Use cases don't know about each other or about routing. Each feature exposes a `dispatch()` that matches its `CoreCommand` variants.

```rust
// app/features/profile/mod.rs
pub fn dispatch(
    cmd: CoreCommand,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> Option<CoreResult> {
    match cmd {
        CoreCommand::CreateProfile { input } => {
            Some(create::run(&input, core.store.as_ref(), sink))
        }
        CoreCommand::DeleteProfile { id, scope } => {
            Some(delete::run(&id, scope, core.store.as_ref(), sink))
        }
        _ => None,
    }
}

// app/core.rs — the only place that knows the feature list
pub fn execute(&self, cmd: CoreCommand, sink: &mut dyn CoreEventSink) -> CoreResult {
    if let Some(r) = features::profile::dispatch(cmd.clone(), self, sink) { return r; }
    if let Some(r) = features::vault::dispatch(cmd.clone(), self, sink)   { return r; }
    // ...
    sink.on_error(format!("Command {:?} not yet implemented", cmd));
    Ok(CoreOutcome::Ok)
}
```

**Why:** Adding a feature = adding a directory + one line in `core.rs`. No central match arm to keep in sync.

## Newtype Pattern for Type Safety

Used heavily in AGK to prevent ID mix-ups. Keep using it; resist the urge to "simplify" to plain strings.

```rust
// domain/profile.rs
pub struct ProfileId(pub String);
pub struct SkillId(pub String);
pub struct McpServerId(pub String);

// Call sites cannot accidentally swap IDs
fn attach_skill(profile: &ProfileId, skill: &SkillId, /* ... */) { /* ... */ }
```

## Enum State Machines (with exhaustive matching)

Model UI mode and command state as enums. Match exhaustively — no wildcard `_` for business-critical enums; let the compiler nag you when you add a variant.

```rust
pub enum ListMode {
    Normal,
    AttachVault,
    AttachVaultBranch,
    RegisterMcpStepName,
    // ... 20+ variants
    ProfileWizard,
}

// GOOD — exhaustive match catches new modes at compile time
match state.list_mode {
    ListMode::Normal => render_list(state, frame),
    ListMode::AttachVault | ListMode::AttachVaultBranch | /* ... */ => render_modal(state, frame),
    ListMode::ProfileWizard => render_wizard(state, frame),
}

// BAD — wildcard hides missing handlers
match state.list_mode {
    ListMode::Normal => render_list(state, frame),
    _ => render_modal(state, frame),  // silently swallows new variants
}
```

The exception: in `app/features/<f>/mod.rs::dispatch()`, the `_ => None` arm is intentional — each feature handles only its own variants and lets the dispatch chain try the next feature.

## Builder Pattern for Many-Field Inputs

Used for `CreateProfileInput`, `ApplyConfigInput`, etc. — when an input struct has 4+ fields, half of them optional.

```rust
// app/features/profile/command.rs
pub struct CreateProfileInput {
    pub id: ProfileId,
    pub provider_id: ProviderId,
    pub skill_refs: Vec<SkillId>,
    pub mcp_refs: Vec<McpServerId>,
    pub instruction_refs: Vec<InstructionId>,
    pub description: String,
    pub scope: Scope,
}

impl CreateProfileInput {
    pub fn new(
        id: impl Into<ProfileId>,
        provider_id: impl Into<ProviderId>,
        scope: Scope,
    ) -> Self {
        Self {
            id: id.into(),
            provider_id: provider_id.into(),
            skill_refs: vec![],
            mcp_refs: vec![],
            instruction_refs: vec![],
            description: String::new(),
            scope,
        }
    }

    pub fn with_skill(mut self, id: impl Into<SkillId>) -> Self {
        self.skill_refs.push(id.into());
        self
    }
}
```

Keep the builder methods on the struct itself — we don't need a separate `Builder` type until the struct grows past ~10 fields.

## Sealed Traits for Adapter Identity

Use `pub(crate)` super-traits to prevent external implementations of internal ports. AGK ports that should only be implemented inside our own `infra/`:

```rust
// app/ports/mod.rs
mod sealed {
    pub trait Sealed {}
}

pub trait VaultPort: sealed::Sealed + Send + Sync {
    fn id(&self) -> &str;
    // ...
}

// infra/vault/github.rs
impl crate::app::ports::sealed::Sealed for GithubVaultAdapter {}
impl VaultPort for GithubVaultAdapter { /* ... */ }
```

Apply this sparingly — only to ports where downstream extension would break invariants (e.g., `VaultPort` because `bootstrap` enumerates them).

## CoreEvent Sink Pattern (AGK-specific)

All use cases emit results via a `&mut dyn CoreEventSink`. Adapters implement the sink in their own style:

```rust
// app/outcome.rs
pub trait CoreEventSink {
    fn on_event(&mut self, event: CoreEvent);
    fn on_error(&mut self, error: String);
}

// cli/presenter.rs
impl CoreEventSink for CliPresenter {
    fn on_event(&mut self, e: CoreEvent) { /* println! or JSON */ }
    fn on_error(&mut self, msg: String) { /* eprintln! */ }
}

// tui/presenter.rs
impl CoreEventSink for TuiPresenter {
    fn on_event(&mut self, e: CoreEvent) {
        let _ = self.tx.send(AppEvent::CoreEvent(e));
    }
    fn on_error(&mut self, msg: String) {
        let _ = self.tx.send(AppEvent::CoreEvent(CoreEvent::Error(msg)));
    }
}

// tests use NullSink (no-op) or RecordingSink (asserts)
```

## What NOT to Do

- ❌ Direct `&dyn Concrete` parameters when a `&dyn Port` works — keep use cases unaware of adapter types.
- ❌ Generic `Box<dyn Service>` wrappers around a port — `Arc<dyn Port>` is enough; `Box` prevents sharing.
- ❌ Service objects that hold mutable state (`self.cache: Mutex<...>`) inside use cases — caching belongs in adapters.
- ❌ Returning `Result<CoreEvent>` from a use case — emit via `sink.on_event(...)` and return `CoreResult`.
- ❌ Splitting one feature's logic across `app/features/` AND `app/utils/` — keep it co-located.
