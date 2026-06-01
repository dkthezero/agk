# Release Plan: AGK v0.3.2 — "Ship Polish"

**Status:** Implemented — merged to master
**Target Date:** 1–2 weeks from kickoff
**Epic:** Clean up remaining gaps from v0.3 + v0.3.1 before official release tag

---

## 1. Release Overview

v0.3 and v0.3.1 are feature-complete and merged to master (413 tests passing, architecture tests clean). This release closes the remaining polish gaps: config migration, live token count in the editor, profile diff, documentation, and manual QA. No new features — only making what exists shippable.

**Theme:** *Every checkbox on the v0.3 and v0.3.1 acceptance criteria turns green.*

---

## 2. What's Left (Gap Analysis)

### From v0.3 — Phase 5 (Polish)

| # | Item | Current State | Action Needed |
|---|------|---------------|---------------|
| 1 | Config migration: old flat format → structured on first write | `ProfileAssetRef` deserializer handles old strings → `vault: "auto"`, but no explicit write-back migration | Add write-migration: on first `config.save()` with old-format profiles, rewrite to `ProfileAssetRef` array format |
| 2 | TUI launch simulation overlay | Not implemented | Defer — this is a UI polish item, not a ship blocker. Move to v0.4 backlog. |
| 3 | F3 Editor live token count | Token estimation utility exists but F3 editor modal doesn't display it | Add token badge to `edit_profile_modal.rs` |
| 4 | Manual QA checklist | Not done | Execute checklist: vault attach, profile install, provider toggle, MCP register, profile start, GHES attach, export/import |
| 5 | User docs update | Not done | Update `docs/product/` to reflect v0.3 + v0.3.1 features |

### From v0.3.1 — Phase 4 (Polish)

| # | Item | Current State | Action Needed |
|---|------|---------------|---------------|
| 6 | Manual QA: GHES attach, export/import, security badges | Not done | Combine with item 4 above into single QA pass |
| 7 | Documentation for v0.3.1 features | Not done | Combine with item 5 above |

### From v0.3.1 Could-Have

| # | Item | Current State | Action Needed |
|---|------|---------------|---------------|
| 8 | Profile Diff (vs Vault) | Not implemented | New feature: compare local profile config vs vault source, show drift |

---

## 3. Phase Breakdown

### Phase 1: Config Migration + Token Count — Days 1–3
**Goal:** Close the two code gaps that affect existing users.

| Work Item | Owner | LOE | Dependencies |
|---|---|---|---|
| Add write-migration for old flat `skills = ["name"]` → `ProfileAssetRef` array on config save | Backend | 2d | — |
| Add live token count badge to F3 profile editor modal | Frontend | 1d | `token_estimate.rs` (exists) |
| Unit tests for migration (old config in → new config out) | QA | 1d | Migration logic |

**Phase 1 Exit Criteria:**
- [x] Loading an old-format `config.toml` with `skills = ["name"]` and saving it writes the new `[[profiles.skills]]` array format.
- [x] F3 Editor shows estimated token count badge that updates when skills/MCPs change.
- [x] `cargo test` passes; architecture tests pass.

---

### Phase 2: Profile Diff — Days 3–5
**Goal:** Let users see when their local profile has drifted from the vault source.

| Work Item | Owner | LOE | Dependencies |
|---|---|---|---|
| `ProfileDiff` domain model: compare local profile vs vault-discovered profile | Backend | 2d | `ProfileAssetRef` (exists) |
| CLI: `agk profile diff <name>` shows added/removed skills, MCPs, tools | Frontend | 1d | Diff model |
| TUI: Profile detail view shows drift indicators (`[⇄]` badge when drifted) | Frontend | 1d | Diff model |
| Unit tests for diff computation | QA | 1d | Diff logic |

**Phase 2 Exit Criteria:**
- [x] `agk profile diff <name>` shows which skills/MCPs/tools differ between local and vault version.
- [x] TUI shows drift badge on profiles that differ from their vault source.
- [x] `cargo test` passes.

---

### Phase 3: QA + Docs — Days 5–10
**Goal:** Manual QA, documentation, and release readiness.

| Work Item | Owner | LOE | Dependencies |
|---|------|---|---|
| Manual QA: vault attach → profile install → start flow | QA | 1d | All prior |
| Manual QA: GHES vault attach, profile export/import, MCP security | QA | 1d | All prior |
| Manual QA: provider toggle, MCP register/enable/disable roundtrip | QA | 1d | All prior |
| Update `docs/product/` PRDs to mark v0.3/v0.3.1 features as implemented | Docs | 2d | All prior |
| Update `docs/SUPPORT.md` with new CLI commands | Docs | 1d | All prior |
| Version bump to v0.3.2 | Backend | 1d | All prior |

**Phase 3 Exit Criteria:**
- [ ] Manual QA checklist complete (all flows verified).
- [x] All product PRDs updated to reflect current implementation status.
- [x] `cargo test` passes (unit + integration).
- [x] `cargo test --test architecture -- --ignored` passes with zero allowlists.
- [x] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- [x] `cargo fmt --check` passes.

---

## 4. Deferred to v0.4

These items from the original plans are explicitly deferred — they're nice-to-have but not ship blockers:

| Item | Why Deferred |
|---|---|
| TUI launch simulation overlay | UI polish; no user requests; high LOE for low impact |
| `agk auth switch` multi-host support | GHES multi-host is edge case; current single-host works |
| Stale-skill telemetry report | Not requested; can be added as telemetry extension later |
| Skill signing / GPG provenance | Depends on enterprise policy engine (P7) |

---

## 5. Cross-Cutting Concerns

### Architecture Integrity
- No `.rs` file > 300 lines of non-test logic.
- `domain/` remains pure — diff model is domain-only, no I/O.
- Architecture tests pass with zero allowlists.

### Backward Compatibility
- Config migration writes new format on save only — old configs load unchanged.
- Profile diff is read-only — never modifies local or vault state.
- Token count in editor is advisory ("Est.") — never blocks save.

---

## 6. Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| Config migration corrupts user profiles | High | `#[serde(default)]` on all new fields; dry-run migration mode; backup old config before write |
| Token estimation is inaccurate in editor | Low | Label "Est."; not a hard limit |
| Profile diff false positives (hash mismatch) | Medium | Compare by identity name + vault, not by hash; show "may differ" not "definitely drifted" |

---

## 7. Milestones

| Milestone | Date | Deliverable |
|---|---|---|
| M1: Code Gaps Closed | Day 3 | Config migration + live token count |
| M2: Profile Diff | Day 5 | `agk profile diff` + TUI drift badge |
| M3: Release Ready | Day 10 | QA passed; docs updated; version bumped |

---

*Release Plan v0.3.2 — 2026-06-01*