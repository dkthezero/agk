# Proposal: Architectural Convergence Plan

**Status:** Planning — Phase A in progress  
**Last updated:** 2026-05-26  
**Target:** Complete convergence so CLI, TUI, and simulator share identical core behavior paths.

---

## 0. Executive Summary

AGK has the right raw ingredients to become a lightweight, production-grade standard for AI coding environment setup. Horizontal roots (`domain`, `app`, `infra`, `cli`, `tui`), core contracts (`CoreCommand`, `CoreEvent`, `CoreOutcome`), port traits, and profile runtime abstractions all exist.

The issue is **incomplete convergence**:
- `main.rs` still routes CLI into legacy `cli::commands::run` instead of `cli/core_dispatcher.rs`.
- `tui/command_mapper.rs` only maps ~6 of ~20+ intents.
- `tui/event.rs` is ~2,400 lines and owns background tasks, interactive process launching, vault refresh, and modal workflows — violating the pure-reducer contract.
- Several use cases (`create_profile`, `register_mcp`) are still Phase 1 stubs.
- Architecture tests exist but are `#[ignore]`d; CI does not run them.
- Config is TOML-only with no `ManifestCodecPort` abstraction.
- No Cargo feature matrix for lightweight builds.

The strategy is **tightening, not rewriting**: preserve roots, finish routing behavior through `AgkCore`, split feature logic horizontally, and lock boundaries in with simulator-backed contract tests and CI gates.

---

## 1. Product Vision & Charter

| Charter item | Recommended statement |
|---|---|
| Product promise | **AGK is the standard, lightweight way to define, share, and launch AI coding environments across solo, team, and enterprise contexts.** |
| Primary user | Solo engineers who want fast, repeatable setups. |
| Secondary user | Platform teams standardizing AI workflows across repositories and organizations. |
| Enterprise buyer | Security, platform, and governance stakeholders who need onboarding consistency, auditability, and policy controls. |
| Core UX principle | Every interactive flow must have a headless equivalent or a `--dry-run` contract. |
| Core architecture principle | Adapters translate intent; only the application core owns behavior. |
| Core packaging principle | Profiles compose references; they do not duplicate skill, provider, vault, or MCP definitions. |
| Lightweight principle | Heavy remote and UI dependencies must be possible to disable through Cargo features and runtime configuration. |

---

## 2. Architecture Overview

We follow a **hybrid horizontal + feature-slice** structure.

```
TUI (tui/)  →  App (app/)  →  Domain (domain/)
                   ↓
              Infra (infra/)
                  ↑
            CLI (cli/)
```

### Dependency Rules (Enforced by `tests/architecture.rs`)

| # | Rule | Enforce by |
|---|------|------------|
| 1 | `domain/` depends on nothing outside `domain/`. | Unignored architecture tests + code review. |
| 2 | `app/` depends on `domain/` and app-internal contracts only. | `tests/architecture.rs` and feature tests. |
| 3 | `infra/` depends on `domain/` and `app::ports`, never on `cli` or `tui`. | Existing architecture tests expanded and enabled. |
| 4 | `cli` and `tui` may depend on `app::{command,event,outcome,snapshot,ports}` and `domain`, but never on concrete `infra`. | Existing TUI infra test plus new behavior tests. |
| 5 | Only `main.rs` and `app/bootstrap.rs` may wire concrete adapters. | Grep-style arch tests and review checklist. |
| 6 | No direct `std::process::Command` calls outside `infra/process/`. | New architecture rule test. |
| 7 | No file may mix more than one feature's business behavior. | AGENTS checklist and LOC/concern split rule. |

### Proposed File Tree (Target)

```
src/
├── main.rs                          # Pure composition root
├── app/
│   ├── bootstrap.rs                 # ONLY place where concrete infra is wired
│   ├── command.rs                   # CoreCommand enum
│   ├── event.rs                     # CoreEvent enum
│   ├── outcome.rs                   # CoreOutcome enum
│   ├── snapshot.rs                  # UI-oriented view models
│   ├── registry.rs                  # Provider/feature registry
│   ├── ports/
│   │   ├── mod.rs
│   │   ├── config_store.rs
│   │   ├── manifest_codec.rs
│   │   ├── vault.rs
│   │   ├── provider.rs
│   │   ├── mcp_registry.rs
│   │   ├── profile_runtime.rs
│   │   └── process_runner.rs
│   ├── features/                    # Feature slices
│   │   ├── apply/
│   │   ├── profiles/
│   │   ├── providers/
│   │   ├── mcps/
│   │   ├── assets/
│   │   └── bundles/
│   └── usecases/                    # Migration path → features/
├── domain/                          # Pure models & invariants
│   ├── asset/
│   ├── profile/
│   ├── vault/
│   ├── mcp/
│   ├── provider/
│   ├── config/
│   └── bundle/
├── infra/                           # Adapters
│   ├── config/
│   │   ├── store.rs
│   │   └── codecs/
│   │       ├── toml.rs
│   │       └── yaml.rs
│   ├── provider/
│   ├── vault/
│   ├── mcp/
│   ├── process/
│   ├── package/
│   └── audit/
├── cli/                             # Thin CLI adapter
│   ├── entry.rs
│   ├── presenter.rs
│   ├── core_dispatcher.rs
│   └── commands/
│       ├── mod.rs
│       ├── apply.rs
│       ├── profiles.rs
│       ├── providers.rs
│       ├── mcps.rs
│       ├── assets.rs
│       └── bundles.rs
└── tui/                             # Thin TUI adapter
    ├── entry.rs
    ├── runtime_loop.rs
    ├── app_state.rs
    ├── reducer.rs
    ├── command_mapper.rs
    ├── presenter.rs
    ├── render.rs
    ├── layout.rs
    ├── features/
    │   ├── profiles/
    │   ├── providers/
    │   ├── vaults/
    │   └── mcps/
    └── widgets/
        └── mod.rs
```

---

## 3. Refactor Actions

| # | Action | Current State | Target State |
|---|--------|---------------|--------------|
| **R1** | Unignore & run architecture tests in CI | All arch tests are `#[ignore]`; CI absent | Architecture job runs on every PR |
| **R2** | Make `main.rs` a pure composition root | Branches to `cli::commands::run` (legacy) | Delegates to `cli::entry::run_headless()` and `tui::entry::run_interactive()` |
| **R3** | Promote `cli/core_dispatcher.rs` to sole CLI path | `cli/commands.rs` has inline business logic | Thin parsing only; all commands route through `AgkCore` |
| **R4** | Split `tui/event.rs` into runtime loop + feature controllers | ~2,400-line imperative hub | `runtime_loop.rs` + `features/*/controller.rs` |
| **R5** | Expand `command_mapper.rs` to full intent coverage | Partial mapping | 100% intent → `CoreCommand` mapping |
| **R6** | Finish stubbed use cases | `create_profile`, `register_mcp` are stubs | Real config/MCP save through ports |
| **R7** | Introduce `ManifestCodecPort` (YAML/TOML) | Hard-coded TOML | Trait-based codec selection by extension |
| **R8** | Extract process launching to `infra/process/` | Direct `std::process::Command` in `main.rs` | `ProcessRunnerPort` + `StdProcessRunner` adapter |
| **R9** | Add Cargo feature matrix | All features unconditional | Default `tui,remote,toml`; optional `yaml,enterprise` |
| **R10** | File-size lint in CI & AGENTS.md | Rule present, not enforced | CI fails on >~300 LOC non-test files |
| **R11** | Introduce `app/features/` slices | Flat `app/usecases/` | Grouped by feature with `usecase.rs`, `planner.rs` |
| **R12** | Add contract/fixture tests | No end-to-end contract tests | Golden fixtures for profile start dry-run |

---

## 4. Phased Implementation Plan

### Phase A — Enforce Boundaries (Foundation)
**Goal:** Make architecture rules real before any code moves.

1. Remove `#[ignore]` from `tests/architecture.rs` (or add a non-ignored runner).
2. Update CI with `architecture` and `feature-matrix` jobs.
3. If violations exist, create temporary allowlists with `TODO(#ticket)` comments.

**Duration:** 1–2 days  
**Risk:** Existing violations block CI. **Mitigation:** Temporarily allowlist with comments.

---

### Phase B — CLI Convergence
**Goal:** All headless commands route through `AgkCore`.

1. In `main.rs`, replace `cli::commands::run(cli, &workspace)` with `cli::core_dispatcher::dispatch(cli, workspace, &core)`.
2. Expand `cli/core_dispatcher.rs::to_core_command()` to cover all `Commands` variants.
3. Move inline business logic from `cli/commands.rs` into `app/usecases/`.
4. Shrink `cli/commands.rs` to output-formatting helpers and argument parsing.

**Duration:** 3–5 days  
**Risk:** Some commands have complex legacy fallbacks. **Mitigation:** Keep thin compatibility shim.

---

### Phase C — TUI Decomposition
**Goal:** `tui/event.rs` is <~300 lines; all feature behavior lives in reducers/mappers/controllers.

1. Extract `tui/runtime_loop.rs` that only matches `AppEvent` variants and delegates to feature controllers.
2. Create `tui/features/vaults/controller.rs`, `tui/features/profiles/controller.rs`, `tui/features/mcps/controller.rs`, `tui/features/providers/controller.rs`.
3. Move side-effects (vault refresh spawning, ClawHub search, interactive process launch) into controllers that emit `CoreCommand`s or handle `CoreEvent`s.
4. Expand `command_mapper.rs` to 100% coverage.

**Duration:** 5–7 days  
**Risk:** Transient UI regressions. **Mitigation:** Migrate tab-by-tab; preserve reducer unit tests.

---

### Phase D — Use-Case Completion & Feature Slicing
**Goal:** Every stub is a real use case with fake-port tests.

1. Rewrite `create_profile` to load/save through `ConfigStorePort`.
2. Rewrite `register_mcp` to use `McpRegistryPort`.
3. Add missing use cases (`delete_profile`, `update_profile`, `apply_manifest`, `export_bundle`).
4. Migrate completed use cases from `app/usecases/` → `app/features/<feature>/`.

**Duration:** 4–6 days  
**Risk:** Config persistence shape changes. **Mitigation:** Additive only; no breaking TOML schema.

---

### Phase E — Config Codecs & Feature Slimming
**Goal:** YAML/TOML coexistence and optional feature builds.

1. Define `ManifestCodecPort` in `app/ports.rs`.
2. Create `infra/config/codecs/toml.rs` and `infra/config/codecs/yaml.rs`.
3. Refactor `toml_store.rs` → `infra/config/store.rs` with auto-selected codec.
4. Add Cargo features: `tui`, `remote`, `telemetry`, `yaml`, `clawhub`, `enterprise`.
5. Update CI with `feature-matrix` job.
6. Gate heavy dependencies behind features.

**Duration:** 3–5 days  
**Risk:** Migration/round-trip for existing `config.toml`. **Mitigation:** Default TOML; preserve extension.

---

### Phase F — Simulator & Contract Tests
**Goal:** HTML simulator becomes contract source for key flows.

1. Add `fixtures/contracts/` directory with JSON contract fixtures.
2. Add `tests/contract_tests.rs` using `assert_cmd` to verify `agk p <name> --dry-run --json` matches fixture.
3. Add snapshot tests for TUI tab rendering (`insta` + `ratatui::TestBackend`).
4. Document simulator workflow in `AGENTS.md`.

**Duration:** 2–4 days  
**Risk:** Fixture drift. **Mitigation:** Version fixtures per feature; wire into CI.

---

### Phase G — Docs + Agent Harness Finalization
**Goal:** `AGENTS.md` governs the workflow.

1. Update `AGENTS.md` with feature implementation contract, search order, boundary rules, file split rule, simulator-first rule.
2. Create `docs/product/vision.md`, `docs/product/charter.md`.
3. Create `docs/architecture/hexagon.md`, `docs/architecture/simulator.md`.

**Duration:** 1 day

---

## 5. Metrics of Success

After all phases complete:

- [ ] Architecture tests pass on every PR.
- [ ] `cargo check --no-default-features --features tui,pack,vault-clawhub` succeeds.
- [ ] `tui/event.rs` has < 400 lines.
- [ ] CLI `agk p <name> --dry-run --json` and TUI profile launch share identical core path and produce matching contracts.
- [ ] New features are added primarily by writing use-case + tests first.
- [ ] `main.rs` is < 100 lines.
- [ ] `cli/commands.rs` has zero business logic beyond parsing + presenter glue.

---

## 6. Module Migration Map

| Current module/function | Current role | Target home | Refactor action |
|---|---|---|---|
| `src/main.rs` | entrypoint; routes CLI to legacy `cli::commands::run` | `main.rs`, `cli/entry.rs`, `tui/entry.rs` | Pure composition root; delegate outward |
| `src/app/core.rs` | core façade | keep in `app/core.rs` | Broaden command coverage until legacy paths are gone |
| `src/app/usecases/create_profile.rs` | stub | `app/features/profiles/create.rs` | Real config load/save + uniqueness validation |
| `src/app/usecases/register_mcp.rs` | stub | `app/features/mcps/register.rs` | Route through `McpRegistryPort` |
| `src/cli/commands.rs` | large legacy inline handler | `cli/commands/*` + `app/features/*` | Split by feature; delete business logic |
| `src/cli/core_dispatcher.rs` | partial dispatcher | keep in `cli/core_dispatcher.rs` | Promote to sole CLI path |
| `src/tui/reducer.rs` | pure reducer | keep in `tui/reducer.rs` | Expand until all flows resolve to intents |
| `src/tui/command_mapper.rs` | partial mapper | keep in `tui/command_mapper.rs` | Expand to all feature-confirmation intents |
| `src/tui/event.rs` | ~2,400-line imperative hub | `tui/runtime_loop.rs` + `features/*/controller.rs` | Split into runtime loop, controllers, task bridge |
| `src/infra/config/toml_store.rs` | TOML-only storage | `infra/config/store.rs` + `infra/config/codecs/*` | Keep TOML compat, add YAML, preserve extension |
| `tests/architecture.rs` | ignored boundary tests | keep under `tests/architecture.rs` | Unignore, extend, run in CI |

---

## 7. CI Changes

Proposed `.github/workflows/ci.yml` additions:

```yaml
  architecture:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --test architecture -- --ignored

  feature-matrix:
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        features:
         - "--no-default-features --features tui,pack,vault-clawhub"
         - "--all-features"
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo check ${{ matrix.features }}

  contract-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test profile_start_dry_run_matches_contract_fixture -- --nocapture
```

---

## 8. Final Recommendation

The most important architectural rule for AGK is this: **UI and CLI should never be the place where AGK "decides what the product does."** They should only decide how input is collected and how results are shown.

The repo already contains enough of the right pieces to enforce that rule: ports, a core façade, profile runtime contracts, snapshot view models, reducer/presenter patterns, architecture tests, and feature docs.

The next milestone is to make those pieces **universal across the codebase**, then lock them in with simulator-backed contract tests and CI gates.

---

*End of proposal — Phase A in progress.*
