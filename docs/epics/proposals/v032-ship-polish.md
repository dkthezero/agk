# Epic Proposal: AGK v0.3.2 — "Ship Polish"

**Status:** Implemented
**Target Release:** v0.3.2
**Theme:** *Close every open checkbox before the official release tag*
**Date:** 2026-06-01

---

## 1. Situation Assessment

### What's Shipped (v0.3 + v0.3.1)

| Capability | State |
|---|---|
| Vault-discoverable MCP servers + profiles | ✅ Shipped |
| Structured profile wizard with 6 archetype templates | ✅ Shipped |
| Vault-aware dependency storage + auto-install with rollback | ✅ Shipped |
| MCP provider expansion (5 providers: Claude Code, OpenCode, Copilot, Gemini, AMP) | ✅ Shipped |
| Token estimation + live preview in wizard review | ✅ Shipped |
| GHES vault adapter with SSO token resolution | ✅ Shipped |
| Profile export/import (CLI + TUI) | ✅ Shipped |
| Parallel vault scanning (rayon) | ✅ Shipped |
| Template + profile launch telemetry | ✅ Shipped |
| MCP security scorecard with heuristic flags | ✅ Shipped |
| Telemetry CSV export | ✅ Shipped |

### What's Still Open

1. **Config migration doesn't write back** — Old `skills = ["name"]` deserializes correctly (vault: "auto"), but saving the config doesn't upgrade the format. Users who never re-save keep the old format forever.
2. **F3 Editor missing live token count** — Token estimation works in the wizard review step, but the F3 profile editor modal doesn't show a token badge.
3. **No profile diff** — Users can't see if their local profile has drifted from the vault source. The only way to compare is manual inspection.
4. **No manual QA** — All code is tested via unit/integration tests (413 passing), but nobody has manually walked through the full TUI flow.
5. **Documentation stale** — Product PRDs still show "Draft" status for implemented features. Support docs don't list new CLI commands.

---

## 2. Feature List

### 🔴 Must-Have (P0) — Ship Blockers

| ID | Feature | Problem Solved | LOE |
|---|---------|---------------|-----|
| **F24** | **Config Write-Migration** | Old flat-string profiles never auto-upgrade to structured format | Low |
| **F25** | **F3 Editor Token Badge** | Users can't see profile token cost while editing | Low |
| **F26** | **Profile Diff (vs Vault)** | No way to detect local-vs-vault profile drift | Medium |

### 🟡 Should-Have (P1)

| ID | Feature | Problem Solved | LOE |
|---|---------|---------------|-----|
| **F27** | **Manual QA Checklist** | No human-verified end-to-end flow | Low |
| **F28** | **PRD Status Updates** | Product docs don't reflect shipped features | Low |

### 🔵 Will-Not-Do (Explicitly Deferred)

| Feature | Why | When |
|---|---|---|
| TUI launch simulation overlay | High LOE, low impact, no user requests | v0.4 |
| `gh auth switch` multi-host | Edge case; single GHES host works fine | v0.3.x patch |
| Stale-skill telemetry report | Not requested; easy telemetry extension later | v0.4 |
| Skill signing / GPG provenance | Depends on enterprise policy engine (P7) | After P7 |

---

## 3. Architecture

### F24: Config Write-Migration

On `ConfigStorePort::save()`, scan `profiles` for any entry where `skill_refs` is empty but a legacy `skills: Vec<String>` field exists. Convert each string to `ProfileAssetRef { name, vault: "auto" }` and write the structured array format.

```toml
# Before (old format — still loads):
[[profiles]]
name = "web-app"
skills = ["rust-patterns", "docker"]

# After (new format — written on first save):
[[profiles]]
name = "web-app"

[[profiles.skills]]
name = "rust-patterns"
vault = "auto"

[[profiles.skills]]
name = "docker"
vault = "auto"
```

### F25: F3 Editor Token Badge

Reuse `token_estimate::estimate_tokens()` from the wizard. In `edit_profile_modal.rs`, compute token count from the profile's composed description and display a color badge (green/yellow/red) in the modal header.

### F26: Profile Diff

New domain model:
```rust
pub struct ProfileDiff {
    pub local_skills: Vec<ProfileAssetRef>,
    pub vault_skills: Vec<ProfileAssetRef>,
    pub added_skills: Vec<ProfileAssetRef>,   // in local, not in vault
    pub removed_skills: Vec<ProfileAssetRef>, // in vault, not in local
    // Same for mcps, tools, permission_mode
}
```

Compare by `name` (ignoring vault ID — vault resolution is runtime). TUI shows `[⇄]` badge on drifted profiles. CLI `agk profile diff <name>` prints a diff summary.

---

## 4. Acceptance Criteria

### Must-Have Gate
- [x] Saving a config that loaded old flat-string `skills = ["name"]` writes the new `ProfileAssetRef` array format.
- [x] F3 Editor modal shows estimated token count badge with green/yellow/red coloring.
- [x] `agk profile diff <name>` shows which skills/MCPs/tools differ between local and vault.
- [x] TUI shows drift badge `[⇄]` on profiles that differ from their vault source.
- [x] `cargo test` passes; architecture tests pass with zero allowlists.

### Should-Have Gate
- [ ] Manual QA checklist complete for all v0.3 + v0.3.1 flows.
- [x] All `docs/product/features/*/prd.md` files updated to reflect implementation status.

---

## 5. Related Documents

- Parent releases:
  - [v0.3 Team-Ready Profiles](../v03-team-ready-profiles.md)
  - [v0.3.1 Enterprise Bridge](../v031-enterprise-bridge.md)
- Source proposals (still in `docs/proposals/`):
  - [P7: Enterprise Feature Pack](../../proposals/enterprise-feature-pack.md)
  - [AGK vs. Coder Research](../../proposals/agk-vs-coder-research.md)

---

*End of Epic Proposal — 2026-06-01*