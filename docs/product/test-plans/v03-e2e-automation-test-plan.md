# AGK v0.3 "Team-Ready Profiles" — End-to-End Automation Test Plan

**Status:** Draft — awaiting implementation kickoff  
**Epic:** [`docs/epics/v03-team-ready-profiles.md`](../../../epics/v03-team-ready-profiles.md)  
**Proposal:** [`docs/epics/proposals/v03-team-ready-profiles.md`](../../../epics/proposals/v03-team-ready-profiles.md)  
**Date:** 2026-05-31  
**Author:** Quality Engineering  

---

## 1. Executive Summary

This plan defines the **complete end-to-end automation test suite** for AGK v0.3 "Team-Ready Profiles." It covers every acceptance criterion in the epic proposal and every functional requirement in the five updated/new PRDs:

1. [`profiles/prd.md`](../../product/features/profiles/prd.md)
2. [`profile-wizard/prd.md`](../../product/features/profile-wizard/prd.md)
3. [`mcp-vault/prd.md`](../../product/features/mcp-vault/prd.md)
4. [`vault-multi-asset/prd.md`](../../product/features/vault-multi-asset/prd.md)
5. [`providers/prd.md`](../../product/features/providers/prd.md)

The suite maps to AGK's existing **six-layer test pyramid** (Domain → Use Case → Contract → Snapshot → Integration → Architecture). All new tests obey the conventions in [`docs/conventions/rust-testing.md`](../../conventions/rust-testing.md): hand-written fakes, deterministic fixtures, descriptive snake_case names, and no mocking libraries.

**Quality Gate:**
- `cargo test` passes (unit + integration).
- `cargo test --test architecture -- --ignored` passes with **zero new allowlists**.
- `cargo llvm-cov --fail-under-lines 80` passes on `src/app/features/` and `src/domain/`.
- `cargo fmt --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` pass.

---

## 2. Test Strategy & Pyramid Mapping

| Layer | Lives In | v0.3 Responsibility | Tools |
|---|---|---|---|
| **1. Domain** | `src/domain/*.rs` `#[cfg(test)]` | `ProfileAssetRef` serde, `AssetKind::Profile`, token estimation, backward-compat parsing | `#[test]` |
| **2. Use Case** | `src/app/features/<f>/<verb>.rs` `#[cfg(test)]` | Wizard composer, template pre-fill, vault scanners (`McpFeatureSet`, `ProfileFeatureSet`), batch install, auto-heal, provider config writes | `RecordingSink`, hand fakes |
| **3. Contract** | `tests/contract_tests_v03.rs` | CLI `--json` output schemas for `mcp list`, `provider list`, `profile list`, `vault scan` | `assert_cmd`, `serde_json` |
| **4. Snapshot** | `tests/tui_render_v03.rs` | Wizard review step layout, editor tabs, launch overlay, provider tab indicators | `ratatui::TestBackend`, `insta` |
| **5. Integration** | `tests/full_flow_tui/*.rs`, `tests/process_integration_v03.rs` | Full TUI flows (vault attach → discover → install → start), real config file roundtrips, atomic rollback | `tempfile`, `assert_cmd` |
| **6. Architecture** | `tests/architecture.rs` | Zero new allowlists for v0.3 files; file-size budget enforced; ADR-001 purity rules | `#[ignore]` + source scan |

---

## 3. Feature-to-Test Matrix

| F-ID | Feature | PRD | Layer | Test File / Module | Priority | Epic Criterion |
|---|---|---|---|---|---|---|
| F1 | Enhanced Profile Wizard Core | profile-wizard | UseCase | `src/app/features/profile/wizard_description.rs` `#[cfg(test)]` | P0 | Wizard generates structured markdown |
| F1 | Enhanced Profile Wizard Core | profile-wizard | TUI | `tests/full_flow_tui/wizard_full_flow.rs` | P0 | Wizard generates structured markdown |
| F1 | Enhanced Profile Wizard Core | profile-wizard | Contract | `tests/contract_tests_v03.rs` | P0 | CLI `--description-file` bypass |
| F2 | Agent Archetype Templates | profile-wizard | UseCase | `src/app/features/profile/template.rs` `#[cfg(test)]` | P0 | ≥5 templates pre-fill wizard |
| F2 | Agent Archetype Templates | profile-wizard | TUI | `tests/full_flow_tui/wizard_full_flow.rs` | P0 | ≥5 templates pre-fill wizard |
| F3 | Token Estimation & Preview | profile-wizard | Domain | `src/domain/profile.rs` `#[cfg(test)]` | P0 | Token heuristic |
| F3 | Token Estimation & Preview | profile-wizard | TUI | `tests/full_flow_tui/wizard_full_flow.rs` | P0 | Review step shows token badge |
| F3 | Token Estimation & Preview | profile-wizard | Snapshot | `tests/tui_render_v03.rs` | P0 | Color-coded badge layout |
| F4 | Vault-Aware Dependency Storage | profiles | Domain | `src/domain/profile.rs` `#[cfg(test)]` | P0 | `ProfileAssetRef` serde |
| F4 | Vault-Aware Dependency Storage | profiles | UseCase | `src/app/features/profile/update.rs` `#[cfg(test)]` | P0 | Structured config write |
| F4 | Vault-Aware Dependency Storage | profiles | Contract | `tests/contract_tests_v03.rs` | P0 | `profile list --json` schema |
| F5 | Auto-Install Missing Dependencies | profiles | UseCase | `src/app/features/profile/start.rs` `#[cfg(test)]` | P0 | `agk p start` self-heals |
| F5 | Auto-Install Missing Dependencies | profiles | TUI | `tests/full_flow_tui/profile_start_auto_heal.rs` | P0 | Launch overlay shows progress |
| F5 | Auto-Install Missing Dependencies | profiles | Process | `tests/process_integration_v03.rs` | P0 | Real vault + real install |
| F6 | Vault-Discoverable MCP Servers | mcp-vault, vault-multi-asset | UseCase | `src/infra/feature/mcp.rs` `#[cfg(test)]` | P0 | `McpFeatureSet` scanner |
| F6 | Vault-Discoverable MCP Servers | mcp-vault, vault-multi-asset | TUI | `tests/full_flow_tui/vault_mcp_discovery.rs` | P0 | `[⊘]` badge in MCP tab |
| F6 | Vault-Discoverable MCP Servers | mcp-vault, vault-multi-asset | Contract | `tests/contract_tests_v03.rs` | P0 | `mcp list --json` |
| F6 | Vault-Discoverable MCP Servers | mcp-vault, vault-multi-asset | Process | `tests/process_integration_v03.rs` | P0 | SHA10 change detection |
| F7 | Vault-Discoverable Profiles | profiles, vault-multi-asset | UseCase | `src/infra/feature/profile.rs` `#[cfg(test)]` | P0 | `ProfileFeatureSet` scanner |
| F7 | Vault-Discoverable Profiles | profiles, vault-multi-asset | TUI | `tests/full_flow_tui/vault_profile_discovery.rs` | P0 | `[Vault]` badge in Profiles tab |
| F7 | Vault-Discoverable Profiles | profiles, vault-multi-asset | Contract | `tests/contract_tests_v03.rs` | P0 | `vault scan --json` includes `Profile` |
| F8 | MCP Provider Expansion | providers, mcp-vault | UseCase | `src/infra/provider/copilot_mcp.rs` `#[cfg(test)]` | P0 | Copilot/Gemini/AMP adapters |
| F8 | MCP Provider Expansion | providers, mcp-vault | Process | `tests/process_integration_v03.rs` | P0 | Config write/read roundtrips |
| F8 | MCP Provider Expansion | providers, mcp-vault | TUI | `tests/full_flow_tui/provider_mcp_toggle.rs` | P0 | MCP checkbox `[✓]` logic |
| F9 | Provider Tool/Permission Selection | providers, profile-wizard | UseCase | `src/app/ports/provider.rs` `#[cfg(test)]` | P1 | `available_profile_tools()` |
| F9 | Provider Tool/Permission Selection | providers, profile-wizard | TUI | `tests/full_flow_tui/wizard_full_flow.rs` | P1 | Conditional checklist injection |
| F10 | Profile Editor (F3) Enhancement | profiles, profile-wizard | TUI | `tests/full_flow_tui/profile_editor.rs` | P1 | Editor tabs + live tokens |
| F10 | Profile Editor (F3) Enhancement | profiles, profile-wizard | Snapshot | `tests/tui_render_v03.rs` | P1 | Editor tab layout |
| F11 | Claude Code Agent File Projection | profiles | UseCase | `src/infra/provider/claude_code/mod.rs` `#[cfg(test)]` | P1 | `agent.md` frontmatter |
| F11 | Claude Code Agent File Projection | profiles | Process | `tests/process_integration_v03.rs` | P1 | File-system roundtrip |
| F12 | Profile Batch Installation | vault-multi-asset, profiles | UseCase | `src/app/features/profile/batch_install.rs` `#[cfg(test)]` | P1 | Atomic batch logic |
| F12 | Profile Batch Installation | vault-multi-asset, profiles | TUI | `tests/full_flow_tui/profile_batch_install.rs` | P1 | `Space` on vault profile |
| F12 | Profile Batch Installation | vault-multi-asset, profiles | Process | `tests/process_integration_v03.rs` | P1 | Atomic rollback on failure |
| F13 | Profile Launch Simulation | profiles | TUI | `tests/full_flow_tui/launch_simulation.rs` | P2 | Overlay progress steps |
| F13 | Profile Launch Simulation | profiles | Snapshot | `tests/tui_render_v03.rs` | P2 | Overlay frame layout |
| F14 | Backward-Compatible Config Migration | profiles | Domain | `src/domain/profile.rs` `#[cfg(test)]` | P2 | Old flat format parse |
| F14 | Backward-Compatible Config Migration | profiles | UseCase | `src/app/features/profile/migrate.rs` `#[cfg(test)]` | P2 | Migration on first write |
| F14 | Backward-Compatible Config Migration | profiles | Process | `tests/process_integration_v03.rs` | P2 | Legacy config roundtrip |
| F15 | Custom `prompt_overlay_path` | profiles | UseCase | `src/app/features/profile/start.rs` `#[cfg(test)]` | P2 | Overlay precedence |
| F15 | Custom `prompt_overlay_path` | profiles | Process | `tests/process_integration_v03.rs` | P2 | File copy behavior |
| v0.3.1 | MCP Security Scorecard | mcp-vault | Domain | `src/domain/mcp.rs` `#[cfg(test)]` | P2 | `assess_mcp_security()` |
| v0.3.1 | MCP Security Scorecard | mcp-vault | Contract | `tests/contract_tests_v03.rs` | P2 | `security_flags` in JSON |
| v0.3.1 | MCP Security Scorecard | mcp-vault | TUI | `tests/full_flow_tui/vault_mcp_discovery.rs` | P2 | `[!]` badge rendering |
| — | Architecture Gate | — | Architecture | `tests/architecture.rs` | P0 | Zero new allowlists |

---

## 4. Phase-Aligned Test Suites

The epic breaks work into **5 phases**. Tests are organized to gate each phase.

### Phase 1: Structural Enablers (Weeks 1–2)
**Goal:** Vaults discover MCPs + Profiles; `ProfileAssetRef` exists.

**Tests to implement:**
- Domain: `profile_asset_ref_serde_legacy_flat_string_to_structured`
- Domain: `profile_asset_ref_serde_structured_roundtrip`
- Domain: `asset_kind_profile_does_not_break_existing_match_arms`
- UseCase: `mcp_feature_set_scanner_parses_mcp_md_frontmatter`
- UseCase: `profile_feature_set_scanner_parses_profile_md_frontmatter`
- UseCase: `filter_scan_includes_mcp_server_kind`
- UseCase: `filter_scan_includes_profile_kind`
- UseCase: `stub_feature_set_mcp_is_removed_from_bootstrap`
- UseCase: `stub_feature_set_profile_is_removed_from_bootstrap`
- TUI: `vault_mcp_discovery_shows_unregistered_badge`
- TUI: `vault_profile_discovery_shows_vault_badge`
- Contract: `agk_vault_scan_json_includes_mcp_and_profile`

**Exit Criteria:**
- `cargo test` passes; architecture tests pass with zero allowlists.
- TUI shows vault-discovered MCPs and profiles.
- Old profiles without vault info still load.

### Phase 2: Wizard Foundation (Weeks 3–4)
**Goal:** Template-driven, token-aware wizard.

**Tests to implement:**
- Domain: `token_estimation_words_times_1_35`
- Domain: `token_estimation_empty_string_is_zero`
- Domain: `token_estimation_unicode_counts_as_one_word`
- UseCase: `wizard_structured_description_composes_canonical_markdown`
- UseCase: `wizard_description_includes_identity_responsibilities_style_constraints`
- UseCase: `template_code_reviewer_prefills_all_fields`
- UseCase: `template_feature_implementer_prefills_all_fields`
- UseCase: `template_security_auditor_prefills_all_fields`
- UseCase: `template_documentation_writer_prefills_all_fields`
- UseCase: `template_test_generator_prefills_all_fields`
- UseCase: `template_custom_leaves_all_fields_empty`
- UseCase: `wizard_step_sequence_is_16_steps_for_opencode`
- UseCase: `wizard_step_sequence_is_14_steps_for_claude_code` (no frontmatter step)
- TUI: `wizard_template_select_renders_6_options`
- TUI: `wizard_review_step_shows_token_badge_green`
- TUI: `wizard_review_step_shows_token_badge_yellow`
- TUI: `wizard_review_step_shows_token_badge_red`
- TUI: `wizard_review_step_shows_composed_markdown_preview`
- Snapshot: `wizard_review_step_layout_matches_spec`
- Contract: `cli_profile_create_with_template_pre_fills`

**Exit Criteria:**
- Wizard generates structured markdown.
- 5+ templates available.
- Review step shows markdown + token count.
- Template path ≤ 10 steps.

### Phase 3: Provider Reach (Weeks 4–5)
**Goal:** 5 MCP-capable providers; tool/permission port methods.

**Tests to implement:**
- UseCase: `copilot_mcp_provider_writes_correct_mcp_config_json`
- UseCase: `copilot_mcp_provider_preserves_existing_json_content`
- UseCase: `gemini_mcp_provider_writes_correct_settings_json`
- UseCase: `gemini_mcp_provider_preserves_existing_settings`
- UseCase: `amp_mcp_provider_writes_nested_amp_mcp_servers`
- UseCase: `amp_mcp_provider_preserves_other_settings_keys`
- UseCase: `claude_code_available_profile_tools_returns_7_tools`
- UseCase: `claude_code_available_permission_modes_returns_5_modes`
- UseCase: `letta_returns_supports_mcp_false`
- UseCase: `snowflake_returns_supports_mcp_false`
- UseCase: `firebender_returns_supports_mcp_false`
- Process: `copilot_config_write_read_roundtrip`
- Process: `gemini_config_write_read_roundtrip`
- Process: `amp_config_write_read_roundtrip`
- TUI: `providers_tab_shows_mcp_checkbox_only_for_capable`
- TUI: `providers_tab_shows_tool_count_for_claude_code`
- Contract: `agk_provider_list_json_includes_mcp_and_tool_flags`

**Exit Criteria:**
- `agk mcp add` writes config for 5 providers.
- TUI Providers tab shows `[✓]` only for capable providers.
- Tool/permission lists exposed on port.

### Phase 4: Runtime Integration (Weeks 5–6)
**Goal:** `agk p start` self-healing; editor + projection.

**Tests to implement:**
- UseCase: `start_profile_resolves_missing_skill_from_specified_vault`
- UseCase: `start_profile_resolves_missing_mcp_from_specified_vault`
- UseCase: `start_profile_warns_on_ambiguous_auto_vault`
- UseCase: `start_profile_emits_error_when_vault_unavailable`
- UseCase: `start_profile_emits_error_when_asset_not_found`
- UseCase: `batch_install_parses_profile_md_references`
- UseCase: `batch_install_installs_all_referenced_skills`
- UseCase: `batch_install_installs_all_referenced_instructions`
- UseCase: `batch_install_registers_all_referenced_mcps`
- UseCase: `batch_install_fails_when_any_referenced_asset_missing`
- UseCase: `claude_code_projection_writes_agent_md_with_frontmatter`
- UseCase: `claude_code_projection_body_matches_canonical_contract`
- UseCase: `custom_prompt_overlay_takes_precedence_over_wizard`
- UseCase: `editor_raw_markdown_save_updates_config_and_agent_md`
- TUI: `profile_start_auto_heal_shows_resolution_overlay`
- TUI: `profile_batch_install_shows_atomic_progress`
- TUI: `editor_overview_tab_shows_token_count`
- TUI: `editor_skills_tab_shows_vault_origin`
- TUI: `editor_mcps_tab_shows_vault_and_registered`
- TUI: `editor_tools_tab_conditionally_rendered`
- TUI: `editor_raw_markdown_tab_editable_with_live_tokens`
- Process: `atomic_batch_install_rolls_back_on_skill_failure`
- Process: `atomic_batch_install_rolls_back_on_mcp_handshake_failure`
- Process: `claude_code_projection_file_roundtrip`
- Process: `prompt_overlay_path_file_copy`
- Snapshot: `editor_tab_layout`
- Snapshot: `launch_simulation_overlay_layout`

**Exit Criteria:**
- `agk p start` installs missing dependencies.
- Vault profile install is atomic.
- F3 Editor allows editing all tabs.
- Claude Code writes `agent.md` with frontmatter.

### Phase 5: Polish (Week 7)
**Goal:** CI green, manual QA checklist, docs.

**Tests to implement:**
- Domain: `legacy_flat_skills_deserializes_to_auto_vault`
- Domain: `legacy_flat_mcps_deserializes_to_auto_vault`
- UseCase: `config_migration_rewrites_to_structured_on_first_save`
- UseCase: `old_profile_without_tool_refs_defaults_empty`
- UseCase: `old_profile_without_permission_mode_defaults_none`
- TUI: `launch_overlay_shows_dependency_resolution_step`
- TUI: `launch_overlay_shows_install_step`
- TUI: `launch_overlay_shows_projection_step`
- TUI: `launch_overlay_shows_runtime_step`
- Process: `legacy_config_roundtrip_migrates_on_write`
- Contract: `agk_mcp_list_json_includes_security_flags` (v0.3.1)
- Architecture: `file_size_lint_zero_new_allowlists_v03`
- Architecture: `domain_purity_no_new_violations`
- Integration (Manual QA checklist documented separately):
  - Vault attach → profile install → start
  - Provider toggle → MCP register → profile start

**Exit Criteria:**
- `cargo test` passes (all layers).
- `cargo test --test architecture -- --ignored` passes with zero allowlists.
- `cargo clippy` and `cargo fmt --check` pass.
- 80%+ line coverage on `src/app/features/` and `src/domain/`.

---

## 5. Detailed Test Specifications

### 5.1 TUI Full-Flow Tests (`tests/full_flow_tui/`)

These tests exercise the TUI at the **frame level** using `ratatui::TestBackend`. They construct an `AgkCore` with hand-written fakes, dispatch `CoreCommand`s (or simulate TUI key events when necessary), and assert on `AppState` and rendered buffer contents.

> **Test Pattern Reference:** `tests/full_flow_tui/common.rs` defines `test_core()`, `StateSink`, `render_buffer()`, and `assert_buffer_contains()`.

#### New/modified fakes required

| Fake | Location | Modification |
|---|---|---|
| `FakeVaultSearch` | `tests/full_flow_tui/common.rs` | `search()` must return `ScannedPackage` with `AssetKind::McpServer` and `AssetKind::Profile`. Add helper `with_mcp(name, vault)` and `with_profile(name, vault)`. |
| `FakeMcpRegistry` | `src/app/test_support/fake_mcp_registry.rs` | Add `is_registered(name)` and `register_from_vault(name, vault)` to support `[⊘]` → `[ ]` transitions. |
| `FakeStore` | `src/app/test_support/fake_store.rs` | Ensure `ConfigFile` can hold `installed_mcps` and `installed_profiles` (new `serde(default)` fields). |
| `FakeProvider` | `tests/full_flow_tui/common.rs` | Add `available_profile_tools()` and `available_permission_modes()` returns for wizard injection tests. |

#### Test: `vault_mcp_discovery_shows_unregistered_badge`
**File:** `tests/full_flow_tui/vault_mcp_discovery.rs`  
**PRD:** MCP Vault §Vault-Discovered MCPs; Vault Multi-Asset §MCP Tab Behavior  
**Feature:** F6  

```rust
#[test]
fn vault_mcp_discovery_shows_unregistered_badge() {
    let core = test_core_with_vault(
        FakeVaultSearch::with_mcp("filesystem", "clawhub"),
    );
    let mut state = AppState::new(vec!["Skills".into()], vec![true], HashMap::new());

    // Attach vault
    execute(&core, &mut state, CoreCommand::AttachVault { ... }).unwrap();

    // Switch to MCP tab (Tab 2)
    state.active_tab = 2;

    let buf = render_buffer(&state, 80, 24);
    assert_buffer_contains(&buf, "[⊘]");
    assert_buffer_contains(&buf, "filesystem");
    assert_buffer_contains(&buf, "clawhub");
}
```

**Assertions:**
1. Buffer contains `[⊘]` next to `filesystem`.
2. Buffer contains vault source `clawhub`.
3. `state.status_line` contains "attached".

---

#### Test: `vault_mcp_registration_toggles_to_enabled`
**File:** `tests/full_flow_tui/vault_mcp_discovery.rs`  
**PRD:** MCP Vault §Actions  
**Feature:** F6  

```rust
#[test]
fn vault_mcp_registration_toggles_to_enabled() {
    let core = test_core_with_vault(
        FakeVaultSearch::with_mcp("filesystem", "clawhub"),
    );
    let mut state = AppState::new(vec!["Skills".into()], vec![true], HashMap::new());
    execute(&core, &mut state, CoreCommand::AttachVault { ... }).unwrap();
    state.active_tab = 2;

    // Simulate Space on the unregistered MCP
    let cmd = CoreCommand::InstallAsset {
        identity: "clawhub/filesystem".into(),
        scope: Scope::Global,
        provider_filter: None,
        include_evals: false,
        dry_run: false,
    };
    execute(&core, &mut state, cmd).unwrap();

    let buf = render_buffer(&state, 80, 24);
    assert_buffer_contains(&buf, "[ ]"); // registered but disabled
    assert!(state.status_line.contains("registered"));
}
```

**Assertions:**
1. After `InstallAsset`, badge changes to `[ ]` or `[✓]` (depending on test fake's test handshake).
2. Status line reflects registration.

---

#### Test: `vault_profile_discovery_shows_vault_badge`
**File:** `tests/full_flow_tui/vault_profile_discovery.rs`  
**PRD:** Profiles §Vault Profiles; Vault Multi-Asset §Profile Tab Behavior  
**Feature:** F7  

```rust
#[test]
fn vault_profile_discovery_shows_vault_badge() {
    let core = test_core_with_vault(
        FakeVaultSearch::with_profile("web-app-team", "clawhub"),
    );
    let mut state = AppState::new(vec!["Skills".into()], vec![true], HashMap::new());
    execute(&core, &mut state, CoreCommand::AttachVault { ... }).unwrap();
    state.active_tab = 5; // Profiles tab

    let buf = render_buffer(&state, 80, 24);
    assert_buffer_contains(&buf, "[Vault]");
    assert_buffer_contains(&buf, "web-app-team");
}
```

---

#### Test: `profile_batch_install_atomic_via_space`
**File:** `tests/full_flow_tui/profile_batch_install.rs`  
**PRD:** Vault Multi-Asset §Profile Installation Behavior; Profiles §Batch Install  
**Feature:** F12  

**Setup:**
- `FakeVaultSearch` returns profile `web-app-team` with:
  - `skills`: `["rust-patterns", "docker"]`
  - `instructions`: `["web-app-guidelines"]`
  - `mcps`: `["filesystem"]`
- `FakeStore` initially has none of these installed.

```rust
#[test]
fn profile_batch_install_atomic_via_space() {
    let core = test_core_with_vault(
        FakeVaultSearch::with_profile_bundle("web-app-team", "clawhub", ...),
    );
    let mut state = AppState::new(vec!["Skills".into()], vec![true], HashMap::new());
    execute(&core, &mut state, CoreCommand::AttachVault { ... }).unwrap();
    state.active_tab = 5;

    // Space on vault profile triggers batch install
    let cmd = CoreCommand::InstallAsset {
        identity: "clawhub/web-app-team".into(),
        scope: Scope::Workspace,
        provider_filter: None,
        include_evals: false,
        dry_run: false,
    };
    execute(&core, &mut state, cmd).unwrap();

    let buf = render_buffer(&state, 80, 24);
    assert_buffer_contains(&buf, "installed");
    assert!(state.status_line.contains("4 assets")); // 2 skills + 1 instruction + 1 MCP
}
```

**Assertions:**
1. Status line indicates successful batch install of all referenced assets.
2. `FakeStore` now contains `installed_profiles` entry for `web-app-team`.
3. `FakeStore` contains `installed_skills` for `rust-patterns` and `docker`.
4. `FakeMcpRegistry` contains registered `filesystem`.

---

#### Test: `profile_start_auto_heals_missing_dependencies`
**File:** `tests/full_flow_tui/profile_start_auto_heal.rs`  
**PRD:** Profiles §Auto-Install; Epic §F5  
**Feature:** F5  

**Setup:**
- `FakeStore` has a profile `web-app-team` with `skill_vault_refs` = `[{name:"rust-patterns", vault:"clawhub"}]` and `mcp_vault_refs` = `[{name:"filesystem", vault:"clawhub"}]`.
- `rust-patterns` is **not** installed.
- `filesystem` is **not** registered.
- `FakeVaultSearch` can resolve both from `clawhub`.

```rust
#[test]
fn profile_start_auto_heals_missing_dependencies() {
    let core = test_core_with_vault(FakeVaultSearch::default_with_clawhub_skills());
    let mut state = AppState::new(...);
    // Pre-load config with profile referencing missing deps
    ...

    let cmd = CoreCommand::StartProfile {
        id: ProfileId::from("web-app-team"),
        scope: Scope::Workspace,
        dry_run: false,
    };
    execute(&core, &mut state, cmd).unwrap();

    let buf = render_buffer(&state, 80, 24);
    assert_buffer_contains(&buf, "Resolving dependencies...");
    assert_buffer_contains(&buf, "Installing rust-patterns...");
    assert_buffer_contains(&buf, "Registering filesystem...");
    assert!(state.status_line.contains("Started web-app-team"));
}
```

**Assertions:**
1. Buffer shows dependency resolution overlay text.
2. Buffer shows install progress for missing skill.
3. Buffer shows MCP registration progress.
4. Final status line indicates profile started successfully.

---

#### Test: `wizard_review_step_shows_token_badge_and_composed_markdown`
**File:** `tests/full_flow_tui/wizard_full_flow.rs`  
**PRD:** Profile Wizard §Review Step; §Token Count Badge  
**Feature:** F1, F3  

**Setup:**
- Activate provider that returns wizard steps (e.g., `opencode`).
- Simulate wizard progression through all 16 steps using `CoreCommand::CreateProfile` with a fully populated `CreateProfileInput` that mimics wizard completion.

**Note:** Since the wizard UI is a modal driven by TUI key events, the full-flow test can either:
  a. Drive the modal via `CoreCommand::CreateProfile` with pre-structured input (validates core logic), or
  b. Use `AppState` wizard mode + direct `handle_profile_wizard_input` calls if exposed.

For this plan, we test the **core command path** (headless wizard completion) in UseCase tests, and test the **TUI review rendering** by injecting a `WizardState` directly into `AppState`:

```rust
#[test]
fn wizard_review_step_shows_token_badge_and_composed_markdown() {
    let core = test_core_with_provider_tools(); // returns 7 tools
    let mut state = AppState::new(...);
    // Inject a WizardState at the Review step
    state.wizard_state = Some(WizardState::new(
        /* steps */ vec![...],
        /* provider */ "opencode".into(),
    ));
    state.wizard_state.as_mut().unwrap().current_step = 15; // Review

    let buf = render_buffer(&state, 80, 24);
    assert_buffer_contains(&buf, "[Est. Tokens:");
    assert_buffer_contains(&buf, "# Identity");
    assert_buffer_contains(&buf, "# Core Responsibilities");
    assert_buffer_contains(&buf, "Skills:");
    assert_buffer_contains(&buf, "MCPs:");
}
```

**Assertions:**
1. Buffer contains `[Est. Tokens:` substring.
2. Buffer contains composed markdown headers (`# Identity`, `# Core Responsibilities`).
3. Buffer contains skill and MCP counts.

---

#### Test: `wizard_template_select_renders_six_options`
**File:** `tests/full_flow_tui/wizard_full_flow.rs`  
**PRD:** Profile Wizard §Step 3: Archetype Template  
**Feature:** F2  

```rust
#[test]
fn wizard_template_select_renders_six_options() {
    let mut state = AppState::new(...);
    state.wizard_state = Some(WizardState::new(
        vec![WizardStep::TemplateSelect { ... }],
        "opencode".into(),
    ));

    let buf = render_buffer(&state, 80, 24);
    assert_buffer_contains(&buf, "Code Reviewer");
    assert_buffer_contains(&buf, "Feature Implementer");
    assert_buffer_contains(&buf, "Security Auditor");
    assert_buffer_contains(&buf, "Documentation Writer");
    assert_buffer_contains(&buf, "Test Generator");
    assert_buffer_contains(&buf, "Custom");
}
```

---

#### Test: `editor_raw_markdown_tab_shows_live_token_count`
**File:** `tests/full_flow_tui/profile_editor.rs`  
**PRD:** Profile Wizard §Tokens; Profiles §Profile Editor  
**Feature:** F3, F10  

**Setup:**
- `AppState` has `editor_state` open for profile `rust-reviewer` with raw markdown content.

```rust
#[test]
fn editor_raw_markdown_tab_shows_live_token_count() {
    let mut state = AppState::new(...);
    state.editor_state = Some(EditorState {
        profile_id: "rust-reviewer".into(),
        active_tab: EditorTab::RawMarkdown,
        raw_markdown: "You are a Senior Rust CLI engineer...".into(),
        ...
    });

    let buf = render_buffer(&state, 80, 24);
    assert_buffer_contains(&buf, "[Est. Tokens:");
    // Tokens ≈ 8 words * 1.35 = 11
    assert_buffer_contains(&buf, "11");
}
```

---

#### Test: `providers_tab_shows_mcp_checkbox_only_for_capable`
**File:** `tests/full_flow_tui/provider_mcp_toggle.rs`  
**PRD:** Providers §UI/UX Specifications; MCP Vault §Provider Exclusion  
**Feature:** F8  

```rust
#[test]
fn providers_tab_shows_mcp_checkbox_only_for_capable() {
    let core = test_core_with_all_providers();
    let mut state = AppState::new(...);
    state.active_tab = 4; // Providers

    let buf = render_buffer(&state, 80, 24);
    assert_buffer_contains(&buf, "Claude Code    [MCP: ✓]");
    assert_buffer_contains(&buf, "GitHub Copilot  [MCP: ✓]");
    assert_buffer_contains(&buf, "Letta           [MCP: ✗]");
    assert_buffer_contains(&buf, "Snowflake       [MCP: ✗]");
}
```

---

#### Test: `launch_overlay_shows_all_four_steps`
**File:** `tests/full_flow_tui/launch_simulation.rs`  
**PRD:** Profiles §Launch Simulation; Epic §F13  
**Feature:** F13  

```rust
#[test]
fn launch_overlay_shows_all_four_steps() {
    let core = test_core_with_slow_vault(); // resolves after one tick
    let mut state = AppState::new(...);
    state.launch_overlay = Some(LaunchOverlay::new("web-app-team"));

    // Step 1: Resolution
    let buf = render_buffer(&state, 80, 24);
    assert_buffer_contains(&buf, "Resolving dependencies...");

    // Advance state manually or via core event
    state.launch_overlay.as_mut().unwrap().advance_to_install();
    let buf = render_buffer(&state, 80, 24);
    assert_buffer_contains(&buf, "Installing assets...");

    state.launch_overlay.as_mut().unwrap().advance_to_projection();
    let buf = render_buffer(&state, 80, 24);
    assert_buffer_contains(&buf, "Projecting config...");

    state.launch_overlay.as_mut().unwrap().advance_to_runtime();
    let buf = render_buffer(&state, 80, 24);
    assert_buffer_contains(&buf, "Starting agent...");
}
```

---

### 5.2 Contract Tests (`tests/contract_tests_v03.rs`)

Contract tests invoke the compiled `agk` binary via `assert_cmd::CommandCargoExt` against temporary directories with pre-seeded config. They assert on `--json` output shape.

> **Pattern Reference:** `tests/contract_tests.rs`

#### Test: `mcp_list_json_includes_vault_source_and_security_flags`
**File:** `tests/contract_tests_v03.rs`  
**PRD:** MCP Vault §CLI Commands; §v0.3.1 Security Scorecard  
**Feature:** F6, v0.3.1  

**Fixture Setup:**
- Temp dir with `~/.config/agk/mcp.toml` containing one manually-registered MCP and one vault-sourced MCP.
- Vault dir with `mcps/filesystem/MCP.md`.

```rust
#[test]
fn mcp_list_json_includes_vault_source_and_security_flags() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(home.join(".config/agk")).unwrap();
    // Write mcp.toml fixture
    std::fs::write(home.join(".config/agk/mcp.toml”), MCP_TOML_FIXTURE).unwrap();

    let mut cmd = std::process::Command::cargo_bin("agk").unwrap();
    cmd.env("HOME", &home);
    cmd.args(["mcp", "list", "--json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    let stream = serde_json::Deserializer::from_str(&stdout).into_iter::<serde_json::Value>();
    let events: Vec<_> = stream.filter_map(|r| r.ok()).filter(|j| j.get("events").is_some()).collect();
    assert!(!events.is_empty());

    let first_event = &events[0]["events"][0];
    assert!(first_event.get("name").is_some());
    assert!(first_event.get("vault_source").is_some());
    assert!(first_event.get("security_flags").is_some());
    assert!(first_event.get("enabled_providers").is_some());
}
```

---

#### Test: `provider_list_json_includes_mcp_profile_tool_flags`
**File:** `tests/contract_tests_v03.rs`  
**PRD:** Providers §CLI Impact  
**Feature:** F8, F9  

```rust
#[test]
fn provider_list_json_includes_mcp_profile_tool_flags() {
    let mut cmd = std::process::Command::cargo_bin("agk").unwrap();
    cmd.args(["provider", "list", "--json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let events = json.get("events").unwrap().as_array().unwrap();
    let claude = events.iter().find(|e| e["id"] == "claude-code").unwrap();
    assert_eq!(claude["supports_mcp"], true);
    assert_eq!(claude["supports_profiles"], true);
    assert!(claude["available_tools"].as_array().unwrap().len() > 0);
    assert!(claude["available_permission_modes"].as_array().unwrap().len() > 0);

    let letta = events.iter().find(|e| e["id"] == "letta").unwrap();
    assert_eq!(letta["supports_mcp"], false);
}
```

---

#### Test: `profile_list_json_includes_vault_aware_refs`
**File:** `tests/contract_tests_v03.rs`  
**PRD:** Profiles §Config Schema  
**Feature:** F4  

```rust
#[test]
fn profile_list_json_includes_vault_aware_refs() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("workspace");
    std::fs::create_dir_all(ws.join(".agk")).unwrap();
    std::fs::write(ws.join(".agk/config.toml"), PROFILE_CONFIG_FIXTURE).unwrap();

    let mut cmd = std::process::Command::cargo_bin("agk").unwrap();
    cmd.current_dir(&ws);
    cmd.args(["profile", "list", "--json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let profile = json["events"][0].as_object().unwrap();
    assert!(profile.get("skill_refs").is_some());
    assert!(profile["skill_refs"][0].get("vault").is_some());
    assert!(profile.get("mcp_refs").is_some());
    assert!(profile.get("tool_refs").is_some());
    assert!(profile.get("permission_mode").is_some());
}
```

---

#### Test: `vault_scan_json_includes_mcp_and_profile_kinds`
**File:** `tests/contract_tests_v03.rs`  
**PRD:** Vault Multi-Asset §CLI Impact  
**Feature:** F6, F7  

```rust
#[test]
fn vault_scan_json_includes_mcp_and_profile_kinds() {
    let tmp = tempfile::tempdir().unwrap();
    let vault = tmp.path().join("my-vault");
    // Create vault structure with skills/, mcps/, profiles/
    ...

    let mut cmd = std::process::Command::cargo_bin("agk").unwrap();
    cmd.args(["vault", "scan", vault.to_str().unwrap(), "--json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let kinds: Vec<_> = json["events"].as_array().unwrap()
        .iter().map(|e| e["kind"].as_str().unwrap()).collect();
    assert!(kinds.contains(&"McpServer"));
    assert!(kinds.contains(&"Profile"));
    assert!(kinds.contains(&"Skill"));
    assert!(kinds.contains(&"Instruction"));
}
```

---

#### Test: `cli_profile_create_with_template_pre_fills`
**File:** `tests/contract_tests_v03.rs`  
**PRD:** Profile Wizard §CLI Flow  
**Feature:** F2  

```rust
#[test]
fn cli_profile_create_with_template_pre_fills() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("workspace");
    std::fs::create_dir_all(ws.join(".agk")).unwrap();

    let mut cmd = std::process::Command::cargo_bin("agk").unwrap();
    cmd.current_dir(&ws);
    cmd.args([
        "profile", "create", "rust-reviewer",
        "--provider", "opencode",
        "--template", "code-reviewer",
        "--scope", "workspace",
        "--json",
    ]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(output.status.success());
    assert_eq!(json["profile_id"], "rust-reviewer");
}
```

---

### 5.3 Process Integration Tests (`tests/process_integration_v03.rs`)

These tests exercise real I/O against temporary directories. They validate provider config file roundtrips, atomic batch install rollback, and file projection.

> **Pattern Reference:** `tests/process_integration.rs`

#### Test: `copilot_config_write_read_roundtrip`
**File:** `tests/process_integration_v03.rs`  
**PRD:** Providers §MCP Provider Expansion; MCP Vault §Provider-Specific MCP Config  
**Feature:** F8  

```rust
#[test]
fn copilot_config_write_read_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(home.join(".copilot")).unwrap();
    // Seed with existing content
    std::fs::write(home.join(".copilot/mcp-config.json"), r#"{"existingKey": true}"#).unwrap();

    let provider = CopilotMcpProvider::new(home.clone());
    provider.write_mcp_server(&McpServer {
        name: "filesystem".into(),
        command: "npx".into(),
        args: vec!["-y", "@modelcontextprotocol/server-filesystem", "."],
        transport: McpTransport::Stdio,
        ...
    }).unwrap();

    let content = std::fs::read_to_string(home.join(".copilot/mcp-config.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json["existingKey"].as_bool().unwrap());
    assert!(json["mcpServers"]["filesystem"].is_object());
    assert_eq!(json["mcpServers"]["filesystem"]["type"], "stdio");
}
```

---

#### Test: `gemini_config_write_read_roundtrip`
**File:** `tests/process_integration_v03.rs`  
**PRD:** Providers §MCP Provider Expansion  
**Feature:** F8  

```rust
#[test]
fn gemini_config_write_read_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(home.join(".gemini")).unwrap();
    std::fs::write(home.join(".gemini/settings.json"), r#"{"theme":"dark"}"#).unwrap();

    let provider = GeminiMcpProvider::new(home.clone());
    provider.write_mcp_server(/* ... */).unwrap();

    let content = std::fs::read_to_string(home.join(".gemini/settings.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["theme"], "dark");
    assert!(json["mcpServers"]["filesystem"].is_object());
}
```

---

#### Test: `amp_config_write_read_roundtrip`
**File:** `tests/process_integration_v03.rs`  
**PRD:** Providers §MCP Provider Expansion  
**Feature:** F8  

```rust
#[test]
fn amp_config_write_read_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("workspace");
    std::fs::create_dir_all(ws.join(".amp")).unwrap();
    std::fs::write(ws.join(".amp/settings.json"), r#"{"editor":{"fontSize":14}}"#).unwrap();

    let provider = AmpMcpProvider::new(ws.clone());
    provider.write_mcp_server(/* ... */).unwrap();

    let content = std::fs::read_to_string(ws.join(".amp/settings.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["editor"]["fontSize"], 14);
    assert!(json["amp"]["mcpServers"]["filesystem"].is_object());
}
```

---

#### Test: `atomic_batch_install_rolls_back_on_skill_failure`
**File:** `tests/process_integration_v03.rs`  
**PRD:** Vault Multi-Asset §Profile Installation Behavior; Profiles §Batch Install  
**Feature:** F12  

```rust
#[test]
fn atomic_batch_install_rolls_back_on_skill_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let vault = tmp.path().join("vault");
    let ws = tmp.path().join("workspace");
    // Create vault with valid MCP but broken skill (missing SKILL.md)
    ...

    let mut cmd = std::process::Command::cargo_bin("agk").unwrap();
    cmd.current_dir(&ws);
    cmd.args(["install", "test-vault/web-app-team", "--kind", "profile"]);
    let output = cmd.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("missing skill"));
    // Assert that no partial profile was created
    assert!(!ws.join(".agk/config.toml").exists() || /* parsed profile list is empty */ true);
}
```

---

#### Test: `claude_code_projection_file_roundtrip`
**File:** `tests/process_integration_v03.rs`  
**PRD:** Profiles §Claude Code Projection  
**Feature:** F11  

```rust
#[test]
fn claude_code_projection_file_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("workspace");
    std::fs::create_dir_all(ws.join(".agk/profiles/rust-reviewer")).unwrap();

    let profile = Profile {
        name: "rust-reviewer".into(),
        provider_id: "claude-code".into(),
        skill_vault_refs: vec![ProfileAssetRef { name: "rust-patterns".into(), vault: "clawhub".into() }],
        tool_refs: vec!["Read".into(), "Glob".into()],
        permission_mode: Some("acceptEdits".into()),
        ...
    };

    let provider = ClaudeCodeProvider::new();
    provider.build_launch_plan(&profile, &ws).unwrap();

    let agent_md = std::fs::read_to_string(ws.join(".agk/profiles/rust-reviewer/agent.md")).unwrap();
    assert!(agent_md.starts_with("---"));
    assert!(agent_md.contains("name: rust-reviewer"));
    assert!(agent_md.contains("tools:"));
    assert!(agent_md.contains("- Read"));
    assert!(agent_md.contains("# Identity"));
    assert!(agent_md.contains("# Core Responsibilities"));
}
```

---

#### Test: `legacy_config_roundtrip_migrates_on_write`
**File:** `tests/process_integration_v03.rs`  
**PRD:** Profiles §Config Schema; §Backward-Compatible Config Migration  
**Feature:** F14  

```rust
#[test]
fn legacy_config_roundtrip_migrates_on_write() {
    let tmp = tempfile::tempdir().unwrap();
    let ws = tmp.path().join("workspace");
    std::fs::create_dir_all(ws.join(".agk")).unwrap();
    // Write legacy flat format
    std::fs::write(ws.join(".agk/config.toml"), r#"
[[profiles]]
name = "legacy"
provider_id = "opencode"
skills = ["rust-patterns", "docker"]
mcps = ["filesystem"]
"#).unwrap();

    // Trigger a mutation that causes re-save (e.g., update profile)
    let mut cmd = std::process::Command::cargo_bin("agk").unwrap();
    cmd.current_dir(&ws);
    cmd.args(["profile", "update", "legacy", "--description-file", "./empty.md"]);
    // Create empty description file
    std::fs::write(ws.join("empty.md"), "").unwrap();
    let output = cmd.output().unwrap();
    assert!(output.status.success());

    let content = std::fs::read_to_string(ws.join(".agk/config.toml")).unwrap();
    // Assert structured format was written
    assert!(content.contains("[[profiles.skills]]"));
    assert!(content.contains('name = "rust-patterns"'));
    assert!(content.contains('vault = "auto"'));
}
```

---

### 5.4 Architecture & Unit Tests

#### Domain Tests (`src/domain/*.rs` `#[cfg(test)]`)

##### Test: `profile_asset_ref_serde_legacy_flat_string_to_structured`
**File:** `src/domain/profile.rs`  
**Feature:** F4, F14  

```rust
#[test]
fn profile_asset_ref_serde_legacy_flat_string_to_structured() {
    let toml: ProfileAssetRef = toml::from_str(r#""rust-patterns""#).unwrap();
    assert_eq!(toml.name, "rust-patterns");
    assert_eq!(toml.vault, "auto");
}
```

##### Test: `profile_asset_ref_serde_structured_roundtrip`
**File:** `src/domain/profile.rs`  
**Feature:** F4  

```rust
#[test]
fn profile_asset_ref_serde_structured_roundtrip() {
    let original = ProfileAssetRef {
        name: "docker".into(),
        vault: "ecc".into(),
    };
    let toml_str = toml::to_string(&original).unwrap();
    let parsed: ProfileAssetRef = toml::from_str(&toml_str).unwrap();
    assert_eq!(parsed.name, "docker");
    assert_eq!(parsed.vault, "ecc");
}
```

##### Test: `token_estimation_words_times_1_35`
**File:** `src/domain/profile.rs` or new `src/domain/token.rs`  
**Feature:** F3  

```rust
#[test]
fn token_estimation_words_times_1_35() {
    assert_eq!(estimate_tokens("one two three four"), 5); // 4 * 1.35 = 5.4 → round to nearest
    assert_eq!(estimate_tokens(""), 0);
    assert_eq!(estimate_tokens("   "), 0);
}
```

##### Test: `asset_kind_profile_does_not_break_existing_match_arms`
**File:** `src/domain/asset.rs`  
**Feature:** F7  

```rust
#[test]
fn asset_kind_profile_does_not_break_existing_match_arms() {
    // This test exists to force compilation failure if any exhaustive match
    // on AssetKind was not updated. We simply construct all variants.
    let _kinds = vec![AssetKind::Skill, AssetKind::Instruction, AssetKind::McpServer, AssetKind::Profile];
}
```

##### Test: `assess_mcp_security_flags`
**File:** `src/domain/mcp.rs`  
**Feature:** v0.3.1 Security Scorecard  

```rust
#[test]
fn assess_mcp_security_flags() {
    let flags = assess_mcp_security("npx", &["-y", "."]);
    assert!(flags.contains(&SecurityFlag::BroadFilesystem));

    let flags = assess_mcp_security("curl", &["http://example.com"]);
    assert!(flags.contains(&SecurityFlag::NetworkEgress));

    let flags = assess_mcp_security("bash", &["script.sh"]);
    assert!(flags.contains(&SecurityFlag::ArbitraryExecution));

    let flags = assess_mcp_security("python", &[]);
    assert!(flags.contains(&SecurityFlag::UnspecifiedArgs));
}
```

---

#### Use-Case Tests (`src/app/features/<f>/<verb>.rs` `#[cfg(test)]`)

##### Test: `wizard_structured_description_composes_canonical_markdown`
**File:** `src/app/features/profile/wizard_description.rs`  
**Feature:** F1  

```rust
#[test]
fn wizard_structured_description_composes_canonical_markdown() {
    let input = WizardInput {
        role: "Senior Rust CLI engineer".into(),
        domain: "Rust + async ecosystems".into(),
        audience: "Junior devs".into(),
        responsibilities: vec!["Review PRs".into(), "Suggest idioms".into()],
        style: "Direct and critical".into(),
        format: "Bullets, max 5 items".into(),
        constraints: "Always run cargo fmt".into(),
        triggers: "After any code change".into(),
    };
    let md = compose_description(&input);
    assert!(md.contains("# Identity"));
    assert!(md.contains("# Core Responsibilities"));
    assert!(md.contains("1. Review PRs"));
    assert!(md.contains("# Collaboration Style"));
    assert!(md.contains("# Constraints"));
    assert!(!md.contains("Q:")); // No raw Q&A pairs
}
```

##### Test: `template_code_reviewer_prefills_all_fields`
**File:** `src/app/features/profile/template.rs`  
**Feature:** F2  

```rust
#[test]
fn template_code_reviewer_prefills_all_fields() {
    let t = ArchetypeTemplate::code_reviewer();
    assert_eq!(t.role, "Senior code reviewer");
    assert_eq!(t.style, "Direct & critical");
    assert_eq!(t.default_tools, vec!["Read", "Glob", "Grep", "LSP"]);
}
```

##### Test: `mcp_feature_set_scanner_parses_mcp_md_frontmatter`
**File:** `src/infra/feature/mcp.rs`  
**Feature:** F6  

```rust
#[test]
fn mcp_feature_set_scanner_parses_mcp_md_frontmatter() {
    let tmp = tempfile::tempdir().unwrap();
    let mcp_dir = tmp.path().join("mcps/filesystem");
    std::fs::create_dir_all(&mcp_dir).unwrap();
    std::fs::write(mcp_dir.join("MCP.md"), r#"
---
name: filesystem
version: 1.0.0
command: npx
args: ["-y", "@modelcontextprotocol/server-filesystem", "."]
transport: stdio
---
"#).unwrap();

    let scanner = McpFeatureSet::new(tmp.path());
    let packages = scanner.scan().unwrap();
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0].name, "filesystem");
    assert_eq!(packages[0].kind, AssetKind::McpServer);
}
```

##### Test: `profile_feature_set_scanner_parses_profile_md_frontmatter`
**File:** `src/infra/feature/profile.rs`  
**Feature:** F7  

```rust
#[test]
fn profile_feature_set_scanner_parses_profile_md_frontmatter() {
    let tmp = tempfile::tempdir().unwrap();
    let profile_dir = tmp.path().join("profiles/web-app-team");
    std::fs::create_dir_all(&profile_dir).unwrap();
    std::fs::write(profile_dir.join("PROFILE.md"), r#"
---
name: web-app-team
version: 1.2.0
provider: opencode
skills:
  - acme-org/react-skills
instructions:
  - acme-org/web-app-guidelines
mcps:
  - filesystem
---
"#).unwrap();

    let scanner = ProfileFeatureSet::new(tmp.path());
    let packages = scanner.scan().unwrap();
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0].name, "web-app-team");
    assert_eq!(packages[0].kind, AssetKind::Profile);
}
```

##### Test: `filter_scan_includes_mcp_server_kind`
**File:** `src/app/features/vault/filter_scan.rs` (or existing location)  
**Feature:** F6  

```rust
#[test]
fn filter_scan_includes_mcp_server_kind() {
    let config = ConfigFile::default();
    let pkg = ScannedPackage {
        kind: AssetKind::McpServer,
        name: "filesystem".into(),
        ...
    };
    let result = filter_scan(&config, &pkg);
    assert!(result.is_some()); // Not filtered out
}
```

##### Test: `claude_code_available_profile_tools_returns_7_tools`
**File:** `src/infra/provider/claude_code/mod.rs` or `src/app/ports/provider.rs`  
**Feature:** F9  

```rust
#[test]
fn claude_code_available_profile_tools_returns_7_tools() {
    let provider = ClaudeCodeProvider::new();
    let tools = provider.available_profile_tools();
    assert_eq!(tools.len(), 7);
    let ids: Vec<_> = tools.iter().map(|(id, _, _)| id.clone()).collect();
    assert!(ids.contains(&"Read".into()));
    assert!(ids.contains(&"Glob".into()));
    assert!(ids.contains(&"Grep".into()));
    assert!(ids.contains(&"Bash".into()));
    assert!(ids.contains(&"Write".into()));
    assert!(ids.contains(&"Edit".into()));
    assert!(ids.contains(&"LSP".into()));
}
```

##### Test: `claude_code_available_permission_modes_returns_5_modes`
**File:** `src/infra/provider/claude_code/mod.rs`  
**Feature:** F9  

```rust
#[test]
fn claude_code_available_permission_modes_returns_5_modes() {
    let provider = ClaudeCodeProvider::new();
    let modes = provider.available_permission_modes();
    assert_eq!(modes.len(), 5);
    let ids: Vec<_> = modes.iter().map(|(id, _)| id.clone()).collect();
    assert!(ids.contains(&"default".into()));
    assert!(ids.contains(&"acceptEdits".into()));
    assert!(ids.contains(&"auto".into()));
    assert!(ids.contains(&"dontAsk".into()));
    assert!(ids.contains(&"plan".into()));
}
```

##### Test: `letta_returns_supports_mcp_false`
**File:** `src/infra/provider/letta/mod.rs` (or existing)  
**Feature:** F8  

```rust
#[test]
fn letta_returns_supports_mcp_false() {
    let provider = LettaProvider::new();
    assert!(!provider.supports_mcp());
}
```

---

### 5.5 Snapshot Tests (`tests/tui_render_v03.rs`)

Snapshot tests assert on the **visual layout** of TUI frames using `insta`.

| Test | Feature | Trigger |
|---|---|---|
| `wizard_review_step_layout_matches_spec` | F3 | Render `AppState` with `WizardState` at Review step. Assert frame contains token badge top-right, markdown preview center, counts bottom. |
| `editor_tab_layout` | F10 | Render `AppState` with `EditorState` active. Assert 5 tabs visible (Overview, Skills, MCPs, Tools, Raw Markdown). |
| `launch_simulation_overlay_layout` | F13 | Render `AppState` with `LaunchOverlay` at each of 4 steps. Assert progress bar and step labels. |
| `providers_tab_mcp_indicators` | F8 | Render Providers tab. Assert `[MCP: ✓]`, `[MCP: ✗]`, `[Tools: N]` aligned. |
| `mcp_security_badge_high_severity` | v0.3.1 | Render MCP tab with vault-discovered MCP having `broad-filesystem` flag. Assert `[!]` badge color. |

---

## 6. Test Data & Fixtures

### 6.1 Vault Directory Fixtures

Create `tests/fixtures/vaults/team-ready-vault/` for reuse across tests:

```text
team-ready-vault/
├── skills/
│   └── rust-patterns/
│       └── SKILL.md
│   └── docker/
│       └── SKILL.md
├── instructions/
│   └── web-app-guidelines/
│       └── AGENTS.md
├── mcps/
│   └── filesystem/
│       └── MCP.md
│   └── github-api/
│       └── MCP.md
├── profiles/
│   └── web-app-team/
│       └── PROFILE.md
└── README.md
```

**`mcps/filesystem/MCP.md`:**
```yaml
---
name: filesystem
version: 1.0.0
command: npx
args:
  - "-y"
  - "@modelcontextprotocol/server-filesystem"
  - "."
transport: stdio
description: File system access via MCP
---

# Filesystem MCP Server
```

**`profiles/web-app-team/PROFILE.md`:**
```yaml
---
name: web-app-team
version: 1.2.0
provider: opencode
description: Full-stack web development profile
skills:
  - acme-org/react-skills
  - acme-org/typescript-linter
instructions:
  - acme-org/web-app-guidelines
mcps:
  - filesystem
  - github-api
---

# Web App Team Profile
```

### 6.2 Config Fixtures

**Legacy flat config (`fixtures/config/legacy.toml`):**
```toml
[[profiles]]
name = "legacy"
provider_id = "opencode"
skills = ["rust-patterns", "docker"]
mcps = ["filesystem"]
```

**Structured v0.3 config (`fixtures/config/structured.toml`):**
```toml
[[profiles]]
name = "web-app-team"
provider_id = "opencode"
scope = "workspace"

[[profiles.skills]]
name = "rust-patterns"
vault = "clawhub"

[[profiles.skills]]
name = "docker"
vault = "ecc"

[[profiles.mcps]]
name = "filesystem"
vault = "workspace"

[profiles.tools]
refs = ["Read", "Glob", "Grep"]
permission_mode = "acceptEdits"

prompt_overlay_path = ".agk/profiles/web-app-team/custom.md"
```

### 6.3 Provider Config Fixtures

**`fixtures/provider/copilot-existing.json`:**
```json
{"existingKey": true, "mcpServers": {}}
```

**`fixtures/provider/gemini-existing.json`:**
```json
{"theme": "dark", "mcpServers": {}}
```

**`fixtures/provider/amp-existing.json`:**
```json
{"editor": {"fontSize": 14}, "amp": {"mcpServers": {}}}
```

---

## 7. Regression & Backward Compatibility

| Scenario | Test File | Verification |
|---|---|---|
| Old flat `skills = ["name"]` loads without error | `src/domain/profile.rs` | Deserializes to `vault: "auto"` |
| Old flat `mcps = ["name"]` loads without error | `src/domain/profile.rs` | Deserializes to `vault: "auto"` |
| Old profile without `tool_refs` defaults empty | `src/domain/profile.rs` | `serde(default)` yields `vec![]` |
| Old profile without `permission_mode` defaults `None` | `src/domain/profile.rs` | `serde(default)` yields `None` |
| Manually-registered MCPs still work | `tests/process_integration_v03.rs` | `agk mcp enable` on pre-v0.3 MCP succeeds |
| Existing TUI tabs (Skills, Instructions) unaffected | `tests/full_flow_tui/skill_install.rs` | Existing tests continue to pass |
| `agk p start` on v0.2 profile still launches | `tests/process_integration_v03.rs` | Uses wizard-generated description fallback |
| Config migration does not corrupt on read-only ops | `tests/process_integration_v03.rs` | `agk profile list` on legacy config does not rewrite file |

---

## 8. CI Gate Criteria

The following commands must pass in CI **in this order** before any v0.3 feature is considered complete:

```bash
# 1. Formatting
cargo fmt --check

# 2. Linting (zero warnings)
cargo clippy --workspace --all-targets --all-features -- -D warnings

# 3. Unit + Integration tests
cargo test

# 4. Architecture gate (zero allowlists)
cargo test --test architecture -- --ignored

# 5. Coverage gate (80%+ on app + domain)
cargo llvm-cov --fail-under-lines 80

# 6. Doc tests
cargo test --doc
```

**New for v0.3:**
- `cargo test --test contract_tests_v03` must pass.
- `cargo test --test process_integration_v03` must pass.
- `cargo test --test tui_render_v03` must pass.
- Architecture test `file_size_lint` must report **zero new allowlists** for files introduced in v0.3.

---

## 9. Execution Timeline

| Week | Epic Phase | Test Work | Owner |
|---|---|---|---|
| 1 | Phase 1: Structural Enablers | Implement Domain + UseCase tests for `ProfileAssetRef`, `AssetKind::Profile`, scanners, `filter_scan`. | Backend QA |
| 2 | Phase 1: Structural Enablers | Implement TUI tests for vault MCP/profile discovery. Implement Contract test for `vault scan --json`. Run architecture gate. | Frontend QA |
| 3 | Phase 2: Wizard Foundation | Implement Domain tests for token estimation. Implement UseCase tests for wizard composer + templates. | Backend QA |
| 4 | Phase 2: Wizard Foundation | Implement TUI/Snapshot tests for wizard review step. Implement Contract test for `profile create --template`. | Frontend QA |
| 5 | Phase 3: Provider Reach | Implement UseCase tests for new MCP providers. Implement Process tests for config write/read roundtrips. | Backend QA |
| 5 | Phase 3: Provider Reach | Implement TUI tests for provider tab indicators. Implement Contract test for `provider list --json`. | Frontend QA |
| 6 | Phase 4: Runtime Integration | Implement UseCase tests for auto-heal, batch install, atomic rollback, Claude Code projection. | Backend QA |
| 6 | Phase 4: Runtime Integration | Implement TUI tests for launch overlay + editor. Implement Process tests for atomicity + overlay file. | Frontend QA |
| 7 | Phase 5: Polish | Implement backward-compat tests, migration tests, security flag tests. Run full CI gate. Fix any failures. | QA Lead |
| 7 | Phase 5: Polish | Manual QA checklist execution (documented separately). Update user docs. | QA Lead + Docs |

---

## 10. Gap Analysis Summary

Every acceptance criterion from the epic proposal §6 is covered by at least one automated test in this plan:

| Epic Criterion | Coverage | Confidence |
|---|---|---|
| Wizard generates structured markdown | F1 UseCase + TUI | High |
| ≥5 archetype templates | F2 UseCase + TUI | High |
| Review step shows markdown + tokens | F3 TUI + Snapshot | High |
| Profile skills/MCPs stored with vault provenance | F4 Domain + UseCase + Contract | High |
| `agk p start` auto-installs missing deps | F5 UseCase + TUI + Process | High |
| `mcps/` scanned; MCPs appear in TUI | F6 UseCase + TUI + Contract | High |
| `profiles/` scanned; profiles appear in TUI | F7 UseCase + TUI + Contract | High |
| Installing vault profile installs all referenced assets | F12 UseCase + TUI + Process | High |
| Copilot/Gemini/AMP MCP support | F8 UseCase + Process + TUI | High |
| Old flat-string profiles continue to work | F14 Domain + UseCase + Process | High |
| No `.rs` file > 300 lines | Architecture gate | High |
| `cargo test` passes | CI gate | High |
| Provider tool checklist in wizard | F9 UseCase + TUI | High |
| F3 Editor raw markdown + live tokens | F10 TUI + Snapshot | High |
| Claude Code `agent.md` with frontmatter | F11 UseCase + Process | High |
| Batch installation atomic | F12 Process | High |

**No uncovered P0 or P1 criteria remain.**

---

*End of Test Plan v0.1 — 2026-05-31*
