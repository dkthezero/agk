# Design Spec: "Init as Vault" TUI Feature

**Date:** 2026-06-06
**Status:** Approved
**Epic:** Vault Mode UX

---

## 1. Summary

Add an `[F1] Init as Vault` action to the Vaults tab that lets users initialize the current workspace as a vault source repository without leaving the TUI. Mirrors `agk vault init` semantics exactly. Only visible when the workspace is not already a vault.

---

## 2. User Story

> As a developer, I want to turn my current workspace into a vault from within the TUI so I don't have to drop to the shell to run `agk vault init`.

**Success criteria:**
- Pressing `F1` on the Vaults tab (non-vault workspace) shows a confirm modal
- Confirming runs `vault_init` and immediately switches the TUI to vault mode
- The `[F1]` keybind disappears after init (workspace is now a vault)
- Cancelling does nothing

---

## 3. Design

### 3.1 Keybinds

Vaults tab keybinds change based on `is_vault_workspace`:

| State | Keybinds shown |
|---|---|
| Not vault workspace | `[↑/↓] Move  [F1] Init as Vault  [F2] Attach New  [Space] Toggle  [F4] Refresh  [Esc]x2 Quit` |
| Vault workspace | `[↑/↓] Move  [F2] Attach New  [Space] Toggle  [F4] Refresh  [Esc]x2 Quit` (unchanged) |

`F1` is only wired when `!state.is_vault_workspace`. Pressing `F1` on any other tab does nothing.

### 3.2 Confirmation Modal

Reuses the existing confirm-modal pattern (`ConfirmDetachVault`, `ConfirmDeleteProfile`):

```
┌─────────────────────────────────────────────────────┐
│  Initialize this workspace as a vault?              │
│                                                     │
│  Creates:                                           │
│    .agk/vault.toml   (vault metadata)               │
│    skills/           (skill assets)                 │
│    instructions/     (instruction assets)           │
│    mcps/             (MCP server assets)            │
│    profiles/         (profile assets)               │
│                                                     │
│  Vault name: <folder-name>                          │
│                                                     │
│           [Enter] Confirm   [Esc] Cancel            │
└─────────────────────────────────────────────────────┘
```

The vault name is derived from the workspace folder name (same as CLI). No text input required.

### 3.3 State Machine

```
Normal (Vault tab, !is_vault_workspace)
  → [F1] → ConfirmVaultInit
      → [Esc]  → Normal
      → [Enter] → dispatch CoreCommand::VaultInit { name: None, dry_run: false }
                    → VaultInitialized event
                    → state.is_vault_workspace = true
                    → TriggerReload
                    → scope label: "[VAULT]", toggle_scope locked
```

`ConfirmVaultInit` is a new `ListMode` variant, consistent with `ConfirmDetachVault`.

### 3.4 Post-Init State Transition

`VaultInitialized` is handled in two places:

1. **`core_event_reducer.rs`**: sets `state.is_vault_workspace = true` immediately so the label and scope lock update before the reload completes.
2. **`runtime_loop.rs`**: `VaultInitialized` added to the auto-`TriggerReload` list so the vault entries and package scan refresh.

---

## 4. Affected Files

| File | Change |
|---|---|
| `src/tui/list_mode.rs` | Add `ConfirmVaultInit` variant |
| `src/tui/widgets/status.rs` | Add `[F1] Init as Vault` to Vault tab keybinds when `!is_vault_workspace` |
| `src/tui/features/vaults/controller.rs` | Handle `F1` → `ConfirmVaultInit`; `Enter` in that mode → dispatch `VaultInit` |
| `src/tui/render/modals.rs` | Add `ConfirmVaultInit` arm to the modal match (reuse confirm pattern) |
| `src/tui/core_event_reducer.rs` | On `VaultInitialized`: set `state.is_vault_workspace = true` |
| `src/tui/runtime_loop.rs` | Add `VaultInitialized` to auto-reload match |

No changes to domain, ports, or CLI — `CoreCommand::VaultInit` already exists and routes to `vault_init()`.

---

## 5. Out of Scope

- Prompting for a custom vault name (use folder name, same as CLI)
- Showing a diff/preview of what will be created before confirming
- Un-initializing a vault from the TUI (separate feature)
