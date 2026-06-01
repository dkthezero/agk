# Release Plan: AGK v0.3.1 — "Enterprise Bridge & Profile Portability"

**Status:** Implemented — merged to master via #27
**Target Date:** 3–4 weeks from kickoff
**Epic:** [`proposals/v031-enterprise-bridge.md`](proposals/v031-enterprise-bridge.md)
**Parent Epic:** [`v03-team-ready-profiles.md`](v03-team-ready-profiles.md)

---

## 1. Release Overview

This release is a **fast-follow bridge** between v0.3 (team-ready profiles) and v0.4 (enterprise pack / harness orchestrator). It delivers the highest-value, lowest-effort features that connect AGK to enterprise infrastructure, make profiles portable across machines, improve performance for growing vaults, and add team insights through telemetry.

**Theme:** *A developer at a GHES-backed company attaches the corporate vault, exports a profile to a contractor, and knows which team templates are actually being used.*

---

## 2. Phase Breakdown

### Phase 1: Enterprise Connectivity — Week 1
**Goal:** Enable GHES vault attachment for enterprise customers.

| Work Item | Owner | PRD | LOE | Dependencies |
|---|---|---|---|---|
| Add `enterprise_url` to `VaultConfig` domain model | Backend | [GHES Vault](../../product/features/ghes-vault/prd.md) §Config Schema | 1d | — |
| Update `GithubVaultAdapter` for GHES API base URL | Backend | [GHES Vault](../../product/features/ghes-vault/prd.md) §Adapter | 2d | Config model |
| SSO token resolution: `gh auth` → `GITHUB_TOKEN` → `GITHUB_ENTERPRISE_TOKEN` | Backend | [GHES Vault](../../product/features/ghes-vault/prd.md) §Token Resolution | 1d | Adapter |
| Private repo clone support (same as public, scoped token) | Backend | [GHES Vault](../../product/features/ghes-vault/prd.md) §Private Repos | 1d | Adapter |
| CLI: `agk vault attach` accepts GHES URLs | Frontend | [GHES Vault](../../product/features/ghes-vault/prd.md) §CLI | 1d | — |
| TUI: Vault tab shows `enterprise_url` in detail view | Frontend | [GHES Vault](../../product/features/ghes-vault/prd.md) §TUI | 1d | — |
| Integration test: GHES mock server vault scan | QA | [GHES Vault](../../product/features/ghes-vault/prd.md) §Tests | 2d | Adapter |

**Phase 1 Exit Criteria:**
- [x] `cargo test` passes; architecture tests pass.
- [x] Integration test proves GHES vault scanning against a mock API.
- [ ] Manual QA: attach GHES vault, list skills, install skill.

---

### Phase 2: Profile Portability — Week 1–2
**Goal:** Enable profile export/import for cross-machine sharing.

| Work Item | Owner | PRD | LOE | Dependencies |
|---|---|---|---|---|
| `ExportProfile` domain model + JSON schema | Backend | [Profile Portability](../../product/features/profile-portability/prd.md) §Export Format | 1d | v0.3 profile model |
| `ExportProfile` use case (`app/features/profile/export.rs`) | Backend | [Profile Portability](../../product/features/profile-portability/prd.md) §Export | 2d | Export model |
| `ImportProfile` use case (`app/features/profile/import.rs`) | Backend | [Profile Portability](../../product/features/profile-portability/prd.md) §Import | 2d | Export model |
| CLI: `agk profile export <name> --file <path>` | Frontend | [Profile Portability](../../product/features/profile-portability/prd.md) §CLI | 1d | Export use case |
| CLI: `agk profile import <path>` | Frontend | [Profile Portability](../../product/features/profile-portability/prd.md) §CLI | 1d | Import use case |
| TUI: `Ctrl+E` export modal (scope + file picker) | Frontend | [Profile Portability](../../product/features/profile-portability/prd.md) §TUI | 2d | Export use case |
| TUI: `Ctrl+I` import modal (file picker + preview) | Frontend | [Profile Portability](../../product/features/profile-portability/prd.md) §TUI | 2d | Import use case |
| Integration test: export → import roundtrip | QA | [Profile Portability](../../product/features/profile-portability/prd.md) §Tests | 1d | Both use cases |

**Phase 2 Exit Criteria:**
- [x] Export produces valid JSON that validates against schema.
- [x] Import creates profile entry + writes `agent.md`.
- [x] Roundtrip test: export profile A → import as profile B → assert same config.

---

### Phase 3: Performance & Insights — Week 2–3
**Goal:** Speed up vault scanning and add template/profile usage insights.

| Work Item | Owner | PRD | LOE | Dependencies |
|---|---|---|---|---|
| Add `rayon` dependency + parallel scan loop | Backend | [Vaults](../../product/features/vaults/prd.md) §Parallel Scanning | 1d | — |
| Convert `scan.rs` to parallel feature-set iteration | Backend | [Vaults](../../product/features/vaults/prd.md) §Parallel Scanning | 2d | rayon |
| Benchmark: before/after timing test | QA | [Vaults](../../product/features/vaults/prd.md) §Performance | 1d | Parallel scan |
| Extend `analytics.toml` schema for templates + profiles | Backend | [Telemetry](../../product/features/telemetry/prd.md) §Schema | 1d | — |
| Track template selections in wizard controller | Frontend | [Telemetry](../../product/features/telemetry/prd.md) §Template Tracking | 1d | Schema |
| Track profile launches in `start_profile_session` | Backend | [Telemetry](../../product/features/telemetry/prd.md) §Profile Tracking | 1d | Schema |
| TUI Telemetry tab: Templates section | Frontend | [Telemetry](../../product/features/telemetry/prd.md) §TUI | 2d | Tracking |
| TUI Telemetry tab: Profiles section | Frontend | [Telemetry](../../product/features/telemetry/prd.md) §TUI | 2d | Tracking |
| Background scanner: parse template + profile events | Backend | [Telemetry](../../product/features/telemetry/prd.md) §Background | 1d | Tracking |

**Phase 3 Exit Criteria:**
- [x] Parallel scan reduces sync time by ≥ 50% on 4+ directory vaults.
- [x] Telemetry tab shows template and profile usage data.
- [x] Background scanner writes new fields without corrupting old `analytics.toml`.

---

### Phase 4: Security & Polish — Week 3–4
**Goal:** Add MCP security warnings and ship-quality polish.

| Work Item | Owner | PRD | LOE | Dependencies |
|---|---|---|---|---|
| MCP command heuristic parser (`security_flags.rs`) | Backend | [MCP Vault](../../product/features/mcp-vault/prd.md) §Security | 2d | — |
| `McpSecurityScore` domain model + flag definitions | Backend | [MCP Vault](../../product/features/mcp-vault/prd.md) §Security | 1d | Parser |
| TUI MCP tab: `[!]` badge + detail risk flags | Frontend | [MCP Vault](../../product/features/mcp-vault/prd.md) §Security | 2d | Score model |
| CLI `agk mcp list --json`: include `security_flags` | Frontend | [MCP Vault](../../product/features/mcp-vault/prd.md) §Security | 1d | Score model |
| Telemetry CSV export command | Backend | [Telemetry](../../product/features/telemetry/prd.md) §CSV Export | 1d | Telemetry data |
| Update user docs (`docs/product/`) | Docs | — | 1d | All |
| Full integration test suite + manual QA | QA | — | 2d | All |

**Phase 4 Exit Criteria:**
- [x] All tests pass (`cargo test`, architecture tests, clippy, fmt).
- [ ] Manual QA checklist: GHES attach, profile export/import, sync performance, telemetry display, MCP security badges.
- [ ] Documentation updated for all new features.

---

## 3. Cross-Cutting Concerns

### Architecture Integrity
- No `.rs` file > 300 lines.
- `domain/` remains pure (no I/O).
- New features use existing `CoreCommand` / `CoreEvent` bus.
- Architecture tests pass with zero allowlists.

### Testing Strategy
- **Unit:** Token resolution order, security heuristic parser, JSON serialization.
- **Contract:** CLI `--json` output shape for `profile export`, `mcp list --json`.
- **Integration:** Full TUI export/import flow via `TestBackend`.
- **Process:** GHES mock server tests; parallel scan timing benchmarks.

### Backward Compatibility
- Old `analytics.toml` without template/profile fields deserializes via `serde(default)`.
- Old `VaultConfig` without `enterprise_url` continues to work (github.com default).
- Profile export JSON includes `agk_version` for future compatibility checks.

---

## 4. Risk Register

| Risk | Phase | Impact | Mitigation | Owner |
|---|---|---|---|---|
| **GHES API rate limits differ from github.com** | 1 | Medium | Respect `Retry-After` headers; same backoff as existing GitHub adapter | Backend |
| **Profile export JSON schema changes in v0.4** | 2 | Low | Version field in JSON; import warns but doesn't block on minor version mismatch | Backend |
| **Parallel scan causes filesystem contention** | 3 | Low | Read-only scans; `rayon` thread pool limits concurrent I/O; fallback to serial if needed | Backend |
| **Telemetry tracking adds TUI event lag** | 3 | Low | Telemetry events are batched and written asynchronously; never block render loop | Frontend |
| **MCP security heuristics false-positive on safe tools** | 4 | Low | Advisory only (badges); never block installation | Backend |

---

## 5. PRD Index

| Feature Area | PRD | Technical Design | Covers Features |
|---|---|---|---|
| GHES Vault (new) | [`product/features/ghes-vault/prd.md`](../../product/features/ghes-vault/prd.md) | [`technical_design.md`](../../product/features/ghes-vault/technical_design.md) | F16 |
| Profile Portability (new) | [`product/features/profile-portability/prd.md`](../../product/features/profile-portability/prd.md) | [`technical_design.md`](../../product/features/profile-portability/technical_design.md) | F17 |
| Vaults (updated) | [`product/features/vaults/prd.md`](../../product/features/vaults/prd.md) | existing | F18 |
| Telemetry (updated) | [`product/features/telemetry/prd.md`](../../product/features/telemetry/prd.md) | existing | F19, F21, F22 |
| MCP Vault (updated) | [`product/features/mcp-vault/prd.md`](../../product/features/mcp-vault/prd.md) | existing | F20 |

---

## 6. Milestones

| Milestone | Date | Deliverable |
|---|---|---|
| M1: GHES Support | End of Week 1 | GHES vault attach, list, install working |
| M2: Profile Portability | End of Week 2 | Export/import roundtrip tested in TUI + CLI |
| M3: Performance & Insights | End of Week 3 | Parallel scan benchmarked; Telemetry tab shows templates + profiles |
| M4: v0.3.1 Release Ready | End of Week 4 | MCP security badges; all tests green; docs updated |

---

## 7. Post-Release Fast Follows (v0.3.2)

- Profile diff (compare local profile vs vault source)
- Telemetry: stale-skill report (skills not invoked in 90 days)
- GHES: support for `gh auth switch` multiple enterprise hosts

---

*Release Plan v0.1 — 2026-05-30*
