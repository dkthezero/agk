# ADR-001: Unified Core with Vertical Feature Slices

> Status: **Approved (Revised — 2026-05-29)** | Audience: Core maintainers + AI agents  
> Scope: Single-branch refactor (release-ready)  
> Revision: Extended with domain purity violations, CLI MCP bypass, over-broad allowlists, and updated commit sequence. Supersedes earlier brainstorm + overall-architecture-view docs (now deleted).

---

## 1. Executive Summary

**Goal:** Make TUI and CLI share *exactly* the same execution path through `AgkCore`, while restructuring the application layer into vertical feature slices and eliminating all I/O side effects from the domain layer.

**Non-Goals:**
- Do NOT build `tui/reducer.rs` or pure `UiIntent` (deferred).
- Do NOT make `AgkCore` async (deferred).
- Do NOT split `CoreCommand` / `CoreEvent` into per-feature enums (centralized is pragmatic).

**Result:** A single `CoreCommand` bus, thin adapters, feature-owned usecases, a pure domain layer, and zero architecture test allowlists.

---

## 2. Current State (Evidence)

### 2.1 TUI bypasses `AgkCore`

```
tui/features/assets/controller.rs
  ├── crate::infra::vault::clawhub::cli_install(...)     // Rule 4 VIOLATION
  ├── crate::infra::vault::local::LocalVaultAdapter::new // Rule 4 VIOLATION
  └── crate::infra::feature::skill::SkillFeatureSet      // Rule 4 VIOLATION

tui/features/vaults/controller.rs
  ├── crate::infra::vault::clawhub::cli_search(...)      // Rule 4 VIOLATION
  ├── crate::infra::vault::clawhub::is_cli_available()   // Rule 4 VIOLATION
  └── crate::infra::vault::clawhub::install_cli_via_homebrew() // Rule 4 VIOLATION

tui/features/mcps/controller.rs
  ├── crate::infra::mcp::register(...)                   // Rule 4 VIOLATION
  ├── crate::infra::mcp::enable(...)                     // Rule 4 VIOLATION
  ├── crate::infra::mcp::disable(...)                    // Rule 4 VIOLATION
  └── crate::infra::mcp::test_server(...)                // Rule 4 VIOLATION

tui/runtime_loop.rs
  ├── crate::infra::vault::github::GithubVaultAdapter::new(...)   // constructs concrete adapter
  ├── crate::infra::vault::local::LocalVaultAdapter::new(...)     // constructs concrete adapter
  └── crate::infra::vault::clawhub::ClawHubVaultAdapter::new(...) // constructs concrete adapter
```

### 2.2 CLI still owns business logic

```
cli/commands/profiles.rs::run_profile_create()
  ├── std::fs::read_to_string(path)               // file I/O in adapter
  ├── std::fs::create_dir_all(...)                // file I/O in adapter
  └── std::process::Command::new("opencode")...     // Rule 6 VIOLATION

cli/commands/mcps.rs::dispatch_mcp()              // bypasses AgkCore entirely
  ├── crate::infra::mcp::register(...)            // direct infra call
  ├── crate::infra::mcp::test_server(...)         // direct infra call
  ├── crate::infra::mcp::enable(...)              // direct infra call
  └── crate::infra::mcp::disable(...)             // direct infra call
```

### 2.3 Horizontal flat dumps

- `app/usecases/` — 18 flat files, zero cohesion.
- `app/actions/` — legacy helper layer overlapping usecases; `mod.rs` alone is 364 lines.
- `app/ports.rs` — 422-line monolith owning every port contract.
- `cli/commands/` — contains business logic that belongs in `app/`.

### 2.4 Architecture test allowlists (over-broad)

- `is_tui_infra_allowlisted()` blanket-exempts **all of `tui/features/**/*.rs`** — meaning new violations in controllers are silently ignored.
- `is_tui_infra_allowlisted()` also exempts `tui/event.rs` and `tui/runtime_loop.rs`.
- `is_process_spawn_allowlisted()` allows `cli/commands/profiles.rs` to spawn processes.
- **These allowlists exist because the violations are real, but their scope is too wide.**

### 2.5 CLI MCP commands bypass `AgkCore` entirely

`cli/commands/mcps.rs::dispatch_mcp` does not go through `core_dispatcher.rs` or `AgkCore::execute()`. It calls `crate::infra::mcp::*` functions directly. This means MCP operations run completely different code in CLI vs. TUI — the opposite of the ADR's key invariant.

### 2.6 Domain layer has I/O side effects

The architecture tests only check for `crate::infra` imports, not stdlib I/O primitives. These violations exist and are undetected:

```
domain/paths.rs
  ├── std::process::Command::new("open")     // opens Finder on macOS
  ├── std::process::Command::new("xdg-open") // opens file manager on Linux
  ├── std::process::Command::new("cmd")      // opens Explorer on Windows
  └── std::process::Command::new("which")    // probes PATH

domain/telemetry.rs
  ├── std::fs::read_to_string(path)          // reads analytics file
  ├── std::fs::create_dir_all(parent)        // creates directory
  └── std::fs::write(path, content)          // writes analytics file

domain/hashing.rs
  └── std::fs::read(path)                    // compute_sha10 reads file content
    (the std::fs::write at line 32 is inside #[cfg(test)] — test-only, acceptable)
```

Domain must be pure: no `std::fs`, no `std::process`, no network. Test-module (`#[cfg(test)]`) I/O is allowed but should still be reviewed.

### 2.7 File size violations

| File | Lines | Budget |
|---|---|---|
| `tui/app.rs` | 439 | 300 |
| `tui/runtime_loop.rs` | 406 | 300 (allowlisted) |
| `app/core.rs` | 405 | 300 |
| `app/ports.rs` | 422 | 300 |
| `tui/event.rs` | 367 | 300 (allowlisted) |
| `app/actions/mod.rs` | 364 | 300 |

---

## 3. Target Architecture

### 3.1 Hexagon Dependency Rules (Enforced, Zero Allowlists)

| Layer | May Import | Must NOT Import |
|-------|-----------|-----------------|
| `domain/` | `std` (pure: no `fs`, no `process`, no `net`), `serde`, `anyhow` | `app::*`, `infra::*`, `cli::*`, `tui::*` |
| `app/ports/` | `domain::*` | `app/features/*`, `infra::*`, `cli::*`, `tui::*` |
| `app/features/<f>/` | `domain::*`, `app/ports/*`, `app/command.rs` (types), `app/event.rs` (types) | `infra::*`, `cli::*`, `tui::*`, other features' logic |
| `infra/` | `domain::*`, `app/ports/*` | `app/features/*`, `cli::*`, `tui::*` |
| `cli/` | `app/core.rs`, `app/command.rs`, `app/event.rs`, `app/outcome.rs`, `domain::*` | `infra::*`, `app/features/*::create.rs`, `app/actions::*` |
| `tui/` | `app/core.rs`, `app/command.rs`, `app/event.rs`, `app/outcome.rs`, `domain::*` | `infra::*`, `app/features/*::create.rs`, `app/actions::*` |

**Only `main.rs` and `app/bootstrap.rs` may construct concrete adapters.**

### 3.2 Repository Layout (After)

```
src/
├── main.rs                          -- composition root ONLY
│
├── domain/                          -- pure models (ZERO std::fs / std::process)
│   ├── asset.rs
│   ├── config.rs
│   ├── context.rs
│   ├── hashing.rs                   -- hash algorithms only; no file I/O
│   ├── identity.rs
│   ├── mcp.rs
│   ├── paths.rs                     -- path computation only; no process spawning
│   ├── profile.rs
│   ├── scope.rs
│   ├── telemetry.rs                 -- analytics models only; no file I/O
│   └── validation.rs
│
├── app/
│   ├── mod.rs
│   ├── core.rs                      -- AgkCore façade, feature-dispatched execute()
│   ├── command.rs                   -- CoreCommand enum (centralized)
│   ├── event.rs                     -- CoreEvent enum (centralized)
│   ├── outcome.rs                   -- CoreOutcome, CoreEventSink, NullSink
│   ├── snapshot.rs                  -- read models
│   ├── tab_kind.rs                  -- shared tab classification
│   ├── registry.rs                  -- adapter registry
│   ├── ports/
│   │   ├── mod.rs
│   │   ├── config_store.rs          -- ~60 lines
│   │   ├── context_store.rs         -- ~40 lines
│   │   ├── feature_set.rs           -- ~40 lines
│   │   ├── vault.rs                 -- ~50 lines
│   │   ├── provider.rs              -- ~80 lines
│   │   ├── mcp_registry.rs          -- ~60 lines
│   │   ├── profile_runtime.rs       -- ~60 lines
│   │   ├── process_runner.rs        -- ~20 lines
│   │   ├── file_opener.rs           -- ~20 lines (replaces domain/paths.rs OS open)
│   │   └── telemetry_store.rs       -- ~30 lines (replaces domain/telemetry.rs I/O)
│   │
│   ├── features/                    -- ★ replaces usecases/ + actions/
│   │   ├── mod.rs                   -- re-export + dispatch() registry
│   │   ├── profile/
│   │   │   ├── mod.rs               -- dispatch() + feature-level tests
│   │   │   ├── command.rs           -- CreateProfileInput, UpdateProfilePatch
│   │   │   ├── create.rs
│   │   │   ├── delete.rs
│   │   │   ├── start.rs
│   │   │   ├── attach_skill.rs
│   │   │   ├── detach_skill.rs
│   │   │   ├── attach_mcp.rs
│   │   │   └── detach_mcp.rs
│   │   ├── vault/
│   │   │   ├── mod.rs
│   │   │   ├── command.rs
│   │   │   ├── attach.rs
│   │   │   └── detach.rs
│   │   ├── asset/
│   │   │   ├── mod.rs
│   │   │   ├── command.rs
│   │   │   ├── install.rs
│   │   │   ├── remove.rs
│   │   │   ├── update.rs
│   │   │   ├── sync.rs
│   │   │   ├── search_remote.rs
│   │   │   └── validate.rs
│   │   ├── provider/
│   │   │   ├── mod.rs
│   │   │   ├── command.rs
│   │   │   ├── activate.rs
│   │   │   └── deactivate.rs
│   │   ├── mcp/
│   │   │   ├── mod.rs
│   │   │   ├── command.rs           -- RegisterMcpInput
│   │   │   ├── register.rs
│   │   │   ├── enable.rs
│   │   │   └── disable.rs
│   │   ├── context/
│   │   │   ├── mod.rs
│   │   │   ├── command.rs
│   │   │   ├── switch.rs
│   │   │   └── list.rs
│   │   └── apply/
│   │       ├── mod.rs
│   │       ├── command.rs
│   │       └── run.rs
│   │
│   └── bootstrap/                   -- concrete wiring (updated with new ports)
│
├── infra/
│   ├── config/
│   ├── context/
│   ├── feature/
│   ├── mcp/
│   ├── provider/
│   ├── telemetry/
│   │   └── store.rs                 -- ★ NEW: implements TelemetryStorePort (file I/O)
│   ├── vault/
│   └── process/
│       ├── mod.rs
│       ├── runner.rs                -- implements ProcessRunnerPort (std::process::Command)
│       └── opener.rs                -- ★ NEW: implements FileOpenerPort (OS open command)
│
├── cli/
│   ├── mod.rs
│   ├── entry.rs                     -- Cli struct, global flags ONLY
│   ├── core_dispatcher.rs           -- THE dispatcher (thin, handles ALL commands)
│   ├── presenter.rs
│   └── features/                    -- ★ replaces commands/
│       ├── mod.rs
│       ├── profile.rs
│       ├── asset.rs
│       ├── mcp.rs                   -- McpCommands + to_core_command() (NO infra calls)
│       ├── context.rs
│       ├── apply.rs
│       └── telemetry.rs
│
└── tui/
    ├── mod.rs
    ├── entry.rs
    ├── app.rs                       -- AppState (split: ListMode → list_mode.rs)
    ├── list_mode.rs                 -- ★ NEW: extracted from app.rs
    ├── event.rs
    ├── runtime_loop.rs              -- vault adapters removed; injected via AgkCore
    ├── layout.rs
    ├── render.rs
    ├── presenter.rs                 -- ★ NEW: TuiPresenter (CoreEventSink bridge)
    ├── render/
    ├── widgets/
    └── features/                    -- thinned controllers (zero infra imports)
        ├── profile/
        ├── vault/
        ├── asset/
        ├── provider/
        ├── mcp/
        ├── context/
        └── common/
```

### 3.3 Unified Execution Flow (Both CLI and TUI)

```
CLI Path:
  CLI args ──► cli/features/*.rs ──► CoreCommand ──► AgkCore::execute()
                                                          │
                                                          ├──► feature dispatch
                                                          │      └──► usecase
                                                          │
                                                          └──► CoreEvent ──► CliPresenter

TUI Path:
  Keystroke ──► tui/event.rs ──► feature controller ──► AppEvent::ExecuteCommand(CoreCommand)
                                                              │
                                                              ▼
                                                    tui/runtime_loop.rs
                                                              │
                                                              ├──► tokio::task::spawn_blocking
                                                              │        core.execute(cmd, &mut TuiPresenter)
                                                              │
                                                              └──► TuiPresenter sends AppEvent::CoreEvent(e)
                                                              │        back into the loop
                                                              ▼
                                                    runtime_loop mutates AppState
```

**Key invariant:** `tui/` never calls `app::actions::*`, `app::usecases::*`, or `infra::*` directly.

---

## 4. Detailed Design Decisions

### 4.1 `AgkCore` Thread Safety

Current `AgkCore` fields are all `Arc<dyn Port>`. Audit every port implementation for `Send + Sync`:
- `TomlConfigStore` must use `std::sync::Mutex` internally for file writes.
- `ContextStore` must be `Send + Sync`.
- `Registry` must be `Send + Sync`.

If any port lacks interior mutability, wrap it in `Arc<Mutex<dyn Port>>` in `bootstrap.rs`.

**Test:**
```rust
#[test]
fn agk_core_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<AgkCore>();
}
```

### 4.2 TUI Convergence: Thin Controllers + `TuiPresenter`

1. Controllers derive `CoreCommand` from keystrokes and emit `AppEvent::ExecuteCommand(CoreCommand)` via `ctx.tx`.
2. The runtime loop receives `ExecuteCommand`, spawns `core.execute(cmd, &mut TuiPresenter)` in `spawn_blocking`.
3. `TuiPresenter` implements `CoreEventSink` by sending `AppEvent::CoreEvent(CoreEvent)` back into the loop.
4. The runtime loop applies `CoreEvent` to `AppState`.

**New `AppEvent` variants:**
```rust
pub enum AppEvent {
    // ... existing variants ...
    ExecuteCommand(CoreCommand),
    CoreEvent(crate::app::event::CoreEvent),
}
```

**New `tui/presenter.rs`:**
```rust
pub struct TuiPresenter {
    tx: tokio::sync::mpsc::UnboundedSender<AppEvent>,
}
impl CoreEventSink for TuiPresenter {
    fn on_event(&mut self, event: CoreEvent) {
        let _ = self.tx.send(AppEvent::CoreEvent(event));
    }
    fn on_error(&mut self, error: String) {
        let _ = self.tx.send(AppEvent::CoreEvent(CoreEvent::Error(error)));
    }
}
```

### 4.3 CLI Convergence: Delete `cli/commands/`, Route Everything Through `AgkCore`

> Note: `CoreCommand::RegisterMcp { input: RegisterMcpInput }`, `EnableMcp { name, provider_id, scope }`, and `DisableMcp { name, provider_id, scope }` variants **already exist** in `app/command.rs`. No new variants are needed; the bypass is purely a CLI/TUI routing issue. The Enable/Disable variants take `provider_id`, not `profile_id` — keep this naming when writing call sites.

**MCP commands** (currently bypass `AgkCore` entirely via `dispatch_mcp`) must be migrated to:
1. `app/features/mcp/{register,enable,disable}.rs` — usecases calling `McpRegistryPort`.
2. `cli/features/mcp.rs` — thin arg mapping to `CoreCommand::RegisterMcp` / `EnableMcp` / `DisableMcp`.
3. Remove the `dispatch_mcp` function from `cli/commands/mcps.rs`.

**Profile creation** business logic (`run_profile_create`) moves to:
1. `app/features/profile/create.rs` — usecase calling `ProcessRunnerPort` and `ConfigStorePort`.
2. `cli/features/profile.rs` — thin arg mapping to `CoreCommand::CreateProfile`.
3. `infra/process/runner.rs` — concrete `std::process::Command` wrapper.

### 4.4 Domain Purification: Extract All I/O

| Current | Violation | Target |
|---|---|---|
| `domain/paths.rs` — OS file open | `std::process::Command` | New `FileOpenerPort` in `app/ports/file_opener.rs` + `infra/process/opener.rs` impl |
| `domain/telemetry.rs` — analytics read/write | `std::fs::*` | New `TelemetryStorePort` in `app/ports/telemetry_store.rs` + `infra/telemetry/store.rs` impl |
| `domain/hashing.rs` — file read/write | `std::fs::*` | Move to an `infra/`-layer utility or a caller in `app/features/` |

After extraction, `domain/` contains only pure models. Architecture tests must verify `std::fs` and `std::process::Command` do not appear in any `domain/*.rs` file.

### 4.5 Feature-Dispatched `core.rs`

Replace the giant `match` with a dispatch chain:

```rust
// app/core.rs
pub fn execute(&self, command: CoreCommand, sink: &mut dyn CoreEventSink) -> CoreResult {
    if let Some(r) = features::profile::dispatch(command, self, sink)  { return r; }
    if let Some(r) = features::vault::dispatch(command, self, sink)    { return r; }
    if let Some(r) = features::asset::dispatch(command, self, sink)    { return r; }
    if let Some(r) = features::provider::dispatch(command, self, sink) { return r; }
    if let Some(r) = features::mcp::dispatch(command, self, sink)      { return r; }
    if let Some(r) = features::context::dispatch(command, self, sink)  { return r; }
    if let Some(r) = features::apply::dispatch(command, self, sink)    { return r; }

    sink.on_error(format!("Command {:?} not yet implemented", command));
    Ok(CoreOutcome::Ok)
}
```

Each feature's `mod.rs` exposes:
```rust
pub fn dispatch(
    cmd: CoreCommand,
    core: &AgkCore,
    sink: &mut dyn CoreEventSink,
) -> Option<CoreResult> {
    match cmd {
        CoreCommand::CreateProfile { input } => {
            Some(create::run(input, core.store.as_ref(), sink))
        }
        _ => None,
    }
}
```

**Pros:** `core.rs` shrinks from ~405 lines to ~80 lines.

### 4.6 `app/command.rs` — Centralized Enum, Distributed Inputs

`CoreCommand` stays centralized. Feature-specific input structs move to `app/features/<f>/command.rs`.

```rust
// app/command.rs
pub enum CoreCommand {
    CreateProfile { input: crate::app::features::profile::command::CreateProfileInput },
    AttachVault { input: crate::app::features::vault::command::AttachVaultInput },
    RegisterMcp { input: crate::app::features::mcp::command::RegisterMcpInput },
    // ...
}
```

### 4.7 Allowlist Discipline: Tighten as You Go

The current `is_tui_infra_allowlisted()` is too permissive — it blanket-exempts all of `tui/features/`. After each TUI controller migration (Commit 9), immediately remove that feature from the allowlist. The allowlist must shrink with every commit, never grow.

---

## 5. File Migration Map (Exact Moves & Deletes)

### 5.1 Delete
| Path | Reason |
|------|--------|
| `src/app/actions/` | Logic absorbed into `app/features/asset/`, `app/features/vault/` |
| `src/app/usecases/` | Replaced by `app/features/*/` |
| `src/cli/commands/` | Business logic moved to usecases; args moved to `cli/features/` |
| `src/app/ports.rs` | Split into `app/ports/*.rs` |

### 5.2 Move / Rename
| From | To |
|------|-----|
| `app/usecases/create_profile.rs` | `app/features/profile/create.rs` |
| `app/usecases/delete_profile.rs` | `app/features/profile/delete.rs` |
| `app/usecases/start_profile.rs` | `app/features/profile/start.rs` |
| `app/usecases/attach_skill_to_profile.rs` | `app/features/profile/attach_skill.rs` |
| `app/usecases/detach_skill_from_profile.rs` | `app/features/profile/detach_skill.rs` |
| `app/usecases/attach_mcp_to_profile.rs` | `app/features/profile/attach_mcp.rs` |
| `app/usecases/detach_mcp_from_profile.rs` | `app/features/profile/detach_mcp.rs` |
| `app/usecases/attach_vault.rs` | `app/features/vault/attach.rs` |
| `app/actions/remove.rs::detach_vault` | `app/features/vault/detach.rs` |
| `app/actions/install.rs` | `app/features/asset/install.rs` |
| `app/actions/remove.rs::remove_asset` | `app/features/asset/remove.rs` |
| `app/actions/install.rs::update_asset` | `app/features/asset/update.rs` |
| `app/actions/sync.rs` | `app/features/asset/sync.rs` |
| `app/usecases/search_remote_vault.rs` | `app/features/asset/search_remote.rs` |
| `app/usecases/activate_provider.rs` | `app/features/provider/activate.rs` |
| `app/usecases/deactivate_provider.rs` | `app/features/provider/deactivate.rs` |
| `app/usecases/register_mcp.rs` | `app/features/mcp/register.rs` |
| `app/usecases/enable_mcp.rs` | `app/features/mcp/enable.rs` |
| `app/usecases/disable_mcp.rs` | `app/features/mcp/disable.rs` |
| `app/usecases/switch_context.rs` | `app/features/context/switch.rs` |
| `app/usecases/list_contexts.rs` | `app/features/context/list.rs` |
| `app/usecases/apply_config.rs` | `app/features/apply/run.rs` |
| `app/command.rs::CreateProfileInput` | `app/features/profile/command.rs` |
| `app/command.rs::UpdateProfilePatch` | `app/features/profile/command.rs` |
| `app/command.rs::AttachVaultInput` | `app/features/vault/command.rs` |
| `app/command.rs::RegisterMcpInput` | `app/features/mcp/command.rs` |
| `app/command.rs::ApplyConfigInput` | `app/features/apply/command.rs` |
| `cli/commands/profiles.rs` | Logic → `app/features/profile/create.rs`; args → `cli/features/profile.rs` |
| `cli/commands/assets/*.rs` | Logic → `app/features/asset/`; args → `cli/features/asset.rs` |
| `cli/commands/mcps.rs` | Logic → `app/features/mcp/`; args → `cli/features/mcp.rs` |
| `cli/commands/telemetry.rs` | Args → `cli/features/telemetry.rs` |

### 5.3 Create New
| Path | Purpose |
|------|---------|
| `app/ports/mod.rs` | Re-export all port traits |
| `app/ports/process_runner.rs` | Port for subprocess spawning |
| `app/ports/file_opener.rs` | Port for OS-level file/folder open |
| `app/ports/telemetry_store.rs` | Port for analytics read/write |
| `app/features/*/mod.rs` | Feature dispatch + tests |
| `app/features/*/command.rs` | Feature input structs |
| `cli/features/*.rs` | Thin CLI arg → CoreCommand mappers |
| `infra/process/runner.rs` | ProcessRunnerPort implementation |
| `infra/process/opener.rs` | FileOpenerPort implementation (replaces domain/paths.rs OS calls) |
| `infra/telemetry/store.rs` | TelemetryStorePort implementation (replaces domain/telemetry.rs I/O) |
| `tui/presenter.rs` | TuiPresenter bridging CoreEventSink → AppEvent |
| `tui/list_mode.rs` | ListMode enum extracted from tui/app.rs |

---

## 6. Architecture Test Updates (Remove All Allowlists)

### 6.1 Add: `std::process` and `std::fs` forbidden in `domain/`

```rust
#[test]
fn domain_must_not_use_process() {
    let files = collect_rust_files("domain");
    for path in &files {
        let text = read_file(path);
        assert!(
            !text.contains("std::process::Command"),
            "Domain purity violation in {:?}: found std::process::Command",
            path
        );
    }
}

#[test]
fn domain_must_not_use_fs() {
    let files = collect_rust_files("domain");
    for path in &files {
        let text = read_file(path);
        assert!(
            !text.contains("std::fs::"),
            "Domain purity violation in {:?}: found std::fs",
            path
        );
    }
}
```

### 6.2 Remove `is_tui_infra_allowlisted()` entirely

After migration, `tui/` must import zero `crate::infra` items. Remove the allowlist function and the blanket `tui/features/` exemption in `tui_must_not_import_infra`.

### 6.3 Remove `is_process_spawn_allowlisted()` entirely

- `std::process::Command` lives only in `infra/process/`.
- `main.rs` exception stays until `main.rs` itself uses `ProcessRunnerPort`.

### 6.4 File size limit returns to 300 lines everywhere

- `app/core.rs` shrinks to ~80 lines.
- `app/ports.rs` splits into ~70-line files.
- `tui/app.rs` (439 lines): extract `ListMode` → `tui/list_mode.rs`.
- `tui/runtime_loop.rs` (406 lines): extract reload snapshot logic → `tui/reload.rs`.
- `tui/event.rs` (367 lines): shrinks naturally as each controller migration removes a keyboard branch.

---

## 7. Execution Plan (Revised Commit Sequence)

The commit sequence is ordered for bisectability. Every commit gates on `cargo check && cargo test`.

### Commit 0: Harden architecture tests for domain purity

- Add `domain_must_not_spawn_processes` and `domain_must_not_use_fs` tests in `tests/architecture.rs`.
- Use `#[ignore]` like the other architecture tests so they only run via `cargo test --test architecture -- --ignored`.
- Skip `#[cfg(test)]` blocks (test helpers may legitimately read/write files) — either by trimming test modules before scanning, or by allowing matches only when they appear outside `#[cfg(test)]`.
- Run them; they will fail — that confirms the violations are real.
- **Gate:** These tests fail intentionally (they document the known violations for Commit 1).

### Commit 1: Domain purification

Extract all I/O from `domain/`:

1. Add `FileOpenerPort` to `app/ports/file_opener.rs`.
2. Add `infra/process/opener.rs` implementing it with `std::process::Command`.
3. Wire `FileOpenerPort` into bootstrap; pass it to callers of `domain::paths::open_*`.
4. Add `TelemetryStorePort` to `app/ports/telemetry_store.rs`.
5. Add `infra/telemetry/store.rs` implementing it with `std::fs`.
6. Move file-I/O methods from `domain/telemetry.rs` to `infra/telemetry/store.rs`.
7. Refactor `domain/hashing.rs::compute_sha10` to accept `&[(PathBuf, Vec<u8>)]` (path + bytes) so the domain hashes pure data; callers in `app/features/` and `infra/` perform the `std::fs::read`. The `#[cfg(test)]` `write_temp_file` helper may stay as-is.
8. **Gate:** `domain_must_not_spawn_processes` and `domain_must_not_use_fs` now pass.

### Commit 2: Split `app/ports.rs` → `app/ports/*.rs`

- Create directory and per-capability files.
- Update all imports.
- **Gate:** `cargo check`, `cargo test`

### Commit 3: Create `app/features/` + move usecases/actions

- Move all `app/usecases/*.rs` into `app/features/<f>/`.
- Move all `app/actions/*.rs` logic into `app/features/<f>/`.
- Create each feature's `mod.rs` with `dispatch()` stub.
- **Gate:** `cargo check`, `cargo test`

### Commit 4: Extract input structs to `app/features/<f>/command.rs`

- Move `CreateProfileInput`, `AttachVaultInput`, `RegisterMcpInput`, `ApplyConfigInput`.
- Update `app/command.rs` to import from features.
- **Gate:** `cargo check`, `cargo test`

### Commit 5: Feature-dispatched `core.rs`

- Replace giant `match` with dispatch chain.
- **Gate:** `app/core.rs` ≤ 100 lines, `cargo test`, architecture test.

### Commit 6: Add `ProcessRunnerPort` + infra implementation

- Create `app/ports/process_runner.rs`.
- Create `infra/process/runner.rs` implementing it.
- Wire in `bootstrap.rs`.
- **Gate:** `cargo test`

### Commit 7: Migrate CLI `commands/` → `features/` + wire MCP through AgkCore

- Create `cli/features/*.rs` (profile, asset, mcp, context, apply, telemetry).
- For MCP: add `CoreCommand::RegisterMcp`, `EnableMcp`, `DisableMcp` variants.
- Add dispatch in `app/features/mcp/mod.rs`.
- Move `cli/commands/mcps.rs::dispatch_mcp` business logic into `app/features/mcp/*.rs`.
- Create `cli/features/mcp.rs` with thin `to_core_command()`.
- Delete `cli/commands/`.
- Update `cli/mod.rs`, `core_dispatcher.rs`.
- Remove `is_process_spawn_allowlisted()` for `cli/commands/profiles.rs` (file is deleted).
- **Gate:** `cargo test`, `cargo run -- mcp add --help`, architecture test.

### Commit 8: TUI bridge infrastructure

- Add `AppEvent::ExecuteCommand(CoreCommand)` and `AppEvent::CoreEvent(CoreEvent)`.
- Create `tui/presenter.rs` (`TuiPresenter`).
- Add `Arc<AgkCore>` to `EventContext`.
- In `run_loop`, handle `ExecuteCommand` via `spawn_blocking` + `TuiPresenter`.
- Handle `CoreEvent` by matching variants and mutating `AppState`.
- No controller changes yet.
- **Gate:** `cargo test`, TUI compiles.

### Commit 9: Migrate TUI controllers (feature by feature, shrink allowlist each time)

Migrate in this order (cleanest boundary first):

**9a — MCP controllers:**
- Replace `infra::mcp::*` calls with `CoreCommand::RegisterMcp` / `EnableMcp` / `DisableMcp`.
- Verify: `grep -r "infra::" src/tui/features/mcps/` returns 0.
- Remove `tui/features/mcps/` from `is_tui_infra_allowlisted()`.

**9b — Vault controllers:**
- Replace `infra::vault::clawhub::*` calls with `CoreCommand::SearchRemoteVault` / `AttachVault`.
- Verify: `grep -r "infra::" src/tui/features/vaults/` returns 0.
- Remove `tui/features/vaults/` from allowlist.

**9c — Asset controllers:**
- Replace `infra::vault::clawhub::cli_install`, `LocalVaultAdapter`, `SkillFeatureSet` with `CoreCommand::InstallAsset`.
- Verify: `grep -r "infra::" src/tui/features/assets/` returns 0.
- Remove `tui/features/assets/` from allowlist.

**9d — Provider, Profile, Context controllers:**
- Replace any remaining direct infra calls.
- Remove remaining features from allowlist.

**Gate per sub-commit:** `cargo test`, architecture test, allowlist is smaller.

### Commit 10: Remove vault adapter construction from `tui/runtime_loop.rs`

- Vault adapters are currently constructed inline in `runtime_loop.rs::handle_vault_refresh` (lines 188–206) by matching `VaultConfig` variants.
- Introduce a `VaultFactoryPort` (in `app/ports/vault.rs`) with `fn build(&self, id: String, config: VaultConfig) -> Box<dyn VaultPort>`.
- Implement it in `infra/vault/factory.rs` (the place that currently knows about `Github`/`Local`/`Clawhub` adapters).
- Wire it into bootstrap; expose via `AgkCore`. Pass it to `EventContext` if needed for the reload path.
- Alternative (preferred if scope allows): convert vault refresh into a `CoreCommand::RefreshVault` (the variant already exists) and let `app/features/vault/refresh.rs` handle adapter construction via the factory port.
- Remove `crate::infra::vault::*` direct construction from `runtime_loop.rs`.
- Remove `tui/runtime_loop.rs` from `is_tui_infra_allowlisted()`.
- **Gate:** `grep -r "infra::" src/tui/` returns 0.

### Commit 11: Delete allowlists + delete `app/actions/` and `app/usecases/`

- Delete `is_tui_infra_allowlisted()` function entirely.
- Delete `is_process_spawn_allowlisted()` function entirely.
- Delete `src/app/actions/` and `src/app/usecases/`.
- Verify `tui_must_not_import_infra` passes without any exemptions.
- **Gate:** `cargo test --test architecture -- --ignored` passes with zero allowlist exceptions.

### Commit 12: File size reduction

- Extract `ListMode` enum from `tui/app.rs` → `tui/list_mode.rs` (reduces app.rs by ~130 lines).
- Extract `Progress` / `ProgressStatus` from `tui/app.rs` → `tui/progress.rs`.
- Extract reload snapshot logic from `tui/runtime_loop.rs` → `tui/reload.rs`.
- `tui/event.rs` shrinks naturally from controller migrations (Commit 9); finish if still > 300 lines.
- **Gate:** No `.rs` file > 300 lines of non-test logic.

### Commit 13: Thread-safety test + contract parity test

- Add `tests/agk_core_thread_safety.rs`.
- Add contract test: run same `CoreCommand` through CLI path and TUI path, assert identical `CoreEvent` sequence.
- **Gate:** All tests pass.

---

## 8. Success Criteria

- [ ] `src/app/usecases/` does not exist.
- [ ] `src/app/actions/` does not exist.
- [ ] `src/cli/commands/` does not exist.
- [ ] `src/app/ports.rs` does not exist (split into `ports/*.rs`).
- [ ] `grep -r "std::process::Command" src/domain/` returns **0 matches**.
- [ ] `grep -r "std::fs::" src/domain/` returns **0 matches**.
- [ ] `grep -r "infra::" src/tui/` returns **0 matches**.
- [ ] `grep -r "app::actions::" src/tui/` returns **0 matches**.
- [ ] `grep -r "app::usecases::" src/tui/` returns **0 matches**.
- [ ] `grep -r "std::process::Command::new" src/cli/` returns **0 matches**.
- [ ] `grep -r "crate::infra::" src/cli/features/` returns **0 matches**.
- [ ] `cli/commands/mcps.rs` does not exist (`dispatch_mcp` is gone).
- [ ] Every feature has a folder in `app/features/<f>/` with `{mod.rs, command.rs, usecase files}`.
- [ ] `app/core.rs` has no feature-specific logic (only delegation).
- [ ] `app/core.rs` ≤ 100 lines.
- [ ] No `.rs` file > 300 lines of non-test logic.
- [ ] `is_tui_infra_allowlisted()` does not exist.
- [ ] `is_process_spawn_allowlisted()` does not exist.
- [ ] `cargo test --test architecture -- --ignored` passes **with zero allowlist exceptions**.
- [ ] `cargo test` passes (all unit + integration tests).
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- [ ] `cargo fmt --check` passes.
- [ ] `agk_core_is_send_sync` test passes.
- [ ] Contract test `tui_cli_equivalence_profile_start` passes.
- [ ] TUI manual QA: vault attach, asset install, profile create, provider toggle, MCP register all work.

---

## 9. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Mass file move breaks git blame | Medium | Use `git mv` in every commit. Separate move commits from edit commits. |
| `TomlConfigStore` not `Send+Sync` | High (runtime panic) | Audit before Commit 8. Wrap in `Mutex` in bootstrap if needed. Add `assert_send_sync` test. |
| TUI async event lag causes UI flicker | Low | `UnboundedSender` is non-blocking. If flicker observed, add `AppEvent::Immediate` fast-path later. |
| `CoreCommand` variant missing for TUI flow | Medium | Audit every controller before migration. Add variants to `CoreCommand` as needed. |
| CLI `run_profile_create` subprocess logic is complex | High (regression) | Extract subprocess steps into `infra/process/` adapter. Write integration test with `FakeProcessRunner`. |
| `domain/telemetry.rs` I/O callers are spread | Medium | Map all call sites before extracting. Update each call site to receive port via dependency injection. |
| New `FileOpenerPort` breaks platform-specific open logic | Medium | Keep the platform branching in `infra/process/opener.rs`; domain `paths.rs` keeps path computation only. |
| `tui/runtime_loop.rs` vault adapter removal requires reload redesign | High | Reload must use `AgkCore`'s injected vault ports, not construct new adapters. Wire via bootstrap before Commit 10. |
| Over-broad allowlist hides new violations during migration | High | Tighten allowlist after each sub-commit in Commit 9. Never add new entries; only remove. |
| Tests break due to import changes | Medium | Move test modules with source files. Update `tests/` integration imports. |

---

## 10. Open Questions (Resolved)

1. **Should `CoreEvent` include UI-only events?**  
   *Resolved:* No. UI-only state (wizard step, modal open) stays in `AppEvent`. Domain events go in `CoreEvent`.

2. **Should TUI support `--dry-run`?**  
   *Resolved:* Yes. `CoreCommand` variants that support dry-run already carry the flag. TUI controllers emit the same command with `dry_run: true` when a modifier key is held.

3. **How to handle `CoreOutcome` return values in TUI?**  
   *Resolved:* Most usecases emit `CoreEvent` and return `Ok(CoreOutcome::Ok)`. For list commands, emit a `CoreEvent::ContextList(Vec<Context>)`. Use the event approach to keep the loop single-channel.

4. **When to build `reducer.rs`?**  
   *Resolved:* After this ADR is complete and the branch is stable. The pure reducer is a TUI-internal quality improvement, not a prerequisite.

5. **When to decompose `tui/event.rs`?** *(New question, now resolved)*  
   *Resolved:* Decompose it incrementally as part of Commit 9. Each TUI controller migration removes a keyboard branch from `event.rs`, so the file shrinks naturally. No separate decomposition phase needed.

6. **Should MCP commands be wired through `AgkCore` in CLI?** *(New — was missed in original ADR)*  
   *Resolved:* Yes, unconditionally. `cli/commands/mcps.rs::dispatch_mcp` currently bypasses `AgkCore`. Commit 7 eliminates it entirely by moving logic to `app/features/mcp/` and routing through `core_dispatcher.rs`.

---

*Approved for implementation. The step-by-step execution plan (TDD-friendly task breakdown) will be written into `docs/superpowers/plans/` when execution begins. Until then, §7 above is the authoritative commit sequence.*
