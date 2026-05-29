# Research Report: VibeCode Pro Max Kit vs. AGK

## 1. Executive Summary

**VibeCode Pro Max Kit** (`vibecode-pro-max-kit`) is a **meta-harness** — a system that orchestrates AI coding agents themselves through disciplined, phase-locked workflows. Its tagline is *"Stop your AI from coding before it thinks."* It imposes a RIPER-5 lifecycle (Research → Innovate → Plan → Execute → Update Process) with tool restrictions at each phase, durable disk-based context to resist token-window decay, and twelve specialized subagents.

**AGK** is a **distribution and configuration manager** for AI coding environments — a terminal-based tool (TUI + CLI) that packages, shares, and installs skills, instructions, MCP servers, and profiles across multiple AI providers (Claude Code, OpenCode, Copilot, Gemini, etc.). It follows strict hexagonal architecture (Ports & Adapters) and treats AI environments as reproducible, versioned artifacts.

**The Opportunity:** AGK currently manages *what* goes into an AI environment (skills, instructions, MCPs). VibeCode manages *how* agents behave once inside that environment. By adopting and adapting VibeCode's orchestration patterns, AGK can evolve from a **package manager** into a **Harness Orchestrator** — distributing not just static assets, but entire workflow templates, agent phase configurations, and process guardrails.

---

## 2. VibeCode Pro Max Kit — Deep Analysis

### 2.1 Core Philosophy

The kit treats impulsive, vibe-based coding as a bug. It replaces it with a **spec-driven, approval-gated engineering lifecycle** that compounds knowledge over time rather than losing it to context window compaction.

Key tenets:
- **Phase-locking:** Agents lose capabilities as they progress. Research agents are read-only. Planning agents can only write to `process/` directories. Execution agents require explicit user approval (`"you say 'go'"`).
- **Context durability:** All knowledge lives on disk in `process/` — not in the model's context window. After compaction, agents recover state by reading router files.
- **Agent specialization:** 12 agents with narrow mandates (research, innovate, plan, execute, debug, test, review, simplify, UI/UX, git manager).
- **Skill discovery:** 32 skills activate automatically via keyword scanning before request routing.

### 2.2 Architecture

```
CLAUDE.md          → Orchestrator routing rules, phase transitions, mode labels
AGENTS.md          → Agent registry, skill definitions, dispatch protocol
.claude/
  agents/          → Agent definitions (markdown)
  skills/          → Skill packages with SKILL.md frontmatter
  hooks/           → 7 lifecycle hooks (.cjs): privacy, blocking, init, context injection, quality nudges
.codex/            → Mirror of .claude/ for OpenAI Codex compatibility
.agents/           → Symlinks for cross-tool discovery
process/
  context/
    all-context.md → Root router — domain-routed groups, not monolithic docs
  general-plans/
    active/        → Dated plan files
    completed/
  features/
    <topic>/       → Auto-created after 5 artifacts accumulate
      active/
      completed/
      backlog/
      reports/
      references/
  development-protocols/ → Shared behavioral rules
```

### 2.3 The RIPER-5 Workflow

| Phase | Agent | Capabilities | Tool Access |
|-------|-------|--------------|-------------|
| **R**esearch | `vc-research-agent` | Read-only fact gathering | Read tools, safe shell |
| **I**nnovate | `vc-innovate-agent` | Brainstorming approaches | Discussion-only, no file changes |
| **P**lan | `vc-plan-agent` | Draft specifications | Write restricted to `process/` only |
| **E**xecute | `vc-execute-agent` | Build approved plans | Full tool access, post-approval only |
| **Update Process** | `vc-update-process-agent` | Learning capture, archiving | Write to `process/`, update rules |

**Transition rules:**
- Every orchestrator response MUST begin with `[MODE: MODE_NAME]`.
- `"ENTER EXECUTE MODE"` or user saying `"go"` is required before implementation.
- `"PHASE JUMPING PREVENTED"` — structural guard against skipping stages.

### 2.4 Safety & Quality Mechanisms

1. **Read-only research phase** — No code generation during research.
2. **Bash-less innovation** — No shell commands during brainstorming.
3. **Plan-stage write restriction** — Can only mutate `process/` directories.
4. **50% progress checkpoint** — Mandatory mid-point review.
5. **Deviation halting protocol** — If implementation drifts from spec, stop.
6. **Privacy blocks** — Hooks scan for secrets before any operation.
7. **Drift scoring** — Post-execution evaluation of files modified, harness edits, and notable observations to recommend process updates.
8. **5-persona pre-implementation debate** — Multiple perspectives before coding.
9. **12-dimension edge-case decomposition** — Structured risk analysis.
10. **STRIDE + OWASP audit** — Security review gates.

### 2.5 Context Management

Unlike monolithic `CLAUDE.md` files that grow unbounded, VibeCode uses:
- **Domain-routed groups** — Context is split by topic, routed via `all-context.md`.
- **Auto-promotion** — After 5 artifacts on a topic, a dedicated `features/<topic>/` folder is created.
- **Router pattern** — `all-*.md` files are routers, not full knowledge. Agents MUST follow deep links.
- **Durable reports** — Persisted on disk so work survives token window compaction.

### 2.6 Multi-Tool Support

The `AGENTS.md` open standard aims to make the system portable across:
- Claude Code
- Codex CLI
- Cursor
- Windsurf
- Antigravity
- OpenCode
- GitHub Copilot

No custom plugins — just markdown files and symlinks.

---

## 3. AGK — Current State Analysis

### 3.1 Core Purpose

AGK is the **standard, lightweight way to define, share, and launch AI coding environments** across solo, team, and enterprise contexts. It manages:
- **Vaults** — Sources of skills/instructions (local, GitHub, ClawHub marketplace)
- **Assets** — Skills (`SKILL.md`) and Instructions (`AGENTS.md`)
- **Providers** — Claude Code, OpenCode, Copilot, Gemini, Letta, Snowflake, Firebender, AMP
- **MCP Servers** — JSON-RPC servers registered and enabled per-provider
- **Profiles** — Compositions referencing (not duplicating) skills, instructions, providers, vaults, and MCPs

### 3.2 Architecture (Hexagonal / Ports & Adapters)

```
CLI (cli/)  →  App (app/)  →  Domain (domain/)
                    ↓
               Infra (infra/)
                    ↑
TUI (tui/)  →  App (app/)
```

**Dependency rules (enforced by `tests/architecture.rs`):**
- `domain/` is pure — no `std::fs`, no `std::process`
- `app/` depends only on `domain/` and its own `ports/`
- `infra/` implements ports with concrete adapters
- `cli/` and `tui/` are thin adapters — no business logic, no `infra/` imports
- Only `main.rs` and `app/bootstrap/` construct concrete adapters

### 3.3 Feature Dispatch Pattern

```rust
// app/core.rs
pub fn execute(&self, command: CoreCommand, sink: &mut dyn CoreEventSink) -> CoreResult {
    if let Some(r) = crate::app::features::profile::dispatch(&command, self, sink) { return r; }
    if let Some(r) = crate::app::features::vault::dispatch(&command, self, sink) { return r; }
    // ... chain of feature dispatchers
}
```

Each feature (profile, vault, asset, provider, mcp, context, apply, telemetry) owns its `CoreCommand` variants, input structs, and use-case implementations.

### 3.4 TUI / CLI Dual Interface

- **TUI:** Ratatui-based, async runtime loop, `spawn_blocking` for `AgkCore::execute()`, `TuiPresenter` bridges `CoreEvent` back into the async event loop.
- **CLI:** Clap-based, `core_dispatcher.rs` routes all commands through `AgkCore`, `CliPresenter` implements `CoreEventSink` with `--json`, `--quiet`, and normal modes.
- **Contract parity:** Every interactive flow must have a `--dry-run --json` equivalent producing identical `CoreEvent` sequences.

### 3.5 What AGK Does Well

1. **Reproducible environments** — Profiles + vaults + SHA-based change detection = identical AI setups across machines.
2. **Multi-provider abstraction** — Same skill installed to Claude Code and OpenCode simultaneously via `FeatureSetPort` + `ProviderPort`.
3. **Headless-first design** — Every TUI flow has a CLI equivalent; CI/CD ready.
4. **Dependency resolution** — Skills declare `requires:` and `requires_optional:`; circular dependencies rejected, diamond dependencies deduplicated.
5. **Clean config management** — Empty sections auto-pruned from TOML; empty configs removed.
6. **Architecture rigor** — Hexagonal design with mechanical enforcement via architecture tests.
7. **Skill bundling** — Meta-skills auto-install dependency trees.

### 3.6 Where AGK Has Gaps (Relative to VibeCode)

1. **No workflow orchestration** — AGK installs skills, but doesn't guide *how* agents use them. There's no phase-locking, no approval gates, no mode enforcement.
2. **Static context only** — Skills and instructions are static markdown. There's no durable, auto-organizing `process/` directory that grows with the project.
3. **No agent role definitions** — AGK distributes skills, but doesn't define *which agent* should use *which skill* in *which phase*.
4. **No harness versioning** — VibeCode's `process/` directory is living context. AGK has vault versioning (SHA10) but not workflow-state versioning.
5. **No drift detection** — AGK detects when vault assets change (SHA10), but doesn't detect when an AI session drifts from its plan or spec.
6. **No skill activation rules** — VibeCode skills activate via keyword matching. AGK skills are always installed; there's no conditional activation based on intent.
7. **No multi-agent coordination** — AGK is single-user, single-session. VibeCode dispatches to 12+ specialized subagents.

---

## 4. Comparative Matrix

| Dimension | VibeCode Pro Max Kit | AGK |
|-----------|---------------------|-----|
| **Primary Role** | AI agent orchestrator | AI environment package manager |
| **Unit of Distribution** | Process templates, agent definitions, skills | Skills, instructions, MCP servers, profiles |
| **Workflow Phases** | RIPER-5 (Research → Innovate → Plan → Execute → Update) | None (install/configure only) |
| **Phase Safety** | Tool restrictions per phase, mode labels, guards | None |
| **Context Durability** | `process/` directory with auto-promotion | Vault SHA10 hashing; no runtime context |
| **Agent Specialization** | 12 specialized subagents | None (user is the agent) |
| **Skill Discovery** | Keyword-based automatic activation | Manual install/search |
| **Multi-Tool Support** | Claude, Codex, Cursor, Windsurf, Antigravity, OpenCode, Copilot | Claude, OpenCode, Copilot, Gemini, Letta, Snowflake, Firebender, AMP |
| **Approach to Context Decay** | Disk-based durable memory + router pattern | Re-install from vaults |
| **Architecture Style** | Markdown + symlink + hook conventions | Hexagonal Rust (Ports & Adapters) |
| **User Interface** | Natural language commands inside AI chat | TUI + CLI |
| **CI/CD Readiness** | Manual approval gates | Full headless CLI with `--json`/`--quiet` |
| **Config Format** | Markdown frontmatter, JSON manifests | TOML config, YAML frontmatter |
| **Safety Mechanisms** | Privacy hooks, deviation halting, 50% checkpoints | ProcessRunnerPort for sandboxed execution |
| **Dependency Management** | Skill references via `.claude/skills/` | Full dependency resolution with circular/diamond detection |
| **Change Detection** | Manual drift scoring post-execution | SHA10 automatic change detection |
| **Testing Strategy** | Agent compliance monitoring | 6-layer test pyramid (domain → integration) |

---

## 5. Proposal: Evolving AGK into a Harness Orchestrator

### 5.1 Vision

> **AGK becomes the standard way to distribute not just AI skills, but entire AI workflows — including phase definitions, agent roles, process templates, and harness safety rules.**

When a user runs `agk start-profile <name>`, AGK should not only install skills to `~/.claude/skills/`, but also scaffold the `process/` directory, install agent definitions, configure hooks, and set up the RIPER-5 workflow context appropriate to that profile's purpose.

### 5.2 New Asset Types

Extend AGK's asset model beyond `Skill` and `Instruction` to include:

| New Asset Type | Description | File Marker |
|---------------|-------------|-------------|
| **Harness** | A complete workflow template (RIPER-5 phases, agent roles, safety rules) | `HARNESS.md` |
| **Process Pack** | A reusable `process/` subtree (context routers, plan templates, report templates) | `PROCESS.md` |
| **Agent Definition** | A specialized agent role with capabilities and restrictions | `AGENT.md` |
| **Hook** | A lifecycle script (privacy guard, quality nudge, init script) | `HOOK.cjs` / `HOOK.rs` |
| **Skill Trigger** | A keyword-to-skill activation mapping | `TRIGGERS.md` |

### 5.3 Feature: Harness-Aware Profiles

Extend `Profile` to reference harnesses and process packs:

```toml
[profile]
id = "web-app-team"
name = "Web App Team"
description = "Full-stack web development with safety gates"

[[profile.harnesses]]
vault = "agk-community/ripper5-default"
asset = "full-ripper5"
version = "2.1.0"

[[profile.process_packs]]
vault = "my-org/process"
asset = "web-app-context"
version = "1.3.0"

[[profile.agents]]
vault = "agk-community/agents"
asset = "debugger-agent"

[[profile.skills]]
vault = "clawhub"
asset = "react-parser"

[[profile.mcps]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "."]
```

When `agk profile start web-app-team` runs:
1. Install all skills to provider-specific directories (existing behavior).
2. Install all MCP servers and enable them (existing behavior).
3. **NEW:** Scaffold `process/` directory with the harness's router files, plan templates, and protocol rules.
4. **NEW:** Install agent definitions to `.claude/agents/` (or `.codex/agents/` etc.).
5. **NEW:** Install hooks to `.claude/hooks/` with provider-specific adaptations.
6. **NEW:** Write `AGENTS.md` and `CLAUDE.md` composites derived from the profile's harness + agents.

### 5.4 Feature: Process Context Management

Add a `process/` management subsystem to AGK:

```rust
// New port: ProcessContextPort
pub trait ProcessContextPort: Send + Sync {
    fn scaffold(&self, harness: &Harness, workspace: &Path) -> Result<()>;
    fn list_topics(&self, workspace: &Path) -> Result<Vec<Topic>>;
    fn promote_topic(&self, workspace: &Path, topic: &str) -> Result<()>; // Auto-create feature/ after N artifacts
    fn archive_plan(&self, workspace: &Path, plan: &Plan) -> Result<()>;
    fn load_router(&self, workspace: &Path) -> Result<RouterContext>;
}
```

**TUI Integration:**
- New tab: **Process** (tab `5` or `P`)
- Shows active plans, completed plans, backlog, and topic groups.
- Allows creating new plan from template, archiving completed work.
- Visualizes RIPER-5 phase state with color coding.

### 5.5 Feature: Intent-Based Skill Activation

Instead of always installing all skills from a profile, support conditional activation:

```yaml
---
name: react-debugger
version: 1.0.0
requires:
  - clawhub/react-parser
triggers:
  keywords: ["react", "jsx", "component", "hook"]
  file_patterns: ["*.tsx", "*.jsx"]
  phases: ["research", "debug", "execute"]
---
```

AGK would:
1. Install the skill to the provider directory.
2. Write a trigger manifest to `.claude/skills/.triggers/react-debugger.json`.
3. The harness (or a future AGK agent) reads triggers to decide which skills to load into context for a given task.

### 5.6 Feature: Drift Detection & Process Updates

Leverage AGK's existing SHA10 hashing for a new purpose:

```rust
pub fn compute_drift_score(
    original_plan_hash: &str,
    executed_files: &[(PathBuf, Vec<u8>)],
    harness_edits: &[HarnessEdit],
) -> DriftScore {
    // SHA10 of what was planned vs. what was executed
    // + heuristic for harness mutation
}
```

After a coding session:
1. AGK computes drift between the plan (stored in `process/general-plans/active/`) and the resulting code changes.
2. If drift exceeds threshold, suggest `vc-update-process-agent` workflow.
3. Update `process/context/all-context.md` with learnings.

### 5.7 Feature: Multi-Agent Team Support (Future)

AGK's architecture already supports `ProcessRunnerPort` and `ProfileRuntimePort`. Extend this to support **parallel agent teams**:

```rust
pub trait AgentTeamPort: Send + Sync {
    fn spawn_research_agent(&self, context: &TaskContext) -> Result<AgentHandle>;
    fn spawn_plan_agent(&self, context: &TaskContext, research: &ResearchReport) -> Result<AgentHandle>;
    fn spawn_execute_agent(&self, context: &TaskContext, plan: &Plan) -> Result<AgentHandle>;
    fn await_completion(&self, handles: &[AgentHandle]) -> Result<Vec<AgentResult>>;
}
```

This would integrate with Claude Code's `Agent` tool or GitHub Copilot's multi-agent features, using AGK's profile + harness definitions to configure each subagent's context and restrictions.

### 5.8 Implementation Roadmap

| Phase | Deliverable | AGK Feature Area |
|-------|-------------|-----------------|
| **1. Research** | Audit VibeCode's `HARNESS.md`, `AGENT.md`, `PROCESS.md` formats | Asset model (`domain/asset.rs`) |
| **2. Innovate** | Design AGK-native harness asset format (YAML frontmatter + markdown body) | Domain + Ports |
| **3. Plan** | PRD + technical design in `docs/product/features/harness/` | Product docs |
| **4. Execute** | Implement `Harness` asset kind, `ProcessContextPort`, harness-aware profile start | `app/features/harness/`, `app/features/profile/` |
| **5. Update Process** | Add harness distribution to ClawHub; dogfood RIPER-5 in AGK's own development | Infra, docs, CI |

---

## 6. Why This Makes Sense for AGK

1. **Natural Extension** — AGK already manages `.claude/skills/`, `.claude/mcp.json`, and provider configs. Extending to `.claude/agents/`, `.claude/hooks/`, and `process/` is a logical next step.

2. **Value Proposition Amplification** — Today AGK promises "reproducible AI environments." Tomorrow it promises "reproducible, disciplined, self-improving AI engineering workflows."

3. **Leverages Existing Strengths** — AGK's vault system, SHA10 change detection, dependency resolution, and multi-provider abstraction are exactly what's needed to distribute and version harness templates at scale.

4. **Fills VibeCode's Gaps** — VibeCode is a convention-based markdown system with no package manager, no versioning, no dependency resolution, and no centralized marketplace. AGK provides all of these.

5. **Competitive Differentiation** — No other tool combines package management with workflow orchestration. This would make AGK unique in the AI tooling landscape.

---

## 7. Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| **Scope creep** — Harness orchestration is complex | Start with `HARNESS.md` asset distribution only; phase-locking logic lives in the harness, not AGK |
| **Provider fragmentation** — Not all providers support hooks/agents | Graceful degradation: install what the provider supports, warn about unsupported features |
| **User overwhelm** — RIPER-5 adds friction | Make harnesses opt-in per profile; default profiles remain simple |
| **Format wars** — VibeCode, Superpowers, GSD use different conventions | AGK's `ManifestCodecPort` already abstracts TOML/YAML; extend to normalize harness formats |
| **Architecture integrity** — New features could violate hexagonal rules | Follow existing ADR-001 patterns; add architecture tests for `app/features/harness/` |

---

## 8. Conclusion

VibeCode Pro Max Kit represents the state of the art in AI agent *behavioral* harnessing — phase-locking, durable context, and specialized agent roles. AGK represents the state of the art in AI environment *materialization* — reproducible, versioned, multi-provider skill distribution.

**The synthesis is obvious and valuable:** AGK should grow to distribute harnesses, agent definitions, and process templates as first-class assets. This transforms AGK from a package manager into a **complete operating system for AI coding teams** — handling both *what* agents know and *how* they work.

The proposed evolution preserves AGK's architectural rigor while adding the orchestration layer that the VibeCode kit proves is necessary for serious, long-lived AI-assisted engineering.

---

*Report generated 2026-05-29. Based on analysis of `withkynam/vibecode-pro-max-kit` and AGK commit `20c5d66` (refactor-unified branch).*
