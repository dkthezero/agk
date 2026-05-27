# PR #19 — Fixup Plan: Remove dead_code suppression, wire or delete stubs

## Context

PR #19 (`b8bc54a`) claimed "clippy zero-warnings" but achieved it by adding `#![allow(dead_code)]` to `main.rs` as a blanket suppression. This hides ~47 real dead code items that need to be either wired (made live) or deleted. This plan categorizes every item and specifies the action per AGENTS.md architecture rules.

**Branch:** `feat/architectural-convergence` (PR #19)
**Last commit:** `b8bc54a` — "P0-P2 file decomposition + clippy zero-warnings"
**Status before fixup:** 63 clippy errors with suppression removed; `#![allow(dead_code)]` still in `main.rs`
**Target after fixup:** zero clippy errors WITHOUT any `allow(dead_code)` suppression. Every item: wired, deleted, or annotated with targeted `#[allow(dead_code)]` + explanatory comment.

---

## Category A: Delete Speculative TUI Architecture (Never Wired)

These files were created in Phase 1-2 of convergence (Elm-style reducer pipeline) but were never wired into `runtime_loop.rs`. They have no production callers. The old imperative TUI in `event.rs::handle()` + feature controllers works correctly for users.

> AGENTS.md rule: TUI pattern is "Reducer → Mapper → Presenter" but the imperative `event.rs::handle()` was the Phase C deliverable. The Elm pipeline was Phase C++ which is NOT in scope.

| File | Action | Rationale |
|------|--------|-----------|
| `tui/reducer.rs` | DELETE entire file | `reduce_key()` never called in production; only unit tests. Same logic exists in `event.rs::handle()` and `features/*/controller.rs`. Tests only test dead code. |
| `tui/command_mapper.rs` | DELETE entire file | `map_intents()` never called. Only maps 6/20 intents. Tests only. |
| `tui/intent.rs` | DELETE entire file | `UiIntent` enum never constructed in production. Tests only. |
| `tui/app_state.rs` | DELETE entire file | `TuiState` never constructed; `WizardState` never constructed. Tests only. |
| `tui/presenter.rs` | DELETE entire file | `AppEventSink` never constructed. No TUI presenter bridge exists yet. |

**Post-delete cleanup:**
- Remove `pub mod reducer`, `pub mod command_mapper`, `pub mod intent`, `pub mod app_state`, `pub mod presenter` from `tui/mod.rs`
- Remove tests that import deleted modules from other test files
- Update `tui/runtime_loop.rs` doc comment (remove reference to "pure reducer")

**End-to-end test expected after delete:**
```bash
cargo test --all-features # should pass minus tests in deleted files
cargo clippy --workspace --all-targets --all-features -- -D warnings # should have ~30 fewer errors
```

---

## Category B: Delete True Dead Code (Pre-Existing, Not PR #19)

These items existed before PR #19 and were already dead. PR #19 just hid them.

| File | Item | Action | Rationale |
|------|------|--------|-----------|
| `domain/profile.rs` | `VaultId` struct | **DONE** ✅ | Already deleted in previous session. |
| `domain/apply.rs` | `ApplyConfig` struct + builder | DELETE entire file | Never constructed in production. Only used in tests for `apply_config` use-case. Move test helpers into the test module of `apply_config.rs`. |
| `domain/paths.rs` | `contexts_file_path()` | DELETE | No callers. `TomlContextStore` builds its own paths. |
| `domain/paths.rs` | `current_context_path()` | DELETE | Same. Never called. |
| `domain/paths.rs` | `analytics_path()` | DELETE? | Check `telemetry.rs` — if NOT called, delete. If called, keep. |
| `cli/commands/mod.rs` | `telemetry_to_csv()` | DELETE | Never called. `telemetry.rs` has its own CSV formatter. |
| `cli/commands/mod.rs` | `dispatch_core_command()` | DELETE | Never called. Was a Phase B migration shim. Main.rs does its own core construction. |
| `tui/features/vaults/controller.rs` | `VaultsController` struct | DONE ✅ | Already deleted. |

**End-to-end test expected:**
```bash
cargo test --all-features # pass
cargo clippy --workspace --all-targets --all-features -- -D warnings # ~15 fewer errors
```

---

## Category C: Annotate Contract Stubs (Intentionally Unwired)

These are public API contracts that exist for architectural convergence but are not yet fully wired. They are NOT dead code in the sense that they WILL be used. They should carry targeted `#[allow(dead_code)]` + a comment explaining they are contract stubs.

| File | Item | Action |
|------|------|--------|
| `app/command.rs` | `CoreCommand` enum + variants | Add `#[allow(dead_code)]` to enum with comment "Contract enum — variants wired incrementally as use-cases are implemented" |
| `app/event.rs` | `CoreEvent` enum + variants | Same |
| `app/outcome.rs` | `CoreOutcome` enum + variants | Same |
| `app/ports.rs` | `ProcessRunnerPort` trait | Add `#[allow(dead_code)]` with comment "Phase E infra/process/ port — not yet wired" |
| `app/ports.rs` | `ManifestCodecPort` trait | Same — "Phase E config codec port — TomlCodec only implementation so far" |
| `app/ports.rs` | `VaultSearchPort` trait | Same — "wired in core.rs but no actual search implementation yet" |
| `app/ports.rs` | `McpRegistryPort::build_providers`, `enable`, `disable` | Add `#[allow(dead_code)]` on trait methods — "wired in core_dispatcher but core.rs returns not-yet-wired for EnableMcp/DisableMcp" |
| `infra/config/codecs/toml.rs` | `TomlCodec` struct + `new()` | Add `#[allow(dead_code)]` — "Phase E codec stub — no caller selects codec yet" |
| `infra/vault/search_adapters.rs` | `ClawHubSearchAdapter::vault_id` field | Add `#[allow(dead_code)]` — "trait contract field; vault_id() is called in core.rs" |
| `infra/mcp/adapter.rs` | `InfraMcpRegistryAdapter::workspace_root` field | Add `#[allow(dead_code)]` — "needed for future enable/disable use-cases" |

**IMPORTANT:** `NullSink` in `app/outcome.rs` — this IS used in tests but not in production. It's a test-only helper. Move it to the `#[cfg(test)]` module or mark the struct with `#[cfg(test)]`.

**End-to-end test:**
```bash
cargo test --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings # should be clean
```

---

## Category D: Wire What's Easy

These items have a clear path to being wired. Do it now.

| File | Item | How to wire |
|------|------|-------------|
| `app/core.rs:23` | `vault_search` field never read | DONE ✅ — already wired in SearchRemoteVault match arm in current working tree |
| `app/usecases/search_remote_vault.rs` | `run()` takes `searcher` now | DONE ✅ — test updated to use `FakeSearcher` implementing `VaultSearchPort` |
| `app/core.rs` | `CoreCommand::SearchRemoteVault` | DONE ✅ — now calls `search_remote_vault::run(self.vault_search.as_ref(), ...)` |

---

## Category E: Fix TabKind Utility Functions

| File | Item | Action |
|------|------|--------|
| `app/tab_kind.rs` | `is_asset_like()`, `asset_label()` | Used by `reducer.rs` (deleted) and `entry.rs` (builds tab metadata). Check if `entry.rs` or other production code uses them. If not used anywhere in production → annotate with `#[allow(dead_code)]` as "tab metadata helpers used by render layer". If truly never used by anything that renders → delete. |
| `app/tab_kind.rs` | `tab_kind_for_feature_name()`, `tab_kind_for_asset_kind()` | Same. Check if any render code uses them. If not, delete. |

---

## Category F: CLI Presenter Methods

| File | Item | Action |
|------|------|--------|
| `cli/presenter.rs` | `mode()`, `print_json_event()` | These are public API on `CliPresenter`. They are for external callers (e.g., tests or future code) to query state. Mark with `#[allow(dead_code)]` + comment "pub accessor for tests / future instrumentation". |

---

## Execution Order

1. **Delete Category A** (speculative TUI architecture files) — biggest bang for buck
2. **Delete Category B** (true dead code) — quick wins
3. **Wire Category D** (already done mostly) — verify
4. **Annotate Category C** (contract stubs) — with targeted allows + comments
5. **Fix Category E** (TabKind) — use or delete
6. **Fix Category F** (CLI presenter) — annotate
7. **Verify:** `cargo test --all-features` + `cargo clippy --workspace --all-targets --all-features -- -D warnings` + `cargo fmt --check`
8. **Commit as fixup on top of `b8bc54a`**

---

## Post-Fixup Commit Message Template

```
fixup(#19): remove dead_code suppression, clean speculative architecture

PR #19 used #![allow(dead_code)] as a blanket suppression to achieve
"zero clippy warnings". This fixup removes the suppression and addresses
every item properly:

Deleted (speculative TUI architecture never wired):
- tui/reducer.rs — pure reducer, never called in production
- tui/command_mapper.rs — intent mapper, never called
- tui/intent.rs — UiIntent enum never constructed
- tui/app_state.rs — TuiState/WizardState never constructed
- tui/presenter.rs — AppEventSink never constructed

Deleted (true dead code):
- domain/apply.rs — ApplyConfig only in tests, moved helpers inline
- domain/paths.rs: contexts_file_path(), current_context_path(), analytics_path()
- cli/commands/mod.rs: telemetry_to_csv(), dispatch_core_command()

Annotated with #[allow(dead_code)] + comment (contract stubs):
- app/command.rs CoreCommand
- app/event.rs CoreEvent
- app/outcome.rs CoreOutcome
- app/ports.rs ProcessRunnerPort, ManifestCodecPort, VaultSearchPort
- infra/config/codecs/toml.rs TomlCodec
- infra/vault/search_adapters.rs vault_id field
- infra/mcp/adapter.rs workspace_root field

Wired:
- CoreCommand::SearchRemoteVault now uses vault_search port
- search_remote_vault.rs updated signature + tests

Quality gates:
- cargo test --all-features: ### passed
- cargo clippy --workspace --all-targets --all-features -- -D warnings: clean
- cargo test --test architecture -- --ignored: ### passed
- cargo fmt --check: clean
```

---

*Plan created by agent on 2026-05-27 after reviewing docs/proposals/*
