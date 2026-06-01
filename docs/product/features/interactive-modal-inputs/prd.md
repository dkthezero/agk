# Interactive Modal Inputs

**Status:** Implemented (v0.2)

## Overview
Replace bottom-of-screen question-and-answer interactive prompts with centered pop-up modals, matching the existing provider config root selection UX.

## Feature Goals
- Improve discoverability and visual focus during interactive prompts.
- Unify all interactive flows under a single modal pattern.
- Reduce footer/status-line clutter during multi-step wizards.

## Supported Interactive Flows
1. **Attach Vault Wizard**
   - Step 1: Enter local path or GitHub URL.
   - Step 2: Enter branch (default `main`).
   - Step 3: Enter subfolder path (default `skills/`).
2. **Register MCP Server Wizard**
   - Step 1: MCP server name.
   - Step 2: Command to run.
   - Step 3: Arguments (optional, space-separated).
   - Step 4: Transport (`stdio` by default).
   - Step 5: Description (optional).
   - Step 6: Execute/test confirmation before final registration.
3. **Confirmations**
   - Detach vault confirmation.
   - ClawHub CLI install confirmation.
4. **Provider Config Root Selection** *(already implemented as modal — no functional change, pattern reference)*.

## User-Facing Behavior
- When an interactive action is triggered (`[F2]` Attach Vault, `[F2]` Add MCP, `[Space]` Detach Vault, etc.), a centered modal appears instead of typing into the status line.
- The modal clearly displays the current step title, field label, and the typed value (for input modals) or the confirmation message (for confirmation modals).
- Pressing `[Enter]` confirms, `[Esc]` cancels.
- For confirmation modals, `[Enter]` confirms and `[Esc]` cancels (also supports `[y]` and `[n]`).
- The status/footer bar reverts to standard keybind hints while a modal is open.

## Acceptance Criteria
- [x] All Attach Vault steps render inside a centered input modal.
- [x] All MCP registration steps render inside a centered input modal.
- [x] Detach vault, ClawHub install, and MCP test confirmations render inside a centered confirmation modal.
- [x] Footer keybinds adapt when a modal is open.
- [x] No behavioral regressions: Esc, Enter, Backspace, text input, and y/n confirmations continue to work.
- [x] Code compiles, passes formatting (`cargo fmt`), linting (`cargo clippy`), and tests (`cargo test`).
