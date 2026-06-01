# Release Plan: AGK v0.3 — "Team-Ready Profiles"

**Status:** In Progress — Phases 1–4 implemented on `v03` branch; Phase 5 (Polish) pending
**Target Date:** 7 weeks from kickoff
**Epic:** [`proposals/v03-team-ready-profiles.md`](proposals/v03-team-ready-profiles.md)

---

## 1. Release Overview

This release transforms AGK profiles from shallow Q&A-generated blobs into **structured, portable, team-distributable environment blueprints**. It extends vault scanning to MCP servers and profiles, expands MCP provider coverage, and adds archetype templates + token-aware prompt engineering to the profile wizard.

**Theme:** *A new developer joins a team, attaches the team vault, installs one profile, and has a fully configured AI agent in under 30 seconds.*

---

## 2. Phase Breakdown

### Phase 1: Structural Enablers — Weeks 1–2
**Goal:** Make vaults discover MCP servers and profiles; make profiles store vault provenance.

| Work Item | Owner | PRD | LOE | Dependencies |
|---|---|---|---|---|
| `ProfileAssetRef` domain model + backward-compatible serde | Backend | [Profiles](../../product/features/profiles/prd.md) §Vault-Aware Storage | 2d | — |
| `AssetKind::Profile` enum extension | Backend | [Vault Multi-Asset](../../product/features/vault-multi-asset/prd.md) §Domain | 1d | — |
| `McpFeatureSet` scanner (`mcps/*/MCP.md`) | Backend | [Vault Multi-Asset](../../product/features/vault-multi-asset/prd.md) §McpFeatureSet | 2d | — |
| `ProfileFeatureSet` scanner (`profiles/*/PROFILE.md`) | Backend | [Vault Multi-Asset](../../product/features/vault-multi-asset/prd.md) §ProfileFeatureSet | 2d | `AssetKind::Profile` |
| Update `filter_scan` for `McpServer` + `Profile` | Backend | [Vault Multi-Asset](../../product/features/vault-multi-asset/prd.md) §filter_scan | 1d | Both scanners |
| Delete `StubFeatureSet("mcp")` / `StubFeatureSet("profile")` | Backend | [Vault Multi-Asset](../../product/features/vault-multi-asset/prd.md) §Cleanup | 1d | Scanners |
| TUI MCP tab: show vault-discovered MCPs (`[⊘]`/`[ ]`/`[x]`) | Frontend | [MCP Vault](../../product/features/mcp-vault/prd.md) §Vault-Discovered MCPs | 2d | `McpFeatureSet` |
| TUI Profile tab: show vault-discovered profiles | Frontend | [Profiles](../../product/features/profiles/prd.md) §Vault Profiles | 2d | `ProfileFeatureSet` |

**Phase 1 Exit Criteria:**
- [x] `cargo test` passes; architecture tests pass with zero allowlists.
- [x] TUI shows vault-discovered MCPs and profiles in their respective tabs.
- [x] Old profiles without vault info still load and function.

> **Implementation note:** `StubFeatureSet` still exists but is only used for "provider" and "vault" tabs. MCP and Profile tabs use real `McpFeatureSet` and `ProfileFeatureSet` scanners.

---

### Phase 2: Wizard Foundation — Weeks 3–4
**Goal:** Replace the shallow 3-question wizard with a structured, template-driven, token-aware experience.

| Work Item | Owner | PRD | LOE | Dependencies |
|---|---|---|---|---|
| New `WizardStep` variants: `TemplateSelect`, `ScopeSelect`, `Textarea` | Frontend | [Profile Wizard](../../product/features/profile-wizard/prd.md) §Wizard Steps | 2d | — |
| Structured markdown composer (`wizard_description.rs`) | Backend | [Profile Wizard](../../product/features/profile-wizard/prd.md) §Prompt Contract | 3d | — |
| Archetype template data + pre-fill logic | Backend | [Profile Wizard](../../product/features/profile-wizard/prd.md) §Templates | 2d | Composer |
| Token estimation utility (`words * 1.35`) | Backend | [Profile Wizard](../../product/features/profile-wizard/prd.md) §Tokens | 1d | — |
| Review step: scrollable markdown preview + token badge | Frontend | [Profile Wizard](../../product/features/profile-wizard/prd.md) §Review Step | 2d | Composer + Tokens |
| Update `OpenCodeProvider::profile_wizard_steps()` to 16-step sequence | Backend | [Profiles](../../product/features/profiles/prd.md) §Wizard | 2d | WizardStep variants |
| Update `handle_profile_wizard_input()` for new step types | Frontend | [Profile Wizard](../../product/features/profile-wizard/prd.md) §TUI Integration | 2d | WizardStep variants |

**Phase 2 Exit Criteria:**
- [x] Wizard generates structured markdown (not raw Q&A) for OpenCode provider.
- [x] At least 5 archetype templates available (6 templates: Code Reviewer, Feature Implementer, Security Auditor, Documentation Writer, Test Generator, Custom).
- [x] Review step shows composed markdown with estimated token count.
- [x] Template path completes wizard in ≤ 10 steps (excluding checklist/review).

---

### Phase 3: Provider Reach — Weeks 4–5
**Goal:** Expand MCP support to 5 providers and add tool/permission configurability.

| Work Item | Owner | PRD | LOE | Dependencies |
|---|---|---|---|---|
| Copilot CLI `McpProvider` implementation | Backend | [Providers](../../product/features/providers/prd.md) §MCP Adapters | 2d | — |
| Gemini CLI `McpProvider` implementation | Backend | [Providers](../../product/features/providers/prd.md) §MCP Adapters | 2d | — |
| AMP `McpProvider` implementation | Backend | [Providers](../../product/features/providers/prd.md) §MCP Adapters | 2d | — |
| Mark Letta/Snowflake/Firebender as `supports_mcp: false` | Backend | [Providers](../../product/features/providers/prd.md) §MCP Adapters | 1d | — |
| Wire new providers into `build_mcp_providers()` + bootstrap | Backend | [Providers](../../product/features/providers/prd.md) §MCP Adapters | 1d | All 3 impls |
| `available_profile_tools()` + `available_permission_modes()` on `ProviderPort` | Backend | [Providers](../../product/features/providers/prd.md) §Tool Selection | 2d | — |
| Implement for Claude Code: tool list + permission modes | Backend | [Providers](../../product/features/providers/prd.md) §Tool Selection | 2d | Port methods |
| Implement for OpenCode: per-agent tool config (if exposed) | Backend | [Providers](../../product/features/providers/prd.md) §Tool Selection | 2d | Port methods |

**Phase 3 Exit Criteria:**
- [x] `agk mcp add` writes config for Claude Code, OpenCode, Copilot, Gemini, and AMP.
- [x] TUI Providers tab shows MCP checkbox `[✓]` only for capable providers.
- [x] Provider port exposes tool/permission lists (may be empty for some providers).

> **Implementation note:** The Copilot CLI provider is implemented as `GithubProvider` (id: `github-copilot`) rather than a separate `CopilotCliProvider`. Functionally identical — writes to `.copilot/mcp-config.json` (global) or `.github/mcp-config.json` (workspace).

---

### Phase 4: Runtime Integration — Weeks 5–6
**Goal:** Make `agk p start` self-healing; add editor + Claude Code projection.

| Work Item | Owner | PRD | LOE | Dependencies |
|---|---|---|---|---|
| `agk p start` dependency resolution loop (skills + MCPs) | Backend | [Profiles](../../product/features/profiles/prd.md) §Auto-Install | 3d | `ProfileAssetRef`, vault scanners |
| Auto-install missing skills from specified vaults | Backend | [Profiles](../../product/features/profiles/prd.md) §Auto-Install | 2d | Resolution loop |
| Auto-register missing MCPs from specified vaults | Backend | [Profiles](../../product/features/profiles/prd.md) §Auto-Install | 2d | Resolution loop |
| Clear error when vault unavailable / asset not found | Backend | [Profiles](../../product/features/profiles/prd.md) §Auto-Install | 1d | Resolution loop |
| Profile batch install: atomic skill + MCP + profile creation | Backend | [Vault Multi-Asset](../../product/features/vault-multi-asset/prd.md) §Batch Install | 3d | Scanners + auto-install |
| F3 Editor: skills/MCPs/tools/raw markdown editing | Frontend | [Profiles](../../product/features/profiles/prd.md) §Editor | 3d | Vault-aware storage |
| Live token count update in editor | Frontend | [Profile Wizard](../../product/features/profile-wizard/prd.md) §Tokens | 1d | Token utility |
| Claude Code `agent.md` projection (frontmatter + body) | Backend | [Profiles](../../product/features/profiles/prd.md) §Claude Code Projection | 3d | Structured markdown |
| `prompt_overlay_path` support | Backend | [Profiles](../../product/features/profiles/prd.md) §Custom Overlay | 1d | — |

**Phase 4 Exit Criteria:**
- [x] `agk p start <profile>` on a fresh workspace installs all missing dependencies.
- [x] Installing a vault profile installs all referenced assets atomically (with rollback on failure).
- [x] F3 Editor allows editing skills (with vault), MCPs, and permission mode.
- [x] Claude Code provider writes `.agk/profiles/<name>/agent.md` with frontmatter.
- ⚠️ F3 Editor does not show live token count — token estimation is only in the wizard review step.
- ⚠️ Vault-sourced MCPs auto-register with name as placeholder command; user must verify.

---

### Phase 5: Polish — Week 7
**Goal:** CI green, manual QA, docs, migration.

| Work Item | Owner | PRD | LOE | Dependencies |
|---|---|---|---|---|
| Config migration: old flat format → structured on first write | Backend | [Profiles](../../product/features/profiles/prd.md) §Migration | 2d | Vault-aware storage |
| TUI launch simulation overlay (dep resolution → install → runtime) | Frontend | [Profiles](../../product/features/profiles/prd.md) §Launch Sim | 3d | Auto-install |
| Integration tests: wizard full flow (template → review → save) | QA | [Profile Wizard](../../product/features/profile-wizard/prd.md) §Tests | 2d | Wizard |
| Integration tests: vault profile install + start | QA | [Vault Multi-Asset](../../product/features/vault-multi-asset/prd.md) §Tests | 2d | Batch install |
| Integration tests: MCP provider write/read roundtrips | QA | [Providers](../../product/features/providers/prd.md) §Tests | 2d | MCP adapters |
| Manual QA: TUI vault attach → profile install → start | QA | — | 2d | All |
| Update user docs (`docs/product/`) | Docs | — | 2d | All |

**Phase 5 Exit Criteria:**
- [x] `cargo test` passes (unit + integration).
- [x] `cargo test --test architecture -- --ignored` passes with zero allowlists.
- [x] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- [x] `cargo fmt --check` passes.
- [ ] Integration tests: wizard full flow (template → review → save).
- [ ] Integration tests: vault profile install + start.
- [ ] Integration tests: MCP provider write/read roundtrips.
- [ ] Manual QA checklist complete (vault attach, profile install, provider toggle, MCP register, profile start).
- [ ] Config migration: old flat format → structured on first write.
- [ ] TUI launch simulation overlay.

---

## 3. Cross-Cutting Concerns

### Architecture Integrity
- Every new file must obey ADR-001 dependency rules (`domain/` pure, `app/` owns logic, `infra/` owns I/O, `cli/`/`tui/` thin).
- No `.rs` file may exceed 300 lines of non-test logic.
- All new features must have architecture test coverage (no new allowlists).

### Testing Strategy
- **Unit:** Domain model changes (`ProfileAssetRef`, `AssetKind::Profile`), token estimation.
- **Contract:** CLI `--json` output shape for `profile create`, `mcp list`, `vault scan`.
- **Integration:** Full TUI flow via `TestBackend` (wizard template selection → review → save).
- **Process:** MCP provider config write/read roundtrips against temp directories.

### Backward Compatibility
- Old `skills = ["name"]` deserializes to `ProfileAssetRef { name, vault: "auto" }`.
- Old profiles without `tool_refs` / `permission_mode` default to empty / None.
- Old `McpServer` stub behavior (no vault scanning) is replaced, not removed — existing manually-registered MCPs continue to work.

---

## 4. Risk Register

| Risk | Phase | Impact | Mitigation | Owner |
|---|---|---|---|---|
| Config migration corrupts user profiles | 4–5 | High | Write migration behind `#[serde(default)]`; dry-run mode; backup old config before write | Backend |
| Vault-discovered MCP collides with manually-registered | 1 | Medium | Prefix vault-sourced names with `vault_id/`; warning modal on collision | Backend |
| Token estimation heuristic is misleading | 2 | Low | Label "Est."; do not block save on high counts | Frontend |
| Claude Code `agent.md` format changes mid-release | 4 | Medium | AGK owns canonical body; frontmatter is adapter concern; monitor Claude Code releases | Backend |
| Provider tool lists diverge across versions | 3 | Low | `available_profile_tools()` is runtime query; wizard auto-adapts | Backend |
| Phase 1 + Phase 2 overlap causes merge conflicts | 1–2 | Medium | Phase 1 merges to `master` before Phase 2 branches; feature flags if needed | Tech Lead |

---

## 5. PRD Index

| Feature Area | PRD | Technical Design | Covers Features |
|---|---|---|---|
| Profiles (updated) | [`product/features/profiles/prd.md`](../../product/features/profiles/prd.md) | existing | F4, F5, F10–F15 |
| Profile Wizard (new) | [`product/features/profile-wizard/prd.md`](../../product/features/profile-wizard/prd.md) | to be written | F1–F3 |
| MCP Vault (updated) | [`product/features/mcp-vault/prd.md`](../../product/features/mcp-vault/prd.md) | existing | F6, F8 |
| Vault Multi-Asset (new) | [`product/features/vault-multi-asset/prd.md`](../../product/features/vault-multi-asset/prd.md) | to be written | F6, F7, F12 |
| Providers (updated) | [`product/features/providers/prd.md`](../../product/features/providers/prd.md) | existing | F8, F9 |

---

## 6. Milestones

| Milestone | Date | Deliverable | Status |
|---|---|---|---|
| M1: Structural Enablers Complete | End of Week 2 | Vaults scan MCPs + Profiles; TUI shows them; `ProfileAssetRef` exists | ✅ Done |
| M2: Wizard Foundation Complete | End of Week 4 | Structured markdown wizard; 6 templates; token preview | ✅ Done |
| M3: Provider Reach Complete | End of Week 5 | 5 MCP-capable providers; tool/permission port methods | ✅ Done |
| M4: Runtime Integration Complete | End of Week 6 | `agk p start` self-heals; editor enhanced; Claude Code projection | ✅ Done |
| M5: v0.3.0 Release Ready | End of Week 7 | All tests green; manual QA passed; docs updated | ❌ Pending |

---

## 7. Post-Release Fast Follows (v0.3.x)

- GHES Vault Adapter (small, isolated — [P7](../../proposals/enterprise-feature-pack.md))
- Performance: Parallel vault scanning for `mcps/` + `profiles/` directories
- Telemetry: Track which archetype templates are most used

---

*Release Plan v0.2 — 2026-06-01 — updated to reflect implementation status*
