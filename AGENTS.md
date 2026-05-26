# AGENTS.md

This file provides guidance to AI coding agents (Claude Code, GitHub Copilot, Gemini CLI, etc.) when working with code in this repository.

## Build & Development Commands

```bash
cargo build              # Build
cargo run                # Run TUI
cargo test               # Run all tests

cargo fmt --check        # Check formatting (CI enforced - MUST PASS)
cargo fmt                # Auto-format
cargo clippy -- -D warnings  # Lint (treat warnings as errors)
```

> **Formatting is enforced.** Run `cargo fmt` before every commit. CI will reject unformatted code.

CI (.github/workflows/ci.yml) runs `cargo check`, `cargo fmt --check`, and `cargo test --verbose` on push to master and PRs.

## Architecture

Hexagonal (Ports & Adapters) architecture with four layers:

```
TUI (tui/)  →  App (app/)  →  Domain (domain/)
                   ↓
              Infra (infra/)
```

- **domain/**: Pure data models — no I/O. AssetIdentity, ConfigFile, Scope, ScannedPackage, hashing.
- **app/**: Business logic orchestration. `ports.rs` defines the four core traits. `bootstrap.rs` is the composition root (only place infra is wired). `actions.rs` has reusable operations.
- **infra/**: I/O adapters implementing port traits. Vault backends (local, github, clawhub), provider installers (Claude Code, Copilot, Gemini, etc.), TOML config store.
- **tui/**: Ratatui-based UI. `app.rs` holds reactive AppState. `event.rs` maps keycodes to actions. Background tasks use `tokio::sync::mpsc::UnboundedSender<AppEvent>`.

### Core Port Traits (app/ports.rs)

- `FeatureSetPort` — defines how to scan a package type (skills vs instructions)
- `VaultPort` — vault source abstraction (id, list_packages, refresh)
- `ProviderPort` — target AI platform installer (install, remove)
- `ConfigStorePort` — scoped config persistence (Global vs Workspace)
- `McpRegistryPort` — MCP server registration and lifecycle (register, enable, disable)
- `VaultSearchPort` — remote vault search abstraction (search, vault_id)
- `ProfileRuntimePort` — provider-specific profile session builder (build_launch_plan, run_plan)

### Core Command / Event Contracts (app/command.rs, app/event.rs, app/outcome.rs)

**CoreCommand** — what the user wants.  One enum consumed by `AgkCore::execute()`.
**CoreEvent** — what happened.  Emitted via `CoreEventSink` so TUI and CLI observe the same facts.
**CoreOutcome** — the return value from a use-case (Ok, LaunchPlan, ValidationReport, etc.).
**UiIntent** — what the TUI intends to do next.  Produced by pure `reduce_key()` in `tui/reducer.rs`.

### Key Patterns

- **SHA10 hashing** for asset change detection, not semantic versions. Version is display metadata; sha10 is the source of truth for freshness.
- **Scoped config**: Global (`~/.config/agk/config.toml`) for vaults/providers, Workspace (`.agk/config.toml`) for installed assets.
- **Async I/O**: All network/git operations run on tokio tasks via `AppEvent` channel to keep TUI responsive. Never block the render loop.
- **Bootstrap is the only DI point**: `app/bootstrap.rs` wires infra adapters. No infra imports outside this file and main.rs.
- **ClawHub vault**: CLI-delegated — shells out to `clawhub` binary for search/install/inspect. Uses LocalVaultAdapter to scan its cache at `~/.config/agk/clawhub/`.
- **Pure reducer pattern**: `tui/reducer.rs` contains the only key-event logic; it returns `Vec<UiIntent>` without side effects. `tui/command_mapper.rs` translates intents to `CoreCommand`s.
- **Shared contracts**: Both TUI and CLI produce `CoreCommand`s and observe `CoreEvent`s through the same `AgkCore` façade. `tui/presenter.rs` bridges `CoreEventSink` → `AppEvent` channel.
- **Profile runtime separation**: `ProfileRuntimePort::build_launch_plan()` produces a deterministic `LaunchPlan` (no side effects). `run_plan()` executes it and returns a `ProfileSession` with a cleanup closure.

### Vault Structure Convention

Skills require `SKILL.md`, instructions require `AGENTS.md` as the marker file within their directory under `skills/` or `instructions/`.

## Documentation Requirements

When implementing a new feature or modifying an existing one, always update the corresponding documentation under `docs/product/features/`. Each feature area must have both files:

- `prd.md` — Product requirements: what the feature does, user-facing behavior, functional requirements
- `technical_design.md` — Technical design: trait contracts, data schemas, internal workflows, architecture rules

If adding a new feature area, create a new directory under `docs/product/features/<feature-name>/` with both files.

## Working with Worktrees

Feature branches often use git worktrees at `.worktrees/<branch-name>/`. Code changes in a worktree are isolated from the main working directory — remember to `cd` into the worktree or use its path when building/testing.

## Refactoring History

### Completed Refactors (branch `feature/core-commands`)

**Phase 0** (merged): Added dev-deps, moved `TabKind` from `tui/` → `app/`, extracted view models to `app/snapshot.rs`, created architecture enforcement tests.

**Phase 1** (merged): Introduced `CoreCommand`, `CoreEvent`, `CoreOutcome`, `UiIntent`, and `AgkCore` façade. Domain `Profile` with typed IDs.

**Phase 2** (merged): Pure `reduce_key()` in `tui/reducer.rs`, `TuiState`/`ListMode`/`WizardState` in `tui/app_state.rs`, `command_mapper.rs`. 12 reducer unit tests.

**Phase 3** (merged): Use-case extraction — `attach_vault`, `deactivate_provider`, `register_mcp`, `search_remote_vault` with fake-port tests.

**Phase 3.5** (merged): Wired `AgkCore` with real port injection (`ConfigStorePort`, `McpRegistryPort`, `VaultSearchPort`, `Registry`). Created `InfraMcpRegistryAdapter`, `ClawHubSearchAdapter`, `AppEventSink` presenter bridge.

**Phase 4+4.5** (merged): `CliPresenter` with `--json`/`--quiet`, `cli/core_dispatcher.rs` routing commands through `AgkCore`, added `dry_run` to `ProfileCommands::Create`.

**Phase 5** (merged): `ProfileRuntimePort` trait, real `LaunchPlan` with provider-specific fields, `start_profile` use-case resolves profile from `ConfigStorePort` and runtime port from registry.

## TDD Templates

When adding a new use-case in `app/usecases/<name>.rs`:

```rust
pub fn run(
    // typed inputs
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    // implementation
    Ok(CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::outcome::NullSink;

    #[test]
    fn use_case_happy_path() {
        let mut sink = NullSink;
        let result = run(/* inputs */);
        assert!(result.is_ok());
    }
}
```

When adding a new port trait in `app/ports.rs`:

```rust
pub trait NewPort: Send + Sync {
    fn port_id(&self) -> &str;
    fn operation(&self, input: &Input) -> Result<Output>;
}
```

When adding a new reducer intent in `tui/reducer.rs`:

```rust
fn derive_enter_intent(state: &TuiState) -> UiIntent {
    // pattern match on state.list_mode / state.tab_index
    // return UiIntent::Command(CoreCommand::...)
}
```

## File Size & Splitting Guidelines

**Rule of Thumb**: No file should exceed ~250-300 lines (excluding tests and imports).

### When to Split a File / Function

- **Split if**:
  - The file has more than one major responsibility.
  - A function has > 50 lines or > 3 levels of nesting.
  - A `match` statement has > 8 arms.
  - The file contains both UI logic and business logic.
  - The file directly calls `infra/` from `tui/` or `cli/`.

- **Examples**:
  - `tui/event.rs` → Split into `reducer.rs` + `intent.rs` + `command_mapper.rs` + `presenter.rs`.
  - A use-case doing > 3 things → Split into smaller use-cases or private functions.
  - Provider installer with install + config + wizard → Split into `install.rs`, `config.rs`, `wizard.rs` inside `infra/provider/opencode/`.

### Preferred Patterns

1. **TUI Pattern** (Elm-inspired):
   - Reducer: Pure key → intents
   - Mapper: Intent → CoreCommand
   - Presenter: CoreEvent → UI State

2. **Use Case Pattern**:
   - One file per use case (`app/usecases/attach_vault.rs`)
   - Small, testable, focused on one business action.

3. **Naming Convention**:
   - `handle_xxx` → Only allowed in reducer or very thin presenter glue.
   - Business logic → Must go to `app/usecases/`
