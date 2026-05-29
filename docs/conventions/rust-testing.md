# Rust Testing (AGK Edition)

> Adapted for AGK from `ECC/rules/rust/testing.md`. Mock libraries dropped in favour of hand-written fakes; AGK's six-layer test strategy made explicit.

## Test Framework

- `#[test]` with `#[cfg(test)]` modules for unit tests.
- `#[tokio::test]` for async tests (sparingly — most AGK code is sync).
- **No mocking libraries**. We use hand-written fakes. They are small, explicit, and survive refactors better than auto-generated mocks. See "Fake Ports" below.
- Snapshot tests for TUI rendering use `insta`.
- Contract tests use plain `assert_eq!` against golden JSON fixtures.

## Test Organization

```text
src/
├── app/
│   └── features/profile/
│       ├── create.rs           # #[cfg(test)] mod tests at the bottom
│       └── delete.rs           # #[cfg(test)] mod tests at the bottom
├── infra/
│   └── config/
│       └── toml_store.rs       # #[cfg(test)] mod tests
tests/                          # Integration tests — one file = one binary
├── architecture.rs             # Dependency, file-size, purity rules
├── agk_core_thread_safety.rs   # Send + Sync assertions
├── tui_cli_equivalence.rs      # Contract parity tests
└── fixtures/                   # Golden JSON, sample manifests
    └── contracts/
```

**Rule:** unit tests live next to the code they test (`#[cfg(test)] mod tests` at file bottom). Integration tests live in `tests/` and run as separate binaries.

## AGK's Six Test Layers

| Layer | Lives In | Purpose | Tools |
|---|---|---|---|
| 1. Domain | `src/domain/*.rs` `#[cfg(test)]` | Pure invariants | `#[test]`, assertions |
| 2. Use case | `src/app/features/<f>/<verb>.rs` `#[cfg(test)]` | Behaviour with fake ports | `NullSink`, `RecordingSink`, fake ports |
| 3. Contract | `tests/tui_cli_equivalence.rs` | CLI/TUI produce identical `CoreEvent`s | Golden fixtures |
| 4. Snapshot | `tests/tui_render_*.rs` | TUI rendering shape | `ratatui::TestBackend` + `insta` |
| 5. Integration | `tests/*.rs` | Real-workspace flows | `assert_cmd`, `tempfile` |
| 6. Architecture | `tests/architecture.rs` | Dependency rules, purity, file size | `#[ignore]` + AST/source scan |

## Unit Test Pattern (Use Case)

```rust
// src/app/features/profile/create.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::outcome::CoreEvent;

    // Hand-written fake — small, explicit, no macro magic
    struct InMemoryConfigStore {
        configs: std::sync::Mutex<HashMap<Scope, ConfigFile>>,
    }
    impl ConfigStorePort for InMemoryConfigStore {
        fn load(&self, scope: Scope) -> anyhow::Result<ConfigFile> {
            Ok(self.configs.lock().unwrap().get(&scope).cloned().unwrap_or_default())
        }
        fn save(&self, scope: Scope, config: &ConfigFile) -> anyhow::Result<()> {
            self.configs.lock().unwrap().insert(scope, config.clone());
            Ok(())
        }
    }

    struct RecordingSink(Vec<CoreEvent>);
    impl CoreEventSink for RecordingSink {
        fn on_event(&mut self, e: CoreEvent) { self.0.push(e); }
        fn on_error(&mut self, _: String) {}
    }

    #[test]
    fn creates_profile_emits_event() {
        let store = InMemoryConfigStore { configs: Default::default() };
        let mut sink = RecordingSink(vec![]);
        let input = CreateProfileInput::new("alice", "claude", Scope::Workspace);

        let result = run(&input, &store, &mut sink);

        assert!(result.is_ok());
        assert!(matches!(&sink.0[0], CoreEvent::ProfileCreated { .. }));
    }

    #[test]
    fn rejects_duplicate_profile_id() {
        // ... same pattern, expects on_error
    }
}
```

## Fake Ports (instead of mocks)

For every port trait, write a tiny in-memory or recording fake when you need it in a test. Keep them inside the `#[cfg(test)]` module of the use case that uses them. If a fake gets used by 3+ tests, promote it to `src/app/test_support/`.

Why no `mockall`?
- Our ports are small (2–5 methods). Hand fakes are 10–20 lines.
- Mocks couple tests to method-call sequences; fakes test behaviour.
- Mocks break on refactor; fakes survive a port redesign as long as the behaviour stays sensible.

## Async Tests

Sparingly. Most use cases are sync. For TUI integration tests that need the runtime:

```rust
#[tokio::test]
async fn presenter_forwards_core_event_to_channel() {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut presenter = TuiPresenter { tx };
    presenter.on_event(CoreEvent::ProfileCreated { id: "p".into() });
    let received = rx.try_recv().unwrap();
    assert!(matches!(received, AppEvent::CoreEvent(CoreEvent::ProfileCreated { .. })));
}
```

## Contract Tests (CLI/TUI Equivalence)

This is AGK-specific. Same `CoreCommand`, two adapters, identical events.

```rust
// tests/tui_cli_equivalence.rs
#[test]
fn start_profile_dry_run_emits_same_events_in_both_adapters() {
    let cmd = CoreCommand::StartProfile {
        id: ProfileId::from("alice"),
        scope: Scope::Workspace,
        dry_run: true,
    };

    let cli_events = capture_via_cli_presenter(cmd.clone());
    let tui_events = capture_via_tui_presenter(cmd);

    assert_eq!(cli_events, tui_events);
}
```

## Architecture Tests

In `tests/architecture.rs`. Marked `#[ignore]` so they run only with `cargo test --test architecture -- --ignored`. CI runs them with that flag. These tests are **the source of truth** for our dependency rules.

Always present:
- `domain_must_not_import_app` / `_infra` / `_cli` / `_tui`
- `domain_must_not_use_fs` / `domain_must_not_spawn_processes`
- `app_must_not_import_tui` / `_cli`
- `infra_must_not_import_tui` / `_cli`
- `tui_must_not_import_infra`
- `agk_core_is_send_sync`
- File size budget (~300 LOC for non-test code)

## Test Naming

Descriptive names that explain the scenario, snake_case, no `test_` prefix needed inside `mod tests`:

- ✅ `creates_profile_emits_event`
- ✅ `rejects_invalid_vault_ref_at_boundary`
- ✅ `start_profile_dry_run_does_not_spawn`
- ❌ `test1`, `it_works`, `test_create`

## Coverage

- Target: 80%+ line coverage on `src/app/features/` and `src/domain/`.
- Adapters (`cli/`, `tui/`) are tested via contract + snapshot layers, not line coverage.
- Tool: `cargo llvm-cov`.

```bash
cargo llvm-cov                        # text summary
cargo llvm-cov --html                 # HTML report at target/llvm-cov/html
cargo llvm-cov --fail-under-lines 80  # CI gate
```

## Commands

```bash
cargo test                                       # unit + integration (skips architecture)
cargo test -- --nocapture                        # show println output
cargo test creates_profile                       # run by name pattern
cargo test --lib                                 # unit tests only
cargo test --test architecture -- --ignored      # architecture suite
cargo test --doc                                 # doctests
```

## What NOT to Do

- ❌ `mockall::mock!` — use hand fakes (see above).
- ❌ Shared mutable state across tests (`lazy_static`, global `Mutex`) — each test owns its own fakes.
- ❌ Tests that read real `~/.config/agk/*.toml` — always use `tempfile::tempdir()`.
- ❌ Tests that depend on a specific `cargo test` order — they must pass in isolation.
- ❌ Asserting on `Debug` output strings — assert on the typed values.
- ❌ Skipping the `RecordingSink` and asserting via `println!` — use the sink, return a structured assertion.
