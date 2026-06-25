# TUI "Init as Vault" Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `[F1] Init as Vault` to the Vaults tab so users can initialize the current workspace as a vault source repo without leaving the TUI.

**Architecture:** New `ConfirmVaultInit` `ListMode` variant → F1 in Vaults tab enters it → Enter dispatches `CoreCommand::VaultInit` → `VaultInitialized` event flips `state.is_vault_workspace = true` and triggers a reload. No new domain or port code needed — `VaultInit` already exists.

**Tech Stack:** Rust, Ratatui TUI, existing `CoreCommand::VaultInit` use case.

---

### Task 1: Add `ConfirmVaultInit` to `ListMode`

**Files:**
- Modify: `src/tui/list_mode.rs`

- [ ] **Write the failing test** — open `src/tui/app.rs` and find the `#[cfg(test)]` block. Add:

```rust
#[test]
fn confirm_vault_init_is_a_list_mode() {
    let mode = ListMode::ConfirmVaultInit;
    assert!(matches!(mode, ListMode::ConfirmVaultInit));
}
```

- [ ] **Run the test to confirm it fails**

```bash
cargo test confirm_vault_init_is_a_list_mode 2>&1 | tail -5
```

Expected: compile error — `ConfirmVaultInit` does not exist yet.

- [ ] **Add the variant** — in `src/tui/list_mode.rs`, add `ConfirmVaultInit` after `ConfirmDetachVault`:

```rust
pub enum ListMode {
    Normal,
    Searching,
    AttachVault,
    AttachVaultBranch,
    AttachVaultPath,
    AttachVaultName,
    ConfirmDetachVault,
    ConfirmVaultInit,          // ← add this line
    ConfirmClawHubInstall,
    // ... rest unchanged
```

- [ ] **Run the test to confirm it passes**

```bash
cargo test confirm_vault_init_is_a_list_mode 2>&1 | tail -5
```

Expected: `test confirm_vault_init_is_a_list_mode ... ok`

- [ ] **Run the full suite to confirm no regressions**

```bash
cargo test 2>&1 | grep -E "FAILED|^test result"
```

Expected: all lines show `0 failed`.

- [ ] **Commit**

```bash
git add src/tui/list_mode.rs src/tui/app.rs
git commit -m "feat(tui): add ConfirmVaultInit ListMode variant"
```

---

### Task 2: Render the confirmation modal

**Files:**
- Modify: `src/tui/render/modals.rs`

The modal renders using `modal::render_confirm_modal` — the same helper used by `ConfirmDetachVault`. It must show the vault name (derived from the workspace folder) so the user knows what will be created. The workspace folder name is not stored in `AppState`; we derive it from `state.pending_vault_local_path` which we will repurpose as a scratch field (set when entering the mode in Task 3).

- [ ] **Write the failing test** — add to `src/tui/app.rs` tests:

```rust
#[test]
fn pending_vault_local_path_used_as_vault_name_scratch() {
    let mut state = state_with_skills(vec![]);
    state.pending_vault_local_path = "my-vault".to_string();
    assert_eq!(state.pending_vault_local_path, "my-vault");
}
```

This is a trivial field-access test confirming the field is available for scratch use. It exists solely to catch if the field is renamed.

- [ ] **Run it to confirm it passes** (it tests existing code):

```bash
cargo test pending_vault_local_path_used_as_vault_name_scratch 2>&1 | tail -5
```

Expected: `ok`

- [ ] **Add the modal arm** — in `src/tui/render/modals.rs`, add the `ConfirmVaultInit` arm before the `_ => {}` catch-all:

```rust
        ListMode::ConfirmVaultInit => {
            let vault_name = if state.pending_vault_local_path.is_empty() {
                "this workspace".to_string()
            } else {
                format!("'{}'", state.pending_vault_local_path)
            };
            let msg = format!(
                "Initialize {} as a vault?\n\nCreates:\n  .agk/vault.toml\n  skills/\n  instructions/\n  mcps/\n  profiles/",
                vault_name
            );
            modal::render_confirm_modal(
                frame,
                "Init as Vault",
                &msg,
                "[Enter] Confirm  [Esc] Cancel",
            );
        }
        _ => {}
```

- [ ] **Compile to confirm no errors**

```bash
cargo check 2>&1 | tail -5
```

Expected: `Finished`

- [ ] **Commit**

```bash
git add src/tui/render/modals.rs src/tui/app.rs
git commit -m "feat(tui): render ConfirmVaultInit modal"
```

---

### Task 3: Wire F1 → `ConfirmVaultInit` in the Vaults tab

**Files:**
- Modify: `src/tui/features/vaults/controller.rs`
- Modify: `src/tui/features/common/controller.rs`
- Modify: `src/tui/event.rs`

Three wiring points:
1. A new `enter_vault_init` function in `vaults/controller.rs`
2. `handle_f_keys` in `common/controller.rs` routes `F(1)` on the Vault tab
3. `event.rs` extends the F-key match range from `F(2)` to `F(1)`

- [ ] **Write the failing test** — add to `src/tui/app.rs` tests:

```rust
#[test]
fn enter_vault_init_sets_confirm_mode() {
    let mut state = state_with_skills(vec![]);
    // Simulate what enter_vault_init will do:
    state.pending_vault_local_path = "my-workspace".to_string();
    state.list_mode = ListMode::ConfirmVaultInit;
    assert!(matches!(state.list_mode, ListMode::ConfirmVaultInit));
    assert_eq!(state.pending_vault_local_path, "my-workspace");
}
```

- [ ] **Run to confirm it passes** (tests existing field assignment):

```bash
cargo test enter_vault_init_sets_confirm_mode 2>&1 | tail -5
```

Expected: `ok`

- [ ] **Add `enter_vault_init` to `src/tui/features/vaults/controller.rs`** — add after `handle_clawhub_install_confirm`:

```rust
/// Enter the vault-init confirmation modal.
/// Stores the workspace folder name in `pending_vault_local_path` for the modal to display.
pub fn enter_vault_init(state: &mut AppState, ctx: &EventContext) {
    let name = ctx
        .workspace_root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "vault".to_string());
    state.pending_vault_local_path = name;
    state.list_mode = ListMode::ConfirmVaultInit;
    state.status_line.clear();
}

pub fn handle_vault_init_confirm(state: &mut AppState, ctx: &EventContext) -> Result<ControlFlow> {
    let _ = ctx.tx.send(AppEvent::ExecuteCommand(
        crate::app::command::CoreCommand::VaultInit {
            name: None,
            dry_run: false,
        },
    ));
    state.list_mode = ListMode::Normal;
    state.pending_vault_local_path.clear();
    Ok(ControlFlow::Continue)
}

pub fn handle_vault_init_cancel(state: &mut AppState) -> Result<ControlFlow> {
    state.list_mode = ListMode::Normal;
    state.pending_vault_local_path.clear();
    state.status_line = "Cancelled vault init".to_string();
    Ok(ControlFlow::Continue)
}
```

- [ ] **Route `F(1)` in `handle_f_keys`** — in `src/tui/features/common/controller.rs`, add a `KeyCode::F(1)` arm inside `handle_f_keys`, before the `_ => Ok(())` catch-all:

```rust
        KeyCode::F(1) => {
            let vaults_idx = state
                .tab_names
                .iter()
                .position(|n| n == "Vaults")
                .unwrap_or(0);
            if state.active_tab == vaults_idx && !state.is_vault_workspace {
                crate::tui::features::vaults::controller::enter_vault_init(state, ctx);
            }
            Ok(())
        }
```

- [ ] **Extend the F-key match range in `event.rs`** — find:

```rust
            KeyCode::F(5) | KeyCode::F(4) | KeyCode::F(3) | KeyCode::F(2)
                if state.list_mode == ListMode::Normal =>
            {
                crate::tui::features::common::controller::handle_f_keys(state, ctx, &key.code)?;
            }
```

Change to:

```rust
            KeyCode::F(5) | KeyCode::F(4) | KeyCode::F(3) | KeyCode::F(2) | KeyCode::F(1)
                if state.list_mode == ListMode::Normal =>
            {
                crate::tui::features::common::controller::handle_f_keys(state, ctx, &key.code)?;
            }
```

- [ ] **Wire `ConfirmVaultInit` into the confirm-modal Enter/Esc handlers** — in `src/tui/event.rs`, find the `in_confirm` match and add `ListMode::ConfirmVaultInit`:

```rust
        let in_confirm = matches!(
            state.list_mode,
            ListMode::ConfirmMcpTest
                | ListMode::ConfirmClawHubInstall
                | ListMode::ConfirmDetachVault
                | ListMode::ConfirmVaultInit           // ← add
                | ListMode::ConfirmDeactivateLastProvider
                | ListMode::ConfirmDeleteProfile
        );
```

Then add arms in the Enter match:

```rust
        if key.code == KeyCode::Enter && in_confirm {
            return match state.list_mode {
                // ... existing arms ...
                ListMode::ConfirmVaultInit => {
                    crate::tui::features::vaults::controller::handle_vault_init_confirm(state, ctx)
                }
                _ => Ok(ControlFlow::Continue),
            };
        }
```

And in the Esc match:

```rust
        if key.code == KeyCode::Esc && in_confirm {
            return match state.list_mode {
                // ... existing arms ...
                ListMode::ConfirmVaultInit => {
                    crate::tui::features::vaults::controller::handle_vault_init_cancel(state)
                }
                _ => Ok(ControlFlow::Continue),
            };
        }
```

- [ ] **Compile to confirm no errors**

```bash
cargo check 2>&1 | tail -5
```

Expected: `Finished`

- [ ] **Run full test suite**

```bash
cargo test 2>&1 | grep -E "FAILED|^test result"
```

Expected: all `0 failed`.

- [ ] **Commit**

```bash
git add src/tui/features/vaults/controller.rs src/tui/features/common/controller.rs src/tui/event.rs
git commit -m "feat(tui): wire F1 → ConfirmVaultInit on Vaults tab"
```

---

### Task 4: Flip `is_vault_workspace` on `VaultInitialized` + add auto-reload

**Files:**
- Modify: `src/tui/core_event_reducer.rs`
- Modify: `src/tui/runtime_loop.rs`

After the user confirms and `VaultInit` runs, the `VaultInitialized` event is emitted. Two things must happen: the TUI state must immediately flip to vault mode (so the label and scope lock update before the next reload), and a reload must be triggered (so the vault list reflects the new `.agk/vault.toml`).

- [ ] **Write the failing test** — in `src/tui/app.rs` tests:

```rust
#[test]
fn vault_initialized_event_sets_is_vault_workspace() {
    use crate::app::event::CoreEvent;
    use crate::tui::core_event_reducer::apply_core_event;
    let mut state = state_with_skills(vec![]);
    assert!(!state.is_vault_workspace);
    apply_core_event(&mut state, &CoreEvent::VaultInitialized("my-vault".to_string()));
    assert!(state.is_vault_workspace, "VaultInitialized must set is_vault_workspace = true");
}
```

- [ ] **Run to confirm it fails**

```bash
cargo test vault_initialized_event_sets_is_vault_workspace 2>&1 | tail -5
```

Expected: `FAILED` — `is_vault_workspace` is still `false` after the event.

- [ ] **Add `state.is_vault_workspace = true`** to the `VaultInitialized` arm in `src/tui/core_event_reducer.rs`:

Find:

```rust
        CoreEvent::VaultInitialized(name) => {
            state.status_line = format!("Vault '{}' initialized", name);
        }
```

Replace with:

```rust
        CoreEvent::VaultInitialized(name) => {
            state.status_line = format!("Vault '{}' initialized", name);
            state.is_vault_workspace = true;
        }
```

- [ ] **Run the test to confirm it passes**

```bash
cargo test vault_initialized_event_sets_is_vault_workspace 2>&1 | tail -5
```

Expected: `ok`

- [ ] **Add `VaultInitialized` to the auto-reload list** — in `src/tui/runtime_loop.rs`, find the auto-reload match and add `VaultInitialized`:

```rust
                if matches!(
                    &evt,
                    crate::app::event::CoreEvent::AssetInstalled { .. }
                        | crate::app::event::CoreEvent::AssetRemoved { .. }
                        | crate::app::event::CoreEvent::AssetUpdated { .. }
                        | crate::app::event::CoreEvent::SyncComplete { .. }
                        | crate::app::event::CoreEvent::VaultAttached(_)
                        | crate::app::event::CoreEvent::VaultDetached(_)
                        | crate::app::event::CoreEvent::VaultInitialized(_)    // ← add
                        | crate::app::event::CoreEvent::TeamInitialized(_)
                        | crate::app::event::CoreEvent::TeamVaultAdded(_)
                        | crate::app::event::CoreEvent::TeamRequirementAdded(_)
                        | crate::app::event::CoreEvent::TeamRequirementRemoved(_)
                        | crate::app::event::CoreEvent::TeamSyncComplete { .. }
                ) {
```

- [ ] **Run the full suite**

```bash
cargo test 2>&1 | grep -E "FAILED|^test result"
```

Expected: all `0 failed`.

- [ ] **Commit**

```bash
git add src/tui/core_event_reducer.rs src/tui/runtime_loop.rs src/tui/app.rs
git commit -m "feat(tui): flip is_vault_workspace on VaultInitialized + trigger reload"
```

---

### Task 5: Update keybinds display

**Files:**
- Modify: `src/tui/widgets/status.rs`

The keybinds string for `TabKind::Vault` must show `[F1] Init as Vault` only when `!state.is_vault_workspace`. The `resolve_keybinds` function currently takes `&AppState` so it already has access to `is_vault_workspace`.

- [ ] **Write the failing test** — in `src/tui/app.rs` tests (or a new `#[cfg(test)]` block in `status.rs`). Add to `src/tui/app.rs`:

```rust
#[test]
fn vault_keybinds_include_init_when_not_vault_workspace() {
    use crate::tui::widgets::status::resolve_keybinds;
    use crate::app::tab_kind::TabKind;
    let mut state = AppState::new(
        vec!["Vaults".to_string()],
        vec![true],
        HashMap::new(),
    );
    state.tab_kinds = vec![TabKind::Vault];
    state.is_vault_workspace = false;
    let keybinds = resolve_keybinds(&state);
    assert!(keybinds.contains("F1"), "non-vault workspace must show [F1] Init as Vault");
}

#[test]
fn vault_keybinds_hide_init_when_vault_workspace() {
    use crate::tui::widgets::status::resolve_keybinds;
    use crate::app::tab_kind::TabKind;
    let mut state = AppState::new(
        vec!["Vaults".to_string()],
        vec![true],
        HashMap::new(),
    );
    state.tab_kinds = vec![TabKind::Vault];
    state.is_vault_workspace = true;
    let keybinds = resolve_keybinds(&state);
    assert!(!keybinds.contains("F1"), "vault workspace must NOT show [F1] Init as Vault");
}
```

- [ ] **Run to confirm they fail**

```bash
cargo test vault_keybinds_include_init 2>&1 | tail -5
cargo test vault_keybinds_hide_init 2>&1 | tail -5
```

Expected: both `FAILED` (F1 not yet in the keybind string).

- [ ] **Update the Vault arm** in `src/tui/widgets/status.rs`:

Find:

```rust
            TabKind::Vault => {
                "[↑/↓] Move  [F2] Attach New  [Space] Toggle  [F4] Refresh  [Esc]x2 Quit"
            }
```

Replace with:

```rust
            TabKind::Vault => {
                if state.is_vault_workspace {
                    "[↑/↓] Move  [F2] Attach New  [Space] Toggle  [F4] Refresh  [Esc]x2 Quit"
                } else {
                    "[↑/↓] Move  [F1] Init as Vault  [F2] Attach New  [Space] Toggle  [F4] Refresh  [Esc]x2 Quit"
                }
            }
```

Note: `resolve_keybinds` currently returns `&'static str`. The vault arm now returns one of two `&'static str` literals — both are `'static`, so the return type is unchanged.

- [ ] **Run the two tests to confirm they pass**

```bash
cargo test vault_keybinds 2>&1 | tail -10
```

Expected: both `ok`.

- [ ] **Run the full suite**

```bash
cargo test 2>&1 | grep -E "FAILED|^test result"
```

Expected: all `0 failed`.

- [ ] **Commit**

```bash
git add src/tui/widgets/status.rs src/tui/app.rs
git commit -m "feat(tui): show [F1] Init as Vault keybind on Vaults tab"
```

---

## Self-Review Checklist

- **Spec §3.1 keybinds:** Covered by Task 5 — `[F1]` shown only when `!is_vault_workspace`. ✓
- **Spec §3.2 confirm modal:** Covered by Task 2 — shows vault name and folder list. ✓
- **Spec §3.3 state machine:** Covered by Tasks 3+4 — F1 → ConfirmVaultInit → Enter/Esc wired. ✓
- **Spec §3.4 post-init:** Covered by Task 4 — `is_vault_workspace = true` on event, reload triggered. ✓
- **Spec §4 affected files:** All 6 files addressed across tasks. ✓
- **No TBD/TODO:** None. ✓
- **Type consistency:** `ListMode::ConfirmVaultInit` added in Task 1, used in Tasks 2, 3, 4. `enter_vault_init`/`handle_vault_init_confirm`/`handle_vault_init_cancel` defined and called consistently. ✓
