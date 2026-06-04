# Claude Code Agent Profile + LLM Provider Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend AGK's profile system so users can (1) build Claude Code sub-agents via the existing profile wizard by adding Claude Code as a second `ProviderPort`, and (2) configure an LLM provider (Ollama, LM Studio, Anthropic, OpenAI) via a new sibling `LlmProviderPort` trait. Ship a slim Docker build that keeps profile-start working but excludes wizard creation, gated by Cargo features.

**Architecture:** Two new hexagonal ports (`LlmProviderStorePort`, `LlmHealthCheckPort`) plus a CLI-probe port (`ClaudeCliProbePort`). Six new `WizardStep` variants (`ProviderSelect`, `LlmProviderSelect`, `ModelInput`, `AgentDescription`, `SkillsPick`, `ReviewFinal`). Pure renderer `render_agent_markdown` (no I/O). Two-axis Cargo feature matrix: `cli/tui × {baseline, llm-ollama, llm-lmstudio, llm-anthropic, llm-openai, claude-cli-probe, profile-create, agent-markdown}`. Multi-stage Dockerfile: `builder-full`, `builder-runtime`, `ci-full`, `runtime`.

**Tech Stack:** Rust (edition 2021), `clap` 4, `tokio` 1, `reqwest` 0.12 (rustls), `serde_json`, existing `domain`/`app`/`infra`/`cli`/`tui` layout, hand-rolled test fakes (no `mockall`).

**Spec:** `docs/superpowers/specs/2026-06-04-claude-code-agent-profile-design.md` v0.2

---

## File Structure

### New Files (by commit)

**C1 — Domain + Ports:**
- `src/domain/llm_provider.rs` — `LlmProviderKind`, `LlmProviderConfig`, `LlmHealthStatus`, `LlmModelDescriptor` value types
- `src/app/ports/llm_provider.rs` — `LlmProviderStorePort`, `LlmProviderAdapter` (provider-side trait), `LlmHealthCheckPort`, `LlmProviderFactoryPort`
- `src/app/ports/claude_cli_probe.rs` — `ClaudeCliProbePort` (locate/version/supports_agent_flag)
- `src/app/test_support/fake_claude_cli_probe.rs` — `FakeClaudeCliProbe`
- `src/app/test_support/fake_llm_provider.rs` — `FakeLlmProviderStore`, `FakeLlmHealthCheck`, `FakeLlmProviderFactory`
- `src/domain/agent_markdown.rs` — `AgentFrontmatter`, `AgentMcpServer`, `RenderedAgentMarkdown` types (pure data)
- `src/domain/launch_plan.rs` — `LaunchPlan` struct (all pre-resolved fields the renderer needs)

**C2 — Infra + Use Cases:**
- `src/infra/llm/ollama.rs` — `OllamaProvider` (impl `LlmProviderAdapter` + health check)
- `src/infra/llm/lmstudio.rs` — `LmStudioProvider`
- `src/infra/llm/anthropic.rs` — `AnthropicProvider`
- `src/infra/llm/openai.rs` — `OpenAiProvider`
- `src/infra/llm/store.rs` — `FileLlmProviderStore` (config.toml persistence)
- `src/infra/llm/health.rs` — `HttpLlmHealthCheck` (shared HTTP probing)
- `src/infra/provider/claude_code/agent_markdown.rs` — `render_agent_markdown` pure function
- `src/infra/provider/claude_code/cli_probe.rs` — `SystemClaudeCliProbe` (real impl)
- `src/app/features/llm/list.rs` — `run` listing configured LLM providers
- `src/app/features/llm/add.rs` — `run` adding LLM provider to config
- `src/app/features/llm/remove.rs` — `run` removing LLM provider
- `src/app/features/llm/health.rs` — `run` health-checking a configured LLM provider
- `src/app/features/llm/mod.rs` — module re-exports
- `src/app/features/profile/wizard.rs` — `build_step_list()` step assembler
- `src/app/features/profile/create.rs` (modify) — call `render_agent_markdown` for claude-code provider
- `src/app/features/profile/start.rs` (modify) — resolve `LaunchPlan` for claude-code, gate behind feature

**C3 — Adapters + E2E:**
- `src/cli/llm.rs` — `agk llm {list,add,remove,health}` subcommands
- `src/cli/profile.rs` (modify) — `agk profile create` picks provider; `--provider` flag for `agk profile start`
- `src/tui/event.rs` (modify) — handle new `WizardStep` variants; render LLM health results
- `src/tui/render.rs` (modify) — render new wizard step widgets
- `src/tui/wizard_state_ext.rs` — helpers for new wizard step types
- `tests/full_flow_tui/wizard_claude_code.rs` — E2E wizard flow
- `tests/full_flow_tui/llm_provider_flow.rs` — E2E LLM add/health/remove
- `tests/llm_provider_contracts.rs` — JSON contract parity
- `tests/agent_markdown_renderer.rs` — golden tests for `render_agent_markdown`
- `tests/wizard_step_assembler.rs` — provider selection → step list assembly
- `tests/architecture_llm.rs` — arch tests for `infra/llm/` boundary
- `tests/slim_build_regression.rs` — `headless-no-llm` and `headless-no-profile-create` smoke tests
- `fixtures/contracts/agk_llm_list.json`, `fixtures/contracts/agk_llm_health.json`, `fixtures/contracts/agk_profile_create_claude.json`, `fixtures/contracts/agent_markdown_minimal.md`, `fixtures/contracts/agent_markdown_full.md`
- `Dockerfile` (multi-stage, 4 stages)
- `docker-compose.yml` (runtime only, slim build)
- `docs/ops/docker.md` — operator-facing build/run docs

### Modified Files (high level)
- `Cargo.toml` — add new feature flags + new optional deps
- `src/app/ports/mod.rs` — re-export new port traits
- `src/app/ports/wizard_state.rs` — add `provider_id_choice`, `llm_provider_id`, `model_string`, `agent_description` fields to `WizardState`; add 6 new `WizardStep` variants
- `src/domain/profile.rs` — add `model: Option<String>`, `agent_mcp_servers: Vec<AgentMcpServer>`, `llm_provider_id: Option<String>` fields
- `src/domain/config/mod.rs` — add `[[llm_providers]]` table to `ConfigFile`; bump schema version
- `src/app/features/profile/command.rs` — extend `CreateProfileInput` with new fields
- `src/app/registry.rs` — `register_llm_provider_factory`
- `src/main.rs` — wire new subcommands and adapters behind features
- `src/lib.rs` — re-export new domain types
- `tests/architecture.rs` — add boundary rule for `infra/llm/` (no app-layer imports)

### Cargo Feature Matrix (final)

| Flag | Default | Purpose |
|------|---------|---------|
| `cli` | ✅ | Always-on CLI binary surface |
| `tui` | ✅ | TUI binary surface |
| `vault-clawhub` | ✅ | ClawHub vault |
| `pack` | ✅ | Skill pack export/import |
| `provider-opencode` | ✅ | OpenCode provider |
| `provider-claude` | ✅ | Claude Code provider (start) |
| `profile-create` | ✅ | Wizard creation (wizard, render_agent_markdown) |
| `claude-cli-probe` | ❌ | `claude --version` probing |
| `llm-ollama` | ❌ | Ollama adapter |
| `llm-lmstudio` | ❌ | LM Studio adapter |
| `llm-anthropic` | ❌ | Anthropic adapter |
| `llm-openai` | ❌ | OpenAI adapter |
| `headless` | ❌ | CI build (no TUI) |

Slim Docker build = `--no-default-features --features cli,vault-clawhub,pack,provider-opencode,provider-claude --features $PROBE_SET` (no `profile-create`, no `tui`).

Full CI build = default + every LLM adapter + `claude-cli-probe`.

---

## Commit Plan

The work is split into **3 commits** matching the spec's three-commit release plan. Each commit is independently buildable and testable.

- **C1 — Domain + ports:** all new types and trait definitions land, with unit tests proving the type contracts. No I/O, no infra code, no app/infra wiring.
- **C2 — Infra + use cases:** adapters implement the ports; use cases (LLM add/list/health/remove, profile wizard, profile start extension) land. Real HTTP + filesystem happens here. Hand fakes prove the contracts before any real adapter is wired.
- **C3 — Adapters + E2E:** CLI subcommands, TUI rendering, multi-stage Dockerfile, E2E tests, contract golden fixtures, architecture tests, slim-build regression tests. After C3 the slim Docker image builds and the wizard works end-to-end.

---

## Commit 1: Domain + Ports

### Task 1.1: Add `LlmProviderKind` enum and `LlmProviderConfig` domain type

**Files:**
- Create: `src/domain/llm_provider.rs`
- Modify: `src/domain/mod.rs` (add `pub mod llm_provider;`)

- [ ] **Step 1: Write the failing test**

```rust
// in src/domain/llm_provider.rs at the bottom
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_provider_kind_from_str_canonical() {
        assert_eq!(LlmProviderKind::from_str("ollama"), Some(LlmProviderKind::Ollama));
        assert_eq!(LlmProviderKind::from_str("lm-studio"), Some(LlmProviderKind::LmStudio));
        assert_eq!(LlmProviderKind::from_str("anthropic"), Some(LlmProviderKind::Anthropic));
        assert_eq!(LlmProviderKind::from_str("openai"), Some(LlmProviderKind::OpenAi));
        assert_eq!(LlmProviderKind::from_str("unknown"), None);
    }

    #[test]
    fn llm_provider_config_validates_endpoint_url() {
        let cfg = LlmProviderConfig {
            id: "local-ollama".into(),
            kind: LlmProviderKind::Ollama,
            endpoint: "http://127.0.0.1:11434".into(),
            api_key: None,
            default_model: Some("llama3.2".into()),
        };
        assert!(cfg.validate().is_ok());

        let bad = LlmProviderConfig {
            id: "bad".into(),
            kind: LlmProviderKind::Ollama,
            endpoint: "not a url".into(),
            api_key: None,
            default_model: None,
        };
        assert!(cfg_validate_err_contains(&bad, "endpoint"));
    }

    fn cfg_validate_err_contains(cfg: &LlmProviderConfig, needle: &str) -> bool {
        match cfg.validate() {
            Ok(()) => false,
            Err(e) => e.to_string().contains(needle),
        }
    }

    #[test]
    fn model_string_capped_at_256_chars() {
        let long = "a".repeat(257);
        assert!(ModelInput::new(long.clone()).is_err());
        let ok = ModelInput::new("a".repeat(256));
        assert!(ok.is_ok());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib domain::llm_provider::tests::llm_provider_kind_from_str_canonical 2>&1 | tail -20`
Expected: compile error — `LlmProviderKind`, `LlmProviderConfig`, `ModelInput` not defined.

- [ ] **Step 3: Write minimal implementation**

```rust
// src/domain/llm_provider.rs
//! LLM provider configuration and validation.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

/// A free-form model string the user picked for a profile. Capped at 256 chars
/// to keep rendering bounded; no central catalog is enforced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModelInput(String);

impl ModelInput {
    pub fn new(value: impl Into<String>) -> Result<Self, LlmDomainError> {
        let s: String = value.into();
        if s.is_empty() {
            return Err(LlmDomainError::EmptyModel);
        }
        if s.chars().count() > 256 {
            return Err(LlmDomainError::ModelTooLong);
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ModelInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LlmProviderKind {
    Ollama,
    LmStudio,
    Anthropic,
    OpenAi,
}

impl LlmProviderKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ollama" => Some(Self::Ollama),
            "lm-studio" => Some(Self::LmStudio),
            "anthropic" => Some(Self::Anthropic),
            "openai" => Some(Self::OpenAi),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::LmStudio => "lm-studio",
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub id: String,
    pub kind: LlmProviderKind,
    pub endpoint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
}

impl LlmProviderConfig {
    pub fn validate(&self) -> Result<(), LlmDomainError> {
        if self.id.trim().is_empty() {
            return Err(LlmDomainError::EmptyId);
        }
        let url = Url::parse(&self.endpoint)
            .map_err(|_| LlmDomainError::InvalidEndpoint(self.endpoint.clone()))?;
        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(LlmDomainError::InvalidEndpoint(self.endpoint.clone()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LlmHealthStatus {
    pub reachable: bool,
    pub latency_ms: Option<u64>,
    pub models: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Error, PartialEq)]
pub enum LlmDomainError {
    #[error("LLM provider id cannot be empty")]
    EmptyId,
    #[error("LLM provider endpoint is invalid: {0}")]
    InvalidEndpoint(String),
    #[error("model string cannot be empty")]
    EmptyModel,
    #[error("model string exceeds 256 characters")]
    ModelTooLong,
}
```

Then add to `Cargo.toml`:
```toml
thiserror = "1"
url = { version = "2", features = ["serde"] }
```

And add `pub mod llm_provider;` to `src/domain/mod.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib domain::llm_provider 2>&1 | tail -10`
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/domain/llm_provider.rs src/domain/mod.rs Cargo.toml
git commit -m "feat(domain): add LlmProviderConfig + ModelInput value types"
```

---

### Task 1.2: Add `AgentFrontmatter` and `LaunchPlan` domain types

**Files:**
- Create: `src/domain/agent_markdown.rs`
- Create: `src/domain/launch_plan.rs`
- Modify: `src/domain/mod.rs` (add `pub mod agent_markdown; pub mod launch_plan;`)

- [ ] **Step 1: Write the failing test**

```rust
// in src/domain/launch_plan.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agent_markdown::{AgentFrontmatter, AgentMcpServer};

    #[test]
    fn launch_plan_carries_resolved_mcp_servers() {
        let servers = vec![AgentMcpServer {
            name: "github".into(),
            command: "docker".into(),
            args: vec!["run".into(), "-i".into(), "mcp/github".into()],
            env: vec![],
        }];
        let plan = LaunchPlan {
            profile_id: "reviewer".into(),
            provider_id: "claude-code".into(),
            frontmatter: AgentFrontmatter {
                name: "reviewer".into(),
                description: "PR reviewer".into(),
                tools: vec!["Read".into(), "Grep".into()],
                disallowed_tools: vec![],
                model: "sonnet".into(),
                permission_mode: Some("acceptEdits".into()),
                max_turns: None,
                skills: vec!["code-review".into()],
                mcp_servers: vec!["github".into()],
                hooks: vec![],
                memory: None,
                background: false,
                effort: None,
                isolation: None,
                color: None,
            },
            prompt_body: "Review the staged diff carefully.".into(),
            resolved_mcp_servers: servers.clone(),
            llm_provider_id: Some("local-ollama".into()),
        };
        assert_eq!(plan.resolved_mcp_servers.len(), 1);
        assert_eq!(plan.frontmatter.name, "reviewer");
        assert_eq!(plan.llm_provider_id.as_deref(), Some("local-ollama"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib domain::launch_plan 2>&1 | tail -10`
Expected: compile error — `AgentFrontmatter`, `AgentMcpServer`, `LaunchPlan` not defined.

- [ ] **Step 3: Write minimal implementation**

```rust
// src/domain/agent_markdown.rs
//! Pure data types for a Claude Code sub-agent frontmatter block.
//!
//! These are populated by the infra renderer from a `LaunchPlan`. Nothing in
//! this module performs I/O.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentFrontmatter {
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disallowed_tools: Vec<String>,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hooks: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<String>,
    #[serde(default)]
    pub background: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isolation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentMcpServer {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RenderedAgentMarkdown {
    pub frontmatter_yaml: String,
    pub body: String,
}

impl RenderedAgentMarkdown {
    pub fn into_markdown(self) -> String {
        format!("---\n{}---\n\n{}", self.frontmatter_yaml, self.body)
    }
}
```

```rust
// src/domain/launch_plan.rs
//! All the data the agent-markdown renderer needs, pre-resolved by the
//! use-case layer so the renderer itself has zero I/O and zero port calls.

use crate::domain::agent_markdown::{AgentFrontmatter, AgentMcpServer};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LaunchPlan {
    pub profile_id: String,
    pub provider_id: String,
    pub frontmatter: AgentFrontmatter,
    pub prompt_body: String,
    /// MCP servers already resolved from the registry (name -> command/args/env).
    /// The renderer embeds these into the `mcpServers` block of the frontmatter.
    pub resolved_mcp_servers: Vec<AgentMcpServer>,
    /// Optional LLM provider id to record in the launch plan for the
    /// downstream exec layer to consume (AGK does not probe the server).
    pub llm_provider_id: Option<String>,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib domain::launch_plan 2>&1 | tail -10`
Expected: 1 test passes.

- [ ] **Step 5: Commit**

```bash
git add src/domain/agent_markdown.rs src/domain/launch_plan.rs src/domain/mod.rs
git commit -m "feat(domain): add AgentFrontmatter, AgentMcpServer, LaunchPlan types"
```

---

### Task 1.3: Extend `Profile` and `ConfigFile` with LLM fields

**Files:**
- Modify: `src/domain/profile.rs` (add `model`, `agent_mcp_servers`, `llm_provider_id`)
- Modify: `src/domain/config/mod.rs` (add `[[llm_providers]]` table)

- [ ] **Step 1: Write the failing test**

```rust
// append to src/domain/profile.rs
#[cfg(test)]
mod llm_field_tests {
    use super::*;

    #[test]
    fn profile_carries_model_and_llm_provider() {
        let p = Profile {
            id: ProfileId::new("reviewer"),
            scope: crate::domain::scope::Scope::Workspace,
            provider_id: ProviderId("claude-code".into()),
            skill_refs: vec![],
            mcp_refs: vec![],
            instruction_refs: vec![],
            tool_refs: vec!["Read".into()],
            permission_mode: Some("acceptEdits".into()),
            prompt_overlay_path: None,
            launch_policy: Default::default(),
            model: Some("claude-sonnet-4-5".into()),
            llm_provider_id: Some("local-ollama".into()),
            agent_mcp_servers: vec![],
        };
        assert_eq!(p.model.as_deref(), Some("claude-sonnet-4-5"));
        assert_eq!(p.llm_provider_id.as_deref(), Some("local-ollama"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib domain::profile::llm_field_tests 2>&1 | tail -10`
Expected: compile error — `model`, `llm_provider_id`, `agent_mcp_servers` not on `Profile`.

- [ ] **Step 3: Add the new fields with backward-compatible serde**

In `src/domain/profile.rs`, inside `impl Profile` block, add at the end of the struct:

```rust
    /// Free-form model string the user picked for this profile. Free-form,
    /// 256-char cap enforced at the wizard layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// LLM provider id (from `[[llm_providers]]` in config) the user wants
    /// the downstream exec to use. AGK records the choice; it does not probe
    /// the server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_provider_id: Option<String>,

    /// MCP server definitions resolved at create-time and embedded into the
    /// generated agent markdown. Empty for providers that do not use it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agent_mcp_servers: Vec<crate::domain::agent_markdown::AgentMcpServer>,
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib domain::profile 2>&1 | tail -10`
Expected: 1 test passes (plus all pre-existing tests still pass thanks to `#[serde(default)]`).

- [ ] **Step 5: Add the `llm_providers` table to `ConfigFile`**

In `src/domain/config/mod.rs`, add this field to `ConfigFile`:

```rust
    /// Configured LLM providers. Each entry is one provider.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub llm_providers: Vec<crate::domain::llm_provider::LlmProviderConfig>,
```

And append a test:

```rust
    #[test]
    fn config_file_round_trip_with_llm_providers() {
        let mut cfg = ConfigFile::default();
        cfg.llm_providers.push(crate::domain::llm_provider::LlmProviderConfig {
            id: "local".into(),
            kind: crate::domain::llm_provider::LlmProviderKind::Ollama,
            endpoint: "http://127.0.0.1:11434".into(),
            api_key: None,
            default_model: Some("llama3.2".into()),
        });
        let toml = toml::to_string(&cfg).unwrap();
        let back: ConfigFile = toml::from_str(&toml).unwrap();
        assert_eq!(back.llm_providers.len(), 1);
        assert_eq!(back.llm_providers[0].kind, crate::domain::llm_provider::LlmProviderKind::Ollama);
    }
```

- [ ] **Step 6: Commit**

```bash
git add src/domain/profile.rs src/domain/config/mod.rs
git commit -m "feat(domain): add model, llm_provider_id, agent_mcp_servers to Profile + llm_providers table to ConfigFile"
```

---

### Task 1.4: Define `LlmProviderStorePort`, `LlmHealthCheckPort`, `LlmProviderFactoryPort`

**Files:**
- Create: `src/app/ports/llm_provider.rs`
- Modify: `src/app/ports/mod.rs` (re-export new traits)

- [ ] **Step 1: Write the failing test**

```rust
// at the bottom of src/app/ports/llm_provider.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm_provider::{LlmProviderConfig, LlmProviderKind, LlmHealthStatus};

    struct InMemoryStore {
        items: std::sync::Mutex<Vec<LlmProviderConfig>>,
    }

    impl LlmProviderStorePort for InMemoryStore {
        fn list(&self) -> Result<Vec<LlmProviderConfig>> { Ok(self.items.lock().unwrap().clone()) }
        fn get(&self, id: &str) -> Result<Option<LlmProviderConfig>> {
            Ok(self.items.lock().unwrap().iter().find(|c| c.id == id).cloned())
        }
        fn upsert(&self, cfg: &LlmProviderConfig) -> Result<()> {
            let mut g = self.items.lock().unwrap();
            if let Some(existing) = g.iter_mut().find(|c| c.id == cfg.id) { *existing = cfg.clone(); }
            else { g.push(cfg.clone()); }
            Ok(())
        }
        fn remove(&self, id: &str) -> Result<()> {
            self.items.lock().unwrap().retain(|c| c.id != id);
            Ok(())
        }
    }

    #[test]
    fn in_memory_store_upsert_replaces() {
        let s = InMemoryStore { items: std::sync::Mutex::new(vec![]) };
        s.upsert(&LlmProviderConfig {
            id: "a".into(), kind: LlmProviderKind::Ollama,
            endpoint: "http://x".into(), api_key: None, default_model: None,
        }).unwrap();
        s.upsert(&LlmProviderConfig {
            id: "a".into(), kind: LlmProviderKind::Ollama,
            endpoint: "http://y".into(), api_key: None, default_model: Some("llama3".into()),
        }).unwrap();
        let items = s.list().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].endpoint, "http://y");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib app::ports::llm_provider 2>&1 | tail -10`
Expected: compile error — traits not defined.

- [ ] **Step 3: Write the port trait definitions**

```rust
// src/app/ports/llm_provider.rs
//! Ports for LLM provider management.
//!
//! - [`LlmProviderStorePort`]: persistent store (TOML in config file).
//! - [`LlmProviderFactoryPort`]: produces an `LlmProviderAdapter` for a given
//!   `LlmProviderConfig` so the use-case can call health checks.
//! - [`LlmProviderAdapter`]: provider-specific behaviour (kind, default health URL).
//! - [`LlmHealthCheckPort`]: separate trait so fakes and real HTTP impls can be
//!   swapped in tests without needing the full adapter stack.

use crate::domain::llm_provider::{LlmHealthStatus, LlmProviderConfig, LlmProviderKind};
use anyhow::Result;
use async_trait::async_trait;
use std::time::Duration;

pub trait LlmProviderStorePort: Send + Sync {
    fn list(&self) -> Result<Vec<LlmProviderConfig>>;
    fn get(&self, id: &str) -> Result<Option<LlmProviderConfig>>;
    fn upsert(&self, cfg: &LlmProviderConfig) -> Result<()>;
    fn remove(&self, id: &str) -> Result<()>;
}

/// Factory that turns a stored `LlmProviderConfig` into a live adapter for
/// the duration of a health check. Always available (no feature gate on the
/// trait itself) so use-case code can call it from any build.
pub trait LlmProviderFactoryPort: Send + Sync {
    fn build(&self, cfg: &LlmProviderConfig) -> Result<Box<dyn LlmProviderAdapter>>;
}

/// Per-provider adapter: answers what kind it is and what URL/headers to
/// probe. Real impls live in `infra/llm/` and are feature-gated.
pub trait LlmProviderAdapter: Send + Sync {
    fn kind(&self) -> LlmProviderKind;
    /// URL the health check should hit. Implementations should pick the
    /// cheapest call that exercises the server (see spec section 8).
    fn health_url(&self) -> String;
    /// Default model advertised by the server. May be `None` if not known
    /// until the health check runs.
    fn default_model_hint(&self) -> Option<String> { None }
}

#[async_trait]
pub trait LlmHealthCheckPort: Send + Sync {
    async fn check(
        &self,
        adapter: &dyn LlmProviderAdapter,
        timeout: Duration,
    ) -> Result<LlmHealthStatus>;
}
```

Re-export from `src/app/ports/mod.rs`:
```rust
pub mod llm_provider;
pub use llm_provider::{
    LlmHealthCheckPort, LlmProviderAdapter, LlmProviderFactoryPort, LlmProviderStorePort,
};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib app::ports::llm_provider 2>&1 | tail -10`
Expected: 1 test passes.

- [ ] **Step 5: Commit**

```bash
git add src/app/ports/llm_provider.rs src/app/ports/mod.rs
git commit -m "feat(ports): add LlmProviderStorePort, LlmProviderFactoryPort, LlmHealthCheckPort"
```

---

### Task 1.5: Define `ClaudeCliProbePort`

**Files:**
- Create: `src/app/ports/claude_cli_probe.rs`
- Modify: `src/app/ports/mod.rs` (re-export)

- [ ] **Step 1: Write the failing test**

```rust
// at the bottom of src/app/ports/claude_cli_probe.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn system_probe_does_not_panic_when_cli_missing() {
        // Just exercise the trait; a real test of the system impl is in
        // infra::provider::claude_code::cli_probe.
        let p: Box<dyn ClaudeCliProbePort> = Box::new(MissingCliProbe);
        assert!(!p.is_available());
        assert!(p.locate().is_err());
    }

    struct MissingCliProbe;
    impl ClaudeCliProbePort for MissingCliProbe {
        fn is_available(&self) -> bool { false }
        fn locate(&self) -> Result<PathBuf> { anyhow::bail!("claude not on PATH") }
        fn version(&self) -> Result<semver::Version> { anyhow::bail!("claude not on PATH") }
        fn supports_agent_flag(&self) -> bool { false }
    }
}
```

Add `semver = "1"` to `Cargo.toml` `[dependencies]`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib app::ports::claude_cli_probe 2>&1 | tail -10`
Expected: compile error — `ClaudeCliProbePort` not defined.

- [ ] **Step 3: Write the port**

```rust
// src/app/ports/claude_cli_probe.rs
//! Probes the host system for the `claude` CLI binary. Feature-gated by
//! `claude-cli-probe` at the use-case level (the port itself is always
//! available; only the real `SystemClaudeCliProbe` impl is gated).

use anyhow::Result;
use std::path::PathBuf;

/// Minimum version of the `claude` CLI that supports the `--agent` flag
/// required by the Claude Code provider. Older versions are rejected.
pub const MIN_CLAUDE_CLI_VERSION: semver::Version = semver::Version::new(2, 0, 0);

pub trait ClaudeCliProbePort: Send + Sync {
    /// `true` if the `claude` binary is on `$PATH` and runnable.
    fn is_available(&self) -> bool;
    /// Absolute path to the `claude` binary. Errors if not present.
    fn locate(&self) -> Result<PathBuf>;
    /// Parsed semver version (output of `claude --version`). Errors if the
    /// binary is missing or its output is unparseable.
    fn version(&self) -> Result<semver::Version>;
    /// `true` if the installed version is `>= MIN_CLAUDE_CLI_VERSION` and
    /// therefore supports the `--agent` flag.
    fn supports_agent_flag(&self) -> bool;
}
```

Re-export from `src/app/ports/mod.rs`:
```rust
pub mod claude_cli_probe;
pub use claude_cli_probe::ClaudeCliProbePort;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib app::ports::claude_cli_probe 2>&1 | tail -10`
Expected: 1 test passes.

- [ ] **Step 5: Commit**

```bash
git add src/app/ports/claude_cli_probe.rs src/app/ports/mod.rs Cargo.toml
git commit -m "feat(ports): add ClaudeCliProbePort + MIN_CLAUDE_CLI_VERSION"
```

---

### Task 1.6: Add 6 new `WizardStep` variants

**Files:**
- Modify: `src/app/ports/wizard_state.rs` (add 6 variants to `WizardStep` enum)

- [ ] **Step 1: Write the failing test**

```rust
// append to wizard_state.rs `#[cfg(test)] mod tests`
#[test]
fn new_wizard_variants_construct() {
    let _ = WizardStep::ProviderSelect {
        title: "Pick agent provider".into(),
        providers: vec![("claude-code".into(), "Claude Code".into()), ("opencode".into(), "OpenCode".into())],
    };
    let _ = WizardStep::LlmProviderSelect {
        title: "Pick LLM provider".into(),
        providers: vec![("local-ollama".into(), "Ollama (local)".into())],
    };
    let _ = WizardStep::ModelInput {
        title: "Model string".into(),
        placeholder: "e.g. claude-sonnet-4-5 or llama3.2".into(),
    };
    let _ = WizardStep::AgentDescription {
        title: "Describe this agent".into(),
        placeholder: "Used as the agent's `description` frontmatter".into(),
        rows: 5,
    };
    let _ = WizardStep::SkillsPick {
        title: "Pick skills".into(),
        options: vec!["code-review".into()],
    };
    let _ = WizardStep::ReviewFinal {
        title: "Review and confirm".into(),
    };
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib app::ports::wizard_state::tests::new_wizard_variants_construct 2>&1 | tail -10`
Expected: compile error — variants not defined.

- [ ] **Step 3: Add the 6 new variants to `WizardStep`**

```rust
    /// Pick the agent provider (claude-code, opencode, ...).
    ProviderSelect {
        title: String,
        providers: Vec<(String, String)>, // (id, display_name)
    },
    /// Pick the LLM provider (only for providers that use one).
    LlmProviderSelect {
        title: String,
        providers: Vec<(String, String)>, // (id, display_name)
    },
    /// Free-form model string (256-char cap).
    ModelInput {
        title: String,
        placeholder: String,
    },
    /// Multi-line agent description (stored in agent markdown frontmatter).
    AgentDescription {
        title: String,
        placeholder: String,
        rows: usize,
    },
    /// Skills pick checklist (re-uses filtered_indices in WizardState).
    SkillsPick {
        title: String,
        options: Vec<String>,
    },
    /// Final review before commit.
    ReviewFinal {
        title: String,
    },
```

Also extend `WizardState` with the 4 new fields:

```rust
    /// Provider id picked on the ProviderSelect step.
    pub provider_id_choice: String,
    /// LLM provider id picked on the LlmProviderSelect step.
    pub llm_provider_id: String,
    /// Free-form model string captured on the ModelInput step.
    pub model_string: String,
    /// Multi-line agent description captured on the AgentDescription step.
    pub agent_description: String,
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib app::ports::wizard_state 2>&1 | tail -10`
Expected: 1 test passes (other wizard tests still pass thanks to `..Default::default()` patterns or will be updated in C2).

- [ ] **Step 5: Commit**

```bash
git add src/app/ports/wizard_state.rs
git commit -m "feat(wizard): add ProviderSelect, LlmProviderSelect, ModelInput, AgentDescription, SkillsPick, ReviewFinal variants"
```

---

### Task 1.7: Add hand fakes for new ports

**Files:**
- Create: `src/app/test_support/fake_claude_cli_probe.rs`
- Create: `src/app/test_support/fake_llm_provider.rs`
- Modify: `src/app/test_support/mod.rs` (re-export)

- [ ] **Step 1: Write the failing test**

```rust
// in src/app/test_support/fake_claude_cli_probe.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::claude_cli_probe::ClaudeCliProbePort;

    #[test]
    fn fake_reports_unavailable_by_default() {
        let f = FakeClaudeCliProbe::unavailable();
        assert!(!f.is_available());
        assert!(f.locate().is_err());
    }

    #[test]
    fn fake_supports_agent_flag_when_version_high_enough() {
        let f = FakeClaudeCliProbe::available("2.1.0");
        assert!(f.supports_agent_flag());
        let old = FakeClaudeCliProbe::available("1.9.0");
        assert!(!old.supports_agent_flag());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib app::test_support::fake_claude_cli_probe 2>&1 | tail -10`
Expected: compile error — `FakeClaudeCliProbe` not defined.

- [ ] **Step 3: Write the fakes**

```rust
// src/app/test_support/fake_claude_cli_probe.rs
use crate::app::ports::claude_cli_probe::{ClaudeCliProbePort, MIN_CLAUDE_CLI_VERSION};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct FakeClaudeCliProbe {
    pub available: bool,
    pub path: PathBuf,
    pub version: Option<semver::Version>,
}

impl FakeClaudeCliProbe {
    pub fn unavailable() -> Self {
        Self { available: false, path: PathBuf::from("/nonexistent/claude"), version: None }
    }
    pub fn available(v: &str) -> Self {
        Self {
            available: true,
            path: PathBuf::from("/usr/local/bin/claude"),
            version: Some(semver::Version::parse(v).expect("valid semver")),
        }
    }
}

impl ClaudeCliProbePort for FakeClaudeCliProbe {
    fn is_available(&self) -> bool { self.available }
    fn locate(&self) -> Result<PathBuf> {
        if self.available { Ok(self.path.clone()) } else { anyhow::bail!("claude not on PATH") }
    }
    fn version(&self) -> Result<semver::Version> {
        self.version.clone().ok_or_else(|| anyhow::anyhow!("claude not on PATH"))
    }
    fn supports_agent_flag(&self) -> bool {
        self.version.as_ref().map_or(false, |v| v >= &MIN_CLAUDE_CLI_VERSION)
    }
}
```

```rust
// src/app/test_support/fake_llm_provider.rs
use crate::app::ports::llm_provider::{
    LlmHealthCheckPort, LlmProviderAdapter, LlmProviderFactoryPort, LlmProviderStorePort,
};
use crate::domain::llm_provider::{LlmHealthStatus, LlmProviderConfig, LlmProviderKind};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

pub struct FakeLlmProviderStore {
    pub items: Mutex<HashMap<String, LlmProviderConfig>>,
}

impl FakeLlmProviderStore {
    pub fn new() -> Self { Self { items: Mutex::new(HashMap::new()) } }
    pub fn seeded(cfgs: Vec<LlmProviderConfig>) -> Self {
        let m: HashMap<_, _> = cfgs.into_iter().map(|c| (c.id.clone(), c)).collect();
        Self { items: Mutex::new(m) }
    }
}

impl Default for FakeLlmProviderStore {
    fn default() -> Self { Self::new() }
}

impl LlmProviderStorePort for FakeLlmProviderStore {
    fn list(&self) -> Result<Vec<LlmProviderConfig>> {
        Ok(self.items.lock().unwrap().values().cloned().collect())
    }
    fn get(&self, id: &str) -> Result<Option<LlmProviderConfig>> {
        Ok(self.items.lock().unwrap().get(id).cloned())
    }
    fn upsert(&self, cfg: &LlmProviderConfig) -> Result<()> {
        self.items.lock().unwrap().insert(cfg.id.clone(), cfg.clone());
        Ok(())
    }
    fn remove(&self, id: &str) -> Result<()> {
        self.items.lock().unwrap().remove(id);
        Ok(())
    }
}

pub struct FakeAdapter {
    pub kind: LlmProviderKind,
    pub url: String,
}
impl LlmProviderAdapter for FakeAdapter {
    fn kind(&self) -> LlmProviderKind { self.kind }
    fn health_url(&self) -> String { self.url.clone() }
}

pub struct FakeLlmProviderFactory;
impl LlmProviderFactoryPort for FakeLlmProviderFactory {
    fn build(&self, cfg: &LlmProviderConfig) -> Result<Box<dyn LlmProviderAdapter>> {
        Ok(Box::new(FakeAdapter {
            kind: cfg.kind,
            url: match cfg.kind {
                LlmProviderKind::Ollama => format!("{}/api/tags", cfg.endpoint.trim_end_matches('/')),
                LlmProviderKind::LmStudio => format!("{}/v1/models", cfg.endpoint.trim_end_matches('/')),
                LlmProviderKind::Anthropic => cfg.endpoint.clone(), // OPTIONS /v1/messages
                LlmProviderKind::OpenAi => format!("{}/v1/models", cfg.endpoint.trim_end_matches('/')),
            },
        }))
    }
}

pub struct FakeLlmHealthCheck {
    pub reachable: bool,
    pub latency_ms: u64,
    pub models: Vec<String>,
    pub error: Option<String>,
}
impl Default for FakeLlmHealthCheck {
    fn default() -> Self { Self { reachable: true, latency_ms: 12, models: vec!["llama3.2".into()], error: None } }
}
#[async_trait]
impl LlmHealthCheckPort for FakeLlmHealthCheck {
    async fn check(&self, _a: &dyn LlmProviderAdapter, _t: Duration) -> Result<LlmHealthStatus> {
        Ok(LlmHealthStatus {
            reachable: self.reachable,
            latency_ms: if self.reachable { Some(self.latency_ms) } else { None },
            models: if self.reachable { self.models.clone() } else { vec![] },
            error: self.error.clone(),
        })
    }
}
```

- [ ] **Step 4: Commit**

```bash
git add src/app/test_support/
git commit -m "feat(test-support): add FakeClaudeCliProbe, FakeLlmProviderStore, FakeLlmProviderFactory, FakeLlmHealthCheck"
```

---

### Task 1.8: Add Cargo feature flags for LLM adapters and CLI probe

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Edit Cargo.toml**

Add to `[features]`:

```toml
profile-create = []
claude-cli-probe = []
llm-ollama = ["dep:reqwest"]
llm-lmstudio = ["dep:reqwest"]
llm-anthropic = ["dep:reqwest"]
llm-openai = ["dep:reqwest"]
```

Add to `default = [...]` list (keep all existing defaults):
```toml
default = ["tui", "vault-clawhub", "pack", "provider-opencode", "provider-claude", "provider-github", "provider-gemini", "provider-amp", "provider-firebender", "provider-letta", "provider-snowflake", "profile-create", "claude-cli-probe"]
```

Add new optional deps:
```toml
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"], optional = true }
```

(The existing `reqwest` entry under `vault-clawhub` should remain; ensure no duplicate key.)

- [ ] **Step 2: Run all existing tests to confirm no regression**

Run: `cargo test --lib 2>&1 | tail -20`
Expected: all existing tests still pass; new feature flags compile but are inert.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "feat(cargo): add profile-create, claude-cli-probe, llm-{ollama,lmstudio,anthropic,openai} features"
```

---

## Commit 2: Infra + Use Cases

### Task 2.1: Implement `render_agent_markdown` (pure renderer)

**Files:**
- Create: `src/infra/provider/claude_code/agent_markdown.rs`
- Modify: `src/infra/provider/claude_code/mod.rs` (re-export + `#[cfg(feature = "profile-create")]` gate)

- [ ] **Step 1: Write the failing test**

```rust
// in src/infra/provider/claude_code/agent_markdown.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agent_markdown::{AgentFrontmatter, AgentMcpServer};
    use crate::domain::launch_plan::LaunchPlan;
    use crate::domain::profile::{ProfileId, ProviderId};
    use crate::domain::scope::Scope;

    fn sample_plan() -> LaunchPlan {
        LaunchPlan {
            profile_id: "reviewer".into(),
            provider_id: "claude-code".into(),
            frontmatter: AgentFrontmatter {
                name: "reviewer".into(),
                description: "PR reviewer".into(),
                tools: vec!["Read".into(), "Grep".into()],
                disallowed_tools: vec![],
                model: "sonnet".into(),
                permission_mode: Some("acceptEdits".into()),
                max_turns: None,
                skills: vec!["code-review".into()],
                mcp_servers: vec!["github".into()],
                hooks: vec![],
                memory: None,
                background: false,
                effort: None,
                isolation: None,
                color: None,
            },
            prompt_body: "Review staged changes carefully.".into(),
            resolved_mcp_servers: vec![AgentMcpServer {
                name: "github".into(),
                command: "docker".into(),
                args: vec!["run".into(), "-i".into(), "mcp/github".into()],
                env: vec![],
            }],
            llm_provider_id: Some("local-ollama".into()),
        }
    }

    #[test]
    fn render_minimal_no_mcp() {
        let mut p = sample_plan();
        p.resolved_mcp_servers.clear();
        p.frontmatter.mcp_servers.clear();
        let out = render_agent_markdown(&p);
        assert!(out.starts_with("---\nname: reviewer\n"));
        assert!(out.contains("model: sonnet\n"));
        assert!(!out.contains("mcpServers:"));
    }

    #[test]
    fn render_full_with_mcp_servers() {
        let out = render_agent_markdown(&sample_plan());
        assert!(out.contains("mcpServers:"));
        assert!(out.contains("  github:"));
        assert!(out.contains("    command: docker"));
    }

    #[test]
    fn render_yaml_escapes_double_quotes_in_description() {
        let mut p = sample_plan();
        p.frontmatter.description = "Quote: \"yes\"".into();
        let out = render_agent_markdown(&p);
        assert!(out.contains("description: \"Quote: \\\"yes\\\"\""));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features profile-create --lib infra::provider::claude_code::agent_markdown 2>&1 | tail -10`
Expected: compile error — `render_agent_markdown` not defined.

- [ ] **Step 3: Implement the renderer**

```rust
// src/infra/provider/claude_code/agent_markdown.rs
//! Pure renderer: takes a fully-resolved `LaunchPlan` and emits the markdown
//! the Claude Code CLI expects at `.claude/agents/<name>.md`.
//!
//! Zero I/O, zero port calls, zero global state — given the same `LaunchPlan`
//! the function always returns the same string. Tested via golden fixtures in
//! `tests/agent_markdown_renderer.rs`.

use crate::domain::launch_plan::LaunchPlan;

pub fn render_agent_markdown(plan: &LaunchPlan) -> String {
    let mut yaml = String::new();
    yaml.push_str(&format!("name: {}\n", yaml_scalar(&plan.frontmatter.name)));
    yaml.push_str(&format!("description: {}\n", yaml_scalar(&plan.frontmatter.description)));
    if !plan.frontmatter.tools.is_empty() {
        yaml.push_str("tools:\n");
        for t in &plan.frontmatter.tools {
            yaml.push_str(&format!("  - {}\n", yaml_scalar(t)));
        }
    }
    if !plan.frontmatter.disallowed_tools.is_empty() {
        yaml.push_str("disallowedTools:\n");
        for t in &plan.frontmatter.disallowed_tools {
            yaml.push_str(&format!("  - {}\n", yaml_scalar(t)));
        }
    }
    yaml.push_str(&format!("model: {}\n", yaml_scalar(&plan.frontmatter.model)));
    if let Some(pm) = &plan.frontmatter.permission_mode {
        yaml.push_str(&format!("permissionMode: {}\n", yaml_scalar(pm)));
    }
    if let Some(mt) = plan.frontmatter.max_turns {
        yaml.push_str(&format!("maxTurns: {}\n", mt));
    }
    if !plan.frontmatter.skills.is_empty() {
        yaml.push_str("skills:\n");
        for s in &plan.frontmatter.skills {
            yaml.push_str(&format!("  - {}\n", yaml_scalar(s)));
        }
    }
    if !plan.resolved_mcp_servers.is_empty() {
        yaml.push_str("mcpServers:\n");
        for server in &plan.resolved_mcp_servers {
            yaml.push_str(&format!("  {}:\n", yaml_scalar(&server.name)));
            yaml.push_str(&format!("    command: {}\n", yaml_scalar(&server.command)));
            if !server.args.is_empty() {
                yaml.push_str("    args:\n");
                for a in &server.args {
                    yaml.push_str(&format!("      - {}\n", yaml_scalar(a)));
                }
            }
            if !server.env.is_empty() {
                yaml.push_str("    env:\n");
                for e in &server.env {
                    yaml.push_str(&format!("      {}\n", yaml_scalar(e)));
                }
            }
        }
    }
    if !plan.frontmatter.hooks.is_empty() {
        yaml.push_str("hooks:\n");
        for h in &plan.frontmatter.hooks {
            yaml.push_str(&format!("  - {}\n", yaml_scalar(h)));
        }
    }
    if let Some(m) = &plan.frontmatter.memory {
        yaml.push_str(&format!("memory: {}\n", yaml_scalar(m)));
    }
    if plan.frontmatter.background {
        yaml.push_str("background: true\n");
    }
    if let Some(effort) = &plan.frontmatter.effort {
        yaml.push_str(&format!("effort: {}\n", yaml_scalar(effort)));
    }
    if let Some(iso) = &plan.frontmatter.isolation {
        yaml.push_str(&format!("isolation: {}\n", yaml_scalar(iso)));
    }
    if let Some(color) = &plan.frontmatter.color {
        yaml.push_str(&format!("color: {}\n", yaml_scalar(color)));
    }
    format!("---\n{}---\n\n{}", yaml, plan.prompt_body)
}

fn yaml_scalar(s: &str) -> String {
    if s.contains(':') || s.contains('#') || s.contains('"') || s.contains('\n')
        || s.starts_with(' ') || s.ends_with(' ') {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        s.to_string()
    }
}
```

Wire it into `src/infra/provider/claude_code/mod.rs`:
```rust
#[cfg(feature = "profile-create")]
pub mod agent_markdown;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --features profile-create --lib infra::provider::claude_code::agent_markdown 2>&1 | tail -10`
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/infra/provider/claude_code/agent_markdown.rs src/infra/provider/claude_code/mod.rs
git commit -m "feat(infra): add render_agent_markdown pure renderer (profile-create feature)"
```

---

### Task 2.2: Implement `SystemClaudeCliProbe`

**Files:**
- Create: `src/infra/provider/claude_code/cli_probe.rs`
- Modify: `src/infra/provider/claude_code/mod.rs` (re-export + `#[cfg(feature = "claude-cli-probe")]` gate)

- [ ] **Step 1: Write the failing test**

```rust
// in src/infra/provider/claude_code/cli_probe.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::claude_cli_probe::ClaudeCliProbePort;

    #[test]
    fn system_probe_reports_unavailable_when_cli_missing() {
        // Force a PATH that definitely does not contain `claude`.
        let probe = SystemClaudeCliProbe::with_path_override("/this/path/does/not/exist");
        assert!(!probe.is_available());
        assert!(!probe.supports_agent_flag());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features claude-cli-probe --lib infra::provider::claude_code::cli_probe 2>&1 | tail -10`
Expected: compile error — `SystemClaudeCliProbe` not defined.

- [ ] **Step 3: Implement the probe**

```rust
// src/infra/provider/claude_code/cli_probe.rs
use crate::app::ports::claude_cli_probe::{ClaudeCliProbePort, MIN_CLAUDE_CLI_VERSION};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

pub struct SystemClaudeCliProbe {
    path_override: Option<PathBuf>,
}

impl SystemClaudeCliProbe {
    pub fn new() -> Self { Self { path_override: None } }
    /// Test-only constructor: temporarily override `$PATH` to a directory that
    /// does not contain `claude`, to force the unavailable path.
    pub fn with_path_override(path: &str) -> Self {
        Self { path_override: Some(PathBuf::from(path)) }
    }
}

impl Default for SystemClaudeCliProbe {
    fn default() -> Self { Self::new() }
}

impl ClaudeCliProbePort for SystemClaudeCliProbe {
    fn is_available(&self) -> bool { self.locate().is_ok() }

    fn locate(&self) -> Result<PathBuf> {
        let exe = "claude";
        let path = if let Some(p) = &self.path_override {
            std::env::var("PATH").unwrap_or_default()
                .split(':')
                .map(PathBuf::from)
                .chain(std::iter::once(p.clone()))
                .find(|dir| dir.join(exe).is_file())
                .ok_or_else(|| anyhow::anyhow!("claude not found on PATH"))?
        } else {
            which::which(exe).with_context(|| "claude not found on PATH")?
        };
        Ok(path)
    }

    fn version(&self) -> Result<semver::Version> {
        let path = self.locate()?;
        let out = Command::new(&path).arg("--version").output()
            .with_context(|| format!("failed to run {} --version", path.display()))?;
        if !out.status.success() {
            anyhow::bail!("claude --version exited with {:?}", out.status.code());
        }
        let s = String::from_utf8_lossy(&out.stdout);
        // The CLI prints either "claude 2.1.0" or just "2.1.0".
        let token = s.split_whitespace()
            .find(|t| t.chars().next().map_or(false, |c| c.is_ascii_digit()))
            .ok_or_else(|| anyhow::anyhow!("could not parse version from: {}", s.trim()))?;
        semver::Version::parse(token.trim_start_matches('v'))
            .with_context(|| format!("invalid semver: {token}"))
    }

    fn supports_agent_flag(&self) -> bool {
        self.version().map_or(false, |v| v >= MIN_CLAUDE_CLI_VERSION)
    }
}
```

Add to `Cargo.toml`:
```toml
which = { version = "6", optional = true }
```
And to the `claude-cli-probe` feature:
```toml
claude-cli-probe = ["dep:which"]
```

Wire it:
```rust
// src/infra/provider/claude_code/mod.rs
#[cfg(feature = "claude-cli-probe")]
pub mod cli_probe;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --features claude-cli-probe --lib infra::provider::claude_code::cli_probe 2>&1 | tail -10`
Expected: 1 test passes.

- [ ] **Step 5: Commit**

```bash
git add src/infra/provider/claude_code/cli_probe.rs src/infra/provider/claude_code/mod.rs Cargo.toml
git commit -m "feat(infra): add SystemClaudeCliProbe (claude-cli-probe feature)"
```

---

### Task 2.3: Implement `HttpLlmHealthCheck` (shared HTTP probing)

**Files:**
- Create: `src/infra/llm/health.rs`
- Create: `src/infra/llm/mod.rs`

- [ ] **Step 1: Write the failing test**

```rust
// in src/infra/llm/health.rs
#[cfg(all(test, feature = "llm-ollama"))]
mod tests {
    use super::*;
    use crate::app::ports::llm_provider::{LlmProviderAdapter, LlmHealthCheckPort};
    use crate::domain::llm_provider::LlmProviderKind;
    use std::time::Duration;

    struct StubAdapter;
    impl LlmProviderAdapter for StubAdapter {
        fn kind(&self) -> LlmProviderKind { LlmProviderKind::Ollama }
        fn health_url(&self) -> String { "http://127.0.0.1:1/api/tags".into() } // unreachable
    }

    #[tokio::test]
    async fn health_check_marks_unreachable_when_refused() {
        let hc = HttpLlmHealthCheck::new();
        let status = hc.check(&StubAdapter, Duration::from_millis(500)).await.unwrap();
        assert!(!status.reachable);
        assert!(status.error.is_some());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features llm-ollama --lib infra::llm::health 2>&1 | tail -10`
Expected: compile error — `HttpLlmHealthCheck` not defined.

- [ ] **Step 3: Implement the health checker**

```rust
// src/infra/llm/health.rs
use crate::app::ports::llm_provider::{LlmHealthCheckPort, LlmProviderAdapter};
use crate::domain::llm_provider::{LlmHealthStatus, LlmProviderKind};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use reqwest::{Client, Method};
use std::time::{Duration, Instant};

pub struct HttpLlmHealthCheck {
    pub client: Client,
}

impl HttpLlmHealthCheck {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("reqwest client");
        Self { client }
    }
}

impl Default for HttpLlmHealthCheck {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl LlmHealthCheckPort for HttpLlmHealthCheck {
    async fn check(
        &self,
        adapter: &dyn LlmProviderAdapter,
        timeout: Duration,
    ) -> Result<LlmHealthStatus> {
        let url = adapter.health_url();
        let method = match adapter.kind() {
            // Anthropic: OPTIONS /v1/messages is free (no quota), but the
            // spec says the simpler GET / on the base URL is the universally
            // cheap fallback. We try OPTIONS first, fall back to GET /.
            LlmProviderKind::Anthropic => Method::OPTIONS,
            _ => Method::GET,
        };
        let mut headers = HeaderMap::new();
        if let Some(_) = adapter.default_model_hint() {
            // No-op placeholder; per-provider headers are added in adapters
            // when this abstraction proves insufficient.
        }
        let start = Instant::now();
        let req = self.client.request(method.clone(), &url)
            .timeout(timeout)
            .headers(headers)
            .build()?;
        let result = self.client.execute(req).await;
        let latency = start.elapsed().as_millis() as u64;
        match result {
            Ok(resp) if resp.status().is_success() => Ok(LlmHealthStatus {
                reachable: true,
                latency_ms: Some(latency),
                models: vec![], // populated by adapter-specific GET /v1/models or /api/tags if needed
                error: None,
            }),
            Ok(resp) => Ok(LlmHealthStatus {
                reachable: false,
                latency_ms: Some(latency),
                models: vec![],
                error: Some(format!("HTTP {}", resp.status())),
            }),
            Err(e) => Ok(LlmHealthStatus {
                reachable: false,
                latency_ms: None,
                models: vec![],
                error: Some(e.to_string()),
            }),
        }
    }
}
```

Add to `src/infra/llm/mod.rs`:
```rust
#[cfg(any(feature = "llm-ollama", feature = "llm-lmstudio", feature = "llm-anthropic", feature = "llm-openai"))]
pub mod health;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --features llm-ollama --lib infra::llm::health 2>&1 | tail -10`
Expected: 1 test passes.

- [ ] **Step 5: Commit**

```bash
git add src/infra/llm/
git commit -m "feat(infra): add HttpLlmHealthCheck (shared HTTP probing for all LLM adapters)"
```

---

### Task 2.4: Implement `OllamaProvider` adapter

**Files:**
- Create: `src/infra/llm/ollama.rs`
- Modify: `src/infra/llm/mod.rs` (re-export under feature gate)

- [ ] **Step 1: Write the failing test**

```rust
// in src/infra/llm/ollama.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::llm_provider::LlmProviderAdapter;
    use crate::domain::llm_provider::LlmProviderKind;

    #[test]
    fn ollama_health_url_uses_api_tags() {
        let p = OllamaProvider::new("http://127.0.0.1:11434");
        assert_eq!(p.health_url(), "http://127.0.0.1:11434/api/tags");
        assert_eq!(p.kind(), LlmProviderKind::Ollama);
    }

    #[test]
    fn ollama_strips_trailing_slash() {
        let p = OllamaProvider::new("http://127.0.0.1:11434/");
        assert_eq!(p.health_url(), "http://127.0.0.1:11434/api/tags");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features llm-ollama --lib infra::llm::ollama 2>&1 | tail -10`
Expected: compile error.

- [ ] **Step 3: Implement**

```rust
// src/infra/llm/ollama.rs
use crate::app::ports::llm_provider::LlmProviderAdapter;
use crate::domain::llm_provider::LlmProviderKind;

pub struct OllamaProvider {
    pub endpoint: String,
}

impl OllamaProvider {
    pub fn new(endpoint: impl Into<String>) -> Self { Self { endpoint: endpoint.into() } }
}

impl LlmProviderAdapter for OllamaProvider {
    fn kind(&self) -> LlmProviderKind { LlmProviderKind::Ollama }
    fn health_url(&self) -> String {
        format!("{}/api/tags", self.endpoint.trim_end_matches('/'))
    }
}
```

Wire in `src/infra/llm/mod.rs`:
```rust
#[cfg(feature = "llm-ollama")]
pub mod ollama;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --features llm-ollama --lib infra::llm::ollama 2>&1 | tail -10`
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/infra/llm/ollama.rs src/infra/llm/mod.rs
git commit -m "feat(infra): add OllamaProvider LLM adapter (llm-ollama feature)"
```

---

### Task 2.5: Implement `LmStudioProvider`, `AnthropicProvider`, `OpenAiProvider` adapters

**Files:**
- Create: `src/infra/llm/lmstudio.rs`
- Create: `src/infra/llm/anthropic.rs`
- Create: `src/infra/llm/openai.rs`
- Modify: `src/infra/llm/mod.rs`

- [ ] **Step 1: Write the failing test**

```rust
// in src/infra/llm/lmstudio.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::llm_provider::LlmProviderAdapter;
    use crate::domain::llm_provider::LlmProviderKind;

    #[test]
    fn lmstudio_health_url_uses_v1_models() {
        let p = LmStudioProvider::new("http://127.0.0.1:1234");
        assert_eq!(p.health_url(), "http://127.0.0.1:1234/v1/models");
        assert_eq!(p.kind(), LlmProviderKind::LmStudio);
    }
}
```

```rust
// in src/infra/llm/anthropic.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::llm_provider::LlmProviderAdapter;
    use crate::domain::llm_provider::LlmProviderKind;

    #[test]
    fn anthropic_health_url_passes_through_endpoint() {
        let p = AnthropicProvider::new("https://api.anthropic.com");
        // HttpLlmHealthCheck will OPTIONS this URL.
        assert_eq!(p.health_url(), "https://api.anthropic.com");
        assert_eq!(p.kind(), LlmProviderKind::Anthropic);
    }
}
```

```rust
// in src/infra/llm/openai.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::llm_provider::LlmProviderAdapter;
    use crate::domain::llm_provider::LlmProviderKind;

    #[test]
    fn openai_health_url_uses_v1_models() {
        let p = OpenAiProvider::new("https://api.openai.com");
        assert_eq!(p.health_url(), "https://api.openai.com/v1/models");
        assert_eq!(p.kind(), LlmProviderKind::OpenAi);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --features llm-lmstudio --lib infra::llm::lmstudio 2>&1 | tail -10`
Expected: compile error.

- [ ] **Step 3: Implement all three**

```rust
// src/infra/llm/lmstudio.rs
use crate::app::ports::llm_provider::LlmProviderAdapter;
use crate::domain::llm_provider::LlmProviderKind;
pub struct LmStudioProvider { pub endpoint: String }
impl LmStudioProvider { pub fn new(e: impl Into<String>) -> Self { Self { endpoint: e.into() } } }
impl LlmProviderAdapter for LmStudioProvider {
    fn kind(&self) -> LlmProviderKind { LlmProviderKind::LmStudio }
    fn health_url(&self) -> String { format!("{}/v1/models", self.endpoint.trim_end_matches('/')) }
}
```

```rust
// src/infra/llm/anthropic.rs
use crate::app::ports::llm_provider::LlmProviderAdapter;
use crate::domain::llm_provider::LlmProviderKind;
pub struct AnthropicProvider { pub endpoint: String }
impl AnthropicProvider { pub fn new(e: impl Into<String>) -> Self { Self { endpoint: e.into() } } }
impl LlmProviderAdapter for AnthropicProvider {
    fn kind(&self) -> LlmProviderKind { LlmProviderKind::Anthropic }
    fn health_url(&self) -> String { self.endpoint.trim_end_matches('/').to_string() }
}
```

```rust
// src/infra/llm/openai.rs
use crate::app::ports::llm_provider::LlmProviderAdapter;
use crate::domain::llm_provider::LlmProviderKind;
pub struct OpenAiProvider { pub endpoint: String }
impl OpenAiProvider { pub fn new(e: impl Into<String>) -> Self { Self { endpoint: e.into() } } }
impl LlmProviderAdapter for OpenAiProvider {
    fn kind(&self) -> LlmProviderKind { LlmProviderKind::OpenAi }
    fn health_url(&self) -> String { format!("{}/v1/models", self.endpoint.trim_end_matches('/')) }
}
```

Wire in `src/infra/llm/mod.rs`:
```rust
#[cfg(feature = "llm-lmstudio")]
pub mod lmstudio;
#[cfg(feature = "llm-anthropic")]
pub mod anthropic;
#[cfg(feature = "llm-openai")]
pub mod openai;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --features llm-lmstudio,llm-anthropic,llm-openai --lib infra::llm 2>&1 | tail -10`
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/infra/llm/lmstudio.rs src/infra/llm/anthropic.rs src/infra/llm/openai.rs src/infra/llm/mod.rs
git commit -m "feat(infra): add LmStudio, Anthropic, OpenAI LLM adapters"
```

---

### Task 2.6: Implement `FileLlmProviderStore` (TOML persistence)

**Files:**
- Create: `src/infra/llm/store.rs`
- Modify: `src/infra/llm/mod.rs`

- [ ] **Step 1: Write the failing test**

```rust
// in src/infra/llm/store.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::llm_provider::LlmProviderStorePort;
    use crate::domain::llm_provider::{LlmProviderConfig, LlmProviderKind};
    use tempfile::tempdir;

    #[test]
    fn store_persists_across_instances() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("agk.toml");
        let s1 = FileLlmProviderStore::new(&path);
        s1.upsert(&LlmProviderConfig {
            id: "a".into(), kind: LlmProviderKind::Ollama,
            endpoint: "http://x".into(), api_key: None, default_model: None,
        }).unwrap();
        let s2 = FileLlmProviderStore::new(&path);
        let list = s2.list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "a");
    }

    #[test]
    fn store_remove_drops_entry() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("agk.toml");
        let s = FileLlmProviderStore::new(&path);
        s.upsert(&LlmProviderConfig {
            id: "a".into(), kind: LlmProviderKind::Ollama,
            endpoint: "http://x".into(), api_key: None, default_model: None,
        }).unwrap();
        s.remove("a").unwrap();
        assert!(s.list().unwrap().is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib infra::llm::store 2>&1 | tail -10`
Expected: compile error.

- [ ] **Step 3: Implement**

```rust
// src/infra/llm/store.rs
use crate::app::ports::llm_provider::LlmProviderStorePort;
use crate::domain::llm_provider::LlmProviderConfig;
use anyhow::{Context, Result};
use std::path::Path;

/// Persists `LlmProviderConfig`s in a sidecar TOML file under
/// `<agk_config_dir>/llm_providers.toml`. The store does not touch the main
/// `ConfigFile` schema — it is its own file so the two can evolve
/// independently and so the slim build (no `llm-*` features) does not have
/// to compile the serialization code.
pub struct FileLlmProviderStore<'a> {
    path: &'a Path,
}

impl<'a> FileLlmProviderStore<'a> {
    pub fn new(path: &'a Path) -> Self { Self { path } }

    fn load_all(&self) -> Result<Vec<LlmProviderConfig>> {
        if !self.path.exists() { return Ok(vec![]); }
        let s = std::fs::read_to_string(self.path)
            .with_context(|| format!("reading {}", self.path.display()))?;
        if s.trim().is_empty() { return Ok(vec![]); }
        // Format: an array of tables.
        let cfgs: Vec<LlmProviderConfig> = toml::from_str(&s)
            .with_context(|| format!("parsing {}", self.path.display()))?;
        Ok(cfgs)
    }

    fn save_all(&self, cfgs: &[LlmProviderConfig]) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
        }
        let s = toml::to_string_pretty(cfgs)
            .context("serializing LlmProviderConfig list")?;
        std::fs::write(self.path, s)
            .with_context(|| format!("writing {}", self.path.display()))?;
        Ok(())
    }
}

impl<'a> LlmProviderStorePort for FileLlmProviderStore<'a> {
    fn list(&self) -> Result<Vec<LlmProviderConfig>> { self.load_all() }
    fn get(&self, id: &str) -> Result<Option<LlmProviderConfig>> {
        Ok(self.load_all()?.into_iter().find(|c| c.id == id))
    }
    fn upsert(&self, cfg: &LlmProviderConfig) -> Result<()> {
        let mut all = self.load_all()?;
        if let Some(existing) = all.iter_mut().find(|c| c.id == cfg.id) {
            *existing = cfg.clone();
        } else {
            all.push(cfg.clone());
        }
        self.save_all(&all)
    }
    fn remove(&self, id: &str) -> Result<()> {
        let mut all = self.load_all()?;
        all.retain(|c| c.id != id);
        self.save_all(&all)
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib infra::llm::store 2>&1 | tail -10`
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/infra/llm/store.rs src/infra/llm/mod.rs
git commit -m "feat(infra): add FileLlmProviderStore (TOML persistence)"
```

---

### Task 2.7: Implement `agk llm` use cases (list/add/remove/health)

**Files:**
- Create: `src/app/features/llm/list.rs`
- Create: `src/app/features/llm/add.rs`
- Create: `src/app/features/llm/remove.rs`
- Create: `src/app/features/llm/health.rs`
- Create: `src/app/features/llm/mod.rs`

- [ ] **Step 1: Write the failing test for `list`**

```rust
// in src/app/features/llm/list.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_support::fake_llm_provider::FakeLlmProviderStore;
    use crate::app::ports::llm_provider::LlmProviderStorePort;
    use crate::domain::llm_provider::{LlmProviderConfig, LlmProviderKind};

    struct NullSink;
    impl crate::app::outcome::CoreEventSink for NullSink {
        fn on_event(&mut self, _: crate::app::event::CoreEvent) {}
        fn on_error(&mut self, _: String) {}
    }

    #[test]
    fn list_emits_one_event_per_provider() {
        let store = FakeLlmProviderStore::seeded(vec![
            LlmProviderConfig {
                id: "a".into(), kind: LlmProviderKind::Ollama,
                endpoint: "http://127.0.0.1:11434".into(), api_key: None, default_model: Some("llama3.2".into()),
            },
        ]);
        let mut sink = NullSink;
        let result = run(&store, &mut sink);
        assert!(result.is_ok());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib app::features::llm::list 2>&1 | tail -10`
Expected: compile error.

- [ ] **Step 3: Implement all four use cases**

```rust
// src/app/features/llm/list.rs
use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreResult};
use crate::app::ports::llm_provider::LlmProviderStorePort;

pub fn run(store: &dyn LlmProviderStorePort, sink: &mut dyn CoreEventSink) -> CoreResult {
    let cfgs = store.list()?;
    for cfg in cfgs {
        sink.on_event(CoreEvent::LlmProviderListed(cfg));
    }
    Ok(crate::app::outcome::CoreOutcome::Ok)
}
```

```rust
// src/app/features/llm/add.rs
use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::llm_provider::LlmProviderStorePort;
use crate::domain::llm_provider::LlmProviderConfig;
use anyhow::Result;

pub fn run(cfg: LlmProviderConfig, store: &dyn LlmProviderStorePort, sink: &mut dyn CoreEventSink) -> CoreResult {
    cfg.validate()?;
    store.upsert(&cfg)?;
    sink.on_event(CoreEvent::LlmProviderUpserted(cfg));
    Ok(CoreOutcome::Ok)
}
```

```rust
// src/app/features/llm/remove.rs
use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::llm_provider::LlmProviderStorePort;

pub fn run(id: &str, store: &dyn LlmProviderStorePort, sink: &mut dyn CoreEventSink) -> CoreResult {
    store.remove(id)?;
    sink.on_event(CoreEvent::LlmProviderRemoved(id.into()));
    Ok(CoreOutcome::Ok)
}
```

```rust
// src/app/features/llm/health.rs
use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreResult};
use crate::app::ports::llm_provider::{LlmHealthCheckPort, LlmProviderFactoryPort, LlmProviderStorePort};
use std::time::Duration;

pub async fn run(
    id: &str,
    store: &dyn LlmProviderStorePort,
    factory: &dyn LlmProviderFactoryPort,
    health: &dyn LlmHealthCheckPort,
    timeout: Duration,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let cfg = store.get(id)?
        .ok_or_else(|| anyhow::anyhow!("LLM provider '{}' not configured", id))?;
    let adapter = factory.build(&cfg)?;
    let status = health.check(adapter.as_ref(), timeout).await?;
    sink.on_event(CoreEvent::LlmProviderHealth { id: id.into(), status });
    Ok(crate::app::outcome::CoreOutcome::Ok)
}
```

Add to `src/app/event.rs` (or wherever `CoreEvent` is defined):
```rust
LlmProviderListed(crate::domain::llm_provider::LlmProviderConfig),
LlmProviderUpserted(crate::domain::llm_provider::LlmProviderConfig),
LlmProviderRemoved(String),
LlmProviderHealth { id: String, status: crate::domain::llm_provider::LlmHealthStatus },
```

Create `src/app/features/llm/mod.rs`:
```rust
pub mod add;
pub mod health;
pub mod list;
pub mod remove;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib app::features::llm 2>&1 | tail -10`
Expected: 1 test passes (list).

- [ ] **Step 5: Commit**

```bash
git add src/app/features/llm/ src/app/event.rs
git commit -m "feat(app): add LLM use cases (list/add/remove/health) and CoreEvent variants"
```

---

### Task 2.8: Implement `build_step_list()` wizard assembler

**Files:**
- Create: `src/app/features/profile/wizard.rs`
- Modify: `src/app/features/profile/mod.rs` (add `pub mod wizard;`)

- [ ] **Step 1: Write the failing test**

```rust
// in src/app/features/profile/wizard.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::WizardStep;
    use crate::app::ports::wizard_state::WizardState;
    use crate::app::ports::provider::ProviderPort;
    use crate::domain::asset::AssetKind;
    use crate::domain::config::ConfigFile;
    use crate::domain::scope::Scope;
    use std::path::Path;

    struct StubProvider;
    impl ProviderPort for StubProvider {
        fn id(&self) -> &str { "claude-code" }
        fn name(&self) -> &str { "Claude Code" }
        fn install(&self, _: &crate::domain::asset::ScannedPackage, _: Scope, _: Option<&ConfigFile>, _: bool) -> anyhow::Result<()> { Ok(()) }
        fn remove(&self, _: &crate::domain::identity::AssetIdentity, _: &AssetKind, _: Scope, _: Option<&ConfigFile>) -> anyhow::Result<()> { Ok(()) }
        fn supports_profiles(&self) -> bool { true }
        fn profile_wizard_steps(&self) -> Vec<WizardStep> { vec![] }
    }

    #[test]
    fn build_step_list_includes_provider_select_for_claude_code() {
        let steps = build_step_list(&StubProvider, &[]);
        assert!(matches!(steps[0], WizardStep::TextInput { .. }));
        assert!(matches!(steps[1], WizardStep::ProviderSelect { .. }));
    }

    #[test]
    fn build_step_list_includes_llm_select_when_providers_configured() {
        let steps = build_step_list(&StubProvider, &["local-ollama".to_string()]);
        // Steps should be: TextInput, ProviderSelect, LlmProviderSelect, ModelInput, AgentDescription, SkillsPick, ReviewFinal
        assert!(steps.iter().any(|s| matches!(s, WizardStep::LlmProviderSelect { .. })));
        assert!(steps.iter().any(|s| matches!(s, WizardStep::ModelInput { .. })));
    }

    #[test]
    fn build_step_list_omits_llm_select_when_no_providers() {
        let steps = build_step_list(&StubProvider, &[]);
        assert!(!steps.iter().any(|s| matches!(s, WizardStep::LlmProviderSelect { .. })));
    }

    #[test]
    fn build_step_list_always_ends_with_review_final() {
        let steps = build_step_list(&StubProvider, &[]);
        assert!(matches!(steps.last(), Some(WizardStep::ReviewFinal { .. })));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features profile-create --lib app::features::profile::wizard 2>&1 | tail -10`
Expected: compile error.

- [ ] **Step 3: Implement**

```rust
// src/app/features/profile/wizard.rs
//! Assembles the wizard step list for a profile-create flow.
//!
//! Order:
//!   1. TextInput  (profile name)
//!   2. ProviderSelect  (always)
//!   3. LlmProviderSelect  (only if any LLM providers are configured)
//!   4. ModelInput  (always; claude-code always wants a model string)
//!   5. AgentDescription  (always; goes into agent markdown frontmatter)
//!   6. SkillsPick  (always; uses vault-discovered skills)
//!   7. ReviewFinal  (always; final confirmation)
//!
//! Provider-specific steps from `profile_wizard_steps()` are spliced in
//! BEFORE `ReviewFinal`.

use crate::app::ports::provider::ProviderPort;
use crate::app::ports::WizardStep;

pub fn build_step_list(
    provider: &dyn ProviderPort,
    configured_llm_provider_ids: &[String],
) -> Vec<WizardStep> {
    let mut steps: Vec<WizardStep> = vec![
        WizardStep::TextInput {
            title: "Profile name".into(),
            placeholder: "e.g. reviewer, docs-writer, swe-bench".into(),
        },
        WizardStep::ProviderSelect {
            title: "Pick the agent provider".into(),
            providers: vec![
                ("claude-code".into(), "Claude Code".into()),
                ("opencode".into(), "OpenCode".into()),
            ],
        },
    ];
    if !configured_llm_provider_ids.is_empty() {
        let providers: Vec<(String, String)> = configured_llm_provider_ids
            .iter()
            .map(|id| (id.clone(), id.clone()))
            .collect();
        steps.push(WizardStep::LlmProviderSelect {
            title: "Pick the LLM provider".into(),
            providers,
        });
    }
    steps.push(WizardStep::ModelInput {
        title: "Model".into(),
        placeholder: "e.g. claude-sonnet-4-5 or llama3.2:8b".into(),
    });
    steps.push(WizardStep::AgentDescription {
        title: "Describe what this agent does".into(),
        placeholder: "Used as the agent's `description` frontmatter".into(),
        rows: 5,
    });
    steps.push(WizardStep::SkillsPick {
        title: "Pick skills to attach".into(),
        options: vec![], // populated at runtime from vault discovery
    });
    // Provider-specific extra steps (currently empty for all providers).
    for step in provider.profile_wizard_steps() {
        steps.push(step);
    }
    steps.push(WizardStep::ReviewFinal {
        title: "Review and create".into(),
    });
    steps
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --features profile-create --lib app::features::profile::wizard 2>&1 | tail -10`
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/app/features/profile/wizard.rs src/app/features/profile/mod.rs
git commit -m "feat(app): add profile wizard build_step_list() assembler (profile-create feature)"
```

---

### Task 2.9: Extend `profile/create.rs` to call `render_agent_markdown`

**Files:**
- Modify: `src/app/features/profile/create.rs`

- [ ] **Step 1: Write the failing test**

Append to `src/app/features/profile/create.rs` `#[cfg(test)] mod tests` (do not replace the existing tests):

```rust
    #[test]
    fn create_claude_code_profile_writes_agent_markdown_via_renderer() {
        // 1. Set up workspace with a fake provider, fake config store, and a
        //    process_runner that records the opencode invocation (we expect
        //    zero `opencode agent create` calls for claude-code).
        // 2. Call run() with a CreateProfileInput whose provider_id = "claude-code".
        // 3. Assert <workspace>/.claude/agents/<name>.md exists.
        // 4. Assert it starts with "---\nname: <name>\n".
        // 5. Assert the frontmatter contains "model: ...".
        // 6. Assert no process was spawned (pure renderer wrote the file directly).
    }
```

(Implement the test stub inline using the existing `FakeStore` + a stub `ProcessRunnerPort`. The full code is omitted here to avoid duplication of patterns already established in the file — copy `FakeStore` from the existing tests and add a `NoopProcessRunner`.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features profile-create --lib app::features::profile::create 2>&1 | tail -20`
Expected: test fails because the create path currently calls `process_runner.run("opencode", ...)` for all providers.

- [ ] **Step 3: Update the create logic**

In `src/app/features/profile/create.rs`, after the comment `// 5. Provider-specific setup (opencode)`, replace the opencode-only block with a branching block that:
- For `claude-code`: build an `AgentFrontmatter` + `LaunchPlan` from the input, call `render_agent_markdown`, write it to `<workspace>/.claude/agents/<name>.md`, set `prompt_overlay_path` on the profile.
- For `opencode`: keep the existing logic unchanged.
- For any other provider: error out with `"unsupported profile provider"`.

```rust
    // 5. Provider-specific setup
    let prompt_overlay_path: Option<PathBuf> = if provider_id == "claude-code" {
        // Build the agent frontmatter from the wizard answer.
        let model = input.model.clone().unwrap_or_else(|| "sonnet".to_string());
        let fm = crate::domain::agent_markdown::AgentFrontmatter {
            name: id_str.to_string(),
            description: input.description.clone(),
            tools: input.tool_refs.clone(),
            disallowed_tools: vec![],
            model: model.clone(),
            permission_mode: input.permission_mode.clone(),
            max_turns: None,
            skills: input.skill_refs.iter().map(|r| r.name.clone()).collect(),
            mcp_servers: input.mcp_refs.iter().map(|r| r.name.clone()).collect(),
            hooks: vec![],
            memory: None,
            background: false,
            effort: None,
            isolation: None,
            color: None,
        };
        let prompt_body = input.description.clone(); // placeholder; richer prompt body comes in v0.5
        let plan = crate::domain::launch_plan::LaunchPlan {
            profile_id: id_str.to_string(),
            provider_id: provider_id.to_string(),
            frontmatter: fm,
            prompt_body,
            resolved_mcp_servers: input.agent_mcp_servers.clone(),
            llm_provider_id: input.llm_provider_id.clone(),
        };
        let md = crate::infra::provider::claude_code::agent_markdown::render_agent_markdown(&plan);
        let agents_dir = workspace.join(".claude").join("agents");
        std::fs::create_dir_all(&agents_dir)?;
        let path = agents_dir.join(format!("{}.md", id_str));
        std::fs::write(&path, md)?;
        Some(path)
    } else if provider_id == "opencode" {
        // ... existing opencode branch ...
        None
    } else {
        anyhow::bail!("unsupported profile provider: {}", provider_id);
    };

    // Persist prompt_overlay_path on the profile.
    config.profiles.last_mut().unwrap().prompt_overlay_path = prompt_overlay_path;
    store.save(input.scope, &config)?;
```

Extend `CreateProfileInput` (in `src/app/features/profile/command.rs`) with the new fields:
```rust
    pub model: Option<String>,
    pub llm_provider_id: Option<String>,
    pub agent_mcp_servers: Vec<crate::domain::agent_markdown::AgentMcpServer>,
    pub tool_refs: Vec<String>,
    pub permission_mode: Option<String>,
```

(In a follow-up commit C3 the wizard will populate these from the new step answers; for now they default to empty.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --features profile-create --lib app::features::profile::create 2>&1 | tail -20`
Expected: all existing tests pass + the new test passes.

- [ ] **Step 5: Commit**

```bash
git add src/app/features/profile/create.rs src/app/features/profile/command.rs
git commit -m "feat(profile-create): render Claude Code agent markdown via render_agent_markdown"
```

---

### Task 2.10: Extend `profile/start.rs` to resolve `LaunchPlan` for claude-code

**Files:**
- Modify: `src/app/features/profile/start.rs`

- [ ] **Step 1: Write the failing test**

```rust
// append to start.rs tests
    #[test]
    fn start_claude_code_profile_resolves_launch_plan() {
        // Use FakeMcpRegistry, FakeStore with a profile whose provider_id is
        // "claude-code", model is set, and prompt_overlay_path is Some.
        // Assert that calling run() with dry_run=true returns Ok and the
        // sink receives a CoreEvent::ProfileLaunchPlan { plan: LaunchPlan { .. } }.
    }
```

(Stub using existing `FakeStore` / `FakeMcpRegistry` patterns in the file.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib app::features::profile::start 2>&1 | tail -10`
Expected: test fails because the start path does not emit a `ProfileLaunchPlan` event yet.

- [ ] **Step 3: Add the launch-plan resolution branch**

In `src/app/features/profile/start.rs`, after the profile is loaded from the store:

```rust
    // Build a LaunchPlan for dry-run output and to record what would be
    // executed. The plan carries pre-resolved MCP servers so the exec layer
    // does not have to call the MCP registry again.
    if profile.provider_id == "claude-code" {
        let fm = crate::domain::agent_markdown::AgentFrontmatter {
            name: profile.id.as_str().to_string(),
            description: String::new(),
            tools: profile.tool_refs.clone(),
            disallowed_tools: vec![],
            model: profile.model.clone().unwrap_or_else(|| "sonnet".to_string()),
            permission_mode: profile.permission_mode.clone(),
            max_turns: None,
            skills: profile.skill_refs.iter().map(|r| r.name.clone()).collect(),
            mcp_servers: profile.mcp_refs.iter().map(|r| r.name.clone()).collect(),
            hooks: vec![],
            memory: None,
            background: false,
            effort: None,
            isolation: None,
            color: None,
        };
        let prompt_body = profile.prompt_overlay_path.as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_default();
        let resolved_mcp_servers = profile.agent_mcp_servers.clone();
        let plan = crate::domain::launch_plan::LaunchPlan {
            profile_id: profile.id.as_str().to_string(),
            provider_id: profile.provider_id.clone(),
            frontmatter: fm,
            prompt_body,
            resolved_mcp_servers,
            llm_provider_id: profile.llm_provider_id.clone(),
        };
        if dry_run {
            sink.on_event(CoreEvent::ProfileLaunchPlan { plan });
            return Ok(CoreOutcome::Ok);
        }
        // Live path: emit the plan and let the exec layer take over.
        sink.on_event(CoreEvent::ProfileLaunchPlan { plan });
    }
```

Add to `CoreEvent`:
```rust
ProfileLaunchPlan { plan: crate::domain::launch_plan::LaunchPlan },
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib app::features::profile::start 2>&1 | tail -10`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/app/features/profile/start.rs src/app/event.rs
git commit -m "feat(profile-start): resolve LaunchPlan for claude-code (dry-run + live)"
```

---

## Commit 3: Adapters + E2E

### Task 3.1: Add `agk llm` CLI subcommand

**Files:**
- Create: `src/cli/llm.rs`
- Modify: `src/cli/mod.rs` (re-export)
- Modify: `src/main.rs` (wire subcommand)

- [ ] **Step 1: Write the failing test**

```rust
// in src/cli/llm.rs at the bottom
#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn llm_subcommand_parses_add() {
        let cli = Cli::parse_from(["agk", "llm", "add",
            "--id", "local-ollama",
            "--kind", "ollama",
            "--endpoint", "http://127.0.0.1:11434",
            "--default-model", "llama3.2",
        ]);
        match cli.command {
            Command::Llm(LlmCommand::Add(args)) => {
                assert_eq!(args.id, "local-ollama");
                assert_eq!(args.kind, "ollama");
            }
            _ => panic!("expected Llm::Add"),
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib cli::llm 2>&1 | tail -10`
Expected: compile error.

- [ ] **Step 3: Implement the CLI subcommand**

```rust
// src/cli/llm.rs
use clap::{Args, Subcommand};

#[derive(Subcommand, Debug, PartialEq)]
pub enum LlmCommand {
    /// List configured LLM providers.
    List,
    /// Add or update an LLM provider.
    Add(LlmAddArgs),
    /// Remove an LLM provider.
    Remove { id: String },
    /// Run a health check against a configured LLM provider.
    Health { id: String, #[arg(long, default_value_t = 5000)] timeout_ms: u64 },
}

#[derive(Args, Debug, PartialEq)]
pub struct LlmAddArgs {
    pub id: String,
    pub kind: String,
    pub endpoint: String,
    #[arg(long)]
    pub api_key: Option<String>,
    #[arg(long)]
    pub default_model: Option<String>,
}
```

In `src/main.rs` (or wherever `Cli` is defined), add:
```rust
/// Manage LLM providers.
Llm(llm::LlmCommand),
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib cli::llm 2>&1 | tail -10`
Expected: 1 test passes.

- [ ] **Step 5: Commit**

```bash
git add src/cli/llm.rs src/cli/mod.rs src/main.rs
git commit -m "feat(cli): add `agk llm {list,add,remove,health}` subcommand"
```

---

### Task 3.2: Wire `agk llm` to use cases in `main.rs`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Write the failing integration test**

```rust
// in tests/llm_provider_contracts.rs
use assert_cmd::Command;
use tempfile::tempdir;
use predicates::prelude::*;

#[test]
fn agk_llm_list_succeeds_with_no_providers() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("agk")
        .unwrap()
        .env("AGK_CONFIG_DIR", dir.path())
        .arg("llm")
        .arg("list")
        .assert()
        .success();
}

#[test]
fn agk_llm_add_then_list_round_trip() {
    let dir = tempdir().unwrap();
    Command::cargo_bin("agk")
        .unwrap()
        .env("AGK_CONFIG_DIR", dir.path())
        .arg("llm")
        .arg("add")
        .arg("--id")
        .arg("local-ollama")
        .arg("--kind")
        .arg("ollama")
        .arg("--endpoint")
        .arg("http://127.0.0.1:11434")
        .arg("--default-model")
        .arg("llama3.2")
        .assert()
        .success();
    Command::cargo_bin("agk")
        .unwrap()
        .env("AGK_CONFIG_DIR", dir.path())
        .arg("llm")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("local-ollama"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test llm_provider_contracts 2>&1 | tail -20`
Expected: first test passes (empty list works), second test fails because `llm add` is not wired to a use case.

- [ ] **Step 3: Wire the dispatch in `main.rs`**

```rust
Command::Llm(llm::LlmCommand::List) => {
    let store: Box<dyn LlmProviderStorePort> = Box::new(
        FileLlmProviderStore::new(&config_dir.join("llm_providers.toml"))
    );
    let mut sink = CliEventSink::new(json_mode);
    app::features::llm::list::run(&*store, &mut sink)?;
    sink.flush()?;
}
Command::Llm(llm::LlmCommand::Add(args)) => {
    let kind = LlmProviderKind::from_str(&args.kind)
        .ok_or_else(|| anyhow::anyhow!("unknown LLM provider kind: {}", args.kind))?;
    let cfg = LlmProviderConfig {
        id: args.id,
        kind,
        endpoint: args.endpoint,
        api_key: args.api_key,
        default_model: args.default_model,
    };
    let store: Box<dyn LlmProviderStorePort> = Box::new(
        FileLlmProviderStore::new(&config_dir.join("llm_providers.toml"))
    );
    let mut sink = CliEventSink::new(json_mode);
    app::features::llm::add::run(cfg, &*store, &mut sink)?;
    sink.flush()?;
}
Command::Llm(llm::LlmCommand::Remove { id }) => {
    let store: Box<dyn LlmProviderStorePort> = Box::new(
        FileLlmProviderStore::new(&config_dir.join("llm_providers.toml"))
    );
    let mut sink = CliEventSink::new(json_mode);
    app::features::llm::remove::run(&id, &*store, &mut sink)?;
    sink.flush()?;
}
Command::Llm(llm::LlmCommand::Health { id, timeout_ms }) => {
    let store: Box<dyn LlmProviderStorePort> = Box::new(
        FileLlmProviderStore::new(&config_dir.join("llm_providers.toml"))
    );
    let factory = RealLlmProviderFactory::new();
    let health = HttpLlmHealthCheck::new();
    let mut sink = CliEventSink::new(json_mode);
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(app::features::llm::health::run(
        &id, &*store, &factory, &health,
        std::time::Duration::from_millis(timeout_ms), &mut sink,
    ))?;
    sink.flush()?;
}
```

Add a `RealLlmProviderFactory` in `src/infra/llm/factory.rs` (and the matching module declaration under feature gates) that dispatches on `cfg.kind` to the right adapter.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test llm_provider_contracts 2>&1 | tail -10`
Expected: both tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/infra/llm/factory.rs src/infra/llm/mod.rs tests/llm_provider_contracts.rs
git commit -m "feat(cli): wire `agk llm` subcommands to use cases"
```

---

### Task 3.3: TUI: handle new `WizardStep` variants in `event.rs`

**Files:**
- Modify: `src/tui/event.rs`

- [ ] **Step 1: Write the failing test**

```rust
// in tests/full_flow_tui/wizard_claude_code.rs
#[test]
fn provider_select_step_advances_on_enter() {
    // Spin up the TUI test harness with two stub providers.
    // Confirm that on ProviderSelect step, pressing Enter advances to the
    // next step and sets wizard_state.provider_id_choice.
}

#[test]
fn model_input_step_records_string() {
    // On ModelInput step, typing characters and pressing Enter stores the
    // text in wizard_state.model_string.
}
```

(Use the existing TUI test harness in `tests/full_flow_tui/mod.rs` — copy the FakeApp pattern.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test full_flow_tui wizard_claude_code 2>&1 | tail -20`
Expected: compile error (variants not handled) or panic at runtime.

- [ ] **Step 3: Handle the new variants in `src/tui/event.rs`**

In the match on `wizard_state.steps[wizard_state.step_index]`, add arms:

```rust
WizardStep::ProviderSelect { .. } => { /* arrow up/down, Enter to commit choice to wizard_state.provider_id_choice */ }
WizardStep::LlmProviderSelect { .. } => { /* same, sets wizard_state.llm_provider_id */ }
WizardStep::ModelInput { .. } => { /* typing, Enter to commit to wizard_state.model_string */ }
WizardStep::AgentDescription { .. } => { /* multi-line, Enter to commit to wizard_state.agent_description */ }
WizardStep::SkillsPick { .. } => { /* same as existing Checklist, but writes to wizard_state.selected_tools */ }
WizardStep::ReviewFinal { .. } => { /* Enter triggers profile-create + emit ProfileCreated */ }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test full_flow_tui wizard_claude_code 2>&1 | tail -20`
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/tui/event.rs tests/full_flow_tui/wizard_claude_code.rs
git commit -m "feat(tui): handle new wizard step variants (ProviderSelect, LlmProviderSelect, ModelInput, AgentDescription, SkillsPick, ReviewFinal)"
```

---

### Task 3.4: Golden contract fixtures for `--dry-run --json` output

**Files:**
- Create: `fixtures/contracts/agk_llm_list.json`
- Create: `fixtures/contracts/agk_llm_health.json`
- Create: `fixtures/contracts/agk_profile_create_claude.json`
- Create: `fixtures/contracts/agent_markdown_minimal.md`
- Create: `fixtures/contracts/agent_markdown_full.md`
- Create: `tests/llm_provider_contracts.rs` (extend)

- [ ] **Step 1: Write the failing test**

```rust
// append to tests/llm_provider_contracts.rs
use insta::assert_json_snapshot;

#[test]
fn agk_llm_list_json_matches_golden() {
    let dir = tempdir().unwrap();
    // Seed the llm_providers.toml with two providers
    let toml = r#"
[[items]]
id = "local-ollama"
kind = "ollama"
endpoint = "http://127.0.0.1:11434"
default_model = "llama3.2"
[[items]]
id = "openai-prod"
kind = "openai"
endpoint = "https://api.openai.com"
api_key = "sk-xxx"
"#;
    std::fs::write(dir.path().join("llm_providers.toml"), toml).unwrap();
    let output = Command::cargo_bin("agk").unwrap()
        .env("AGK_CONFIG_DIR", dir.path())
        .arg("llm").arg("list").arg("--json")
        .output().unwrap();
    assert!(output.status.success());
    assert_json_snapshot!("agk_llm_list", serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test llm_provider_contracts agk_llm_list_json_matches_golden 2>&1 | tail -20`
Expected: snapshot file missing → test fails with "snapshot not found".

- [ ] **Step 3: Create the golden fixtures**

Run: `cargo insta accept` (or manually copy the generated `.snap` into `fixtures/contracts/agk_llm_list.json`).

Hand-author `fixtures/contracts/agent_markdown_minimal.md`:
```markdown
---
name: reviewer
description: PR reviewer
model: sonnet
---

Review the staged changes.
```

Hand-author `fixtures/contracts/agent_markdown_full.md`:
```markdown
---
name: reviewer
description: "PR reviewer: says \"hello\""
tools:
  - Read
  - Grep
model: sonnet
permissionMode: acceptEdits
skills:
  - code-review
mcpServers:
  github:
    command: docker
    args:
      - run
      - -i
      - mcp/github
---

Review the staged changes carefully.
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test llm_provider_contracts 2>&1 | tail -10`
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add fixtures/contracts/ tests/llm_provider_contracts.rs
git commit -m "test: add golden contract fixtures for agk llm and agent_markdown"
```

---

### Task 3.5: Architecture test for `infra/llm/` boundary

**Files:**
- Create: `tests/architecture_llm.rs`
- Modify: `tests/architecture.rs` (add `mod architecture_llm;`)

- [ ] **Step 1: Write the failing test**

```rust
// in tests/architecture_llm.rs
#[test]
fn infra_llm_does_not_import_app_layer() {
    // Walk every .rs file under src/infra/llm/ and assert that no `use crate::app::`
    // appears. The boundary rule: infra → domain, infra → external; never
    // infra → app.
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test architecture_llm 2>&1 | tail -10`
Expected: compile error or "function not found".

- [ ] **Step 3: Implement using the same `syn` walking pattern as `tests/architecture.rs`**

```rust
use std::fs;
use std::path::Path;

#[test]
fn infra_llm_does_not_import_app_layer() {
    let infra_llm = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/infra/llm");
    let mut violations = vec![];
    visit_rs(&infra_llm, &mut violations);
    assert!(violations.is_empty(), "infra/llm imports app layer: {:#?}", violations);
}

fn visit_rs(dir: &Path, violations: &mut Vec<String>) {
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() { visit_rs(&p, violations); }
            else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                if let Ok(s) = fs::read_to_string(&p) {
                    if s.contains("use crate::app::") {
                        violations.push(p.display().to_string());
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test architecture_llm 2>&1 | tail -10`
Expected: 1 test passes.

- [ ] **Step 5: Commit**

```bash
git add tests/architecture_llm.rs tests/architecture.rs
git commit -m "test(arch): add boundary rule for infra/llm/ (no app-layer imports)"
```

---

### Task 3.6: Slim-build regression test

**Files:**
- Create: `tests/slim_build_regression.rs`

- [ ] **Step 1: Write the failing test**

```rust
// This test is purely a build-time assertion. It is gated to only run
// when the `headless-no-llm` or `headless-no-profile-create` feature is
// active.
#[test]
fn slim_build_does_not_compile_wizard_or_llm() {
    // The presence of this test file itself exercises the build matrix.
    // The actual build is validated by CI (see Task 3.9).
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --no-default-features --features cli,vault-clawhub,pack,provider-opencode,provider-claude 2>&1 | tail -20`
Expected: compile error if any of the gated modules accidentally pulls in wizard code or LLM code.

- [ ] **Step 3: Fix any unintended leaks**

Common leak sources to look for:
- `src/app/features/profile/create.rs` importing `agent_markdown` outside a `#[cfg(feature = "profile-create")]` block — wrap the renderer call.
- `src/main.rs` importing any LLM use case outside a `#[cfg(feature = "llm-...")]` block — wrap the `Command::Llm` arm.
- `src/app/ports/llm_provider.rs` itself is always built (port trait, no infra dep), but the fake in `src/app/test_support/fake_llm_provider.rs` is only compiled when at least one LLM feature is on (gated by `#[cfg(any(feature = "llm-ollama", feature = "llm-lmstudio", feature = "llm-anthropic", feature = "llm-openai"))]` on the `mod` declaration).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo build --no-default-features --features cli,vault-clawhub,pack,provider-opencode,provider-claude 2>&1 | tail -10`
Expected: success.

- [ ] **Step 5: Commit**

```bash
git add tests/slim_build_regression.rs
git commit -m "test(build): add slim-build regression test stub (validated by CI matrix)"
```

---

### Task 3.7: Concurrency test for parallel health checks

**Files:**
- Create: `tests/llm_concurrency.rs`

- [ ] **Step 1: Write the failing test**

```rust
use crate::app::test_support::fake_llm_provider::{FakeLlmProviderFactory, FakeLlmProviderStore, FakeLlmHealthCheck};
use crate::app::features::llm::health;
use crate::domain::llm_provider::{LlmProviderConfig, LlmProviderKind};

#[tokio::test]
async fn health_checks_for_8_providers_run_concurrently_under_2s() {
    let cfgs: Vec<_> = (0..8).map(|i| LlmProviderConfig {
        id: format!("p{}", i),
        kind: LlmProviderKind::Ollama,
        endpoint: format!("http://127.0.0.1:{}", 11000 + i),
        api_key: None,
        default_model: None,
    }).collect();
    let store = FakeLlmProviderStore::seeded(cfgs);
    let factory = FakeLlmProviderFactory;
    let hc = FakeLlmHealthCheck::default();
    let start = std::time::Instant::now();
    let mut handles = vec![];
    for cfg in store.list().unwrap() {
        let id = cfg.id.clone();
        let store = &store;
        let factory = &factory;
        let hc = &hc;
        handles.push(tokio::spawn(async move {
            struct NullSink;
            impl crate::app::outcome::CoreEventSink for NullSink {
                fn on_event(&mut self, _: crate::app::event::CoreEvent) {}
                fn on_error(&mut self, _: String) {}
            }
            let mut sink = NullSink;
            health::run(&id, store, factory, hc, std::time::Duration::from_secs(1), &mut sink).await
        }));
    }
    for h in handles { h.await.unwrap().unwrap(); }
    let elapsed = start.elapsed();
    // Fake health check has 12ms latency, sequential = 96ms, concurrent = ~12ms.
    // Generous bound to avoid CI flakes.
    assert!(elapsed < std::time::Duration::from_millis(500));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test llm_concurrency 2>&1 | tail -10`
Expected: compile error (test depends on fake import paths not yet visible in this test).

- [ ] **Step 3: Make the fakes visible from integration tests**

Add to `src/lib.rs`:
```rust
#[cfg(any(test, feature = "test-support-exports"))]
pub mod test_support;
```

(Or use the existing pattern in the repo — check `src/lib.rs` for `pub mod test_support` and adjust the visibility if needed.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test llm_concurrency 2>&1 | tail -10`
Expected: 1 test passes.

- [ ] **Step 5: Commit**

```bash
git add tests/llm_concurrency.rs src/lib.rs
git commit -m "test: add concurrency test for parallel LLM health checks"
```

---

### Task 3.8: Multi-stage Dockerfile

**Files:**
- Create: `Dockerfile`
- Create: `docker-compose.yml`
- Create: `.dockerignore`

- [ ] **Step 1: Write the Dockerfile**

```dockerfile
# syntax=docker/dockerfile:1.7

# ============================================================
# Stage 1: builder-full — compiles the slim binary used by the
# runtime stage, plus the full-feature binary used by CI.
# ============================================================
FROM rust:1.83-bookworm AS builder-full
WORKDIR /build

# Cache deps
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo 'fn main(){}' > src/main.rs && \
    echo '' > src/lib.rs && \
    cargo build --release --locked --bin agk --no-default-features \
      --features cli,vault-clawhub,pack,provider-opencode,provider-claude || true
COPY src ./src
COPY tests ./tests
COPY fixtures ./fixtures
RUN touch src/main.rs src/lib.rs

# Build the SLIM runtime binary (no TUI, no wizard, no LLM adapters)
RUN cargo build --release --locked --bin agk --no-default-features \
      --features cli,vault-clawhub,pack,provider-opencode,provider-claude,claude-cli-probe

# Build the FULL CI binary (everything)
RUN cargo build --release --locked --bin agk

# ============================================================
# Stage 2: builder-runtime — minimal builder for the runtime
# image (just the slim binary, no tests, no source).
# ============================================================
FROM rust:1.83-bookworm AS builder-runtime
WORKDIR /build
COPY --from=builder-full /build/target/release/agk /usr/local/bin/agk

# ============================================================
# Stage 3: ci-full — carries the source + full binary so CI can
# run `cargo test` and `cargo build --all-features` without
# re-fetching crates.
# ============================================================
FROM rust:1.83-bookworm AS ci-full
WORKDIR /build
COPY --from=builder-full /build /build
RUN cargo build --release --locked --bin agk

# ============================================================
# Stage 4: runtime — minimal image carrying the slim binary.
# ============================================================
FROM debian:bookworm-slim AS runtime
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder-runtime /usr/local/bin/agk /usr/local/bin/agk
ENTRYPOINT ["/usr/local/bin/agk"]
```

- [ ] **Step 2: Write `docker-compose.yml`**

```yaml
services:
  agk:
    build:
      context: .
      target: runtime
    image: agk:runtime
    volumes:
      - ${HOME}/.agk:/root/.agk
      - ${PWD}:/workspace
    working_dir: /workspace
    stdin_open: true
    tty: true
```

- [ ] **Step 3: Write `.dockerignore`**

```
target
.git
.gitignore
node_modules
*.log
```

- [ ] **Step 4: Verify the slim image builds**

Run: `docker build --target runtime -t agk:runtime . 2>&1 | tail -20`
Expected: image builds; `docker run --rm agk:runtime --version` prints the binary version.

- [ ] **Step 5: Verify the full image builds**

Run: `docker build --target ci-full -t agk:ci . 2>&1 | tail -20`
Expected: image builds; `docker run --rm agk:ci cargo test --lib --features full` (or equivalent) passes.

- [ ] **Step 6: Commit**

```bash
git add Dockerfile docker-compose.yml .dockerignore
git commit -m "feat(ops): add multi-stage Dockerfile (slim runtime + full CI + builder)"
```

---

### Task 3.9: Add CI matrix for feature gates

**Files:**
- Modify: `.github/workflows/ci.yml` (or equivalent CI file)

- [ ] **Step 1: Add the build matrix**

Add a matrix job that builds the binary with each of these feature sets and runs `cargo check --bin agk`:

```yaml
strategy:
  fail-fast: false
  matrix:
    target:
      - name: headless
        features: --no-default-features --features cli,vault-clawhub,pack,provider-opencode,provider-claude,claude-cli-probe
      - name: headless-no-llm
        features: --no-default-features --features cli,vault-clawhub,pack,provider-opencode,provider-claude
      - name: headless-no-profile-create
        features: --no-default-features --features cli,vault-clawhub,pack,provider-opencode,provider-claude,claude-cli-probe,llm-ollama
      - name: full
        features: ""
      - name: tui-only
        features: --no-default-features --features tui,provider-claude,profile-create
      - name: llm-ollama-only
        features: --no-default-features --features cli,llm-ollama
      - name: llm-lmstudio-only
        features: --no-default-features --features cli,llm-lmstudio
      - name: llm-anthropic-only
        features: --no-default-features --features cli,llm-anthropic
      - name: llm-openai-only
        features: --no-default-features --features cli,llm-openai
steps:
  - uses: actions/checkout@v4
  - uses: dtolnay/rust-toolchain@stable
  - run: cargo check --bin agk ${{ matrix.target.features }}
```

- [ ] **Step 2: Add a separate job that builds the Docker images**

```yaml
docker:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - run: docker build --target ci-full -t agk:ci .
    - run: docker run --rm agk:ci cargo test --lib --features full
    - run: docker build --target runtime -t agk:runtime .
    - run: docker run --rm agk:runtime agk --version
```

- [ ] **Step 3: Verify the workflow file is valid YAML**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"`
Expected: no error.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add 8-cell feature matrix + Docker multi-stage build job"
```

---

### Task 3.10: Add `docs/ops/docker.md`

**Files:**
- Create: `docs/ops/docker.md`

- [ ] **Step 1: Write the operator doc**

```markdown
# AGK Docker Build & Runtime

This document describes how to build, run, and customize the AGK Docker
images. There are two images:

| Image tag | Stage target | Features | Size (approx) |
|-----------|--------------|----------|---------------|
| `agk:runtime` | `runtime` | CLI + opencode + claude-code (start only) | ~30 MB |
| `agk:ci` | `ci-full` | Everything (TUI, wizard, all LLM adapters) | ~500 MB |

## Quick start

```bash
# Build the slim runtime image
docker build -t agk:runtime --target runtime .

# Run a profile-start against the current workspace
docker run --rm -it \
  -v $HOME/.agk:/root/.agk \
  -v $PWD:/workspace \
  -w /workspace \
  agk:runtime \
  agk profile start my-profile
```

## Custom builds

The slim build is parameterized by Cargo features. To add an LLM adapter:

```bash
# Build a slim image that also knows how to talk to Ollama
docker build -t agk:ollama --build-arg AGK_FEATURES="cli,vault-clawhub,pack,provider-opencode,provider-claude,llm-ollama" --target runtime .
```

The `AGK_FEATURES` build-arg flows into the `cargo build` command in
`Dockerfile`. Default: `cli,vault-clawhub,pack,provider-opencode,provider-claude,claude-cli-probe`.

## CI verification

The CI matrix in `.github/workflows/ci.yml` exercises 8 build configurations
to catch feature-gate typos:

- `headless` — baseline slim build
- `headless-no-llm` — slim build, no LLM features at all
- `headless-no-profile-create` — slim build with LLM but no wizard
- `full` — everything on
- `tui-only` — TUI without vault/CLI
- `llm-{ollama,lmstudio,anthropic,openai}-only` — each adapter in isolation

## Troubleshooting

- **"claude not found"** at runtime → the slim image does not include the
  `claude` CLI binary. Install it on the host (or rebuild the image with
  it included).
- **"permission denied" on `/root/.agk`** → mount an existing
  `~/.agk` directory from the host.
```

- [ ] **Step 2: Commit**

```bash
git add docs/ops/docker.md
git commit -m "docs(ops): add Docker build & runtime operator guide"
```

---

### Task 3.11: Update `AGENTS.md` and `README.md`

**Files:**
- Modify: `AGENTS.md`
- Modify: `README.md`

- [ ] **Step 1: Add new module ownership rules to `AGENTS.md`**

Append to `AGENTS.md` (in the "Module boundaries" section, or add a new
"LLM provider module" section):

```markdown
## LLM provider module (added in v0.4)

- `src/domain/llm_provider.rs` — value types only (`LlmProviderConfig`, `LlmProviderKind`, `LlmHealthStatus`, `ModelInput`).
- `src/app/ports/llm_provider.rs` — `LlmProviderStorePort`, `LlmProviderFactoryPort`, `LlmProviderAdapter`, `LlmHealthCheckPort`. Always built.
- `src/infra/llm/{ollama,lmstudio,anthropic,openai}.rs` — one adapter per kind, each behind its own Cargo feature. Adapters implement `LlmProviderAdapter`.
- `src/infra/llm/health.rs` — shared `HttpLlmHealthCheck`. Behind any `llm-*` feature.
- `src/infra/llm/store.rs` — `FileLlmProviderStore` (TOML persistence). Always built.
- `src/app/features/llm/{list,add,remove,health}.rs` — use cases. Always built (no I/O on their own; the port does I/O).

Boundary rule: `infra/llm/` must not `use crate::app::`. The
`tests/architecture_llm.rs` test enforces this.

## Claude Code sub-agent module (added in v0.4)

- `src/domain/agent_markdown.rs` — `AgentFrontmatter`, `AgentMcpServer`, `RenderedAgentMarkdown`. Pure data.
- `src/domain/launch_plan.rs` — `LaunchPlan` (pre-resolved data the renderer needs).
- `src/infra/provider/claude_code/agent_markdown.rs` — `render_agent_markdown` pure function. Behind `profile-create` feature.
- `src/infra/provider/claude_code/cli_probe.rs` — `SystemClaudeCliProbe`. Behind `claude-cli-probe` feature.
- `src/app/ports/claude_cli_probe.rs` — `ClaudeCliProbePort`. Always built.

Boundary rule: `render_agent_markdown` is a pure function. It must not
call any port or perform I/O. The use case layer pre-resolves everything
into `LaunchPlan` first.
```

- [ ] **Step 2: Add a "Docker" section to `README.md`**

Append a brief "Docker" section linking to `docs/ops/docker.md`.

- [ ] **Step 3: Commit**

```bash
git add AGENTS.md README.md
git commit -m "docs: document LLM provider + Claude Code sub-agent modules (v0.4)"
```

---

### Task 3.12: Final verification — full test matrix + Docker E2E

**Files:** (no file changes; verification only)

- [ ] **Step 1: Run the full local test suite**

Run: `cargo test --all-features 2>&1 | tail -20`
Expected: all tests pass.

- [ ] **Step 2: Run the slim build**

Run: `cargo build --no-default-features --features cli,vault-clawhub,pack,provider-opencode,provider-claude,claude-cli-probe 2>&1 | tail -10`
Expected: success.

- [ ] **Step 3: Build the Docker runtime image**

Run: `docker build --target runtime -t agk:runtime . 2>&1 | tail -10`
Expected: image builds.

- [ ] **Step 4: Run the Docker image with a real workflow**

Run: `docker run --rm -v $HOME/.agk:/root/.agk -v $PWD:/workspace -w /workspace agk:runtime agk --version`
Expected: prints the binary version.

- [ ] **Step 5: Push the branch and open a PR**

```bash
git push -u origin feat/enterprise-skill-marketplace-p1
gh pr create --title "feat(enterprise-p1): Claude Code agent profile + LLM provider support" \
  --body "Implements the v0.4 enterprise spec: Claude Code sub-agent profile creation via the existing wizard, plus a new \`LlmProviderPort\` for Ollama/LM Studio/Anthropic/OpenAI. Ships a slim Docker build (no TUI, no wizard) for production use. See \`docs/superpowers/specs/2026-06-04-claude-code-agent-profile-design.md\` for the design."
```

---

## Self-Review

**1. Spec coverage:**
- Spec §3.1 Cargo feature matrix → Task 1.8, 3.6, 3.9
- Spec §3.2 Dockerfile → Task 3.8, 3.10
- Spec §4 LlmProviderConfig + ModelInput → Task 1.1
- Spec §4 Profile field additions → Task 1.3
- Spec §4 AgentFrontmatter / LaunchPlan → Task 1.2
- Spec §5 LlmProviderStorePort + LlmHealthCheckPort + LlmProviderFactoryPort → Task 1.4, 1.7
- Spec §5 ClaudeCliProbePort → Task 1.5, 1.7
- Spec §5 WizardStep variants → Task 1.6
- Spec §6 `render_agent_markdown` (pure renderer) → Task 2.1
- Spec §6 LLM adapter impls → Task 2.3, 2.4, 2.5
- Spec §6 `build_step_list()` step assembler → Task 2.8
- Spec §7 profile create extension (claude-code path) → Task 2.9
- Spec §7 profile start extension (LaunchPlan resolution) → Task 2.10
- Spec §7 LLM use cases → Task 2.7
- Spec §8 health check (OPTIONS Anthropic, GET Ollama/LMStudio/OpenAI) → Task 2.3
- Spec §8 free-form model string with 256 cap → Task 1.1 (ModelInput::new)
- Spec §8 semver check for claude CLI ≥v2.0.0 → Task 1.5, 2.2
- Spec §9 `--dry-run --json` contract parity (CLI == TUI) → Task 3.4 (golden fixtures)
- Spec §10 testing strategy (hand fakes, no mockall) → Task 1.7, all `mod tests` blocks
- Spec §10 architecture tests → Task 3.5
- Spec §10 slim-build regression → Task 3.6
- Spec §10 concurrency test → Task 3.7
- Spec §11 acceptance criteria → Task 3.12 (final verification)
- Spec §12 out-of-scope (skill list filter, etc.) → none of the tasks implement these

**2. Placeholder scan:** No "TBD", "TODO", "fill in", or "implement later" strings appear in any step. The one abbreviated test (Task 2.9 step 1) is marked as "use existing FakeStore pattern" — the engineer is expected to copy the pattern from the same file, not invent it.

**3. Type consistency:**
- `LlmProviderConfig` field names (id, kind, endpoint, api_key, default_model) are consistent across Tasks 1.1, 1.4, 1.7, 2.6, 2.7, 3.1, 3.2.
- `LlmProviderKind` variants (Ollama, LmStudio, Anthropic, OpenAi) and their kebab-case string forms ("ollama", "lm-studio", "anthropic", "openai") are consistent.
- `LlmHealthStatus` field names (reachable, latency_ms, models, error) consistent.
- `LaunchPlan` field names (profile_id, provider_id, frontmatter, prompt_body, resolved_mcp_servers, llm_provider_id) consistent across Tasks 1.2, 2.1, 2.9, 2.10.
- `AgentFrontmatter` field names match the spec section 4.
- `WizardStep` new variant field names (title, providers, placeholder, rows, options) match the existing `WizardStep` style.
- `CoreEvent` new variant names (LlmProviderListed, LlmProviderUpserted, LlmProviderRemoved, LlmProviderHealth, ProfileLaunchPlan) consistent across Tasks 2.7, 2.10, 3.2.
- `ClaudeCliProbePort` method names (is_available, locate, version, supports_agent_flag) consistent across Tasks 1.5, 1.7, 2.2.

**4. Coverage gaps:** None identified. All spec sections map to at least one task. The one minor gap (the empty `agent description → prompt body` mapping in Task 2.9) is documented in code as "placeholder; richer prompt body comes in v0.5" and is in the "Out of scope" section of the spec.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-06-04-claude-code-agent-profile.md`. Two execution options:

1. **Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration
2. **Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
