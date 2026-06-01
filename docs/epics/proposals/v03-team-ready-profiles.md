# Epic Proposal: AGK v0.3 — "Team-Ready Profiles"

**Status:** Implemented — merged to master via #26
**Target Release:** v0.3.0
**Theme:** *From personal skill manager to team environment blueprint*
**Author:** Technical Product Owner (Claude synthesis)
**Date:** 2026-05-30

---

## 1. Situation Assessment

### What's Shipped (v0.2.x — Strong Foundation)

| Capability | State |
|---|---|
| Hexagonal architecture (ADR-001) | ✅ Complete — unified core, vertical slices, zero allowlists |
| TUI / CLI parity | ✅ Complete — same `CoreCommand` bus, contract tests enforce equivalence |
| Vault scanning (skills, instructions) | ✅ Complete — local, GitHub, ClawHub backends |
| MCP registry & enable/disable | ✅ Complete — Claude Code + OpenCode |
| Profile creation & launch | ✅ Complete — wizard + `agk p start`, but shallow |
| Asset dependency resolution | ✅ Complete — `requires:` with cycle + diamond handling |
| P8: Full-flow testing | ✅ Complete — 3-layer test pyramid (terminal → contract → process) |
| P9: Underground observability | ✅ Complete — `TaskTrackerPort`, hung-task detection |

### What's Broken / Weak

1. **Profile wizard produces garbage prompts** — 3 free-text Q&A questions concatenated into a raw blob. No role framing, no triggers, no templates.
2. **Profiles are not vault-discoverable** — Teams cannot distribute a complete environment blueprint (skills + MCPs + profile) through a vault.
3. **MCP servers are not vault-discoverable** — Every developer must manually register MCP servers one-by-one.
4. **No vault provenance on profile dependencies** — `skills = ["rust-patterns"]` breaks if the skill exists in multiple vaults.
5. **MCP support is narrow** — Only 2 of 5 capable providers have MCP adapters.
6. **No tool/permission configuration** — Wizard offers no least-privilege tool selection.

### Strategic Context

From the [AGK vs. Coder research](../../proposals/agk-vs-coder-research.md), AGK should own **portable skill packaging** and **cross-provider sync**, not enterprise governance or workspace provisioning. The next release must close the gap between "AGK distributes skills" and "AGK distributes *complete, ready-to-run agent environments*."

---

## 2. Epic Narrative

> A platform team maintains `github.com/acme-org/ai-workflows`.
> **Today:** New hire Alice attaches the vault, then manually installs 8 skills, registers 4 MCP servers, and fumbles through a 3-question profile wizard. **Time: 10 minutes.**
> **v0.3 Goal:** Alice attaches the vault and sees **Skills, Instructions, MCP Servers, and Profiles** all listed. She installs the team's profile with `Space` — it auto-installs all bundled assets. She starts the profile. **Time: 30 seconds.**

This epic makes the **Profile** the capstone artifact of AGK. A profile becomes a **portable, versioned, self-healing environment blueprint** that teams can distribute through vaults.

---

## 3. Feature List (Prioritized)

### 🔴 Must-Have (P0) — Ship Blockers

| ID | Feature | Source Proposal | Problem Solved | LOE |
|---|---|---|---|---|
| **F1** | **Enhanced Profile Wizard Core** | ~~P10~~ (implemented) Phases 1+4 | Wizard generates structured markdown with role, domain, style, triggers instead of raw Q&A | Medium |
| **F2** | **Agent Archetype Templates** | ~~P10~~ (implemented) §4.5 | Users start from "Code Reviewer" or "Feature Implementer" templates, not blank slate | Low |
| **F3** | **Token Estimation & Preview** | ~~P10~~ (implemented) §4.4 | Review step shows composed markdown + estimated tokens; warns if >800 tokens | Low |
| **F4** | **Vault-Aware Dependency Storage** | ~~P10~~ (implemented) Phase 2 | Skills/MCPs in profile store originating vault; enables auto-resolve | Medium |
| **F5** | **Auto-Install Missing Dependencies** | ~~P10~~ (implemented) Phase 2 | `agk p start` resolves missing skills/MCPs from specified vaults before launching | Medium |
| **F6** | **Vault-Discoverable MCP Servers** | ~~Vault Multi-Asset~~ (implemented) Phase 1 | `mcps/` directory in vault scanned; MCP definitions appear in TUI MCP tab | Medium |
| **F7** | **Vault-Discoverable Profiles** | ~~Vault Multi-Asset~~ (implemented) Phase 2 | `profiles/` directory in vault scanned; team profiles installable with `Space` | Medium |
| **F8** | **MCP Provider Expansion** | ~~P6~~ (implemented) | Add Copilot CLI, Gemini CLI, AMP MCP adapters (3 new providers) | Medium |

### 🟡 Should-Have (P1) — High Value, Can Slip

| ID | Feature | Source Proposal | Problem Solved | LOE |
|---|---|---|---|---|
| **F9** | **Provider Tool/Permission Selection** | ~~P10~~ (implemented) Phase 3 | Wizard checklist for least-privilege tools (Claude Code: Read/Glob/Grep/etc.) | Medium |
| **F10** | **Profile Editor (F3) Enhancement** | ~~P10~~ (implemented) §5 | Edit skills (with vault), MCPs, tools, and raw markdown with live token count | Medium |
| **F11** | **Claude Code Agent File Projection** | ~~P10~~ (implemented) Phase 5 (partial) | Write `.agk/profiles/<name>/agent.md` with frontmatter for providers without native wizard | Medium |
| **F12** | **Profile Batch Installation** | ~~Vault Multi-Asset~~ (implemented) §5.2 | Installing a vault profile installs all referenced skills, instructions, and MCPs atomically | Medium |

### 🟢 Could-Have (P2) — Nice to Have

| ID | Feature | Source Proposal | Problem Solved | LOE |
|---|---|---|---|---|
| **F13** | **Profile Launch Simulation** | ~~P10~~ (implemented) HTML sim | Visual dependency resolution → install → projection → runtime in TUI | Low |
| **F14** | **Backward-Compatible Config Migration** | ~~P10~~ (implemented) §4.6 | Old flat `skills = ["name"]` auto-upgrades to structured on first write | Low |
| **F15** | **Custom `prompt_overlay_path` Support** | ~~P10~~ (implemented) §5 | Allow users to supply their own `agent.md` instead of wizard-generated | Low |

### 🔵 Will-Not-Do (Explicitly Out of Scope)

| Feature | Why Excluded | When |
|---|---|---|
| Harness Orchestrator (RIPER-5, `process/` management) | Visionary; needs format research + community validation | v0.4 or later |
| Enterprise Policy Engine ([P7](../../proposals/enterprise-feature-pack.md)) | Large, governance-focused; different persona | Separate epic "AGK Enterprise" |
| Skill Signing & GPG ([P7](../../proposals/enterprise-feature-pack.md)) | Depends on policy engine infrastructure | After P7 |
| Team Config Sync / `.agk/team.toml` ([P7](../../proposals/enterprise-feature-pack.md)) | Needs policy engine first to avoid config drift chaos | After P7 |
| GHES Vault Adapter ([P7](../../proposals/enterprise-feature-pack.md)) | Small but isolated; can be a point release | v0.3.x patch |
| Coder Provider Adapter | Strategic integration; requires Tailnet/codersdk research | Future partnership |

---

## 4. Architecture & Sequencing

### Release Phases

**Phase 1: Structural Enablers (Weeks 1–2)**
- F4: `ProfileAssetRef` struct + backward-compatible serde
- F6: `McpFeatureSet` scanner + `MCP.md` format
- F7: `ProfileFeatureSet` scanner + `PROFILE.md` format + `AssetKind::Profile`
- Update `filter_scan` to handle `McpServer` and `Profile`
- Delete `StubFeatureSet("mcp")` and `StubFeatureSet("profile")`

**Phase 2: Wizard Foundation (Weeks 3–4)**
- F1: New `WizardStep` variants (`TemplateSelect`, `ScopeSelect`, `Textarea`)
- Rewrite `composed_description()` → structured markdown generator
- F2: Archetype template data structures + pre-fill logic
- F3: Token estimation utility (`words * 1.35`) + Review step preview

**Phase 3: Provider Reach (Weeks 4–5, parallel with Phase 2)**
- F8: Implement `McpProvider` for Copilot CLI, Gemini CLI, AMP
- F9: `available_profile_tools()` / `available_permission_modes()` on `ProviderPort`

**Phase 4: Runtime Integration (Weeks 5–6)**
- F5: `agk p start` dependency resolution + auto-install loop
- F10: F3 Editor extension for raw markdown + token tracking
- F11: Claude Code direct `agent.md` projection
- F12: Atomic batch install for vault profiles

**Phase 5: Polish (Week 7)**
- F13: TUI launch simulation overlay
- F14: Config migration on first write
- Full integration test coverage + manual QA

### New File Inventory

| Path | Purpose | Status |
|---|---|---|
| `src/infra/feature/mcp.rs` | `McpFeatureSet` scanner | ✅ Implemented |
| `src/infra/feature/profile.rs` | `ProfileFeatureSet` scanner | ✅ Implemented |
| `src/domain/asset.rs` | `AssetKind::Profile` enum variant | ✅ Implemented |
| `src/domain/profile.rs` | `ProfileAssetRef` + `skill_refs`, `mcp_refs`, `instruction_refs`, `tool_refs`, `permission_mode`, `prompt_overlay_path` | ✅ Implemented |
| `src/app/ports/provider.rs` | `available_profile_tools()`, `available_permission_modes()`, `WizardStep` variants, `ArchetypeTemplate` | ✅ Implemented |
| `src/app/features/profile/wizard_description.rs` | Structured markdown composer | ✅ Implemented |
| `src/app/features/profile/template.rs` | Archetype definitions (6 templates) | ✅ Implemented |
| `src/app/features/profile/token_estimate.rs` | Token estimation utility (`words * 1.35`) | ✅ Implemented |
| `src/app/features/profile/batch_install.rs` | Batch dependency resolution + rollback | ✅ Implemented |
| `src/infra/provider/github.rs` | GitHub Copilot `McpProvider` (note: `github.rs`, not `copilot_mcp.rs`) | ✅ Implemented |
| `src/infra/provider/gemini.rs` | Gemini CLI `McpProvider` (note: `gemini.rs`, not `gemini_mcp.rs`) | ✅ Implemented |
| `src/infra/provider/amp.rs` | AMP `McpProvider` | ✅ Implemented |
| `src/tui/widgets/edit_profile_modal.rs` | F3 profile editor (skills, MCPs, permissions) | ✅ Implemented |
| `src/infra/provider/claude_code/session.rs` | Claude Code `agent.md` projection + `compose_agent_markdown()` | ✅ Implemented |

> **Note:** The original plan listed `copilot_mcp.rs`, `gemini_mcp.rs`, and `amp_mcp.rs` as separate MCP-specific files. The implementation integrates MCP support directly into the provider modules (`github.rs`, `gemini.rs`, `amp.rs`) via the `McpProvider` trait, which is cleaner and avoids file proliferation.

---

## 5. Design Decisions

### 5.1 The "AGK Prompt Contract"

Regardless of provider, AGK composes a canonical structured markdown body:

```markdown
# Identity
You are a {role} specializing in {domain}. You work with {audience}.

# Core Responsibilities
{numbered_responsibilities}

# Collaboration Style
{tone_and_style}
...
```

- **OpenCode:** AGK feeds this body as `--description` to `opencode agent create` (native wizard generates frontmatter).
- **Claude Code:** AGK writes full frontmatter (`name`, `description` with `<example>` blocks, `tools`, `model`) + body to `.agk/profiles/<name>/agent.md`.
- **Future providers:** `build_launch_plan()` decides whether to consume `agent.md` or patch native config.

### 5.2 Vault-Aware Config Schema (Option A — Structured Array)

```toml
[[profiles]]
name = "web-app-team"
provider_id = "opencode"

[[profiles.skills]]
name = "rust-patterns"
vault = "clawhub"

[[profiles.skills]]
name = "docker"
vault = "ecc"
```

- Old format `skills = ["rust-patterns"]` deserializes to `vault = "auto"` (runtime resolution).
- At `agk p start`, "auto" vaults are resolved by scanning all attached vaults for the skill name. If ambiguous, warn and pick first.

### 5.3 MCP & Profile Vault Asset Formats

```markdown
---
# mcps/filesystem/MCP.md
name: filesystem
version: 1.0.0
command: npx
args: ["-y", "@modelcontextprotocol/server-filesystem", "."]
transport: stdio
---

# profiles/web-app-team/PROFILE.md
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
```

---

## 6. Acceptance Criteria

### Must-Have Gate

- [x] Wizard generates structured markdown from 6–8 structured prompts (not raw Q&A).
- [x] At least 5 archetype templates pre-fill wizard answers (6 templates: Code Reviewer, Feature Implementer, Security Auditor, Documentation Writer, Test Generator, Custom).
- [x] Review step shows scrollable composed markdown + estimated token count.
- [x] Profile skills/MCPs stored with vault provenance in `config.toml` (via `ProfileAssetRef` with backward-compatible serde).
- [x] `agk p start <profile>` auto-installs missing skills/MCPs from specified vaults (with rollback on failure).
- [x] `mcps/` directory in vault is scanned; MCPs appear in TUI MCP tab with `[⊘]`/`[ ]`/`[x]` states.
- [x] `profiles/` directory in vault is scanned; profiles appear in TUI Profile tab.
- [x] Installing a vault profile installs all referenced skills, instructions, and MCPs (atomic with rollback).
- [x] Copilot CLI (`GithubProvider`, id: `github-copilot`), Gemini CLI, and AMP support MCP register/enable/disable.
- [x] Old flat-string `skills = ["name"]` profiles continue to work (backward compatibility via `vault: "auto"` serde default).
- [x] Architecture tests pass with zero allowlists (14/14 pass). `cargo clippy` and `cargo fmt` clean.
- [ ] `cargo test` passes including new integration tests for wizard + vault scanning (some Phase 5 integration tests pending).

### Should-Have Gate

- [x] Provider tool checklist appears in wizard when provider advertises options (`ToolSelect` and `PermissionSelect` wizard steps).
- [x] F3 Editor supports raw markdown editing with live token updates (token badge added in v0.3.2 via F25).
- [x] Claude Code provider writes `.agk/profiles/<name>/agent.md` with frontmatter (YAML frontmatter + composed body, with `prompt_overlay_path` fallback).
- [x] Batch profile installation is atomic (all referenced assets or none, with rollback on failure).

---

## 7. Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| **Config schema migration breaks existing users** | Custom serde accepts both old and new formats; writes new format only on mutation |
| **More wizard steps = user fatigue** | Templates pre-fill 80% of fields; "Custom" is the only long path |
| **Token estimation inaccurate** | Label it "Est."; actual provider counts may differ |
| **MCP registry collision (vault vs manual)** | Vault-sourced MCPs prefixed with `vault_id/` or warning modal |
| **Provider tool/permission divergence** | `available_profile_tools()` is runtime query; wizard auto-adapts |
| **Claude Code `agent.md` format changes** | AGK owns canonical body; frontmatter is provider adapter concern |

---

## 8. Success Metrics

| Metric | Baseline | Target |
|---|---|---|
| Profile creation time (template path) | ~3 min (raw Q&A + manual skill/MCP selection) | < 45 sec |
| Team onboarding time (vault → running profile) | ~10 min (manual everything) | < 30 sec |
| Wizard token count (average profile) | Unknown (raw Q&A) | 300–800 tokens |
| MCP provider coverage | 2/9 (Claude Code, OpenCode) | 5/9 (+Copilot, Gemini, AMP) |
| Vault asset type coverage | 2/4 (Skill, Instruction) | 4/4 (+MCP, Profile) |

---

## 9. Why This Epic, Why Now?

1. **Profiles are the capstone feature** — They connect vaults, assets, MCPs, and providers into a single user action (`agk p start`). The current implementation is the weakest link.
2. **Team distribution is the killer use case** — Solo users can manage skills manually. Teams cannot. Vault-discoverable profiles + MCPs solve this.
3. **Builds on completed foundation** — ADR-001 architecture, P8 testing, and P9 observability give us the structural confidence to ship complex multi-file features.
4. **Defensible differentiation** — No other tool combines cross-provider skill sync with team-ready profile blueprints. Coder does governance; AGK does portability.
5. **Right-sized scope** — 7 weeks, ~15 features, clear boundaries. Excludes visionary but unproven harness orchestration and heavy enterprise governance.

---

## 10. Related Documents

- Source Proposals (implemented — removed from `docs/proposals/`):
  - ~~P10: Profile Wizard Enhancement~~ (implemented in v0.3)
  - ~~Vault Multi-Asset Scanning~~ (implemented in v0.3)
  - ~~P6: MCP Provider Expansion~~ (implemented in v0.3)
  - ~~ADR-001: Unified Core~~ (implemented in v0.2)
- Source Proposals (still in `docs/proposals/`):
  - [P7: Enterprise Feature Pack](../../proposals/enterprise-feature-pack.md)
  - [AGK vs. Coder Research](../../proposals/agk-vs-coder-research.md)
  - [VibeCode Research](../../proposals/research-vibecode-agk-report.md)
- Release Plan:
  - [`../v03-team-ready-profiles.md`](../v03-team-ready-profiles.md)

---

*End of Epic Proposal — updated 2026-06-01 to reflect implementation status*
