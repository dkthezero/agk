# Technical Design: Headless CLI Operations

## Overview

The headless CLI is the non-interactive API surface for `agk`. It reuses the same business logic as the TUI but executes it synchronously (or via direct `await` of async functions) without spawning the Ratatui/Crossterm event loop.

## Architecture Rules

1. **No TUI imports in CLI commands.** All subcommands must be implementable using only `app/` and `domain/` modules, plus the registry from `bootstrap`.
2. **Pure async functions in `app/actions.rs`.** The TUI's `spawn_blocking` wrappers are TUI-specific; the CLI awaits these functions directly.
3. **Exit codes are product surface.** They must be consistent, documented, and tested.
4. **JSON output is a schema contract.** Once shipped, field names and structures are stable.

## Data Schemas

### Exit Codes
```rust
pub enum ExitCode {
    Success = 0,
    GeneralFailure = 1,
    ValidationFailure = 2,
    PartialSuccess = 3,
}
```

### JSON Output Schema (Common Wrapper)
```rust
#[derive(Serialize)]
struct JsonResult<T> {
    success: bool,
    exit_code: u8,
    data: Option<T>,
    errors: Vec<String>,
}
```

### SyncResult
```rust
#[derive(Serialize)]
struct SyncResult {
    installed: Vec<AssetIdentity>,
    updated: Vec<AssetIdentity>,
    removed: Vec<AssetIdentity>,
    skipped: Vec<AssetIdentity>,
    errors: Vec<SyncError>,
}

#[derive(Serialize)]
struct SyncError {
    identity: AssetIdentity,
    error: String,
}
```

### InstallResult
```rust
#[derive(Serialize)]
struct InstallResult {
    installed: bool,
    identity: AssetIdentity,
    providers: Vec<String>,
    sha10: String,
}
```

### ValidateResult
```rust
#[derive(Serialize)]
struct ValidateResult {
    passed: bool,
    assets: Vec<AssetValidation>,
}

#[derive(Serialize)]
struct AssetValidation {
    identity: AssetIdentity,
    vault_id: String,
    sha10_match: bool,
    provider_paths_exist: Vec<(String, bool)>,
    parse_error: Option<String>,
}
```

## Internal Workflows

### `agk sync` Workflow
1. Parse `--global` / `--scope workspace` to determine `Scope`.
2. Load config for the scope.
3. For each asset in config's installed lists:
   a. Resolve the source vault.
   b. If `--dry-run`: collect what would change, skip I/O.
   c. Scan the vault to get the latest `ScannedPackage`.
   d. Compare `sha10`. If different, call `update_asset()`.
   e. If missing from provider, call `install_asset()`.
4. Report results.

### `agk install` Workflow
1. Parse identity string (`vault/name:version` or `name`).
2. If vault not specified, search all configured vaults.
3. If version not specified, use the latest available.
4. For each active provider in scope: call `install_asset()`.
5. Report results.

### `agk validate` Workflow
1. Load config for the scope.
2. For each installed asset:
   a. Locate the source vault.
   b. Scan and find the matching package.
   c. Compare `sha10`.
   d. Verify provider directories exist.
   e. Parse the marker file (SKILL.md / AGENTS.md) for integrity.
3. Report pass/fail per asset.

### `agk pack` Workflow
1. Resolve identity to `ScannedPackage`.
2. Determine target format from `--target` flag.
3. For Claude Desktop zip: recursively copy the package directory into a temp folder, then zip.
4. Write to output path or stdout.

## Trait Contracts

No new traits are required. The CLI uses existing:
- `ConfigStorePort::load/save`
- `VaultPort::list_packages`
- `ProviderPort::install/remove`
- `FeatureSetPort::is_package`, `hash_files`

## Module Structure

```
src/cli/
  mod.rs                # Re-export
  entry.rs              # clap parser (extended with subcommands)
  commands.rs           # Legacy command implementations (sync, install, validate, pack)
  core_dispatcher.rs      # Phase 4 — CLI → CoreCommand translation + dispatch via AgkCore
  presenter.rs          # Phase 4 — CoreEventSink with --quiet / --json support
```

### `cli/core_dispatcher.rs`

`to_core_command()` translates clap `Commands` enum into `app::command::CoreCommand` variants:

| CLI Subcommand | CoreCommand Variant |
|---------------|-------------------|
| `profile start <name> --dry-run` | `StartProfile { id, scope, dry_run }` |
| `profile create <name> --provider ...` | `CreateProfile { input }` |
| `mcp add ...` | `RegisterMcp { input }` |
| `sync --dry-run` | `SyncAssets { scope, dry_run }` |
| `install <identity> --dry-run` | `InstallAsset { identity, ... }` |

`dispatch(cli, workspace, core)` creates a `CliPresenter`, runs `core.execute()`, and maps `CoreOutcome` to process exit codes.

### `cli/presenter.rs`

`CliPresenter` implements `CoreEventSink` with three modes:
- **Quiet** — swallows all events except errors (writes to stderr).
- **Normal** — prints human-readable lines to stdout.
- **JSON** — emits structured JSON objects per event (`event_to_json` helper maps `CoreEvent` → `serde_json::Value` without requiring `CoreEvent` to derive `Serialize`).

The presenter buffers events when `--json` is used so the final output is a single JSON array.

## Testing Strategy

- **Unit tests:** Each command function tested with fake store, fake provider, fake vault.
- **Integration tests:** `tests/cli_*.rs` using `assert_cmd` to verify exit codes and JSON output.
- **Regression:** Ensure `cargo run` (TUI mode) still works exactly as before.

---

*End of Technical Design.*
