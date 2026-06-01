# Epic Proposal: AGK v0.3.1 — "Enterprise Bridge & Profile Portability"

**Status:** Ready for planning — v0.3 Phases 1–4 complete
**Target Release:** v0.3.1
**Theme:** *Connect team-ready profiles to enterprise infrastructure and make them portable across environments*
**Author:** Technical Product Owner (Claude synthesis)
**Date:** 2026-05-30

---

## 1. Situation Assessment

### What's Shipping in v0.3

| Capability | State |
|---|---|
| Vault-discoverable MCP servers + profiles | ✅ Shipped |
| Structured profile wizard with archetype templates | ✅ Shipped |
| Vault-aware dependency storage + auto-install | ✅ Shipped |
| MCP provider expansion (5 providers) | ✅ Shipped |
| Token estimation + live preview | ✅ Shipped |

### What v0.3 Leaves Behind

v0.3 makes profiles **team-ready** within a single workspace, but several gaps remain:

1. **No cross-machine profile portability** — A developer cannot export their carefully-tuned profile and share it with a teammate on another machine. The only sharing mechanism is vault distribution, which requires the team to maintain a git repo.
2. **No GitHub Enterprise Server (GHES) support** — Companies running private GitHub instances cannot attach their internal skill vaults. AGK only supports github.com.
3. **Vault scanning is serial** — As vaults grow to include `mcps/`, `profiles/`, `skills/`, and `instructions/`, refresh times increase linearly. No parallel scanning.
4. **No insights into template or profile usage** — Teams don't know which wizard templates are most used, which profiles are actively launched, or which skills in a profile are dead weight.
5. **No security warnings for high-risk MCPs** — An MCP server that requests broad filesystem access or network egress gets the same security modal as a read-only MCP. No differentiation.

### Strategic Context

v0.3.1 is a **fast-follow bridge release** between v0.3 (team-ready) and v0.4 (enterprise pack / harness orchestrator). It delivers the highest-value, lowest-effort features that:
- Connect AGK to enterprise infrastructure (GHES)
- Improve day-to-day operability (export/import, performance, insights)
- Lay groundwork for the full Enterprise Pack ([P7](../../proposals/enterprise-feature-pack.md))

---

## 2. Epic Narrative

> Sarah is a platform engineer at Acme Corp. Her team just adopted AGK v0.3 and created a `web-app-team` profile in their vault. > 
> **The problem:** Sarah's company runs GHES, not github.com. She can't attach the corporate skill vault. > She also wants to export the `web-app-team` profile as a JSON bundle to share with contractors who don't have vault access. > And she notices `agk sync` takes 8 seconds now that the vault has 40 skills + 12 MCPs + 5 profiles.
> 
> **v0.3.1 solves this:** Sarah attaches `github.acme.internal/acme-org/ai-workflows` via the GHES adapter. > She runs `agk profile export web-app-team --file ./web-app-team.agk.json` and sends it to a contractor, who runs `agk profile import ./web-app-team.agk.json`. > `agk sync` now finishes in 2 seconds thanks to parallel directory scanning. > And the Telemetry tab shows her that 80% of the team uses the "Feature Implementer" template, while the "Documentation Writer" template has zero usage — so she removes it from the team vault.

---

## 3. Feature List (Prioritized)

### 🔴 Must-Have (P0) — Ship Blockers

| ID | Feature | Source | Problem Solved | LOE |
|---|---|---|---|---|
| **F16** | **GHES Vault Adapter** | [P7](../../proposals/enterprise-feature-pack.md) §5 | Companies with private GitHub instances cannot attach internal vaults | Low |
| **F17** | **Profile Export/Import** | Profiles PRD out-of-scope | No way to share a profile outside of a vault | Low |
| **F18** | **Parallel Vault Scanning** | v0.3 release plan fast-follow | `agk sync` slows linearly as vaults grow | Low |
| **F19** | **Template Usage Telemetry** | v0.3 release plan fast-follow | Teams don't know which templates/profiles are used | Low |

### 🟡 Should-Have (P1) — High Value, Can Slip

| ID | Feature | Source | Problem Solved | LOE |
|---|---|---|---|---|
| **F20** | **MCP Security Scorecard** | [Coder Research](../../proposals/agk-vs-coder-research.md) §9.2 #4 | High-risk MCPs (broad filesystem, network) get the same warning as read-only MCPs | Low |
| **F21** | **Profile Launch Telemetry** | Telemetry PRD extension | No insight into which profiles are actively launched, how often, by whom | Low |

### 🟢 Could-Have (P2) — Nice to Have

| ID | Feature | Source | Problem Solved | LOE |
|---|---|---|---|---|
| **F22** | **Telemetry CSV Export** | [P7](../../proposals/enterprise-feature-pack.md) §4 | Managers need a shareable report of skill/profile usage | Low |
| **F23** | **Profile Diff (vs Vault)** | Profiles PRD extension | No way to see if a local profile has drifted from its vault source | Medium |

### 🔵 Will-Not-Do (Explicitly Out of Scope)

| Feature | Why Excluded | When |
|---|---|---|
| Enterprise Policy Engine (full P7) | Large governance feature; needs its own epic | v0.4 "AGK Enterprise" |
| Team Config Sync / `.agk/team.toml` | Depends on policy engine to avoid config drift | After P7 |
| Skill Signing / GPG | Depends on policy engine infrastructure | After P7 |
| Coder Provider Adapter | Strategic integration; requires Tailnet research | Future partnership |
| Harness Orchestrator (RIPER-5) | Visionary; needs format research + community validation | v0.4+ |
| Real-time MCP health monitoring | Provider responsibility, not AGK's scope | Never |

---

## 4. Architecture & Sequencing

### Release Phases (3–4 weeks)

**Phase 1: Enterprise Connectivity (Week 1)**
- F16: GHES vault adapter
  - Add `enterprise_url` field to `VaultConfig`
  - Update `GithubVaultAdapter` to use GHES API base URL when present
  - SSO token pass-through via `gh auth` or `GITHUB_TOKEN`
  - Private repo support (same flow as public, assuming token has access)

**Phase 2: Profile Portability (Week 1–2)**
- F17: Profile export/import
  - `ExportProfile` command serializes profile + referenced skill names + MCP names to JSON
  - `ImportProfile` command deserializes, resolves vaults, installs missing assets
  - TUI: `Ctrl+E` export, `Ctrl+I` import modal
  - CLI: `agk profile export <name> --file <path>`, `agk profile import <path>`

**Phase 3: Performance & Insights (Week 2–3)**
- F18: Parallel vault scanning
  - Convert `scan.rs` vault loop to `rayon` or `tokio::spawn` parallel iteration
  - Feature sets scan independently; results merged
  - Benchmark: 4-directory vault scans in <1s vs current 3-4s
- F19: Template usage telemetry
  - Extend `analytics.toml` to track template selections, profile launches, wizard completions
  - TUI Telemetry tab new columns: Template, Selections, Profile Launches
  - Background scanner already exists; extend parsers

**Phase 4: Security & Polish (Week 3–4)**
- F20: MCP Security Scorecard
  - Heuristic parser for MCP command + args: flags `broad-filesystem` (args contain `/`, `~`, `.`), `network-egress` (urls, `curl`, `wget`), `arbitrary-execution` (shell scripts)
  - TUI: Security badge `[!]` on high-risk MCPs; detail view shows risk flags
  - CLI: `agk mcp list --json` includes `security_flags: [...]`
- F21: Profile launch telemetry
  - Track `agk p start` invocations in analytics
  - TUI Telemetry tab: "Most Launched Profiles" section
- Integration tests + manual QA

---

## 5. Design Decisions

### 5.1 GHES Vault Config

```toml
[[vaults]]
id = "acme-private"
type = "github"
url = "https://github.acme.internal/acme-org/ai-workflows"
enterprise_url = "https://github.acme.internal"
# Token resolution order:
# 1. `gh auth token --hostname github.acme.internal` (if gh CLI installed)
# 2. `GITHUB_TOKEN` env var
# 3. `GITHUB_ENTERPRISE_TOKEN` env var
```

### 5.2 Profile Export Format

```json
{
  "agk_version": "0.3.1",
  "exported_at": "2026-06-15T10:00:00Z",
  "profile": {
    "name": "web-app-team",
    "provider_id": "opencode",
    "scope": "workspace",
    "structured_answers": {
      "role": "Senior full-stack engineer",
      "domain": "React + Node.js",
      "triggers": "After any component file change"
    },
    "skills": [
      { "name": "react-patterns", "vault": "clawhub" },
      { "name": "node-testing", "vault": "acme-private" }
    ],
    "mcps": [
      { "name": "filesystem", "vault": "auto" }
    ],
    "tools": ["Read", "Glob", "Grep"],
    "permission_mode": "acceptEdits",
    "agent_markdown": "# Identity\nYou are a..."
  }
}
```

**Import behavior:**
1. Validate `agk_version` compatibility (warn if > minor version ahead).
2. Create profile entry in config.
3. For each skill/MCP: resolve vault. If vault not attached, warn and set to `"auto"`.
4. Write `agent_markdown` to `.agk/profiles/<name>/agent.md`.
5. Does NOT auto-install dependencies — user runs `agk p start` for that.

### 5.3 Parallel Scanning Strategy

```rust
// app/bootstrap/scan.rs
use rayon::prelude::*;

pub fn scan(registry: &Registry, vaults: &[Box<dyn VaultPort>]) -> Result<ScanResult> {
    let packages_by_tab: Vec<_> = registry.feature_sets
        .par_iter() // ← parallel over feature sets
        .map(|feature| {
            let mut tab_packages = Vec::new();
            if !feature.is_stub() {
                for vault in vaults {
                    match vault.list_packages(feature.as_ref()) { ... }
                }
            }
            tab_packages
        })
        .collect();
    Ok(ScanResult { packages_by_tab })
}
```

- Uses `rayon` (already common in Rust ecosystem; no new async complexity).
- Thread-safe: `VaultPort` implementations must be `Send + Sync` (already required).
- Benchmark before/after with `cargo bench` or integration test timing.

### 5.4 Telemetry Schema Extension

```toml
# ~/.config/agk/analytics.toml
[settings]
enabled = true

# Existing: skill invocations
[skills."react-patterns"]
total_invocations = 42
last_used = "2026-05-01T14:32:00Z"
providers = ["claude-code"]

# NEW: template usage
[templates."feature-implementer"]
selections = 15
last_selected = "2026-05-10T09:00:00Z"

[templates."code-reviewer"]
selections = 8

# NEW: profile launches
[profiles."web-app-team"]
launches = 23
last_launched = "2026-05-12T16:45:00Z"
provider = "opencode"
```

### 5.5 MCP Security Scorecard Heuristics

| Flag | Trigger | Severity |
|------|---------|----------|
| `broad-filesystem` | Args contain `/`, `~`, `.` (root or cwd access) | High |
| `network-egress` | Command/args contain `http`, `curl`, `wget`, `fetch` | High |
| `arbitrary-execution` | Command is `bash`, `sh`, `python` with unverified script | Critical |
| `env-exfiltration` | Args reference env vars (`$HOME`, `$SSH_KEY`) | Medium |
| `unspecified-args` | No args provided (command may default to dangerous behavior) | Low |

---

## 6. Acceptance Criteria

### Must-Have Gate
- [ ] GHES private repo can be added as vault and listed successfully via `agk vault attach`.
- [ ] `gh auth` SSO token is respected for GHES vaults when `gh` CLI is installed.
- [ ] `GITHUB_TOKEN` / `GITHUB_ENTERPRISE_TOKEN` env vars work as fallback.
- [ ] `agk profile export <name> --file <path>` produces valid JSON with all profile data.
- [ ] `agk profile import <path>` creates profile config and writes `agent.md`.
- [ ] Parallel vault scanning reduces `agk sync` time by ≥ 50% on multi-directory vaults.
- [ ] Telemetry tracks template selections and profile launches.
- [ ] TUI Telemetry tab shows "Templates" and "Profiles" sections.
- [ ] Old `analytics.toml` without new fields deserializes correctly (backward compatible).
- [ ] `cargo test` passes; architecture tests pass with zero allowlists.

### Should-Have Gate
- [ ] MCP security flags appear in `agk mcp list --json`.
- [ ] TUI MCP tab shows `[!]` badge for high-risk MCPs.
- [ ] Telemetry CSV export command produces shareable report.

---

## 7. Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| **GHES API divergence from github.com** | Abstract API base URL in `GithubVaultAdapter`; test against GHES mock server |
| **Profile export contains sensitive agent markdown** | Document that `.agk.json` files may contain internal prompts; recommend secure sharing channels |
| **Parallel scanning introduces race conditions** | Use immutable vault adapters; `rayon` scope guarantees thread safety for read-only scans |
| **Telemetry tracking slows TUI** | Telemetry is already background-only; new fields add negligible overhead |
| **MCP security heuristics have false positives** | Heuristics are advisory (badges/warnings), not blocking. User can still install. |
| **GHES token scope issues** | Clear error messages: "Token lacks `repo` scope for GHES vault. Run `gh auth refresh --scopes repo`." |

---

## 8. Success Metrics

| Metric | Baseline | Target |
|---|---|---|
| GHES vault attach success rate | 0% (unsupported) | 100% for repos where token has access |
| Profile export → import roundtrip | Manual vault-only | < 2 minutes (export + send + import) |
| `agk sync` duration (40 assets) | ~8s (serial) | < 3s (parallel) |
| Telemetry coverage | Skills only | Skills + Templates + Profiles + MCPs |
| MCP security flagged | 0% | 100% of MCPs with broad filesystem/network access |

---

## 9. Why This Epic, Why Now?

1. **Natural v0.3 follow-up** — v0.3 makes profiles team-ready. v0.3.1 makes them *portable* and *enterprise-connectable*. These are the top user requests that didn't fit v0.3's scope.
2. **Smallest viable enterprise entry point** — GHES support is the #1 enterprise blocker. It's small (one adapter field + token resolution) but unlocks the entire enterprise segment.
3. **Performance is user trust** — As teams adopt v0.3 and their vaults grow, slow `agk sync` becomes a daily friction. Parallel scanning is a quick win with measurable impact.
4. **Data-informed product decisions** — Template usage telemetry tells us which wizard archetypes to invest in, which to deprecate, and whether the wizard is actually being used.
5. **Security differentiation** — MCP Security Scorecard is a genuine differentiator. No other MCP manager warns about command-level risks before registration.

---

## 10. Related Documents

- Source Proposals:
  - [P7: Enterprise Feature Pack](../../proposals/enterprise-feature-pack.md)
  - [AGK vs. Coder Research](../../proposals/agk-vs-coder-research.md)
- Parent Epic:
  - [v0.3 Team-Ready Profiles](../v03-team-ready-profiles.md)
- Release Plan (this epic, once approved):
  - [`../v031-enterprise-bridge.md`](../v031-enterprise-bridge.md)

---

*End of Epic Proposal*
