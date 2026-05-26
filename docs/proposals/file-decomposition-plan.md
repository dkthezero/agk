# File Decomposition Plan: >300 LOC Files

## Executive Summary

18 files exceed 300 LOC. 4 files are CRITICAL/HIGH complexity and must be split immediately for maintainability. 10 are MEDIUM and should be split in subsequent PRs. 4 are LOW complexity but large (mostly data/schema definitions) and can stay or be split later.

## Priority Matrix

| Priority | File | LOC | Rating | Concerns | Action |
|---|---|---|---|---|---|
| **P0** | `src/tui/event.rs` | 2417 | CRITICAL | 196 branches, UI+I/O+Logic, 63 functions | Split into 4 feature controllers |
| **P0** | `src/tui/render.rs` | 391 | CRITICAL | Single 390-line `draw` function | Split by tab/widget |
| **P1** | `src/cli/commands.rs` | 1231 | HIGH | 85 branches, CLI+Infra+Logic | Split per-command |
| **P1** | `src/infra/provider/opencode.rs` | 1068 | HIGH | 133-line session fn, 82 branches | Split by concern |
| **P2** | `src/app/bootstrap.rs` | 508 | HIGH | I/O+Logic mixed | Split registry/scan/state |
| **P2** | `src/app/actions.rs` | 523 | HIGH | 25 functions, mixed logic | Split by feature |
| **P2** | `src/app/ports.rs` | 391 | HIGH | 66 items, 8 traits | Split into port modules |
| **P3** | `src/app/core.rs` | 362 | MEDIUM | 22 functions | Split execute match |
| **P3** | `src/app/usecases/apply_config.rs` | 362 | MEDIUM | 16 functions | Split apply |
| **P3** | `src/tui/reducer.rs` | 374 | MEDIUM | 20 functions | Already partial |
| **P3** | `src/tui/runtime_loop.rs` | 316 | MEDIUM | Already extracted | Leave as-is |
| **P4** | `src/infra/telemetry/parser.rs` | 458 | MEDIUM | 38 functions | Split by provider |
| **P4** | `src/app/bundling.rs` | 320 | MEDIUM | 15 functions | Acceptable |
| **P5** | `src/tui/app.rs` | 441 | MEDIUM | Data-heavy | Acceptable |
| **P5** | `src/tui/widgets/detail.rs` | 415 | LOW | UI only | Acceptable |
| **P5** | `src/tui/widgets/modal.rs` | 449 | LOW | UI only | Acceptable |
| **P5** | `src/tui/widgets/list.rs` | 321 | LOW | UI only | Acceptable |
| **P5** | `src/cli/entry.rs` | 351 | LOW | Declarative args | Acceptable |
| **P5** | `src/domain/config.rs` | 371 | LOW | Data schema | Acceptable |

## Phase I: Critical Files (P0)

### 1A. `src/tui/event.rs` → Feature Controllers

**Files to Create (10 new):**
```
src/tui/features/
├── profiles/controller.rs      # Profile create/delete/wizard
├── vaults/controller.rs        # Vault attach/detach/refresh (partial exists)
├── mcps/controller.rs           # MCP register/enable/disable
├── providers/controller.rs     # Provider toggle/roots
├── assets/controller.rs        # Install/remove/validate
├── mod.rs                      # Re-exports
```

**Migration:**
- Extract `handle_*` functions into controllers
- Keep `pub fn handle()` as thin dispatcher in `event.rs`
- Target: `event.rs` < 400 lines

### 1B. `src/tui/render.rs` → Per-Tab Widgets  

**Files to Create (7 new):**
```
src/tui/widgets/
├── tabs.rs, skills_tab.rs, mcp_tab.rs, providers_tab.rs
├── profiles_tab.rs, vaults_tab.rs, instructions_tab.rs
```

Target: `render.rs` < 100 lines

## Phase II: High Priority (P1-P2)

### 2A. `src/cli/commands.rs` → Per-Feature Modules

**Files to Create (8 new):**
```
src/cli/commands/
├── mod.rs, profiles.rs, assets.rs, mcps.rs
├── vaults.rs, providers.rs, telemetry.rs, contexts.rs
```

Target: `commands.rs` < 300 lines

### 2B. `src/infra/provider/opencode.rs`

**Files to Create (4 new):**
```
src/infra/provider/opencode/
├── mod.rs, config.rs, install.rs, session.rs, mcp.rs
```

## Phase III: Medium (P3-P4)

### 3A. `src/app/bootstrap.rs`
Split into `bootstrap/mod.rs`, `registry.rs`, `scan.rs`, `state.rs`

### 3B. `src/app/actions.rs`
Split into `actions/mod.rs`, `install.rs`, `remove.rs`, `sync.rs`

### 3C. `src/app/ports.rs`
Split into `ports/mod.rs` + `ports/{config_store,vault,provider,mcp,process}.rs`

### 3D. `src/infra/telemetry/parser.rs`
Split into `parser/mod.rs` + per-provider parsers

## Phase IV: Low Priority

Widgets, entry.rs, config.rs — acceptable as-is. Add architecture allowlists with justification.

## Estimates

| Phase | Duration | Files |
|---|---|---|
| I (tui/event.rs) | 5-7 days | 10 |
| II (render.rs + CLI) | 5-8 days | 15 |
| III (provider + app) | 4-6 days | 10 |
| IV (ports + telemetry) | 3-5 days | 8 |
| V (Low priority) | 2-3 days | 4 |

**Total: ~17-25 engineering days**
