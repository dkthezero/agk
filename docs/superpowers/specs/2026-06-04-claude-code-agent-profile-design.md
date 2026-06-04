# Design Spec: Claude Code Agent Profile + LLM Provider Support (v0.4.3)

**Date:** 2026-06-04
**Status:** Draft — Pending Review
**Epic:** v0.4.3 — Claude Code Agent Profile + LLM Provider Support
**Parent:** Continuation of the v0.4 series (Skill Marketplace P1 already shipped)
**Memory:** `v04-claude-agent-profile.md`

---

## 1. Executive Summary

AGK's profile wizard currently supports the `opencode` agent provider. This epic adds **`claude-code`** as a first-class agent provider target and introduces a new **`LlmProvider`** concept (Ollama, LM Studio, Anthropic, OpenAI) so users can bind a profile to a model server. The work is gated behind Cargo features so a Docker runtime build stays minimal.

**Core value:** "I can build a Claude Code sub-agent profile in AGK, point it at Ollama or LM Studio, and ship it."

**Release plan:** Single release `v0.4.3`, three commits:

| Commit | Surface | Title |
|---|---|---|
| **C1** | Domain + ports | "v0.4.3: add LlmProvider domain + LlmProviderPort" |
| **C2** | Infra + use cases | "v0.4.3: wire LLM providers + Claude Code agent markdown" |
| **C3** | Adapters + E2E | "v0.4.3: add CLI/TUI/contract tests + Dockerfile" |

---

## 2. Key Design Decisions

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | **Agent profile = Claude Code sub-agent markdown** (`.claude/agents/<name>.md`). | Confirmed by user. AGK already generates this format. The "agent profile" *is* the existing `Profile` struct, just with new fields. |
| 2 | **LLM provider is a separate concept from agent provider.** New `LlmProviderPort` sibling to `ProviderPort`. | Agent provider = harness (Claude Code, opencode). LLM provider = model server (Ollama, LM Studio). Different roles, different lifecycle. Conflating them forces provider-agnostic install/remove/sessions on a thing that doesn't have any. |
| 3 | **Free-form model string.** `Profile.model: Option<String>`. | Model IDs change faster than AGK can ship. A central catalog becomes a maintenance burden. Provider exec (`claude --agent X`) accepts the string verbatim. |
| 4 | **`Profile` gets exactly 2 new fields**: `model: Option<String>`, `llm_provider_id: Option<LlmProviderId>`. No other bloat. | Honors AGENTS.md charter: "Profiles as compositions / references only." Tool selection, permission mode, color, memory stay in provider-internal state and are serialized to frontmatter by `render_agent_markdown()`. |
| 5 | **Two-axis Cargo feature matrix**: compile surface (cli/tui) × feature set (core, profile-create, llm-providers, ...). | Matches AGENTS.md charter: "Heavy subsystems must be optional via Cargo features." A user can build a Docker runtime with just `cli,core,claude-code-provider,opencode-provider`. |
| 6 | **`profile-create` feature (default on)** gates the create/wizard path. | Docker runtime doesn't need to *create* profiles; only to *start* them. Gating makes the slim build possible. |
| 7 | **`LlmProviderAdapter` and `LlmProviderStorePort` port traits always built.** Impls are feature-gated. | Domain models and port shapes are always available. Only concrete adapters and registries are gated. Matches the "data is always present, behavior is optional" pattern. |
| 8 | **LLM health check returns `LlmHealth { reachable: false, error }`, NOT `Err`.** | A health check that "fails" is *expected* output. The user wants to *see* the failure (Ollama not running). Bubbling it as `Err` exits non-zero and prints a stack trace. |
| 9 | **One release, three commits.** | Each commit is independently revertable. CI runs at every commit. No surprise at the end. |
| 10 | **Multi-stage `Dockerfile` + `docker-compose.yml` for E2E.** | CI runs the full feature set end-to-end against a real AGK binary. The `runtime` stage produces the same artifact a hand-built slim cargo would produce. |

---

## 3. Cargo Feature Matrix

### 3.1 Axes

- **Compile surface:** `cli` (default), `tui`
- **Feature sets:** `core` (default), `claude-code-provider`, `opencode-provider`, plus opt-ins (`profile-create`, `team-sync`, `vault-create`, `policy`, `telemetry-team`, `llm-providers`, `ghes-vault`, `mcp-providers`)

### 3.2 Feature Set (initial)

| Feature | Default | Depends on | What it gates |
|---|:---:|---|---|
| `core` | ✓ | – | scan, install, remove, vault list, profile **start**, asset query, telemetry local — minimum every AGK build must have |
| `claude-code-provider` | ✓ | – | `ClaudeCodeProvider` (install/remove/session/wizard steps for `.claude/agents/*.md`) |
| `opencode-provider` | ✓ | – | `OpenCodeProvider` (existing) |
| `profile-create` | ✓ | – | `agk profile create` + interactive wizard (CLI + TUI), CreateProfileInput model+llm fields |
| `llm-providers` | | – | `LlmProviderPort` impls, Ollama/LM Studio/Anthropic adapters, `agk llm *` |
| `team-sync` | | – | `agk team *` + `agk sync` |
| `vault-create` | | – | `agk vault init` |
| `policy` | | – | `agk policy *` |
| `telemetry-team` | | – | `agk telemetry team-report` |
| `ghes-vault` | | – | GHES vault adapter |
| `mcp-providers` | | – | MCP provider adapters |

`default = ["cli", "tui", "core", "claude-code-provider", "opencode-provider", "profile-create"]`

### 3.3 Cargo.toml

```toml
[features]
default = ["cli", "tui", "core", "claude-code-provider", "opencode-provider", "profile-create"]

cli = []
tui = ["dep:ratatui", "dep:crossterm"]
core = []

profile-create = []
team-sync = ["dep:serde_yaml"]
vault-create = []
policy = []
telemetry-team = ["dep:reqwest"]
llm-providers = ["dep:reqwest"]
claude-code-provider = ["dep:serde_yaml"]
opencode-provider = []
ghes-vault = []
mcp-providers = []
```

### 3.4 Concrete builds

```bash
# Full local dev (default features)
cargo build

# Headless CI / power user
cargo build --no-default-features --features 'cli,core,profile-create,team-sync,policy,llm-providers,claude-code-provider'

# Docker runtime — minimal
cargo build --release --no-default-features --features 'cli,core,claude-code-provider,opencode-provider'
# Supports: agk profile start, agk profile list, agk vault list
# Does NOT exist: agk profile create, agk profile wizard, agk team *, agk sync, agk llm *

# Solo TUI full-fat
cargo build --release --features 'tui,core,profile-create,team-sync,policy,telemetry-team,llm-providers,claude-code-provider,opencode-provider'
```

### 3.5 CI matrix (`.github/workflows/build.yml`)

```yaml
strategy:
  matrix:
    build:
      - name: "full (default)"
        features: ""
      - name: "docker-runtime-min"
        features: "cli,core,claude-code-provider,opencode-provider"
      - name: "headless-ci"
        features: "cli,core,profile-create,team-sync,policy,llm-providers,claude-code-provider"
      - name: "tui-min"
        features: "tui,core,claude-code-provider,opencode-provider"
      - name: "tui-full"
        features: "tui,core,profile-create,team-sync,policy,telemetry-team,llm-providers,claude-code-provider,opencode-provider,ghes-vault,mcp-providers"
```

### 3.6 Feature-gating rules

| Layer | What gets gated | What stays un-gated |
|---|---|---|
| `domain/` | Nothing — all models always built | – |
| `app/features/profile/create.rs`, `wizard.rs` | Whole file behind `#[cfg(feature = "profile-create")]` | `start.rs` always built (Docker needs it) |
| `app/features/llm/*` | Behind `#[cfg(feature = "llm-providers")]` | – |
| `app/ports/llm_provider.rs` | Trait file always built; only impls gated | trait shape always available |
| `infra/llm/*` | Whole module behind `#[cfg(feature = "llm-providers")]` | – |
| `infra/provider/claude_code/*` | Whole module behind `#[cfg(feature = "claude-code-provider")]` | – |
| `cli/features/profile.rs` | `create`/`wizard` subcommands behind `#[cfg(feature = "profile-create")]`; `start`/`list` always | – |
| `cli/features/llm.rs` | Whole file behind `#[cfg(feature = "llm-providers")]` | – |
| `tui/features/profile/wizard/*` | Whole dir behind `#[cfg(feature = "profile-create")]` | – |
| `tui/features/llm/*` | Whole dir behind `#[cfg(feature = "llm-providers")]` | – |
| `app/bootstrap/state.rs` | Conditional `if cfg!(feature = "...")` to construct only enabled adapters | composition root handles absence |

---

## 4. Dockerfile for E2E CI

### 4.1 `Dockerfile` (repo root)

Multi-stage: `builder-full` (compiles all features), `builder-runtime` (slim), `ci-full` (test image with tools), `runtime` (slim production image).

```dockerfile
# syntax=docker/dockerfile:1.7
FROM rust:1.83-bookworm AS builder-full
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN find src -name "*.rs" -exec touch {} +
RUN cargo build --release --locked --features 'cli,tui,core,profile-create,team-sync,vault-create,policy,telemetry-team,llm-providers,claude-code-provider,opencode-provider,ghes-vault,mcp-providers'

FROM rust:1.83-bookworm AS builder-runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked --no-default-features \
    --features 'cli,core,claude-code-provider,opencode-provider'

FROM debian:bookworm-slim AS ci-full
RUN apt-get update && apt-get install -y --no-install-recommends \
    git jq curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder-full /build/target/release/agk /usr/local/bin/agk
RUN mkdir -p /workspace && agk --version && agk --help
WORKDIR /workspace

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder-runtime /build/target/release/agk /usr/local/bin/agk
ENTRYPOINT ["agk"]
CMD ["--help"]
```

### 4.2 `.dockerignore`

```
target
.git
.worktrees
docs
fixtures
*.md
!README.md
```

### 4.3 `docker-compose.yml`

```yaml
version: "3.9"
services:
  agk-e2e:
    build:
      context: .
      target: ci-full
    image: agk:ci-full
    working_dir: /workspace
    volumes:
      - ./fixtures/e2e:/workspace
      - ./target/e2e-artifacts:/artifacts
    environment:
      - AGK_E2E=1
      - RUST_BACKTRACE=1
    command: ["bash", "/workspace/run-e2e.sh"]

  agk-runtime:
    build:
      context: .
      target: runtime
    image: agk:runtime
    entrypoint: ["agk"]
    command: ["--help"]
```

### 4.4 `fixtures/e2e/run-e2e.sh`

Exercises the full happy path: vault init, team init, profile create with claude-code, llm add, dry-run start, sync, policy check, frontmatter verification.

```bash
#!/usr/bin/env bash
set -euo pipefail
agk vault init --name "ci-vault"
agk team init --name "ci-team"
agk team add-vault clawhub-public --type clawhub --url https://clawhub.ai
agk profile create --provider claude-code \
  --name ci-reviewer \
  --model claude-sonnet-4-6 \
  --tools "Read,Grep,Glob" \
  --permission-mode plan
agk llm add ollama --type ollama --endpoint http://localhost:11434
agk llm list
agk profile start ci-reviewer --dry-run --json > /artifacts/launch-plan.json
jq . /artifacts/launch-plan.json
agk sync --dry-run --json > /artifacts/sync-plan.json
jq . /artifacts/sync-plan.json
agk policy check ci-reviewer --vault clawhub-public --json
test -f /workspace/.claude/agents/ci-reviewer.md
grep -q "^name: ci-reviewer$" /workspace/.claude/agents/ci-reviewer.md
grep -q "^model: claude-sonnet-4-6$" /workspace/.claude/agents/ci-reviewer.md
grep -q "^permissionMode: plan$" /workspace/.claude/agents/ci-reviewer.md
echo "✓ E2E passed"
```

### 4.5 `.github/workflows/e2e.yml`

```yaml
name: E2E
on:
  pull_request:
    paths: ['src/**', 'Cargo.toml', 'Cargo.lock', 'Dockerfile', 'fixtures/e2e/**']
jobs:
  e2e:
    runs-on: ubuntu-latest
    timeout-minutes: 20
    steps:
      - uses: actions/checkout@v4
      - name: Build full-feature image
        run: docker build --target ci-full --tag agk:ci-full .
      - name: Run E2E suite
        run: |
          mkdir -p target/e2e-artifacts
          docker compose up agk-e2e --abort-on-container-exit
      - name: Upload artifacts
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: e2e-output
          path: target/e2e-artifacts
```

---

## 5. Domain Models

All in `src/domain/`. **Domain is always built** (no `#[cfg(feature = "...")]`).

### 5.1 `src/domain/llm_provider.rs` (new)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LlmProviderId(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub id: LlmProviderId,
    pub name: String,
    pub provider_type: LlmProviderType,
    pub endpoint: String,
    #[serde(default)]
    pub env_overrides: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LlmProviderType {
    Ollama,
    LmStudio,
    Anthropic,
    OpenAi,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LlmHealth {
    pub provider_id: LlmProviderId,
    pub reachable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_models: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn builtin_llm_providers() -> Vec<(&'static str, LlmProviderType, &'static str)> {
    vec![
        ("ollama", LlmProviderType::Ollama, "http://localhost:11434"),
        ("lmstudio", LlmProviderType::LmStudio, "http://localhost:1234/v1"),
        ("anthropic", LlmProviderType::Anthropic, "https://api.anthropic.com"),
        ("openai", LlmProviderType::OpenAi, "https://api.openai.com/v1"),
    ]
}

pub fn validate_endpoint(endpoint: &str) -> Result<()> {
    let url = url::Url::parse(endpoint)
        .map_err(|e| anyhow!("invalid endpoint URL: {}", e))?;
    if !matches!(url.scheme(), "http" | "https") {
        anyhow::bail!("endpoint must be http or https");
    }
    Ok(())
}
```

### 5.2 `src/domain/profile.rs` (extend)

```rust
pub struct Profile {
    pub id: ProfileId,
    pub scope: Scope,
    pub provider_id: ProviderId,
    pub skill_refs: Vec<ProfileAssetRef>,
    pub mcp_refs: Vec<ProfileAssetRef>,
    pub instruction_refs: Vec<ProfileAssetRef>,
    pub tool_refs: Vec<String>,
    pub permission_mode: Option<String>,
    pub prompt_overlay_path: Option<PathBuf>,
    pub launch_policy: LaunchPolicy,
    // NEW (this epic):
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_provider_id: Option<LlmProviderId>,
}
```

### 5.3 `src/app/ports/llm_provider.rs` (new, always built)

```rust
pub trait LlmProviderAdapter: Send + Sync {
    fn id(&self) -> &str;
    fn provider_type(&self) -> LlmProviderType;
    fn default_endpoint(&self) -> &str;
}

pub trait LlmProviderStorePort: Send + Sync {
    fn list(&self) -> Result<Vec<LlmProviderConfig>>;
    fn get(&self, id: &LlmProviderId) -> Result<Option<LlmProviderConfig>>;
    fn save(&self, config: &LlmProviderConfig) -> Result<()>;
    fn remove(&self, id: &LlmProviderId) -> Result<()>;
}

pub trait LlmHealthCheckPort: Send + Sync {
    fn check(&self, endpoint: &str, path: Option<&str>) -> Result<LlmHealth>;
}
```

### 5.4 `src/app/ports/provider.rs` (extend `WizardStep`)

```rust
pub enum WizardStep {
    // ... existing variants ...
    ModelInput { title: String, allowed_pattern: Option<String> },
    LlmProviderSelect { title: String, providers: Vec<(LlmProviderId, String)> },
    ColorSelect { title: String, options: Vec<String> },
    MemoryScopeSelect { title: String, options: Vec<(String, String)> },
    MaxTurnsInput { title: String, default: Option<u32> },
    /// Select an agent provider from the registry. Always shown first when
    /// the wizard is invoked without `--provider`. Injected by the dispatcher,
    /// not by a specific provider impl.
    ProviderSelect { title: String, providers: Vec<(ProviderId, String)> },
}
```

### 5.5 Validation rules (domain-pure)

- `Profile.model`: any UTF-8 string ≤256 chars
- `LlmProviderConfig.endpoint`: must parse as http/https URL
- `LlmProviderId`: `^[a-z][a-z0-9-]{1,32}$`

---

## 6. Application Core

### 6.1 `src/app/features/llm/` (gated on `llm-providers`)

```
src/app/features/llm/
├── mod.rs              # dispatch
├── command.rs          # RegisterLlmProviderInput, LlmHealthCheckInput
├── list.rs
├── add.rs
├── remove.rs
├── test.rs
└── registry.rs
```

`add.rs::run` validates the endpoint, ensures ID uniqueness, persists, emits `LlmProviderRegistered`. `test.rs::run` calls `LlmHealthCheckPort::check` and emits `LlmHealthChecked` (NOT `Err` on unreachable — see Section 8).

### 6.2 `src/app/features/profile/wizard.rs` (new, gated on `profile-create`)

Drives the interactive flow. Emits `CoreEvent::WizardStepRequest` for each step from the provider's `profile_wizard_steps()`. On final commit, emits `CoreCommand::CreateProfile`.

Wizard step order for `claude-code`:

```
1. TextInput           { "Profile name?" }
2. ProviderSelect      (registry)
3. ModelInput          { "Model?" }
4. ToolSelect
5. PermissionSelect
6. ColorSelect
7. MemoryScopeSelect
8. ScopeSelect
9. Review
```

For `opencode`, steps 3-7 are skipped.

### 6.3 `src/app/features/profile/create.rs` (extend, gated on `profile-create`)

```rust
pub struct CreateProfileInput {
    pub id: ProfileId,
    pub provider_id: ProviderId,
    pub skill_refs: Vec<ProfileAssetRef>,
    pub mcp_refs: Vec<ProfileAssetRef>,
    pub instruction_refs: Vec<ProfileAssetRef>,
    pub description: String,
    pub scope: Scope,
    // NEW:
    pub model: Option<String>,
    pub llm_provider_id: Option<LlmProviderId>,
}
```

`run()` validates `llm_provider_id` exists in `LlmProviderStorePort` (if provided), persists to `ConfigStore`, emits `ProfileCreated` (extended with model).

### 6.4 `src/app/features/profile/start.rs` (extend, always built)

`LaunchPlan` gets new fields. `build_launch_plan()` resolves `llm_provider_id` → endpoint at plan-build time. Color / memory / maxTurns are agent-decoration fields populated by the wizard (Section 6.2) and persisted on the plan so the provider's `start_profile_session()` doesn't need to reach back into the registry:

```rust
pub struct LaunchPlan {
    // ... existing ...
    pub model: Option<String>,
    pub llm_provider_id: Option<LlmProviderId>,
    pub llm_endpoint: Option<String>,         // resolved from store at plan-build time
    pub agent_color: Option<String>,          // from wizard; None means no color
    pub agent_memory: Option<String>,        // "user" | "project" | "local" | None
    pub agent_max_turns: Option<u32>,
    pub prompt_body: Option<String>,         // pre-loaded from prompt_overlay_path
}
```

These fields are `Option` so they are `None` for `opencode` profiles (which don't use them) and for any profile created via `agk profile create` without the new flags.

### 6.5 `src/infra/llm/` (gated on `llm-providers`)

```
src/infra/llm/
├── mod.rs
├── store.rs            # LlmProviderStorePort impl (TOML)
├── registry.rs         # LlmProviderRegistry
├── health.rs           # ReqwestHealthCheck
├── ollama.rs
├── lmstudio.rs
├── anthropic.rs
└── openai.rs
```

Health-check semantics:

| Provider | Path | Success |
|---|---|---|
| Ollama | `GET /api/tags` | 200 |
| LM Studio | `GET /v1/models` | 200 |
| Anthropic | `POST /v1/messages` with min body | 400 (reachable, auth needed) |
| OpenAI | `GET /v1/models` | 200 |

### 6.6 `src/infra/provider/claude_code/agent_markdown.rs` (gated on `claude-code-provider`)

```rust
/// Pure function: takes a profile + launch plan, returns the full
/// `.claude/agents/<name>.md` content (frontmatter + body).
/// No I/O. Tested with golden output in `tests/`.
pub fn render_agent_markdown(
    profile: &Profile,
    launch_plan: &LaunchPlan,
) -> String {
    let mut fm = serde_yaml::Mapping::new();
    fm.insert("name".into(), profile.id.as_str().into());
    fm.insert("description".into(), profile.description.clone().into());
    fm.insert("model".into(), launch_plan.model.clone()
        .unwrap_or_else(|| "inherit".into()).into());
    if !profile.tool_refs.is_empty() {
        fm.insert("tools".into(), profile.tool_refs.join(",").into());
    }
    if let Some(pm) = &profile.permission_mode {
        fm.insert("permissionMode".into(), pm.clone().into());
    }
    if let Some(c) = &launch_plan.agent_color {
        fm.insert("color".into(), c.clone().into());
    }
    if let Some(m) = &launch_plan.agent_memory {
        fm.insert("memory".into(), m.clone().into());
    }
    if let Some(t) = launch_plan.agent_max_turns {
        fm.insert("maxTurns".into(), t.into());
    }
    // mcpServers: from profile.mcp_refs resolved via mcp_registry
    // body: from profile.prompt_overlay_path if set, else empty
    let yaml = serde_yaml::to_string(&fm).unwrap_or_default();
    let body = launch_plan.prompt_body.clone().unwrap_or_default();
    format!("---\n{yaml}---\n\n{body}")
}
```

All decoration fields (`color`, `memory`, `max_turns`, `mcp_servers`, `prompt_body`) come from `LaunchPlan` (Section 6.4), not from `Profile`. The `Profile` struct stays lean per decision #4.

---

## 7. CLI Adapter

### 7.1 `src/cli/entry.rs` (extend Clap)

```rust
#[derive(Subcommand)]
pub enum Commands {
    #[cfg(feature = "profile-create")]
    #[command(subcommand)] Profile(ProfileCommands),
    #[cfg(feature = "llm-providers")]
    #[command(subcommand)] Llm(LlmCommands),
}

#[cfg(feature = "profile-create")]
#[derive(Subcommand)]
pub enum ProfileCommands {
    /// Always available
    Start { id: String, #[arg(long)] dry_run: bool, #[arg(long)] json: bool },
    List { #[arg(long)] json: bool },
    /// Gated
    Wizard { #[arg(long)] provider: Option<String>, #[arg(long)] scope: Option<String> },
    Create {
        name: String,
        #[arg(long)] provider: String,
        #[arg(long)] model: Option<String>,
        #[arg(long, name = "llm")] llm: Option<String>,
        #[arg(long, value_delimiter = ',')] tools: Vec<String>,
        #[arg(long, name = "permission-mode")] permission_mode: Option<String>,
        #[arg(long)] color: Option<String>,
        #[arg(long)] memory: Option<String>,
        #[arg(long, default_value = "workspace")] scope: String,
    },
}

#[cfg(feature = "llm-providers")]
#[derive(Subcommand)]
pub enum LlmCommands {
    List { #[arg(long)] json: bool },
    Add { id: String, #[arg(long, name = "type")] provider_type: String,
          #[arg(long)] endpoint: String, ... },
    Remove { id: String },
    Test { id: String, #[arg(long)] json: bool },
}
```

### 7.2 `src/cli/presenter.rs` (always built)

Adds renderers for the new `CoreEvent` variants. Exhaustive `match` — missing variant = compile error.

### 7.3 CLI examples

```bash
# Full build
agk profile wizard --provider claude-code
agk profile create code-reviewer \
  --provider claude-code \
  --model claude-sonnet-4-6 \
  --tools "Read,Grep,Glob" \
  --permission-mode plan \
  --color blue \
  --memory project
agk llm add ollama --type ollama --endpoint http://localhost:11434
agk llm test ollama
agk profile start code-reviewer --dry-run --json

# Docker runtime build (no wizard, no llm)
agk profile start code-reviewer --dry-run
agk profile list
```

---

## 8. TUI Adapter

### 8.1 `src/tui/features/profile/wizard/` (gated on `profile-create`)

```
wizard/
├── mod.rs
├── controller.rs       # Keystroke → AppEvent::ExecuteCommand
├── state.rs            # WizardState { steps, step_index, answers, ... }
└── view.rs             # Render current step
```

### 8.2 `src/tui/features/llm/` (gated on `llm-providers`)

```
llm/
├── mod.rs
├── controller.rs       # F8 tab, t = test, a = add, d = remove
└── widget.rs           # Render LLM provider list + health
```

### 8.3 Key bindings

| Tab | Key | Action | Feature |
|---|---|---|---|
| Profiles | `w` | Open profile wizard | `profile-create` |
| Profiles | `m` | Quick-set model | `profile-create` |
| Profiles | `M` | Quick-set LLM provider | `llm-providers` |
| Profiles | `Enter` | Start profile | always |
| LLM (F8) | `a` | Add LLM provider modal | `llm-providers` |
| LLM (F8) | `d` | Remove LLM provider | `llm-providers` |
| LLM (F8) | `t` | Test LLM health | `llm-providers` |

### 8.4 `src/tui/runtime_loop.rs` (always built)

Translates `CoreEvent::WizardStepRequest` to `AppState.wizard` mutations. State mutation stays in the runtime loop, not in controllers.

---

## 9. Error Handling

### 9.1 Rules (per AGENTS.md anti-patterns)

| Source | Rule |
|---|---|
| Domain | Return `anyhow::Result`. No event emission. |
| Use case (create, add, remove) | **Never** swallow. `Err(e) => Some(Err(e))`. Emit `TaskFailed` AND return `Err`. |
| Wizard | Esc → `Ok(Ok(CoreOutcome::Ok))`. Invalid step → `ValidationFailed` event, stay on step. |
| LLM health check | Return `LlmHealth { reachable: false, error: Some(...) }`, NOT `Err`. User wants to *see* the failure. |
| CLI mapper | `anyhow::Result` with Clap-style error message. |
| TUI controller | Defensive: `Ok(ControlFlow::Continue)` on bad state. Never panic. |
| TUI runtime loop | `TaskFailed` → status bar with auto-clear after 5s. |

### 9.2 New `CoreEvent` variants

```rust
pub enum CoreEvent {
    // ... existing ...
    WizardStepRequest { step: Box<WizardStep> },
    ValidationFailed { field: String, reason: String },
    LlmProviderRegistered { id: LlmProviderId, endpoint: String, provider_type: LlmProviderType },
    LlmProviderRemoved { id: LlmProviderId },
    LlmHealthChecked { id: LlmProviderId, reachable: bool,
                       available_models: Option<Vec<String>>,
                       error: Option<String> },
}
```

### 9.3 New `CoreCommand` variants

```rust
pub enum CoreCommand {
    // ... existing ...
    #[cfg(feature = "profile-create")] CreateProfile { input: CreateProfileInput },
    #[cfg(feature = "profile-create")] RunProfileWizard { provider_id: ProviderId, scope: Scope },
    #[cfg(feature = "llm-providers")] ListLlmProviders { json: bool },
    #[cfg(feature = "llm-providers")] RegisterLlmProvider { input: RegisterLlmProviderInput },
    #[cfg(feature = "llm-providers")] RemoveLlmProvider { id: LlmProviderId },
    #[cfg(feature = "llm-providers")] CheckLlmHealth { id: LlmProviderId, json: bool },
}
```

---

## 10. Contract Parity

### 10.1 `--dry-run --json` coverage

| Command | Dry-run | JSON |
|---|---|---|
| `agk profile start X` | ✓ (extended) | ✓ (LaunchPlan with model + endpoint) |
| `agk profile list` | n/a | ✓ (extended with model + llm_provider_id) |
| `agk llm add X` | n/a | ✓ (registered provider) |
| `agk llm test X` | n/a (read-only) | ✓ (LlmHealth) |
| `agk llm list` | n/a | ✓ (provider list) |
| `agk profile create` | n/a (side-effecting) | ✓ (saved profile) |

### 10.2 Golden fixtures (`fixtures/contracts/`)

```
profile_start_dry_run_claude_code.json       # NEW
profile_list_with_model.json                # NEW
llm_health_ollama_reachable.json             # NEW
llm_health_ollama_unreachable.json           # NEW
profile_create_claude_code_saved.json       # NEW
```

### 10.3 Backward compatibility

| Change | Impact |
|---|---|
| `Profile` gets 2 new `Option` fields with `#[serde(default)]` | **None** — existing profiles deserialize unchanged |
| `WizardStep` gets 6 new variants | **None** — exhaustive `match` forces migration in 1 place |
| `CoreCommand`/`CoreEvent` get new variants | **None** — exhaustive `match` forces migration in 1 place |
| `Cargo.toml` features added | Existing default builds now include `claude-code-provider` + `profile-create` |
| `.claude/agents/<name>.md` format | Only AGK-generated files change; user files untouched |

---

## 11. Testing Strategy

| Layer | New tests |
|---|---|
| Domain | `validate_endpoint_rejects_non_http`, `validate_model_string_rejects_too_long`, `LlmProviderId_validation`, `builtin_llm_providers_includes_ollama_and_lmstudio`, `Profile_with_model_and_llm_provider_round_trips_through_serde` |
| Use case | `add_llm_provider_succeeds`, `add_llm_provider_duplicate_id_fails`, `test_llm_provider_unreachable_returns_unhealthy_not_error`, `create_profile_with_model_persists`, `create_profile_with_invalid_llm_provider_id_fails`, `wizard_full_flow_claude_code_builds_correct_create_input` |
| Contract | `profile_start_dry_run_claude_code_matches_fixture`, `llm_health_ollama_reachable_matches_fixture`, `cli_tui_parity_for_create_profile_claude_code` |
| Snapshot (TUI) | `wizard_claude_code_step_1_renders_correctly`, `llm_list_with_three_providers` |
| Binary integration (`assert_cmd`) | `agk profile create --provider claude-code --model X` writes correct frontmatter; `agk llm add ollama` writes `llm_providers.toml` |
| Architecture | `claude_code_provider_module_only_compiles_with_feature`, `slim_build_cargo_check_succeeds` (run `cargo check --no-default-features --features 'cli,core' --tests` in CI) |

---

## 12. Acceptance Criteria

### C1 — Domain + ports

- [ ] `cargo build` succeeds with default features
- [ ] `cargo build --no-default-features --features 'cli,core'` succeeds
- [ ] `cargo test --test architecture` passes (zero allowlist growth)
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean
- [ ] `cargo fmt --check` clean
- [ ] `LlmProviderConfig`, `LlmProviderId`, `LlmProviderType`, `LlmHealth` exist
- [ ] `LlmProviderAdapter` and `LlmProviderStorePort` traits exist
- [ ] `builtin_llm_providers()` returns 4 adapters
- [ ] `Profile` round-trips with `model` and `llm_provider_id`
- [ ] `Profile` round-trips legacy fixture without those fields (both `None`)

### C2 — Infra + use cases

- [ ] `cargo build` succeeds for every matrix cell
- [ ] `agk llm add ollama --type ollama --endpoint http://localhost:11434` writes `llm_providers.toml`
- [ ] `agk llm list --json` emits the registered provider
- [ ] `agk llm test ollama --json` returns `{"reachable": ...}` (not exit-code error)
- [ ] `agk profile create --provider claude-code --model X` persists
- [ ] `agk profile create --provider claude-code --llm ollama` persists
- [ ] `agk profile create` without `--provider` fails clearly
- [ ] `agk profile create --llm nonexistent` fails clearly
- [ ] `agk profile start X --dry-run --json` includes resolved `llm_endpoint`
- [ ] `.claude/agents/<name>.md` written with correct frontmatter
- [ ] `render_agent_markdown` is pure (no I/O) — unit tested with no tempdir
- [ ] `claude` binary missing → clear error, no panic
- [ ] All 4 health adapters tested with `wiremock`

### C3 — Adapters + E2E

- [ ] `agk profile wizard` interactive end-to-end (claude-code)
- [ ] `agk profile wizard --provider opencode` still works (regression)
- [ ] TUI: `w`, `m`, `M`, F8 keybindings work
- [ ] All 6 contract fixtures pass
- [ ] `docker build --target ci-full .` succeeds
- [ ] `docker compose up agk-e2e` exits 0
- [ ] Slim binary: `cargo build --release --no-default-features --features 'cli,core,claude-code-provider,opencode-provider'` works
- [ ] Slim binary: `agk --help` does NOT list `create`/`wizard` under `profile`
- [ ] Help overlay is honest about feature availability
- [ ] README updated

---

## 13. Out of Scope (deferred)

| Feature | Why deferred |
|---|---|
| Auto-probe local LLM runtimes at startup | User said "free-form model string". AGK doesn't connect. |
| Skill signing / GPG provenance | Same deferred decision as v0.4 Marketplace P2. |
| Anthropic/OpenAI direct API execution | AGK is config manager, not runtime. Provider exec does the call. |
| Model picker UI (catalog) | Free-form string. Catalog is YAGNI. |
| Migrating opencode wizard to new `ModelInput` variant | opencode's wizard still works. Refactoring is YAGNI. |
| Per-wizard-step pattern validation (`allowed_pattern` field unused) | Defer until a provider asks. |
| TUI live Claude Code session rendering | AGK waits for process exit. |
| Hot-reload of `llm_providers.toml` | Manual refresh. |
| Multi-profile-batch creation | Single-profile is enough for v0.4.3. |

---

## 14. Risk Register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Claude Code frontmatter schema changes | Medium | Medium | `render_agent_markdown` in 1 file with 1 snapshot test. Bounded blast radius. |
| `claude` binary version skew | Low | Low | Don't validate against version. Worst case: field silently ignored. |
| `reqwest` bloats slim runtime | Low | Medium | `reqwest` gated on `llm-providers`. Docker runtime skips. |
| Port trait always builds but only has impls when feature on | Low | Low | CI runs slim build and confirms. |
| User confusion: `claude` on PATH but `claude --agent X` says "agent not found" | Medium | Medium | Wizard detects `claude` version, clear error. |
| TUI wizard: pressing Enter on last step doesn't emit CreateProfile | Medium | High | Snapshot test for final state. Contract test. |
| Profile schema migration: existing profiles fail to load | Low | High | Field is `Option` + `#[serde(default)]`. Tested with legacy fixture. |

---

## 15. Decision Log

| Date | Decision |
|---|---|
| 2026-06-04 | Agent profile = Claude Code sub-agent markdown. Confirmed. |
| 2026-06-04 | LLM provider is **separate** from agent provider. New `LlmProviderPort`. |
| 2026-06-04 | Free-form model string. No catalog. |
| 2026-06-04 | `Profile` gets exactly 2 new fields. No bloat. |
| 2026-06-04 | Two-axis Cargo feature matrix (compile surface × feature set). |
| 2026-06-04 | Docker runtime build = `cli + core + claude-code + opencode` only. |
| 2026-06-04 | Multi-stage `Dockerfile` (ci-full + runtime). `docker-compose.yml` drives E2E. |
| 2026-06-04 | One release, three commits. |
| 2026-06-04 | `LlmProviderAdapter` and `LlmProviderStorePort` traits always built; impls gated. |
| 2026-06-04 | LLM health check returns `LlmHealth { reachable: false }`, NOT `Err`. |
| 2026-06-04 | `WizardStep` gets 6 new variants (additive). |

---

## 16. Files Inventory (full)

### New files

| File | Purpose | Feature gate |
|---|---|---|
| `src/domain/llm_provider.rs` | LLM provider domain models | none |
| `src/app/ports/llm_provider.rs` | LLM provider port traits | none |
| `src/app/features/llm/{mod,command,list,add,remove,test,registry}.rs` | LLM use cases | `llm-providers` |
| `src/app/features/profile/wizard.rs` | Wizard driver | `profile-create` |
| `src/infra/llm/{mod,store,registry,health,ollama,lmstudio,anthropic,openai}.rs` | LLM infra | `llm-providers` |
| `src/infra/provider/claude_code/agent_markdown.rs` | Frontmatter renderer | `claude-code-provider` |
| `src/cli/features/llm.rs` | CLI LLM mapper | `llm-providers` |
| `src/tui/features/profile/wizard/{mod,controller,state,view}.rs` | TUI wizard | `profile-create` |
| `src/tui/features/llm/{mod,controller,widget}.rs` | TUI LLM tab | `llm-providers` |
| `Dockerfile` | Multi-stage build | n/a |
| `docker-compose.yml` | E2E orchestration | n/a |
| `.dockerignore` | Docker ignore | n/a |
| `.github/workflows/e2e.yml` | E2E CI | n/a |
| `fixtures/e2e/run-e2e.sh` | E2E script | n/a |
| `fixtures/contracts/profile_start_dry_run_claude_code.json` | Fixture | n/a |
| `fixtures/contracts/profile_list_with_model.json` | Fixture | n/a |
| `fixtures/contracts/llm_health_ollama_reachable.json` | Fixture | n/a |
| `fixtures/contracts/llm_health_ollama_unreachable.json` | Fixture | n/a |
| `fixtures/contracts/profile_create_claude_code_saved.json` | Fixture | n/a |
| `docs/product/features/claude-code-provider/{prd,technical_design}.md` | PRD + tech design | n/a |

### Extended files

| File | Change | Feature gate |
|---|---|---|
| `src/domain/profile.rs` | +2 fields on `Profile` | none |
| `src/app/ports/provider.rs` | +5 `WizardStep` variants | none |
| `src/app/command.rs` | +6 `CoreCommand` variants | gated |
| `src/app/event.rs` | +5 `CoreEvent` variants | none |
| `src/app/features/profile/create.rs` | +2 fields on input, validation | `profile-create` |
| `src/app/features/profile/start.rs` | +3 fields on `LaunchPlan`, resolve endpoint | none (always) |
| `src/infra/provider/claude_code/session.rs` | Use `render_agent_markdown` | `claude-code-provider` |
| `src/infra/config/toml_store.rs` | Load/save `llm_providers.toml` | none |
| `src/cli/features/profile.rs` | Add `create`/`wizard` branches | `profile-create` |
| `src/cli/entry.rs` | Add `Llm` + extend `Profile` enums | gated |
| `src/cli/presenter.rs` | Render new `CoreEvent` variants | none |
| `src/cli/core_dispatcher.rs` | Route new `CoreCommand` variants | none |
| `src/tui/app.rs` | Add `wizard`, `llm_list`, `llm_health_results` fields | gated |
| `src/tui/features/profile/controller.rs` | Add `w`/`m`/`M` handlers | gated |
| `src/tui/features/common/keybindings.rs` | Help overlay entries | gated |
| `src/tui/runtime_loop.rs` | Handle `WizardStepRequest` | none |
| `src/tui/tab_kind.rs` | Add `Tab::ProfileWizard`, `Tab::LlmProviders` | gated |
| `src/app/bootstrap/state.rs` | Conditional adapter construction | gated |
| `tests/architecture.rs` | New feature-gate tests | none |
| `tests/contracts.rs` | New contract tests | none |
| `Cargo.toml` | New features, deps | n/a |
| `.github/workflows/build.yml` | Add feature matrix | n/a |
| `README.md` | Document new commands + build matrix | n/a |

---

## 17. Estimated Effort

| Layer | Files | LOC (incl. tests) |
|---|---|---|
| Domain | 1 new + 1 extend | ~150 |
| Ports | 1 new | ~80 |
| Infra (LLM) | 7 new | ~600 |
| Infra (Claude Code) | 1 new + 1 extend | ~250 |
| Use cases (LLM) | 4 new | ~400 |
| Use cases (profile) | 1 new + 2 extend | ~400 |
| CLI | 2 extend + 1 new | ~350 |
| TUI | 4 new + 2 extend | ~700 |
| Tests | 10+ | ~1500 |
| Dockerfile / compose / CI | 3 new | ~250 |
| Docs | 3 | ~600 |
| **Total** | ~35 new + ~10 extend | **~5,300 LOC** |

Three PRs of ~1,500-2,000 LOC each. Each independently shippable.

---

*Design Spec v0.1 — 2026-06-04 — Pending User Review*
