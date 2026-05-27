# Decomposition Plan — P0/P1/P2 File Split + Clippy Cleanup

## Status: In Progress — Phase A/C clippy fixes + P0 tui/event.rs decompose active

---

## Step 1: Fix all clippy/compilation errors (ZERO BEHAVIOR CHANGE)

### 1a. tests/architecture.rs (3 warnings)
- Line 35: `.into_iter()` chain → remove `.into_iter()` + remove `&` from `WalkDir::new(&root)` → `WalkDir::new(root)`
- Line 121: same `&root` → `root`
- Line 331: same `&root` → `root`

### 1b. src/app/snapshot.rs (1 warning)
- Lines 2-3: doc comment followed by empty line → merge into single doc comment block

### 1c. Unused import cleanup
- src/app/usecases/attach_vault.rs: remove `CoreEvent`, `CoreOutcome`, `CoreResult` imports
- src/app/usecases/register_mcp.rs: add `Scope` to test module imports (fix compilation)
- src/cli/commands/mod.rs: remove `ConfigStorePort` import
- src/cli/core_dispatcher.rs: remove `ScopeArg` import, add `ScopeArg` to test module
- src/app/bundling.rs: remove `AssetBucket`, `VaultSection` imports

### 1d. `yaml` feature flag (1 warning)
- `cargo check` notes that `#[cfg(feature = "yaml")]` references a nonexistent feature
- Cargo.toml does not define `yaml` feature; add it to `[features]`: `yaml = []`

### 1e. Safe auto-fixes (already ran `cargo clippy --fix`)
- Fixed: needless borrows, useless conversion, redundant closures
- Remaining to fix manually

### 1f. OpenCodeProvider `build_launch_plan` compilation error
- Tests in `src/infra/provider/opencode/mod.rs` reference `.build_launch_plan()` which lives in `session.rs` on the `ProfileRuntimePort` trait
- Fix: add `use crate::app::ports::ProfileRuntimePort;` in the `#[cfg(test)]` block of opencode/mod.rs

### 1g. Manual fixes required (field_reassign_with_default, unused variables, dead_code)
- `src/tui/event.rs` lines 2313-2314: `ConfigFile::default()` + field assign → use struct literal
- `src/infra/config/toml_store.rs` lines 115-116 + 139-140: same pattern
- `src/app/bundling.rs` test unused vars (`a`, `b`, `registry`, `_root`)
- `src/tui/presenter.rs` unused `vault_id` → prefix with `_`
- `src/tui/reducer.rs` unused functions `handle_modal`, `derive_enter_intent`, `derive_space_intent`, `is_vault_tab_active`, `is_mcp_tab_active`, `is_profile_tab_active` — we won't fix these now (they belong to the reducer/controller migration in a later phase)
- `src/cli/core_dispatcher.rs` unused `display_name` → `_display_name`
- Various dead_code warnings → do not fix (these are architecture stubs per the convergence plan)

## Step 2: Split `src/app/bootstrap.rs` (P2)

Target tree:
```
src/app/bootstrap/
├── mod.rs       # re-export, ~60 lines
├── registry.rs  # build_registry(), register_providers(), register_vaults(), ~200 lines
├── scan.rs      # scan(), filter_scan(), ~120 lines
└── state.rs     # build_vault_entries(), build_provider_entries(), build_profile_entries(), build_tab_kinds(), ~140 lines
```

Keep `src/app/bootstrap.rs` as comment-only shim for now (re-export + note).

## Step 3: Split `src/app/actions.rs` (P2)

Target tree:
```
src/app/actions/
├── mod.rs       # re-export, prune_empty_vault_defs
├── install.rs   # install_asset, install_provider, update_asset
├── remove.rs    # remove_asset, remove_provider, prune_orphans (if any)
└── sync.rs      # sync helpers (if any inline sync functions exist)
```

Keep `src/app/actions.rs` as comment-only shim.
Update callers in `cli/commands/assets.rs` and `tui/event.rs` to use new paths.

## Step 4: Split `src/cli/commands/assets.rs` (P1)

Target tree:
```
src/cli/commands/assets/
├── mod.rs      # re-export + shared helpers (find_package_by_identity, resolve_scope)
├── install.rs  # cmd_install single + bulk
├── remove.rs   # cmd_remove + sync_remove
└── search.rs   # local filter, ClawHub search
```

Keep `src/cli/commands/assets.rs` as comment-only shim.
Update `cli/commands/mod.rs` references.

## Step 5: Decompose `src/tui/event.rs` (P0 — THE BIG ONE)

Target: event.rs becomes a pure dispatcher (< 150 lines).

### Files to create
1. `src/tui/features/mod.rs` — re-export all feature modules
2. `src/tui/features/common/mod.rs` — tab switch, search, esc, backspace, F-keys, nav
3. `src/tui/features/common/controller.rs` — handle_esc, handle_backspace, handle_f_keys, handle_navigation, handle_space dispatch, handle_enter dispatch
4. `src/tui/features/common/actions.rs` — apply_tab_switch, apply_search_char, apply_esc, apply_scope_toggle, apply_space_no_provider, apply_enter_attach_vault, apply_enter_register_mcp, apply_enter_add_profile, parse_github_url, active_providers, execute_attach_vault
5. `src/tui/features/providers/controller.rs` — handle_select_provider_root, handle_deactivate_last_provider_confirm, handle_deactivate_last_provider_cancel, handle_space_provider, toggle_provider
6. `src/tui/features/vaults/controller.rs` — handle_attach_vault_input, handle_detach_confirm, handle_detach_cancel, handle_space_vault, handle_enter_attach_vault (move from common)
7. `src/tui/features/mcps/controller.rs` — handle_register_mcp_input, handle_mcp_register_confirm, handle_space_mcp
8. `src/tui/features/profiles/controller.rs` — handle_profile_wizard_input, handle_delete_profile, handle_delete_profile_no_ctx, handle_delete_profile_confirm
9. `src/tui/features/assets/controller.rs` — handle_space_asset, handle_enter (asset update), handle_f5_update_all, handle_install_remote_clawhub
10. `src/tui/features/assets/actions.rs` — dispatch_clawhub_search (async helper)

### Strategy for migration
- For each function moved out of event.rs:
  - Add `pub(crate)` re-export in the target module
  - Copy function body verbatim into new file
  - Comment out the original function in event.rs (keep as migration marker)
  - Add `// use crate::tui::features::common::controller::handle_esc;` in the new event.rs header
- Once all functions are moved and tests pass, uncomment event.rs and remove commented legacy code.

### Tests
- All inline tests from event.rs move to their respective new controller files (profiles controller gets wizard + delete tests, common gets nav/esc/tab tests, assets gets install/update tests, vaults gets attach/detach tests).

## Step 6: Final verification

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-targets --all-features`
- `cargo test --test architecture -- --ignored`
- `cargo check --all-features`

## Execution Order

1. **Parallel:** Subagent A fixes clippy + compiles in bootstrap.rs, actions.rs, cli/assets.rs.  
2. **Parallel:** Subagent B fixes clippy + compiles in tui/event.rs and splits into feature controllers.  
3. **Central:** Integrate + verify all tests pass.

---

*Started: 2026-05-26 — Phase A/C clippy fixes in progress.*
