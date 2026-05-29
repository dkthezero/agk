# Rust Coding Style

> Adapted for AGK from `ECC/rules/rust/coding-style.md`. Source rules tightened to match our hexagonal architecture (ports, feature slices, `AgkCore`).

## Formatting

- **rustfmt** for enforcement — always run `cargo fmt` before committing (CI rejects unformatted code).
- **clippy** for lints — `cargo clippy --workspace --all-targets --all-features -- -D warnings` (treat warnings as errors).
- 4-space indent, max line width 100 chars (rustfmt defaults).

## Immutability

Rust variables are immutable by default — embrace this:

- Use `let` by default; only use `let mut` when mutation is required.
- Prefer returning new values over mutating in place.
- For `AppState` (TUI), mutation is centralized in the runtime loop — controllers compute new values and emit commands, they don't mutate state directly.

```rust
// GOOD — immutable controller, emits a command
pub fn handle_attach(state: &AppState, ctx: &EventContext) -> Result<ControlFlow> {
    let cmd = CoreCommand::AttachVault {
        input: AttachVaultInput {
            vault_id: state.pending_vault_id.clone(),
            config: VaultConfig::from(&state.pending_vault_url),
            scope: state.active_scope,
        },
    };
    let _ = ctx.tx.send(AppEvent::ExecuteCommand(cmd));
    Ok(ControlFlow::Continue)
}
```

## Naming

Standard Rust conventions:
- `snake_case` — functions, methods, variables, modules
- `PascalCase` — types, traits, enums, type parameters
- `SCREAMING_SNAKE_CASE` — constants and statics
- Lifetimes — short lowercase (`'a`); descriptive only for complex cases (`'core`, `'sink`)

AGK-specific conventions:
- `run` — single entry point per use case in `app/features/<f>/<verb>.rs`
- `dispatch` — feature-level `CoreCommand` router in `app/features/<f>/mod.rs`
- `to_core_command` — CLI arg mapper in `cli/features/<f>.rs`
- `handle_*` — TUI controller keystroke handlers in `tui/features/<f>/controller.rs`
- Port traits — end in `Port` (e.g. `ConfigStorePort`, `ProcessRunnerPort`)
- Port impls — describe the backing tech (e.g. `TomlConfigStore`, `StdProcessRunner`)

## Ownership and Borrowing

- Borrow (`&T`) by default; take ownership only when you need to store or consume.
- Never clone to satisfy the borrow checker without understanding the root cause.
- Function params: prefer `&str` over `&String`, `&[T]` over `&Vec<T>`, `impl AsRef<Path>` over `&Path` for ergonomic APIs.
- Use `impl Into<String>` in constructors that need an owned `String`.
- `Arc<dyn Port>` for shared port handles (every port is `Send + Sync`); never `Rc<dyn Port>`.

```rust
// GOOD — borrows what it needs, owns what it stores
impl AgkCore {
    pub fn new(
        store: Arc<dyn ConfigStorePort>,
        registry: Arc<Registry>,
        runtime_ports: HashMap<String, Arc<dyn ProfileRuntimePort>>,
    ) -> Self {
        Self { store, registry, runtime_ports, /* ... */ }
    }
}
```

## Error Handling

- AGK is an **application**, so use `anyhow::Result` for flow-through errors.
- Add context with `.with_context(|| format!("failed to ..."))?` — never propagate bare `?` from I/O without context.
- Use `?` for propagation; never `unwrap()` in production code.
- Reserve `unwrap()` / `expect()` for tests, `lazy_static`-style init, and truly unreachable states.
- Use case functions return `CoreResult = anyhow::Result<CoreOutcome>`; errors are also emitted via `sink.on_error(...)`.

```rust
use anyhow::Context;

// GOOD — context at every I/O boundary
pub fn run(
    input: &CreateProfileInput,
    store: &dyn ConfigStorePort,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let mut config = store
        .load(input.scope)
        .with_context(|| format!("failed to load {:?} config", input.scope))?;
    config.profiles.push(Profile::from(input));
    store
        .save(input.scope, &config)
        .with_context(|| format!("failed to save {:?} config", input.scope))?;
    sink.on_event(CoreEvent::ProfileCreated { id: input.id.clone() });
    Ok(CoreOutcome::Ok)
}
```

## Iterators Over Loops

Prefer iterator chains for transformations; use loops for complex control flow:

```rust
// GOOD — declarative
let active: Vec<&dyn ProviderPort> = registry
    .providers
    .iter()
    .filter(|p| config.providers.contains(&p.id().to_string()))
    .map(|p| p.as_ref())
    .collect();
```

## Module Organization

Organize by **domain feature**, not by technical type. In AGK:

- `app/features/profile/` — all profile use cases together (not split into `commands/`, `services/`, `dtos/`).
- `app/ports/` — one trait per file, named for the capability (not "interfaces").
- `infra/<area>/` — adapters grouped by the technical area they wrap (`process/`, `vault/`, `mcp/`).

## Visibility

- Default to private; promote to `pub(crate)` for internal sharing across modules in the same crate.
- Only mark `pub` what is part of the binary's documented surface (re-exported from `lib.rs` if/when AGK exposes one).
- Feature internals should be `pub(super)` or `pub(crate)` — never `pub` unless an adapter directly imports them.

## What NOT to Do

- ❌ `unwrap()` in production code paths — it's a panic.
- ❌ `.clone()` to satisfy the borrow checker without understanding why.
- ❌ `String` parameters when `&str` suffices.
- ❌ Direct `std::fs` / `std::process` calls outside `infra/` (and `main.rs`).
- ❌ Hidden mutation inside "pure" helpers — make mutation visible at the call site.
- ❌ Generic catch-all errors (`anyhow::anyhow!("error")`) without context.
