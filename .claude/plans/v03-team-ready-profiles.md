# AGK v0.3 — "Team-Ready Profiles" Implementation Plan

## Overview

This plan implements the v0.3 release which transforms AGK profiles from shallow Q&A-generated blobs into structured, portable, team-distributable environment blueprints. It extends vault scanning to MCP servers and profiles, expands provider coverage, and adds archetype templates + token-aware prompt engineering.

## Current State Assessment

**Already implemented (from prior work):**
- GitHub Copilot, Gemini CLI, and AMP already have `McpProvider` implementations
- `build_mcp_providers()` already wires all 5 MCP-capable providers
- Claude Code and OpenCode already have profile session support
- Hexagonal architecture (ADR-001) is enforced with architecture tests

**What needs to be built:**
- `AssetKind::Profile` domain extension
- `ProfileAssetRef` vault-aware storage with backward-compatible serde
- `McpFeatureSet` and `ProfileFeatureSet` scanners
- `filter_scan` updates for new asset kinds
- Delete `StubFeatureSet("mcp")` / `StubFeatureSet("profile")`
- TUI MCP tab: show vault-discovered MCPs with `[⊘]`/`[ ]`/`[x]` badges
- TUI Profile tab: show vault-discovered profiles
- New `WizardStep` variants: `TemplateSelect`, `ScopeSelect`, `Textarea`
- Structured markdown composer (`wizard_description.rs`)
- Archetype template data + pre-fill logic
- Token estimation utility (`words * 1.35`)
- Review step with scrollable markdown preview + token badge
- Update `OpenCodeProvider::profile_wizard_steps()` to 16-step sequence
- `available_profile_tools()` + `available_permission_modes()` on `ProviderPort`
- Letta/Snowflake/Firebender marked as `supports_mcp: false`
- `agk p start` dependency resolution loop (skills + MCPs)
- Auto-install missing skills/MCPs from specified vaults
- Profile batch install: atomic skill + MCP + profile creation
- Claude Code `agent.md` projection (frontmatter + body)
- `prompt_overlay_path` support
- Config migration: old flat format → structured on first write
- Integration tests for all new flows
- Architecture test compliance (zero new allowlists)

## Commit Strategy

Each phase is a separate commit to keep history clean and bisectable.

---

### Commit 1: Phase 1A — Domain Model & Scanners (Structural Enablers)

**Files to modify:**
1. `src/domain/asset.rs` — Add `AssetKind::Profile`
2. `src/domain/profile.rs` — Add `ProfileAssetRef` struct, update `Profile` to use vault-aware refs
3. `src/domain/config.rs` — Update `Profile` serde for backward compatibility, add `installed_mcps` / `installed_profiles` buckets, add `is_mcp_registered` / `is_profile_installed` helpers
4. `src/infra/feature/mcp.rs` — New `McpFeatureSet` scanner (reads `mcps/*/MCP.md`)
5. `src/infra/feature/profile.rs` — New `ProfileFeatureSet` scanner (reads `profiles/*/PROFILE.md`)
6. `src/infra/feature/mod.rs` — Export new feature sets
7. `src/app/bootstrap/scan.rs` — Update `filter_scan` for `McpServer` + `Profile`
8. `src/app/tab_kind.rs` — Update `tab_kind_for_asset_kind` for `Profile`
9. `src/app/bootstrap/registry.rs` — Replace `StubFeatureSet("mcp")` with `McpFeatureSet`, replace `StubFeatureSet("profile")` with `ProfileFeatureSet`

**Tests:**
- Unit tests for `ProfileAssetRef` serde roundtrip
- Unit tests for backward-compatible flat format deserialization
- Unit tests for `McpFeatureSet::is_package` and `hash_files`
- Unit tests for `ProfileFeatureSet::is_package` and `hash_files`
- Architecture tests must pass with zero new allowlists

---

### Commit 2: Phase 1B — TUI Integration (Vault-Discovered Assets)

**Files to modify:**
1. `src/tui/widgets/mcp/render.rs` — Add `[⊘]` badge for vault-discovered MCPs not in global registry
2. `src/tui/widgets/mcp/state.rs` — Merge vault-discovered MCPs into `McpState` (if needed)
3. `src/tui/widgets/list_entity.rs` — Update `render_profiles` to show vault-discovered profiles with `[Vault]` badge
4. `src/tui/render/content.rs` — Wire new tab rendering
5. `src/tui/event.rs` — Handle `Space` on vault-discovered MCP to register, handle `Space` on vault profile to batch-install
6. `src/app/features/profile/create.rs` — Batch install logic for vault profiles

**Tests:**
- TUI full-flow tests via `TestBackend`
- Contract tests for `agk mcp list --json` with vault-discovered MCPs

---

### Commit 3: Phase 2 — Wizard Foundation

**Files to modify:**
1. `src/app/ports/provider.rs` — Add `WizardStep::TemplateSelect`, `ScopeSelect`, `Textarea` variants; add `available_profile_tools()` and `available_permission_modes()` defaults on `ProviderPort`
2. `src/app/features/profile/wizard_description.rs` — New module: structured markdown composer with the "AGK Prompt Contract"
3. `src/app/features/profile/wizard_templates.rs` — New module: 6 archetype templates (Code Reviewer, Feature Implementer, Security Auditor, Documentation Writer, Test Generator, Custom)
4. `src/app/features/profile/token_estimate.rs` — New module: `estimate_tokens(text: &str) -> usize` using `words * 1.35`
5. `src/infra/provider/opencode/mod.rs` — Update `profile_wizard_steps()` to return 16-step sequence
6. `src/infra/provider/claude_code.rs` — Implement `available_profile_tools()` and `available_permission_modes()`
7. `src/tui/features/profiles/controller.rs` — Update `handle_profile_wizard_input()` for new step types (TemplateSelect pre-fill, Textarea multiline input, Review step with markdown preview)
8. `src/tui/widgets/wizard/render.rs` — Render new step types

**Tests:**
- Unit tests for token estimation
- Unit tests for structured markdown composer output
- Unit tests for template pre-fill logic
- Integration test: wizard full flow (template → review → save)

---

### Commit 4: Phase 3 — Provider Reach & Tool/Permission Config

**Files to modify:**
1. `src/infra/provider/letta.rs` — Add explicit `supports_mcp: false` (impl `McpProvider` returning false)
2. `src/infra/provider/snowflake.rs` — Add explicit `supports_mcp: false`
3. `src/infra/provider/firebender.rs` — Add explicit `supports_mcp: false`
4. `src/infra/provider/claude_code.rs` — Implement `available_profile_tools()` (Read, Glob, Grep, Bash, Write, Edit, LSP) and `available_permission_modes()` (default, acceptEdits, auto, dontAsk, plan)
5. `src/infra/provider/opencode/mod.rs` — Implement `available_profile_tools()` and `available_permission_modes()` (if exposed, else empty)
6. `src/tui/widgets/provider/render.rs` — Show MCP checkbox `[✓]` only for capable providers, show tool count

**Tests:**
- Contract tests for `agk provider list --json` with new fields
- Tests for write/read roundtrips for all MCP providers

---

### Commit 5: Phase 4 — Runtime Integration

**Files to modify:**
1. `src/app/features/profile/start.rs` — Dependency resolution loop: read `profile.skill_refs` + `profile.mcp_refs`, auto-install missing skills from vaults, auto-register missing MCPs from vaults
2. `src/app/features/profile/create.rs` — Batch profile install: atomic skill + MCP + profile creation
3. `src/infra/provider/claude_code.rs` — `agent.md` projection with YAML frontmatter + structured body
4. `src/infra/provider/opencode/session.rs` — Support `tool_refs` and `permission_mode` in launch plan
5. `src/domain/profile.rs` — Add `prompt_overlay_path` field
6. `src/tui/widgets/profile/editor.rs` — F3 Editor: skills/MCPs/tools/raw markdown editing with live token count

**Tests:**
- Integration tests: vault profile install + start
- Unit tests for Claude Code frontmatter generation

---

### Commit 6: Phase 5 — Migration, Tests & Polish

**Files to modify:**
1. `src/domain/config.rs` — Config migration: old flat `skills = ["name"]` deserializes to `ProfileAssetRef { name, vault: "auto" }`; rewrite to structured format on save
2. `src/app/features/profile/create.rs` — Headless `agk profile create` with `--skills name:vault` syntax
3. `tests/full_flow_tui/` — Integration tests: wizard full flow, vault profile install + start, MCP provider roundtrips
4. `tests/architecture.rs` — Ensure zero new allowlists
5. `docs/product/features/*/prd.md` — Update docs (if time permits)

**Final validation:**
- `cargo test` passes (unit + integration)
- `cargo test --test architecture -- --ignored` passes with zero allowlists
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- `cargo fmt --check` passes

---

## Architecture Compliance Checklist

- [ ] Every new file obeys ADR-001 dependency rules (`domain/` pure, `app/` owns logic, `infra/` owns I/O, `cli/`/`tui/` thin)
- [ ] No `.rs` file exceeds 300 lines of non-test logic
- [ ] All new features have architecture test coverage (no new allowlists)
- [ ] `domain/` contains no `std::fs::` or `std::process::` outside `#[cfg(test)]`
- [ ] `app/` does not import `tui/` or `cli/`
- [ ] `infra/` does not import `tui/` or `cli/`
- [ ] `tui/` does not import `infra/`

## Risk Mitigations

| Risk | Mitigation |
|------|-----------|
| Config migration corrupts profiles | `#[serde(default)]` on all new fields; backward-compatible deserializer for flat `skills` / `mcps` arrays |
| Vault-discovered MCP collides with manually-registered | Prefix vault-sourced names with `vault_id/` during registration |
| File exceeds 300 lines | Split into submodules early; use `mod foo;` + `foo.rs` pattern |
| Architecture test fails | Run `cargo test --test architecture -- --ignored` after every commit |

---

*Plan v1.0 — 2026-05-31*
