# Comparative Research Report: AGK vs. Coder

**Date:** 2026-05-30  
**Scope:** Architecture, features, deployment model, extensibility, and strategic positioning of two AI-coding infrastructure projects.  
**Sources:** AGK codebase (`dkthezero/agk`, MIT license), Coder official documentation (`coder.com/docs`), `coder/coder` GitHub repository (AGPL-3.0).

---

## 1. Executive Summary

AGK and Coder solve fundamentally different problems in the AI-assisted development lifecycle, yet they occupy adjacent spaces in the emerging "AI coding infrastructure" landscape.

- **AGK** is a **client-side skill distribution manager** — a Rust CLI/TUI that packages, versions, and distributes AI agent skills (structured markdown instructions, MCP server configs) across multiple IDE and agent platforms. It is a **local toolchain** analogous to `npm` or `brew`, but for AI agent context rather than code libraries.

- **Coder** is a **server-side cloud development environment (CDE) platform** — a Go/TypeScript self-hosted suite that provisions remote workspaces (VMs, K8s pods, containers) and runs governed AI coding agents inside a centralized control plane. It is an **enterprise platform** analogous to GitHub Codespaces or Gitpod, but with built-in AI agent governance.

| Dimension | AGK | Coder |
|-----------|-----|-------|
| **Primary Concern** | Skill/asset distribution | Workspace provisioning + AI governance |
| **Deployment Model** | Local binary (client-side) | Self-hosted server cluster (control plane) |
| **User Scale** | Individual / small team | 1 – 10,000+ users (enterprise) |
| **AI Loop Location** | External (provider-native) | Internal (control-plane hosted) |
| **Security Model** | Local file system | Network-isolated workspaces, zero LLM creds in workspaces |
| **License** | MIT | AGPL-3.0 |

Both projects share a common thesis: **AI coding workflows need structured infrastructure** — not just raw LLM access. AGK attacks this from the "skill package" layer; Coder attacks it from the "governed execution environment" layer.

---

## 2. Project Overviews

### 2.1 AGK: Agent Skill Package Manager

AGK (Agent Knowledge Gateway) is a terminal-based manager written in Rust that distributes reusable AI agent skills and instructions across multiple provider ecosystems. It treats AI skills as versioned, hashable, dependency-resolved packages.

**Core Purpose:**  
A developer maintaining a team-wide set of Claude Code skills, OpenCode profiles, or Copilot custom instructions can centralize them in a Git-backed "vault," then install, update, and sync them across all active providers from a single TUI or CI command.

**Key Artifacts:**
- `SKILL.md` — Markdown files with YAML frontmatter (`name`, `version`, `requires:` dependency trees) that define agent skills.
- `AGENTS.md` — Markdown files defining system-level instructions.
- `config.toml` — Scoped TOML configuration (global `~/.config/agk/` and workspace `.agk/`) tracking vaults, providers, installed assets, and MCP server registrations.
- Vaults — Local directories, GitHub repositories, or the ClawHub community marketplace.

**Workflow Example:**
```bash
agk vault attach ./company-skills      # local vault
agk provider toggle claude-code on     # activate provider
agk install company-skills/rust-patterns # install skill
agk sync                               # CI/CD: sync all assets
```

### 2.2 Coder: Governed Cloud Development Environment + AI Agents

Coder is an open-source, self-hosted platform that provisions cloud-based development workspaces and runs governed AI coding agents inside a centralized control plane. It is written primarily in Go (backend) and TypeScript (frontend/dashboard).

**Core Purpose:**  
An enterprise platform team can offer developers ephemeral, Terraform-defined workspaces with built-in AI agent support. The AI agent loop runs in the control plane (`coderd`), not in the workspace, ensuring LLM credentials never touch developer machines or remote compute.

**Key Artifacts:**
- **Templates** — Terraform definitions specifying workspace infrastructure (EC2, Kubernetes, Docker, etc.).
- **Modules** — Reusable Terraform snippets from [registry.coder.com](https://registry.coder.com) (IDE integrations, dotfiles, git-clone).
- **Agent Chats** — Persistent background jobs in PostgreSQL that record the full LLM conversation, tool calls, and token usage.
- **Skills** — Structured instruction sets placed in `.agents/skills/` inside workspaces.
- **MCP Tools** — Model Context Protocol servers exposed via workspace templates.

**Workflow Example:**
1. Developer opens Coder dashboard, selects a template (e.g., "Kubernetes + JetBrains").
2. Coder provisions the workspace via `provisionerd` + Terraform.
3. Developer or AI agent submits a prompt; `coderd` streams it to Anthropic/OpenAI via the AI Gateway.
4. Agent executes tool calls (`read_file`, `execute`) over the existing Tailnet tunnel.

---

## 3. Architecture Comparison

### 3.1 High-Level Paradigm

| Aspect | AGK | Coder |
|--------|-----|-------|
| **Architectural Pattern** | Clean Architecture (Hexagonal / Ports & Adapters) | Micro-service-ish monolith with control-plane / worker separation |
| **Primary Language** | Rust | Go + TypeScript |
| **Build Tool** | Cargo | Go modules + pnpm/npm |
| **UI Paradigm** | Dual-mode: TUI (`ratatui`) + headless CLI | Web dashboard + IDE plugins + CLI (`coder` binary) |

### 3.2 AGK: Layered Clean Architecture

AGK adopts Robert C. Martin's Clean Architecture with three concentric layers:

```
┌─────────────────────────────────────────┐
│  Adapters (TUI / CLI / Infra)          │
│  ├─ tui/      ratatui event loop       │
│  ├─ cli/      clap subcommands         │
│  └─ infra/    file system, HTTP, JSON  │
├─────────────────────────────────────────┤
│  Application Layer (app/)              │
│  ├─ core.rs   AgkCore façade           │
│  ├─ command.rs CoreCommand enum        │
│  ├─ features/ use-case dispatchers     │
│  │   ├─ vault/   attach/detach/refresh │
│  │   ├─ asset/   install/remove/update │
│  │   ├─ mcp/     register/enable/test │
│  │   ├─ profile/ create/start/manage   │
│  │   ├─ telemetry/ export analytics     │
│  │   └─ ...                             │
│  └─ ports/     trait definitions       │
├─────────────────────────────────────────┤
│  Domain Layer (domain/)                │
│  ├─ config.rs   ConfigFile, Vault, etc. │
│  ├─ asset.rs   AssetIdentity, hashes  │
│  ├─ mcp.rs     McpServer definition    │
│  ├─ profile.rs Profile, WizardStep     │
│  └─ telemetry.rs AnalyticsEvent        │
└─────────────────────────────────────────┘
```

**Key Architectural Decisions:**
- **AgkCore façade** (`src/app/core.rs`) receives a `CoreCommand` and dispatches it to feature-specific `dispatch()` functions. Each feature module owns its use-case logic and emits `CoreEvent`s back to adapters via the `CoreEventSink` trait.
- **TUI/CLI equivalence** — Both adapters construct the same `CoreCommand` enum and feed it to `AgkCore::execute()`. Contract tests enforce behavioral parity.
- **Registry pattern** — `app::registry::Registry` holds collections of `ProviderPort`, `VaultPort`, and `FeatureSetPort` trait objects, enabling runtime plugin-like extension without dynamic loading.
- **Scoped persistence** — `ConfigStorePort` abstracts TOML read/write for both global (`~/.config/agk/`) and workspace (`.agk/`) scopes.

### 3.3 Coder: Control-Plane / Workspace Split

Coder separates the **control plane** (where decisions and AI logic live) from **workspace compute** (where code and commands execute):

```
┌──────────────────────────────────────────────────────────────┐
│  CONTROL PLANE                                               │
│  ├─ coderd         API server, dashboard, auth, AI loop   │
│  ├─ aibridged      AI Gateway (in-memory, inside coderd)    │
│  ├─ provisionerd   Terraform executor (internal or external)│
│  └─ PostgreSQL     State store, chat history, audit logs    │
├──────────────────────────────────────────────────────────────┤
│  NETWORKING                                                  │
│  └─ Tailnet / WireGuard / DERP   Reverse tunnel to workspaces│
├──────────────────────────────────────────────────────────────┤
│  WORKSPACE COMPUTE                                           │
│  ├─ Coder Agent    In-workspace daemon (SSH, port-forward)  │
│  ├─ AI-skills      .agents/skills/ markdown files            │
│  └─ User code      Actual project source                     │
└──────────────────────────────────────────────────────────────┘
```

**Key Architectural Decisions:**
- **Agent loop in control plane** — `coderd` streams prompts to LLMs and interprets tool calls. Workspaces are "dumb" compute with zero AI awareness.
- **Tailnet tunnel reuse** — Agent tool calls traverse the same WireGuard/derp tunnel that developers use for SSH and VS Code Remote. No new inbound ports.
- **External provisioners** — Recommended for production; each daemon handles exactly one concurrent build, enabling horizontal scaling of Terraform execution.
- **Lazy provisioning** — Workspaces are created only when the agent needs to execute file or shell commands. Pure Q&A chats never spin up infrastructure.

### 3.4 Technology Stack Deep Dive

| Layer | AGK | Coder |
|-------|-----|-------|
| **Runtime** | Native binary (single Rust compile) | Go binary + Node.js dashboard |
| **Concurrency** | `std::sync::Arc` + trait objects; async via tokio where needed | Goroutines + channels; PostgreSQL for persistence |
| **UI Framework** | `ratatui` (TUI), `clap` (CLI), `crossterm` | React/TypeScript (web), VS Code/JetBrains extensions |
| **Config Format** | TOML | TOML (server env), JSON (API), Terraform (templates) |
| **Persistence** | Local TOML files | PostgreSQL (required external managed DB) |
| **Networking** | HTTPS for GitHub/ClawHub | WireGuard / DERP / Tailnet; HTTPS for API |
| **IaC** | None (static file copying) | Terraform (HashiCorp) |
| **Packaging** | Cargo crate, Homebrew formula, GitHub Releases | Helm chart, Docker image, `.deb`/`.rpm` |

---

## 4. Feature Comparison Matrix

### 4.1 Core Capabilities

| Feature | AGK | Coder |
|---------|-----|-------|
| **Skill Packaging** | Native (`SKILL.md` + YAML frontmatter + `requires:` deps) | Native (`.agents/skills/` markdown) |
| **Multi-Provider Asset Distribution** | Yes (Claude Code, Copilot, Gemini, Letta, Snowflake, Firebender, AMP, OpenCode) | Yes (AI Gateway proxies Anthropic, OpenAI, Azure, Bedrock, Copilot, etc.) |
| **MCP Server Management** | Register, test (JSON-RPC handshake), enable/disable per provider | Expose via `.mcp.json` in workspace templates |
| **Vault / Marketplace** | Local, GitHub, ClawHub (community marketplace via external CLI) | Coder Registry (modules + templates, community + verified) |
| **Versioning & Updates** | SHA-based change detection; `update` and `sync` commands | Template versioning via Git; module semver |
| **Dependency Resolution** | Recursive `requires:` / `requires_optional:` with cycle detection | None for skills; Terraform module composition for infra |
| **Profile Management** | Create wizard-driven profiles; start sessions for providers (e.g., OpenCode) | N/A (user identity is OIDC/SSO based) |
| **Telemetry / Analytics** | Export usage events (asset installs, provider toggles) | Full audit logging (prompts, token usage, tool calls, 60-day retention) |
| **CI/CD Headless Mode** | Full CLI with `--json`, `--quiet`, deterministic exit codes | REST API + CLI (`coder` binary) for automation |
| **Workspace Provisioning** | None | Full IaC via Terraform (EC2, K8s, Docker, GCP, Azure) |
| **IDE Integration** | Indirect (installs files to provider config dirs) | Native VS Code, JetBrains plugins; web-based code-server |
| **Dev Container Support** | None | Yes, via `envbuilder` (devcontainer spec) |
| **Sub-Agent Orchestration** | No | Yes (`spawn_agent`, `wait_agent`, parallel child agents) |
| **Plan Mode** | No | Yes (investigate → Markdown plan → user review → implement) |
| **Context Compaction** | No | Yes (automatic summarization of old conversation history) |

### 4.2 Asset Lifecycle Comparison

**AGK Asset Lifecycle:**
1. **Scan** — Vault backends (`LocalVaultAdapter`, `GithubVaultAdapter`, `ClawHubVaultAdapter`) scan directories for `SKILL.md` and `AGENTS.md`.
2. **Discover** — `FeatureSetPort` trait (`SkillFeatureSet`, `InstructionFeatureSet`) identifies valid packages.
3. **Resolve** — `AssetResolver` handles `requires:` dependency trees (diamond deduplication, cycle rejection).
4. **Install** — `ProviderPort::install()` copies files to provider-specific config directories (e.g., `~/.claude/skills/`, `~/.config/opencode/skills/`).
5. **Track** — Installed assets recorded in scoped `config.toml`.
6. **Detect Changes** — SHA-based hashing compares installed assets against vault source; `update`/`sync` refreshes stale assets.
7. **Remove** — `ProviderPort::remove()` deletes files and prunes config entries.

**Coder Asset/Agent Lifecycle:**
1. **Template** — Admin authors a Terraform template defining workspace infrastructure + base image.
2. **Provision** — `provisionerd` executes Terraform to create VM/pod/container.
3. **Agent Install** — Coder Agent binary auto-injected into workspace; dials back to `coderd` over Tailnet.
4. **Skill Load** — Agent reads `.agents/skills/` from workspace file system on-demand.
5. **Prompt** — User submits chat; `coderd` runs the agent loop, dispatching tool calls to workspace.
6. **Audit** — All prompts, tool calls, and token usage logged to PostgreSQL via AI Gateway.
7. **Destroy** — Workspace auto-stops or is deleted; ephemeral by design.

---

## 5. Target Audience & Use Cases

### 5.1 AGK

**Primary Personas:**
- **Individual power users** who work across multiple AI platforms (Claude Code for deep reasoning, Copilot for autocomplete, OpenCode for open-source projects) and want a unified skill management layer.
- **Small team leads** (2–20 developers) who want to share a curated vault of team-specific skills without setting up a server.
- **Open-source skill authors** who publish to ClawHub and want a standard package format.

**Ideal Use Cases:**
- Maintaining a team wiki of `SKILL.md` files that teach Claude Code your internal API conventions.
- Distributing MCP server configurations to all team members' Claude Code setups.
- CI pipeline (`agk sync`) that ensures every developer's local agent context is up-to-date with the latest team standards.
- Packaging a skill with dependency tree (`requires: clawhub/react-parser`) for complex multi-step agent behaviors.

### 5.2 Coder

**Primary Personas:**
- **Enterprise platform engineers** who must provide secure, governed development environments to hundreds or thousands of developers.
- **Security/compliance officers** in regulated industries (finance, healthcare, government) who need audit trails of every AI prompt and tool invocation.
- **AI-native development teams** who want to run autonomous coding agents on centralized infrastructure with full observability.

**Ideal Use Cases:**
- Onboarding a new developer with a one-click workspace pre-configured with IDE, dependencies, and AI agent access — no local setup.
- Running AI agents in air-gapped environments where LLM credentials must remain in a hardened control plane.
- Scaling build farms where each CI job gets an ephemeral workspace with exact resource specs.
- Enforcing "agent firewall" policies that restrict which domains AI tools can query.

---

## 6. Deployment & Operations Model

### 6.1 AGK: Zero-Ops Local Binary

| Aspect | Detail |
|--------|--------|
| **Installation** | `cargo install agk`, Homebrew, or GitHub Release binary. Single static binary (~few MB). |
| **Configuration** | TOML files on local disk. No server to run, no database to maintain. |
| **Updates** | Self-update via package manager or `cargo install`. |
| **Multi-user** | Each user runs their own AGK instance with their own `~/.config/agk/`. |
| **CI/CD** | Binary invoked in pipeline; `--json` output parsed by downstream steps. |
| **Observability** | Limited; telemetry export is manual/event-based. No central dashboard. |

**Operational Complexity:** Minimal. AGK is a developer tool, not a service. The only external dependency is the `clawhub` CLI (optional, for marketplace access) and Git (for GitHub vaults).

### 6.2 Coder: Self-Hosted Platform Engineering

| Aspect | Detail |
|--------|--------|
| **Installation** | Helm chart on Kubernetes (`coder-v2/coder`), Docker Compose, or systemd on VM. |
| **Database** | External PostgreSQL (managed RDS/Cloud SQL recommended). **Not optional.** |
| **Networking** | Requires ingress for dashboard/API; Tailnet/derp for workspace tunnels. |
| **Scaling** | Vertical scaling for `coderd` (~1 vCPU + 2 GB RAM per 250 users); horizontal scaling for provisioners (1 daemon = 1 concurrent build). |
| **High Availability** | Multi-replica `coderd` (keep <10 replicas); external provisioners; DB connection pooling. |
| **Backup / DR** | PostgreSQL backups; template Git repos as IaC source of truth. |
| **Observability** | Built-in Prometheus metrics; optional Helm-deployed Grafana/Loki/Alertmanager stack. |
| **Security** | OIDC SSO, RBAC, audit logs, AI Gateway logs, Agent Firewall. |

**Operational Complexity:** High. Coder is a platform product that requires a dedicated platform/SRE team for production deployments at scale. The documentation explicitly recommends against autoscaling `coderd` because it maintains long-lived WebSocket connections.

---

## 7. Extensibility & Ecosystem

### 7.1 AGK Extensibility

AGK's extension model is **compile-time trait-based**:

- **`ProviderPort`** — Implement to add a new AI platform. Must define `install()`, `remove()`, `install_path_for()`, and optionally `supports_profiles()` + `start_profile_session()`.
- **`VaultPort`** — Implement to add a new vault backend. Already supports local directories, GitHub repos, and ClawHub (via external CLI wrapper).
- **`FeatureSetPort`** — Implement to define new asset kinds beyond `Skill` and `Instruction`.
- **`CoreEventSink`** — Implement custom adapters (e.g., a GUI or LSP extension) that consume `CoreEvent` streams.

**Ecosystem Status:**
- **ClawHub** (`clawhub.ai`) is a nascent community marketplace. AGK integrates via the external `clawhub` CLI.
- **No formal plugin loading** — New providers must be compiled into the binary. The registry pattern makes this mechanically easy but does not support dynamic `.so` plugins.
- **GitHub ecosystem** — GitHub vaults allow any public or private repo to serve as a vault.

### 7.2 Coder Extensibility

Coder's extension model is **runtime template-driven**:

- **Terraform Templates** — Full infrastructure-as-code flexibility. Any cloud provider, container runtime, or VM platform that Terraform supports can be a Coder workspace.
- **Coder Registry** (`registry.coder.com`) — Community marketplace with ~80+ contributors, 186+ releases. Modules for VS Code, JetBrains, Cursor, file browsers, dotfiles, and git-clone.
- **Modules** — Reusable Terraform snippets that extend workspaces without rewriting entire templates.
- **IDE Plugins** — Official VS Code and JetBrains Gateway plugins; compatible with any desktop or web IDE.
- **AI Skills / MCP** — Workspace-level `.agents/skills/` and `.mcp.json` allow per-project agent customization.
- **API** — Full REST API (`codersdk`) and CLI (`coder`) for third-party integrations.

**Ecosystem Status:**
- Mature Terraform ecosystem (thousands of providers).
- Active GitHub community (`coder/coder`: 13k+ stars).
- Verified module namespace for official Coder-published modules.
- Backstage integration for platform engineering portals.

---

## 8. Strengths & Weaknesses

### 8.1 AGK

**Strengths:**
1. **Zero infrastructure overhead** — Single binary, no database, no server. A developer can adopt it in minutes.
2. **Multi-provider symmetry** — Uniquely treats Claude Code, Copilot, Gemini, OpenCode, etc., as peers. Skills are portable across providers (modulo provider-specific install paths).
3. **Dependency-aware skill packaging** — The `requires:` / `requires_optional:` system with cycle detection and diamond deduplication is sophisticated for a client-side tool.
4. **Deterministic CI/CD** — `--json`, `--quiet`, and exit-code contracts make it suitable for automation.
5. **TUI/CLI equivalence** — Clean Architecture ensures the interactive and headless modes are behaviorally identical, tested by contract tests.
6. **MCP lifecycle management** — JSON-RPC handshake testing before enabling a server is a genuine safety feature.

**Weaknesses:**
1. **No execution environment** — AGK distributes skills but does not run them. It cannot execute code, manage compute, or host agents.
2. **No multi-user governance** — No RBAC, audit logs, or centralized policy enforcement. Team adoption relies on Git-backed vaults and social contract.
3. **No network isolation** — Skills and MCP configs run on the user's local machine with their local permissions.
4. **Limited observability** — Telemetry is export-oriented, not real-time dashboard-oriented.
5. **Compile-time extension only** — Adding a new provider requires modifying the Rust source and recompiling.
6. **Small ecosystem** — ClawHub is early-stage; no large corporate backing.

### 8.2 Coder

**Strengths:**
1. **Enterprise-grade governance** — SOC 2 Type II, SSO, RBAC, audit logging of every AI interaction, and air-gapped deployment support.
2. **Security-first AI architecture** — Zero LLM credentials in workspaces; all AI logic runs in the hardened control plane. Agent Firewall restricts outbound access.
3. **Scalable workspace provisioning** — Terraform-backed infrastructure scales from single containers to 10,000-user EC2/K8s fleets.
4. **Unified human + AI workspaces** — Same Tailnet tunnel, same workspace, same IDE for developers and agents. No "AI sidecar" complexity.
5. **Rich ecosystem** — Terraform registry, Coder Registry modules, official IDE plugins, Backstage integration.
6. **Sub-agent orchestration** — Built-in parallel agent spawning with independent context windows.
7. **Lazy provisioning** — Chats that don't need code execution never spin up costly infrastructure.

**Weaknesses:**
1. **High operational burden** — Requires Kubernetes, PostgreSQL, and ongoing SRE attention. Not a "install and forget" tool.
2. **No client-side skill portability** — Skills are workspace-local (`.agents/skills/`). There is no cross-provider skill package format like AGK's `SKILL.md`.
3. **License friction** — AGPL-3.0 may discourage commercial embedding or white-labeling without open-sourcing derivatives.
4. **Overkill for individuals** — A solo developer does not need Terraform, K8s, and a control plane to manage a few Claude Code skills.
5. **Beta AI features** — Coder Agents is actively developed; APIs and behavior may change.
6. **Provider coupling** — While the AI Gateway supports many LLM providers, the agent loop is Coder-specific. Skills cannot be exported to run in Claude Code or Copilot standalone.

---

## 9. Strategic Insights / Recommendations for AGK

### 9.1 Positioning: Complement, Not Competitor

AGK should **explicitly avoid** competing with Coder on workspace provisioning or enterprise governance. Instead, AGK can position itself as:

> **The portable skill package standard that works inside *and* outside of CDEs.**

A Coder workspace could include AGK in its base image so that developers can install ClawHub skills into their remote workspace's `.agents/skills/` directory. AGK becomes the "npm for AI skills" that feeds content into Coder (and other platforms).

### 9.2 Recommended Strategic Moves

1. **Publish an AGK Skill Format RFC**  
   Formalize the `SKILL.md` + YAML frontmatter + `requires:` specification as an open standard. Invite ClawHub, Coder, and other platforms to adopt it. This elevates AGK from a tool to a standard.

2. **Add a Coder Provider Adapter**  
   Implement a `CoderProviderPort` that syncs AGK-managed skills into a Coder workspace's `.agents/skills/` directory over the Tailnet tunnel (or via the `codersdk` API). This bridges AGK's client-side packaging with Coder's server-side execution.

3. **Introduce a Server-Sync Mode**  
   Extend the vault concept to support a lightweight "team config server" — a simple HTTP API that serves the canonical `config.toml` and vault contents. This addresses AGK's governance gap without requiring full Kubernetes infrastructure.

4. **Expand MCP Test Coverage**  
   AGK's MCP JSON-RPC handshake testing is a genuine differentiator. Double down on it: add security sandboxing warnings (e.g., flag MCP servers that request broad filesystem access), and publish a "MCP Security Scorecard."

5. **Explore Dynamic Plugin Loading**  
   Move from compile-time `ProviderPort` implementations to a WASM-based plugin system. This would allow third parties to ship new provider adapters without recompiling AGK, dramatically expanding the ecosystem.

6. **Leverage the CLI/TUI Equivalence for Testing**  
   AGK's contract-test architecture (TUI/CLI equivalence + architecture tests + full-flow integration tests) is a structural advantage. Market this as "battle-tested reliability" in a world where many AI tools are brittle shell scripts.

7. **Target the "Local-First AI" Niche**  
   As AI agents proliferate, many developers will resist moving all development to the cloud. AGK serves the **local-first** or **hybrid** crowd who want structured skill management without surrendering their local environment.

### 9.3 Competitive Differentiation Matrix

| Capability | AGK Should Own | Coder Will Own |
|------------|----------------|----------------|
| Portable skill packaging | Yes | No |
| Cross-provider skill sync | Yes | No |
| Lightweight local adoption | Yes | No |
| Enterprise workspace governance | No | Yes |
| AI agent loop execution | No | Yes |
| Infrastructure provisioning | No | Yes |
| Audit & compliance | No | Yes |
| Large-scale team onboarding | Partial (via CI) | Yes (via dashboard) |

---

## 10. Conclusion

AGK and Coder represent two complementary vectors in the maturation of AI coding infrastructure:

- **AGK** is a **client-side package manager** for AI skills. Its value lies in portability, zero-ops adoption, and cross-provider symmetry. It excels when the problem is "how do we distribute and version agent instructions across a team's diverse toolset?"

- **Coder** is a **server-side platform** for governed development environments. Its value lies in security isolation, scalable provisioning, and enterprise compliance. It excels when the problem is "how do we run AI agents on controlled infrastructure without exposing credentials to end users?"

**For AGK specifically**, the strategic opportunity is to become the **de facto packaging standard** for AI skills — a role analogous to `npm` for JavaScript or `brew` for macOS tools. By publishing an open skill format, integrating with Coder as a downstream consumer, and maintaining its lightweight operational profile, AGK can capture the "long tail" of developers and teams who need structure without infrastructure.

**For teams evaluating both**, the decision is not "either/or" but "where does each fit?"
- Use **Coder** as the enterprise CDE and AI execution platform.
- Use **AGK** as the skill authoring, packaging, and distribution layer that populates both local developer machines and remote Coder workspaces.

---

*Report synthesized from AGK v0.x codebase analysis (Rust, MIT) and Coder v2.x official documentation (Go/TypeScript, AGPL-3.0).*
