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
│   ├── bootstrap.rs                 # ONLY place where concrete infra is wired
│   ├── command.rs                   # CoreCommand enum
│   ├── event.rs                     # CoreEvent enum
│   ├── outcome.rs                   # CoreOutcome enum
│   ├── snapshot.rs                  # UI-oriented view models
│   ├── ports/                       # All port traits
│   ├── features/                    # Feature slices (apply/, profiles/, etc.)
│   └── usecases/                    # Flat use-case files (migration path → features/)
├── domain/                          # Pure models & invariants
│   ├── asset/
│   ├── profile/
│   ├── vault/
│   ├── mcp/
│   ├── provider/
│   ├── config/
│   ├── bundle/
│   └── ...
├── infra/                           # Adapters (config codecs, providers, vaults, etc.)
│   ├── config/
│   ├── provider/
│   ├── vault/
│   ├── mcp/
│   ├── process/
│   └── ...
├── cli/                             # Thin CLI adapter
│   ├── core_dispatcher.rs           # Routes all CLI commands through AgkCore
│   ├── commands/                    # Per-feature thin CLI modules
│   └── ...
└── tui/                             # Thin TUI adapter (Ratatui)
    ├── reducer.rs                   # Pure key → UiIntent
    ├── command_mapper.rs            # UiIntent → CoreCommand
    ├── features/                    # Per-feature controllers
    └── ...
```

### Dependency Rules (Enforced by `tests/architecture.rs`)

| # | Rule | Enforcement |
|---|------|-------------|
| 1 | `domain/` depends on nothing outside `domain/` | Architecture test + code review |
| 2 | `app/` depends only on `domain/` and its own ports/contracts | Architecture test |
| 3 | `infra/` depends on `domain/` + `app::ports` only | Architecture test |
| 4 | `cli/` and `tui/` depend on `app/` (commands, events, snapshots) + `domain/`. **Never** on `infra/` | Architecture test |
| 5 | Only `main.rs` and `app/bootstrap.rs` may construct concrete adapters | Grep arch tests + review |
| 6 | No `std::process::Command` outside `infra/process/` | New architecture test |
| 7 | No file may own logic for more than one feature (split at ~300 LOC) | CI file-size lint + review |

---

## Development Workflow (Mandatory for Agents & Humans)

**Always follow this order** — this is the most important section for keeping agents on rails.

1. **Product Documentation First**
   - Update or create `docs/product/features/<feature>/prd.md`
   - Update or create `docs/product/features/<feature>/technical_design.md`

2. **Domain Layer**
   - Add/update models and invariants in `domain/`
   - Write domain unit tests

3. **Application Core**
   - Implement or extend use cases in `app/features/<feature>/` or `app/usecases/`
   - Add use-case tests using fake ports (`NullSink`, test doubles)
   - Extend `CoreCommand`, `CoreEvent`, `CoreOutcome` as needed

4. **Ports & Infra**
   - Define/extend port traits in `app/ports.rs`
   - Implement adapters in `infra/`

5. **CLI Adapter**
   - Add command parsing in `cli/commands/` or `cli/entry.rs`
   - Route through `cli/core_dispatcher.rs` → `AgkCore`
   - Never write business logic in CLI files

6. **TUI Adapter** (Last)
   - Add `UiIntent` in `tui/reducer.rs`
   - Map intent → `CoreCommand` in `tui/command_mapper.rs`
   - Add feature controller in `tui/features/<feature>/` if needed
   - Update rendering / widgets
   - Never write business logic in TUI files

### Simulator-First Rule (for interactive flows)

- Design the flow in the HTML simulator first.
- Export contract fixtures (`fixtures/contracts/`).
- Use those fixtures to drive reducer + contract tests.
- Keep `--dry-run --json` output compatible with the same scenario.

---

## Search Order for AI Agents

When you need to understand or modify code, **search in this exact order**:

1. `docs/product/features/<feature>/` (PRD + technical design)
2. `src/app/command.rs`, `src/app/event.rs`, `src/app/outcome.rs`
3. `src/app/features/<feature>/` or `src/app/usecases/`
4. `src/app/ports.rs`
5. `src/infra/<area>/`
6. `src/cli/commands/<feature>.rs`
7. `src/tui/features/<feature>/`

---

## Key Patterns & Rules

- **Pure Reducer**: `tui/reducer.rs` must remain pure (no side effects). Return `Vec<UiIntent>`.
- **Command/Event/Outcome**: All major flows go through `AgkCore::execute()`.
- **No Business Logic in Adapters**: `cli/` and `tui/` may not mutate config directly, own workflows, or call infra directly.
- **Manifest Codecs**: Support both TOML and YAML via `ManifestCodecPort`. Preserve original file extension on rewrite.
- **Profile Start Flow**: Load → Resolve → Build `LaunchPlan` (dry-run) → Execute via `ProfileRuntimePort`.
- **Feature Slicing**: Group new logic under `app/features/<feature>/` and `tui/features/<feature>/`.
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

### New Reducer Intent

```rust
fn derive_enter_intent(state: &TuiState) -> UiIntent {
    // pattern match on state.list_mode / state.tab_index
    // return UiIntent::Command(CoreCommand::...)
}
```

### New Feature Slice (`app/features/<feature>/`)

```
app/features/<feature>/
├── mod.rs          # Re-export public API
├── command.rs      # Input structs, CoreCommand constructors for this feature
├── usecase.rs      # Main use-case file (or split into multiple)
└── planner.rs      # Optional: deterministic planning logic (e.g. LaunchPlan)
```

### New TUI Feature Controller (`tui/features/<feature>/`)

```
tui/features/<feature>/
├── mod.rs          # Re-export
├── controller.rs   # Handles async side effects for this feature tab
└── widget.rs       # Rendering / layout helpers for this feature tab
```

---

## Testing Strategy

### Layer 1: Domain Tests
- Protect invariants in `domain/`.
- Pure functions, no I/O mocks needed.

### Layer 2: Use-Case Tests
- Protect behavior in `app/usecases/` / `app/features/`.
- Use fake port implementations (`NullSink`, in-memory stores).

### Layer 3: Contract Tests
- Protect CLI/TUI equivalence.
- Use `--dry-run --json` golden fixtures.
- Example: `profile_start_dry_run_matches_contract_fixture`.

### Layer 4: Snapshot Tests
- Protect TUI rendering shapes using `ratatui::TestBackend` + `insta`.

### Layer 5: Binary Integration Tests
- Small number of real-workspace flows using `assert_cmd`.

---

## Refactoring Strategy

We are in a **tightening phase**, not a rewrite. Follow the phased plan in `docs/proposals/architectural-convergence-plan.md`.

### Priority Order

| Phase | Goal | Duration | Risk |
|-------|------|----------|------|
| **Phase A** | Enforce architecture tests + CI gates | 1–2 days | Existing violations may block CI |
| **Phase B** | CLI convergence through `core_dispatcher.rs` | 3–5 days | Some commands rely on legacy helpers |
| **Phase C** | Decompose `tui/event.rs` into runtime loop + feature controllers | 5–7 days | Transient UI regressions |
| **Phase D** | Complete stubbed use cases + finish feature slices | 4–6 days | Config persistence shape changes |
| **Phase E** | Add YAML/TOML codecs + Cargo feature slimming | 3–5 days | Feature interaction bugs |
| **Phase F** | Simulator contracts + golden fixture tests | 2–4 days | Fixture drift |
| **Phase G** | Docs + agent harness finalization | 1 day | — |

**Total effort**: ~20–30 engineering days, deliverable in small, revertible PRs.

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
  - A `match` statement has > 8 arms.
  - The file contains both UI logic and business logic.
  - The file directly calls `infra/` from `tui/` or `cli/`.

### Preferred Patterns

1. **TUI Pattern** (Elm-inspired):
   - Reducer: Pure key → intents
   - Mapper: Intent → CoreCommand
   - Presenter: CoreEvent → UI State

2. **Use Case Pattern**:
   - One file per use case (`app/usecases/<name>.rs`)
   - Small, testable, focused on one business action.
   - Migrate to `app/features/<feature>/usecase.rs` as slices mature.

3. **Naming Convention**:
   - `handle_xxx` → Only allowed in reducer or very thin presenter glue.
   - Business logic → Must go to `app/usecases/` or `app/features/<feature>/`.

---

## Completed Refactor History

### Phase 0 (merged)
Added dev-deps, moved `TabKind` from `tui/` → `app/`, extracted view models to `app/snapshot.rs`, created architecture enforcement tests.

### Phase 1 (merged)
Introduced `CoreCommand`, `CoreEvent`, `CoreOutcome`, `UiIntent`, and `AgkCore` façade. Domain `Profile` with typed IDs.

### Phase 2 (merged)
Pure `reduce_key()` in `tui/reducer.rs`, `TuiState`/`ListMode`/`WizardState` in `tui/app_state.rs`, `command_mapper.rs`. 12 reducer unit tests.

### Phase 3 (merged)
Use-case extraction — `attach_vault`, `deactivate_provider`, `register_mcp`, `search_remote_vault` with fake-port tests.

### Phase 3.5 (merged)
Wired `AgkCore` with real port injection (`ConfigStorePort`, `McpRegistryPort`, `VaultSearchPort`, `Registry`). Created `InfraMcpRegistryAdapter`, `ClawHubSearchAdapter`, `AppEventSink` presenter bridge.

### Phase 4 + 4.5 (merged)
`CliPresenter` with `--json`/`--quiet`, `cli/core_dispatcher.rs` routing commands through `AgkCore`, added `dry_run` to `ProfileCommands::Create`.

### Phase 5 (merged)
`ProfileRuntimePort` trait, real `LaunchPlan` with provider-specific fields, `start_profile` use-case resolves profile from `ConfigStorePort` and runtime port from registry.

---

*Last updated: 2026-05-26 — Phases A-F completed; Phase G in progress.*
