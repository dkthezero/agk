# AGENTS.md

This file is the **Agent Harness** for AGK. It defines how both human contributors and AI coding agents must work in this repository to preserve architectural integrity, product vision, and long-term maintainability.

**Core Rule:** Never add business logic in `cli/` or `tui/`. Adapters only translate intent and render results. All behavior lives in the Application Core (`app/`).

---

## Product Vision & Charter

**AGK is the standard, lightweight way to define, share, and launch AI coding environments across solo, team, and enterprise contexts.**

### Core Promises
- **Portable intent**: Take a local or remote manifest and materialize a reproducible AI coding environment.
- **Headless-first**: Every interactive flow must have a complete headless/CLI equivalent (or `--dry-run` contract).
- **Lightweight**: Heavy subsystems (TUI, remote vaults, YAML, enterprise features) must be optional via Cargo features.
- **Profiles as compositions**: Profiles reference (do not duplicate) skills, instructions, providers, vaults, and MCPs.
- **Multi-provider**: Support Claude Code, OpenCode, Gemini, Copilot, and others without vendor lock-in.

**Primary users**: Solo engineers who want fast, repeatable setups.  
**Secondary users**: Platform teams standardizing AI workflows across repositories and organizations.

---

## Architecture (Hexagonal / Ports & Adapters)

We follow a **hybrid horizontal + feature-slice** structure. Preserve the existing top-level roots.

```
TUI (tui/)  →  App (app/)  →  Domain (domain/)
                   ↓
              Infra (infra/)
                  ↑
            CLI (cli/)
```

### Repository Layout (Target)

```
src/
├── main.rs                          # Pure composition root
├── app/
│   ├── bootstrap/                   # ONLY place where concrete infra is wired
│   │   ├── mod.rs
│   │   ├── registry.rs
│   │   ├── scan.rs
│   │   └── state.rs
│   ├── core.rs                      # AgkCore façade, feature-dispatched execute()
│   ├── command.rs                   # CoreCommand enum (centralized)
│   ├── event.rs                     # CoreEvent enum (centralized)
│   ├── outcome.rs                   # CoreOutcome, CoreEventSink, NullSink
│   ├── snapshot.rs                  # UI-oriented view models
│   ├── registry.rs                  # Adapter registry
│   ├── tab_kind.rs                  # Shared tab classification
│   ├── ports/                       # One file per port trait
│   │   ├── mod.rs
│   │   ├── config_store.rs
│   │   ├── context_store.rs
│   │   ├── feature_set.rs
│   │   ├── vault.rs
│   │   ├── provider.rs
│   │   ├── mcp_registry.rs
│   │   ├── profile_runtime.rs
│   │   ├── process_runner.rs
│   │   ├── file_opener.rs
│   │   └── telemetry_store.rs
│   └── features/                    # Vertical feature slices
│       ├── profile/                 # mod.rs (dispatch), command.rs, create.rs, ...
│       ├── vault/
│       ├── asset/
│       ├── provider/
│       ├── mcp/
│       ├── context/
│       └── apply/
├── domain/                          # Pure models — NO std::fs, NO std::process
│   ├── asset.rs
│   ├── config.rs
│   ├── context.rs
│   ├── hashing.rs                   # Pure: accepts bytes, not paths
│   ├── identity.rs
│   ├── mcp.rs
│   ├── paths.rs                     # Path computation only
│   ├── profile.rs
│   ├── scope.rs
│   ├── telemetry.rs                 # Models only — I/O lives in infra/telemetry
│   └── validation.rs
├── infra/                           # Concrete adapters (file, HTTP, process)
│   ├── config/
│   ├── context/
│   ├── feature/
│   ├── mcp/
│   ├── provider/
│   ├── telemetry/
│   │   └── store.rs                 # TelemetryStorePort impl
│   ├── vault/
│   │   └── factory.rs               # VaultFactoryPort impl
│   └── process/
│       ├── runner.rs                # ProcessRunnerPort (std::process::Command)
│       └── opener.rs                # FileOpenerPort (OS open command)
├── cli/                             # Thin CLI adapter
│   ├── entry.rs                     # Cli struct, Clap subcommands
│   ├── core_dispatcher.rs           # Routes ALL CLI commands → AgkCore
│   ├── presenter.rs                 # CliPresenter (CoreEventSink impl)
│   └── features/                    # Per-feature CoreCommand mappers
│       ├── profile.rs
│       ├── asset.rs
│       ├── mcp.rs
│       ├── context.rs
│       ├── apply.rs
│       └── telemetry.rs
└── tui/                             # Thin TUI adapter (Ratatui)
    ├── entry.rs                     # build_state(), terminal setup
    ├── app.rs                       # AppState
    ├── list_mode.rs                 # ListMode enum (split from app.rs)
    ├── progress.rs                  # Progress / ProgressStatus
    ├── event.rs                     # Top-level keyboard dispatch
    ├── runtime_loop.rs              # Async loop; spawn_blocking → AgkCore
    ├── reload.rs                    # compute_reload_snapshot()
    ├── presenter.rs                 # TuiPresenter (CoreEventSink impl)
    ├── layout.rs
    ├── render/                      # Rendering subsystems
    ├── widgets/                     # Reusable widgets
    └── features/                    # Per-feature controllers (zero infra imports)
        ├── profile/
        ├── vault/
        ├── asset/
        ├── provider/
        ├── mcp/
        ├── context/
        └── common/
```

### Dependency Rules (Enforced by `tests/architecture.rs`)

| # | Rule | Enforcement |
|---|------|-------------|
| 1 | `domain/` depends on nothing outside `domain/` | Architecture test + code review |
| 2 | `domain/` must NOT use `std::fs` or `std::process` (pure models only) | Architecture test |
| 3 | `app/` depends only on `domain/` and its own ports/contracts | Architecture test |
| 4 | `infra/` depends on `domain/` + `app::ports` only | Architecture test |
| 5 | `cli/` and `tui/` depend on `app/` (commands, events, snapshots) + `domain/`. **Never** on `infra/` | Architecture test |
| 6 | Only `main.rs` and `app/bootstrap/` may construct concrete adapters | Grep arch tests + review |
| 7 | `std::process::Command` lives only in `infra/process/` (and `main.rs` for exit codes) | Architecture test |
| 8 | All `CoreCommand` variants must be reachable from BOTH `cli/` and `tui/` | Contract parity test |
| 9 | `app/features/<f>/` may not import other features' internals | Architecture test |
| 10 | No file owns logic for more than one feature (split at ~300 LOC) | CI file-size lint + review |

**Zero-allowlist policy:** When an architecture test needs an allowlist exemption, that exemption is a TODO with a tracked removal commit — never a permanent escape hatch. The allowlist must shrink over time, never grow.

---

## Development Workflow (Mandatory for Agents & Humans)

**Always follow this order** — this is the most important section for keeping agents on rails.

1. **Product Documentation First**
   - Update or create `docs/product/features/<feature>/prd.md`
   - Update or create `docs/product/features/<feature>/technical_design.md`

2. **Domain Layer**
   - Add/update pure models and invariants in `domain/`
   - Domain code must NOT call `std::fs` or `std::process` — if you need bytes from a file, accept them as a parameter
   - Write domain unit tests

3. **Application Core**
   - Implement or extend use cases in `app/features/<feature>/<verb>.rs` (one file per verb: `create.rs`, `delete.rs`, etc.)
   - Place feature-specific input structs in `app/features/<feature>/command.rs`
   - Wire the `CoreCommand` variant into `app/features/<feature>/mod.rs::dispatch()`
   - Add use-case tests using fake ports (`NullSink`, in-memory stores)
   - Extend `CoreCommand`, `CoreEvent`, `CoreOutcome` (in `app/command.rs`, `app/event.rs`, `app/outcome.rs`) as needed

4. **Ports & Infra**
   - Define new port traits in `app/ports/<port_name>.rs` (one trait per file; re-export from `app/ports/mod.rs`)
   - Implement adapters in `infra/<area>/`
   - Wire the adapter into `app/bootstrap/state.rs`

5. **CLI Adapter**
   - Add Clap subcommand parsing in `cli/entry.rs`
   - Implement `to_core_command()` in `cli/features/<feature>.rs` — map CLI args → `CoreCommand`
   - Route through `cli/core_dispatcher.rs` → `AgkCore::execute()`
   - CLI files may NOT call `infra/` directly or own business logic

6. **TUI Adapter** (Last)
   - Add a controller function in `tui/features/<feature>/controller.rs`
   - Controllers emit `ctx.tx.send(AppEvent::ExecuteCommand(CoreCommand::...))` — never call `infra/` or `app/features/*` usecases directly
   - The runtime loop dispatches the command via `spawn_blocking` with a `TuiPresenter` sink
   - Resulting `CoreEvent`s arrive back as `AppEvent::CoreEvent(...)` and mutate `AppState`
   - Add widget rendering in `tui/features/<feature>/widget.rs` or `tui/render/`
   - TUI files may NOT call `infra/` directly or own business logic

### Contract-First Rule (for interactive flows)

- Every interactive flow must have a `--dry-run --json` equivalent that produces the same `CoreEvent` sequence as the TUI path.
- Export golden fixtures (`fixtures/contracts/`) from the CLI dry-run output.
- Use those fixtures to drive contract tests that exercise both adapters.
- Contract test pattern: run the same `CoreCommand` through CLI presenter and TUI presenter, assert identical `CoreEvent` sequences.

---

## Rust Conventions (Authoritative for All Rust Code)

Detailed, AGK-specific Rust rules live in `docs/conventions/`. Adapted from `ECC/rules/rust/*` and reframed to match our hexagonal architecture. Read the relevant file before writing Rust in that area.

| Topic | File | Read when… |
|---|---|---|
| **Coding style** | [`docs/conventions/rust-coding-style.md`](docs/conventions/rust-coding-style.md) | Naming, ownership, error handling, module organization, visibility |
| **Patterns** | [`docs/conventions/rust-patterns.md`](docs/conventions/rust-patterns.md) | Adding a port, writing a use case, building a `CoreCommand` variant, modelling state |
| **Security** | [`docs/conventions/rust-security.md`](docs/conventions/rust-security.md) | Touching secrets, process spawning, user input, paths, dependencies |
| **Testing** | [`docs/conventions/rust-testing.md`](docs/conventions/rust-testing.md) | Writing any test — explains the six-layer strategy and our hand-fake (no-mock) policy |

**Key non-obvious conventions from those files:**
- Naming: `run` (use case), `dispatch` (feature router), `to_core_command` (CLI mapper), `handle_*` (TUI controller).
- Ports: every trait ends in `Port`, every impl describes the backing tech (`TomlConfigStore`, `StdProcessRunner`).
- No mocking libraries — write hand fakes inside the use case's `#[cfg(test)]` module; promote to `src/app/test_support/` if reused 3+ times.
- Exhaustive `match` on `CoreCommand` / `CoreEvent` / `ListMode`; wildcard `_` only in feature `dispatch()` (where it intentionally falls through to the next feature).
- No `unsafe`. If you need it, file an issue first — every block requires a `// SAFETY:` comment naming each invariant.

---

## Search Order for AI Agents

When you need to understand or modify code, **search in this exact order**:

1. `docs/product/features/<feature>/` (PRD + technical design)
2. `src/app/command.rs`, `src/app/event.rs`, `src/app/outcome.rs` (the contract)
3. `src/app/features/<feature>/mod.rs` (dispatch map) and `<verb>.rs` (use case bodies)
4. `src/app/ports/<port>.rs` (trait the use case depends on)
5. `src/infra/<area>/` (concrete adapter implementing the port)
6. `src/cli/features/<feature>.rs` (CLI arg → CoreCommand mapping)
7. `src/tui/features/<feature>/controller.rs` (keystroke → CoreCommand emission)
8. `src/tui/presenter.rs` and `src/tui/runtime_loop.rs` (how CoreEvent reaches AppState)

---

## Key Patterns & Rules

- **One Bus**: All major flows go through `AgkCore::execute(CoreCommand, &mut dyn CoreEventSink)`. Both CLI and TUI share this single entry point.
- **Feature Dispatch**: `app/core.rs::execute()` is a thin chain calling `features::<f>::dispatch()`. Each feature's `mod.rs` owns its `CoreCommand` match arms.
- **Centralized Enum, Distributed Inputs**: `CoreCommand` lives in `app/command.rs`, but feature-specific input structs (e.g. `CreateProfileInput`, `AttachVaultInput`) live in `app/features/<f>/command.rs`.
- **No Business Logic in Adapters**: `cli/` and `tui/` may not mutate config directly, own workflows, or call infra directly. They only translate input and render events.
- **TuiPresenter Bridge**: TUI controllers emit `AppEvent::ExecuteCommand(CoreCommand)`. The runtime loop wraps `AgkCore::execute()` in `tokio::task::spawn_blocking` with a `TuiPresenter` sink, which forwards `CoreEvent` back as `AppEvent::CoreEvent(...)`. AppState mutations only happen in the runtime loop, never inside controllers.
- **Domain Purity**: Domain code must be referentially transparent. No `std::fs`, no `std::process`, no network. If a domain function needs file content, accept it as bytes.
- **Manifest Codecs**: Support both TOML and YAML via `ManifestCodecPort`. Preserve original file extension on rewrite.
- **Profile Start Flow**: Load → Resolve → Build `LaunchPlan` (dry-run) → Execute via `ProfileRuntimePort`.
- **Feature Slicing**: Group new logic under `app/features/<feature>/` and `tui/features/<feature>/`. Files that change together live together.
- **File Size Rule**: Split files before they exceed ~300 logical lines of business logic (enforced in CI).
- **SHA10 Hashing**: Use SHA10 (SHA-256 truncated to 10 hex chars) for asset change detection. Version is display-only.

---

## CI & Quality Gates

- `cargo fmt --check` (enforced)
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Architecture tests must pass (`cargo test --test architecture -- --ignored`)
- Feature matrix builds (`--no-default-features` combinations)
- Contract tests for key flows (especially profile start dry-run)
- File-size lint (~300 LOC rule per non-test business-logic file)

---

## TDD / Implementation Templates

### New Use Case

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

### New Port Trait

```rust
pub trait NewPort: Send + Sync {
    fn port_id(&self) -> &str;
    fn operation(&self, input: &Input) -> Result<Output>;
}
```

### New CLI Feature Mapper

```rust
// src/cli/features/<feature>.rs
use crate::app::command::CoreCommand;
use crate::cli::entry::{Cli, <Feature>Commands};

pub fn to_core_command(
    cli: &Cli,
    cmd: &<Feature>Commands,
    workspace: &std::path::Path,
) -> anyhow::Result<CoreCommand> {
    match cmd {
        <Feature>Commands::DoThing { arg } => Ok(CoreCommand::DoThing { arg: arg.clone() }),
        // ...
    }
}
```

### New TUI Feature Controller (`tui/features/<feature>/`)

```
tui/features/<feature>/
├── mod.rs          # Re-export
├── controller.rs   # Keystroke → AppEvent::ExecuteCommand(CoreCommand)
└── widget.rs       # Rendering / layout helpers for this feature tab
```

Controller pattern (no infra calls, no direct usecase calls):

```rust
pub fn handle_thing(state: &mut AppState, ctx: &EventContext) -> Result<ControlFlow> {
    let cmd = crate::app::command::CoreCommand::DoThing {
        arg: state.pending_arg.clone(),
    };
    let _ = ctx.tx.send(crate::tui::app::AppEvent::ExecuteCommand(cmd));
    Ok(ControlFlow::Continue)
}
```

### New Feature Slice (`app/features/<feature>/`)

```
app/features/<feature>/
├── mod.rs          # dispatch(cmd, core, sink) — match CoreCommand variants for this feature
├── command.rs      # Feature-specific input structs (e.g. CreateProfileInput)
├── <verb>.rs       # One file per use case: create.rs, delete.rs, start.rs, ...
└── planner.rs      # Optional: deterministic planning logic (e.g. LaunchPlan)
```

Feature dispatch pattern:

```rust
// src/app/features/<feature>/mod.rs
pub mod command;
pub mod create;
pub mod delete;

pub fn dispatch(
    cmd: crate::app::command::CoreCommand,
    core: &crate::app::core::AgkCore,
    sink: &mut dyn crate::app::outcome::CoreEventSink,
) -> Option<crate::app::outcome::CoreResult> {
    match cmd {
        crate::app::command::CoreCommand::CreateThing { input } => {
            Some(create::run(input, core.store.as_ref(), sink))
        }
        crate::app::command::CoreCommand::DeleteThing { id } => {
            Some(delete::run(&id, core.store.as_ref(), sink))
        }
        _ => None,
    }
}
```

---

## Testing Strategy

### Layer 1: Domain Tests
- Protect invariants in `domain/`.
- Pure functions, no I/O mocks needed.

### Layer 2: Use-Case Tests
- Protect behavior in `app/features/<f>/<verb>.rs`.
- Use fake port implementations (`NullSink`, in-memory stores).
- Each use case should have at least one happy-path test exercising the `CoreEventSink` output.

### Layer 3: Contract Tests
- Protect CLI/TUI equivalence — both adapters must produce identical `CoreEvent` sequences for the same `CoreCommand`.
- Use `--dry-run --json` golden fixtures.
- Example: `profile_start_dry_run_matches_contract_fixture`, `tui_cli_equivalence_<command>`.

### Layer 4: Snapshot Tests
- Protect TUI rendering shapes using `ratatui::TestBackend` + `insta`.

### Layer 5: Binary Integration Tests
- Small number of real-workspace flows using `assert_cmd`.

### Layer 6: Architecture Tests
- `tests/architecture.rs` enforces dependency rules mechanically.
- Run with `cargo test --test architecture -- --ignored`.
- Must pass with **zero allowlist exemptions** as the codebase converges.
- Includes: `agk_core_is_send_sync` (thread safety), `domain_must_not_use_fs`, `domain_must_not_spawn_processes`, `tui_must_not_import_infra`.

---

## Build & Development Commands

```bash
cargo build              # Build
cargo run                # Run TUI
cargo test               # Run all tests (except ignored arch tests)

cargo fmt --check        # Check formatting (CI enforced — MUST PASS)
cargo fmt                # Auto-format
cargo clippy -- -D warnings  # Lint (treat warnings as errors)
```

> **Formatting is enforced.** Run `cargo fmt` before every commit. CI will reject unformatted code.

**Run architecture tests explicitly**:
```bash
cargo test --test architecture -- --ignored
```

**Run specific contract test**:
```bash
cargo test profile_start_dry_run_matches_contract_fixture -- --nocapture
```

---

## Vault Structure Convention

Skills require `SKILL.md`, instructions require `AGENTS.md` as the marker file within their directory under `skills/` or `instructions/`.

---

## Working with Worktrees

Feature branches often use git worktrees at `.worktrees/<branch-name>/`. Code changes in a worktree are isolated from the main working directory — remember to `cd` into the worktree or use its path when building/testing.

---

## File Size & Splitting Guidelines

**Rule of Thumb**: No file should exceed ~250–300 lines (excluding tests and imports).

### When to Split a File / Function

- **Split if**:
  - The file has more than one major responsibility.
  - A function has > 50 lines or > 3 levels of nesting.
  - A `match` statement has > 8 arms (delegate arms to per-feature `dispatch()`).
  - The file contains both UI logic and business logic.
  - The file directly calls `infra/` from `tui/` or `cli/` (this is a Rule 5 violation — fix the call site, don't split around it).
  - The file mixes pure model code with I/O — extract I/O behind a port.

### Preferred Patterns

1. **TUI Flow**:
   - **Controller**: Keystroke → `AppEvent::ExecuteCommand(CoreCommand)` (pure: no I/O, no infra)
   - **Runtime loop**: Receives `ExecuteCommand`, calls `AgkCore::execute()` inside `spawn_blocking` with a `TuiPresenter`
   - **TuiPresenter**: Implements `CoreEventSink`, forwards `CoreEvent` back as `AppEvent::CoreEvent(...)`
   - **State mutation**: Only the runtime loop mutates `AppState` (in response to `AppEvent::CoreEvent`)

2. **Use Case Pattern**:
   - One file per business action: `app/features/<feature>/<verb>.rs`
   - Each file exposes a single `run(...)` function taking typed inputs + ports + `&mut dyn CoreEventSink`
   - Small, testable, focused — most use cases fit under 150 lines

3. **Naming Convention**:
   - `handle_xxx` → Only allowed in TUI controllers (thin keystroke handlers) and the top-level `tui/event.rs`
   - `run` → The single entry point per use case in `app/features/<f>/<verb>.rs`
   - `dispatch` → Feature-level `CoreCommand` router in `app/features/<f>/mod.rs`
   - `to_core_command` → CLI arg mapper in `cli/features/<f>.rs`
   - Business logic → Must go to `app/features/<feature>/`

<!-- CODEGRAPH_START -->
## CodeGraph

This project has a CodeGraph MCP server (`codegraph_*` tools) configured. CodeGraph is a tree-sitter-parsed knowledge graph of every symbol, edge, and file. Reads are sub-millisecond and return structural information grep cannot.

### When to prefer codegraph over native search

Use codegraph for **structural** questions — what calls what, what would break, where is X defined, what is X's signature. Use native grep/read only for **literal text** queries (string contents, comments, log messages) or after you already have a specific file open.

| Question | Tool |
|---|---|
| "Where is X defined?" / "Find symbol named X" | `codegraph_search` |
| "What calls function Y?" | `codegraph_callers` |
| "What does Y call?" | `codegraph_callees` |
| "How does X reach/become Y? / trace the flow from X to Y" | `codegraph_trace` (one call = the whole path, incl. callback/React/JSX dynamic hops) |
| "What would break if I changed Z?" | `codegraph_impact` |
| "Show me Y's signature / source / docstring" | `codegraph_node` |
| "Give me focused context for a task/area" | `codegraph_context` |
| "See several related symbols' source at once" | `codegraph_explore` |
| "What files exist under path/" | `codegraph_files` |
| "Is the index healthy?" | `codegraph_status` |

### Rules of thumb

- **Answer directly — don't delegate exploration.** For "how does X work" / architecture questions, answer with 2-3 codegraph calls: `codegraph_context` first, then ONE `codegraph_explore` for the source of the symbols it surfaces. For a specific **flow** ("how does X reach Y") start with `codegraph_trace` from→to — one call returns the whole path with dynamic hops bridged — then ONE `codegraph_explore` for the bodies; don't rebuild the path with `codegraph_search` + `codegraph_callers`. Codegraph IS the pre-built index, so spawning a separate file-reading sub-task/agent — or running a grep + read loop — repeats work codegraph already did and costs more for the same answer.
- **Trust codegraph results.** They come from a full AST parse. Do NOT re-verify them with grep — that's slower, less accurate, and wastes context.
- **Don't grep first** when looking up a symbol by name. `codegraph_search` is faster and returns kind + location + signature in one call.
- **Don't chain `codegraph_search` + `codegraph_node`** when you just want context — `codegraph_context` is one call.
- **Don't loop `codegraph_node` over many symbols** — one `codegraph_explore` call returns several symbols' source grouped in a single capped call, while each separate node/Read call re-reads the whole context and costs far more.
- **Index lag — check the staleness banner, don't guess a wait.** When a codegraph response starts with "⚠️ Some files referenced below were edited since the last index sync…", the listed files are pending re-index — Read those specific files for accurate content. Files NOT in that banner are fresh and codegraph is authoritative for them. `codegraph_status` also lists pending files under "Pending sync".

### If `.codegraph/` doesn't exist

The MCP server returns "not initialized." Ask the user: *"I notice this project doesn't have CodeGraph initialized. Want me to run `codegraph init -i` to build the index?"*
<!-- CODEGRAPH_END -->
