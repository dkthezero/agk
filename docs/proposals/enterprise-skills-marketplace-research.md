# Research Report & Proposal: Enterprise AI Agent Skills Marketplace

**Date:** 2026-06-02  
**Author:** Market Research — AI Agent Skills Ecosystem  
**Target:** `docs/proposals/`  
**Status:** Draft for review  

---

## 1. Executive Summary

The AI agent skills marketplace is undergoing rapid enterprise maturation in 2025–2026. What began as community-driven `SKILL.md` sharing has evolved into a full **software supply chain artifact** requiring governance, signing, semantic discovery, and runtime policy enforcement. Every major platform—Claude Code, GitHub Copilot, OpenCode, Databricks, JFrog, TrueFoundry, C3 AI—is building or shipping an enterprise-grade skills registry.

**The central insight:** Enterprises no longer want to manage skills as loose markdown files in GitHub repos. They want an **internal marketplace**—a curated, governed, searchable app store for agent intelligence—where platform teams control the catalog, contributors publish vetted skills, and developers consume trusted capabilities with one-click install.

AGK is uniquely positioned to become this **enterprise skill marketplace infrastructure**. It already has the package manager (vaults, profiles, SHA10 change detection), the multi-provider abstraction (Claude Code, OpenCode, Copilot, Gemini, etc.), and the enterprise governance foundation (policy, signing, team sync, telemetry). What it lacks is the **marketplace layer**—the three-sided platform that connects infrastructure teams, contributors, and users.

This report surveys the competitive landscape through the lens of those three parties, identifies convergence patterns, and proposes a **"ClawHub Enterprise"** feature set for AGK v0.4.x.

---

## 2. Market Landscape: The Three Parties

### 2.1 Party 1: Infrastructure — Setting Up the Internal Marketplace

**The Problem:** Platform teams and CISOs need to prevent "Shadow AI"—developers pulling unvetted skills from public marketplaces that may exfiltrate secrets or execute arbitrary code. They need a single system of record for all agent capabilities, with the same rigor as Docker image registries or npm private registries.

**Competitive Solutions:**

| Vendor | Product | Key Infrastructure Features |
|--------|---------|----------------------------|
| **JFrog** | [Agent Skills Registry](https://jfrog.com/ai-catalog/skills-registry/) | Cryptographic signing, versioning, access control, semantic search, blocks unvetted public skills, integrates with NVIDIA NeMoClaw |
| **TrueFoundry** | [Skills Registry](https://www.truefoundry.com/blog/introducing-skills-registry-reusable-agent-skills-for-production-ai-systems) | On-demand context loading (token optimization), sandboxed multi-file skill execution, GitOps/CI-CD integration, full auditability |
| **C3 AI** | [MCP Gateway](https://c3.ai/blog/how-c3-ai-scales-agent-expertise-not-just-agent-tools/) | Centralized publishing with curated defaults, dual discovery (human UI + agent semantic search), cross-agent compatibility (Cursor, Copilot, Claude Code, Codex), usage analytics |
| **HiMarket** (Higress Group) | [himarket](https://github.com/higress-group/himarket/) | Open-source enterprise AI platform; manages Models, MCP Servers, Agents, and Skills via standardized API product format; security control, observability, metering/billing, multi-version management, gray-scale releases |
| **Databricks** | [Unity Catalog + AI Gateway](https://www.databricks.com/blog/governing-ai-agents-scale-unity-catalog) | Delegated access (on-behalf-of tokens), runtime service policies (allow/deny/consent per tool call), cost intelligence with budget thresholds, unified agent traces + data governance |
| **SkillReg** | [Private Registry](https://skillreg.dev/) | SaaS/private registry for `SKILL.md` files; semantic versioning, scoped access control, security scanning, audit trails, environment variable management |
| **iFlytek** | [SkillHub](https://github.com/iflytek/skillhub/) | Open-source (Apache 2.0), self-hosted; RBAC & audit logs, team namespaces, semantic versioning with custom tags, pluggable storage (local/S3/MinIO) |
| **agentregistry** | [agentregistry](https://github.com/agentregistry-dev/agentregistry/) | Open-source unified catalog for npm/PyPI/Docker/OCI/HTTP endpoints; curation workflows for platform team review; pairs with agentgateway for auth + observability |

**Common Infrastructure Patterns:**

1. **Centralized registry** — Single source of truth for all skills, MCPs, and agents.
2. **Cryptographic signing** — Every skill is signed; unverified skills are blocked by default.
3. **Policy engine** — Runtime rules (allow/deny/ask) enforced at the gateway, not just at publish time.
4. **Semantic discovery** — Natural language search so both humans and autonomous agents can find capabilities.
5. **Audit trail** — Who installed what, when, and how many times it was invoked.
6. **Self-hosting / air-gapped** — Regulated industries demand on-premise deployment.
7. **Metering & cost control** — Token usage per skill, per team, per project; budget thresholds.

---

### 2.2 Party 2: Contributors — Creating and Publishing Skills

**The Problem:** Subject-matter experts (senior engineers, DevOps, security teams) have tribal knowledge that should be encoded as skills. But they face friction: no standard template, no validation pipeline, no feedback loop on whether the skill actually works, and no easy way to publish to an internal catalog.

**Competitive Solutions:**

| Platform | Contributor Experience |
|----------|----------------------|
| **Claude Code** | [`skill-creator` meta-skill](https://github.com/anthropics/skills/blob/main/skills/skill-creator/SKILL.md) guides authors; `SKILL.md` standard with YAML frontmatter + markdown body; [plugin marketplace](https://code.claude.com/docs/en/plugin-marketplaces) supports GitHub/GitLab/npm/monorepo subdirs; version pinning via git refs/SHAs |
| **OpenCode** | [Plugin-based skill bundling](https://github.com/anomalyco/opencode/issues/9010) via npm; skills load from `~/.config/opencode/skills/` or `.opencode/skills/`; [pattern-based permissions](https://open-code.ai/en/docs/skills) (`allow`/`deny`/`ask`) for governance |
| **Microsoft Fabric** | [Enterprise skill authoring guide](https://github.com/microsoft/skills-for-fabric/blob/main/docs/skill-authoring-guide.md) with strict naming conventions, mandatory "Update Check" notices, Must/Prefer/Avoid sections, quality checker scripts before PR submission |
| **Developer Toolkit** | [Team skill development guide](https://developertoolkit.ai/en/shared-workflows/skills-ecosystem/building-custom-skills/) advocates `rules/` directory for architecture/testing/deployment standards, `install.md` for post-install onboarding, updating skills in same PR as code changes |
| **AgentPowers** | Commercial marketplace for OpenCode; skills priced $5–$15; one-command MCP-based installation |
| **Yarmoluk / agentskills.io** | [17-chapter open standard](https://github.com/Yarmoluk/custom-skill-developer) with quality scoring rubrics (100-point), meta-skill routers, token efficiency strategies, 30-skill limit management |

**Common Contributor Patterns:**

1. **Template scaffolding** — Every skill starts from a template with required frontmatter (`name`, `description`, `version`, `dependencies`).
2. **Progressive disclosure** — Three-level loading: metadata → `SKILL.md` body → bundled resources (scripts, references) to manage context window.
3. **Validation pipeline** — Lint frontmatter, check trigger accuracy, test against benchmark cases before publishing.
4. **Versioning & pinning** — Semantic versions + git SHAs so consumers can lock to stable releases.
5. **Co-evolution with code** — Best practice is to update the skill in the same PR as the codebase it describes.
6. **Must/Prefer/Avoid** — Structured guidance sections so the AI knows what to enforce, not just what to do.
7. **Trigger optimization** — Descriptions are tuned using train/test eval sets so skills activate at the right time.

---

### 2.3 Party 3: Users — Discovering and Consuming Skills

**The Problem:** Developers need to find the right skill quickly, trust that it won't break their environment, install it with minimal friction, and keep it synchronized with their team's standards.

**Competitive Solutions:**

| Platform | User Experience |
|----------|-----------------|
| **Claude Code** | Tiered skill levels: **Enterprise** (managed settings, org-wide) > **Personal** (`~/.claude/skills/`) > **Project** (`.claude/skills/`) > **Plugin** (`/skills/` within plugin). Enterprise overrides personal. `strictKnownMarketplaces` policy restricts which marketplaces users can add. |
| **GitHub Copilot CLI** | [Enterprise-managed plugins](https://github.blog/changelog/2026-05-06-enterprise-managed-plugins-in-github-copilot-cli-are-now-in-public-preview/) (public preview May 2026): admins auto-distribute plugins via `.github-private/.github/copilot/settings.json`. Private marketplaces via `marketplace.json` in `.github/plugin/`. Users add via `copilot plugin marketplace add OWNER/REPO`. |
| **OpenCode** | Skills load from global, project, or plugin paths with defined precedence. `opencode-skills-collection` npm plugin bundles 1000+ skills with **SkillPointer** architecture (on-demand loading, ~80k token bloat prevention). Risk-based filtering (`safe`/`critical`/`offensive`/`unknown`). |
| **HiMarket** | Self-service developer portal with **HiChat** (test skills) and **HiCoding** (sandboxed programming). Browse → Subscribe → Install workflow for skill packages. |
| **C3 AI** | Dual discovery: searchable UI for humans + semantic search tools/CLI for agents to auto-install skills. Curated defaults ship to every agent; teams opt-in to additional capabilities. |

**Common User Patterns:**

1. **Tiered override** — Enterprise skills beat personal skills beat project skills. Ensures governance without blocking local experimentation.
2. **One-click install** — From marketplace name, GitHub repo, npm package, or local path.
3. **Trust signals** — Signed/verified badges, org-approved stamps, usage counts, star ratings, last-updated dates.
4. **Team sync** — Auto-install required skills for a team/project; diff view showing what's missing.
5. **On-demand loading** — Skills don't bloat context until triggered by keywords or file patterns.
6. **Sandboxed testing** — Try a skill in a safe environment before installing to production workspace.
7. **Usage analytics** — "Which skills does the frontend team actually use?" informs marketplace curation.

---

## 3. Market Convergence: 7 Trends Every Platform is Chasing

After analyzing 10+ platforms, 7 convergence trends emerge:

| # | Trend | Evidence |
|---|-------|----------|
| 1 | **Skill-as-Artifact** | `SKILL.md` (Anthropic's Agent Skills open standard) is the de facto packaging format. 40+ tools support it. |
| 2 | **MCP as Transport** | MCP (Model Context Protocol) is the universal adapter for tools. Skills wrap MCP servers; gateways govern them. |
| 3 | **Registry = New Package Manager** | Enterprises want npm/Docker-style registries for skills: versioning, signing, scoped access, audit logs. |
| 4 | **Governance at Runtime** | Policy enforcement is shifting from "approve at publish" to "enforce at runtime" via gateways (Databricks service policies, OpenCode permissions). |
| 5 | **Semantic Discovery** | Both humans and agents search via natural language, not just keyword matching. |
| 6 | **Cost-Aware Loading** | On-demand context loading (TrueFoundry, OpenCode SkillPointer) prevents token bloat from unused skills. |
| 7 | **Unified Catalog** | JFrog and Databricks argue AI assets (skills, MCPs, models, prompts) must live in the same catalog as traditional software artifacts. |

---

## 4. AGK's Current Position & Gap Analysis

### 4.1 What AGK Already Has (Strengths)

| Capability | AGK Implementation |
|------------|-------------------|
| **Package management** | Vaults, assets, profiles, dependency resolution with circular/diamond detection |
| **Multi-provider** | Claude Code, OpenCode, Copilot, Gemini, Letta, Snowflake, Firebender, AMP |
| **Enterprise governance** | Policy engine (`policy.toml`), skill signing (GPG), team config sync (`team.toml`), telemetry |
| **Change detection** | SHA10 hashing for vault assets |
| **Headless + TUI** | Every flow has `--json` CLI equivalent; CI/CD ready |
| **Hexagonal architecture** | Ports & Adapters with mechanical enforcement via architecture tests |
| **Distribution** | ClawHub marketplace, GitHub vaults, local vaults, GHES support |

### 4.2 The Gap: AGK is a Package Manager, Not a Marketplace

AGK installs skills brilliantly. But it does not yet **orchestrate the three-party marketplace**:

| Party | What They Need | What AGK Lacks |
|-------|---------------|----------------|
| **Infrastructure** | A registry server with UI, approval workflows, audit dashboards | No registry server; no web UI; no approval pipeline |
| **Contributors** | Scaffold, validate, test, and publish skills to an internal catalog | No `agk skill init` template; no validation beyond syntax; no publish workflow |
| **Users** | Semantic search, trust badges, team recommendations, one-click install from curated catalog | Search is literal (name/identity); no trust scoring; no "most used by your team" |

**The opportunity:** AGK's CLI/TUI can remain the **client**, while a new **ClawHub Enterprise** layer becomes the **server**—or AGK can integrate with existing enterprise registries (JFrog, SkillReg, agentregistry) to fill the server gap without building one from scratch.

---

## 5. Proposal: ClawHub Enterprise — The Three-Sided Marketplace

### 5.1 Vision

> **AGK becomes the standard client for enterprise AI skill marketplaces—connecting platform teams, contributors, and developers through a unified, governed, multi-provider experience.**

Instead of building a registry server from scratch, AGK will:
1. **Integrate** with emerging enterprise registries (JFrog, SkillReg, agentregistry, internal npm) as first-class vault types.
2. **Enhance** the contributor workflow with scaffolding, validation, and publish commands.
3. **Upgrade** the user experience with semantic search, trust signals, and team-aware recommendations.

### 5.2 Feature 1: Registry Vault Integration (Infrastructure)

Extend AGK's vault system to treat enterprise registries as vaults:

```toml
[[vaults]]
identity = "acme-jfrog"
type = "registry"
url = "https://acme.jfrog.io/ai-catalog/skills"
auth = { type = "bearer", token_env = "JFROG_AI_TOKEN" }
# Enterprise registries expose search APIs, signing metadata, and policy state

[[vaults]]
identity = "acme-skillreg"
type = "skillreg"
url = "https://skillreg.acme.internal"
auth = { type = "api_key", key_env = "SKILLREG_API_KEY" }

[[vaults]]
identity = "acme-npm"
type = "npm"
registry = "https://npm.acme.internal"
scope = "@acme-ai"
```

**New behavior:**
- `agk search --vault acme-jfrog "react component testing"` uses the registry's semantic search API instead of local string matching.
- Install from a registry vault fetches signing metadata and validates against `policy.toml` before writing to disk.
- Registry vaults support **curation lists**: platform teams mark skills as `approved`, `experimental`, or `deprecated`; AGK renders badges in TUI.

**Files:**
- `src/domain/vault.rs` — add `RegistryVault` variant
- `src/infra/vault/registry.rs` — `RegistryVaultAdapter` for JFrog/SkillReg/agentregistry APIs
- `src/app/features/vault/registry_auth.rs` — token/api-key management

---

### 5.3 Feature 2: Contributor Workflow (Scaffold → Validate → Publish)

Add a first-class contributor experience to AGK:

```bash
# Scaffold a new skill from enterprise template
agk skill init --name "acme-react-conventions" --template react
# Creates:
#   acme-react-conventions/
#   ├── SKILL.md
#   ├── rules/
#   │   ├── architecture.md
#   │   └── testing.md
#   ├── install.md
#   ├── README.md
#   └── .agk/skill.toml

# Validate before publishing
agk skill validate acme-react-conventions/
# Checks: frontmatter schema, trigger description quality,
#         dependency resolution, policy compliance

# Publish to internal registry
agk skill publish --vault acme-skillreg acme-react-conventions/
# Triggers: platform team review workflow (if configured)
```

**Templates:**
- `agk skill template list` shows enterprise-curated templates (React, DevOps, Security, etc.).
- Templates are themselves skills stored in a vault, so they version and sync like any other asset.

**Validation checks:**
1. YAML frontmatter completeness (`name`, `description`, `version`, `license`).
2. Description quality score (length, trigger keywords, specificity).
3. Dependency resolution (all `requires:` skills exist in reachable vaults).
4. Policy pre-check (would this skill pass `policy.toml` if installed?).
5. Optional: render skill into a sandboxed AI session and run benchmark cases.

**Files:**
- `src/app/features/skill_init/` — scaffolding from templates
- `src/app/features/skill_validate/` — lint + quality checks
- `src/app/features/skill_publish/` — registry upload with review triggers
- `src/infra/templates/` — template storage and rendering

---

### 5.4 Feature 3: Semantic Search & Trust Signals (User Experience)

Upgrade AGK's search from literal to semantic:

```bash
# Local semantic search (if registry vault unavailable)
agk search "how do I test React hooks?"
# Returns skills ranked by description similarity + usage stats

# With trust overlay
agk search --trust-level signed,approved "database migration"
# Only returns skills that are GPG-signed AND platform-team-approved
```

**TUI enhancements:**
- Search tab upgrades to **semantic mode** when a registry vault is attached.
- Trust badges: `[✓ Signed]`, `[★ Approved]`, `[⚠ Experimental]`, `[✕ Deprecated]`.
- Team usage indicator: `[▲ 12 team uses this week]` next to skills your team has installed.
- **Recommended for you** section based on your active profile + team `team.toml`.

**Files:**
- `src/app/features/asset/search_semantic.rs` — semantic ranking layer
- `src/tui/widgets/trust_badge.rs` — badge rendering
- `src/tui/widgets/recommendations.rs` — team-aware suggestions

---

### 5.5 Feature 4: Team Marketplace Sync

Extend the existing `team.toml` concept to support marketplace curation:

```toml
[team]
name = "frontend-platform"
source = "github.com/acme-org/ai-workflows"
branch = "main"

# NEW: Marketplace curation for this team
[[team.marketplace]]
vault = "acme-jfrog"
curation_tag = "frontend-approved"
auto_install = ["acme/react-conventions", "acme/testing-utils"]

# NEW: Skill discovery settings
[team.discovery]
semantic_search = true
trust_threshold = "signed"
show_experimental = false
```

When a new team member runs `agk sync`:
1. Pull latest `team.toml`.
2. Auto-install `auto_install` skills.
3. Configure their TUI search to use the team's curated marketplace view.
4. Show a **Marketplace** tab scoped to `frontend-approved` skills.

**Files:**
- `src/domain/team.rs` — extend `TeamConfig` with marketplace/discovery fields
- `src/app/features/team_sync/marketplace.rs` — curation sync logic

---

### 5.6 Feature 5: Registry-Aware Policy Engine

Integrate the existing policy engine with registry metadata:

```toml
[policy]
# Block all skills except from approved vaults
allow_vaults = ["acme-jfrog", "acme-skillreg"]

# NEW: Require platform-team approval from registry curation
require_approval_tag = "approved"

# NEW: Block skills deprecated in registry
block_deprecated = true

# NEW: Enforce minimum trust score from registry
min_trust_score = 0.7

# Audit trail
audit_log = "~/.config/agk/audit.log"
```

When installing from a registry vault:
1. Fetch skill metadata (signatures, curation tags, trust score, deprecation status).
2. Evaluate against `policy.toml` before downloading the skill body.
3. If blocked, show the specific policy rule and suggest approved alternatives from the same registry.

**Files:**
- `src/domain/policy.rs` — add registry-aware rules
- `src/app/policy.rs` — evaluate registry metadata before install

---

## 6. Implementation Roadmap

| Phase | Deliverable | Target Release |
|-------|-------------|----------------|
| **1. Research** | Finalize registry API specs (JFrog OpenClaw, SkillReg, agentregistry) | v0.3.x |
| **2. Scaffold** | `agk skill init` + `agk skill validate` + built-in templates | v0.4.0 |
| **3. Connect** | Registry vault adapter (JFrog first, then SkillReg/agentregistry) | v0.4.0 |
| **4. Discover** | Semantic search + trust badges in TUI/CLI | v0.4.1 |
| **5. Govern** | Registry-aware policy engine + team marketplace sync | v0.4.1 |
| **6. Scale** | Publish workflow with review triggers + CI validation | v0.4.2 |

---

## 7. Competitive Differentiation

| Competitor | Weakness | AGK Advantage |
|------------|----------|---------------|
| **JFrog** | Heavyweight artifact platform; no native multi-provider AI skill client | AGK is the lightweight, multi-provider client that talks *to* JFrog |
| **SkillReg** | SaaS-only registry; no local CLI/TUI skill management | AGK provides the local client + can sync with SkillReg |
| **Claude Code plugins** | Only works for Claude Code; no team config sync, no policy engine | AGK is cross-provider (Claude, OpenCode, Copilot, Gemini, etc.) with enterprise governance |
| **GitHub Copilot CLI** | GitHub-only ecosystem; no skill signing, no semantic search | AGK supports GHES, private registries, GPG signing, and semantic discovery |
| **HiMarket** | Kubernetes-heavy deployment; complex for small-medium enterprises | AGK's client-first approach: no server required for basic marketplace functionality |
| **OpenCode** | npm-centric; no unified policy across providers | AGK unifies policy, signing, and sync across all 8+ providers |

**The unique pitch:** AGK is the **only cross-provider, enterprise-governed, registry-agnostic skill client** that treats skills as a true software supply chain artifact.

---

## 8. Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| **Registry API fragmentation** | Start with JFrog OpenClaw (most mature); abstract via `RegistryVaultPort` so new registries are adapter-only |
| **Semantic search quality** | Fallback to literal search if registry doesn't support semantic; local embedding index as future enhancement |
| **Contributor adoption** | Ship with 5 high-quality built-in templates; integrate with existing `SKILL.md` community standards |
| **Scope creep vs. Enterprise Feature Pack** | This proposal *extends* P7 (Enterprise Feature Pack), not replaces it. Registry integration builds on existing policy/signing/team-sync work. |
| **Performance** | Registry calls are async/cached; skill metadata is fetched lazily; full bodies only on install |

---

## 9. Conclusion

The enterprise AI skills marketplace is no longer a novelty—it is a **software supply chain requirement**. Every major platform is racing to provide registries, governance, and discovery. But none provide a **unified, cross-provider client** that works with all of them.

AGK's existing architecture (hexagonal, multi-provider, vault-based, policy-aware) makes it the ideal candidate to fill this gap. By adding:
- **Registry vault integration** for infrastructure teams,
- **Contributor scaffolding and validation** for skill authors, and
- **Semantic search with trust signals** for end users,

AGK evolves from a package manager into the **universal client for enterprise AI skill marketplaces**—the `apt`/`brew`/`npm` of the agent intelligence era.

---

## Sources

- [JFrog Agent Skills Registry](https://jfrog.com/ai-catalog/skills-registry/)
- [TrueFoundry Skills Registry](https://www.truefoundry.com/blog/introducing-skills-registry-reusable-agent-skills-for-production-ai-systems)
- [C3 AI MCP Gateway](https://c3.ai/blog/how-c3-ai-scales-agent-expertise-not-just-agent-tools/)
- [HiMarket (Higress Group)](https://github.com/higress-group/himarket/)
- [AgentPlaybooks Enterprise](https://agentplaybooks.ai/enterprise)
- [SkillReg Private Registry](https://skillreg.dev/)
- [iFlytek SkillHub](https://github.com/iflytek/skillhub/)
- [agentregistry](https://github.com/agentregistry-dev/agentregistry/)
- [Databricks Unity Catalog & AI Gateway](https://www.databricks.com/blog/governing-ai-agents-scale-unity-catalog)
- [GitHub Enterprise-Managed Plugins (May 2026)](https://github.blog/changelog/2026-05-06-enterprise-managed-plugins-in-github-copilot-cli-are-now-in-public-preview/)
- [GitHub Copilot Plugin Marketplace Docs](https://docs.github.com/en/copilot/reference/cli-plugin-reference)
- [Claude Code Skills Docs](https://code.claude.com/docs/en/skills.md)
- [Claude Code Plugin Marketplaces](https://code.claude.com/docs/en/plugin-marketplaces)
- [Anthropic skill-creator Skill](https://github.com/anthropics/skills/blob/main/skills/skill-creator/SKILL.md)
- [OpenCode Skills Docs](https://open-code.ai/en/docs/skills)
- [OpenCode Plugin Skill Bundling](https://github.com/anomalyco/opencode/issues/9010)
- [Developer Toolkit — Building Custom Skills](https://developertoolkit.ai/en/shared-workflows/skills-ecosystem/building-custom-skills/)
- [Microsoft Skills for Fabric Authoring Guide](https://github.com/microsoft/skills-for-fabric/blob/main/docs/skill-authoring-guide.md)
- [agentskills.io Open Standard](https://github.com/Yarmoluk/custom-skill-developer)
- [AgentPowers OpenCode Marketplace](https://agentpowers.ai/tools/opencode)
- [opencode-skills-collection npm](https://registry.npmjs.org/opencode-skills-collection)

---

*End of report — generated 2026-06-02.*
