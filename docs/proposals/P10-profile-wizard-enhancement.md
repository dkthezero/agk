# Proposal: Enhanced Profile Wizard & Agent Creation Framework (P10)

**Status:** Draft  
**Author:** Claude (Research Synthesis)  
**Date:** 2026-05-30  
**Scope:** `app::features::profile`, `infra::provider::*`, `tui::features::profiles`  
**Dependencies:** P8 (Testing), P9 (Observability) — UI layer only; core logic is independent.

---

## 1. Problem Statement

AGK's current profile wizard (`agk p add` / `F2` in TUI) is too shallow. It asks only three free-text questions (task, tone, constraints) and produces a raw Q&A blob passed to `opencode agent create --description`. This approach:

1. **Lacks structured identity** — No role, expertise level, audience, or scope boundaries.
2. **Produces weak system prompts** — Raw Q&A does not leverage modern LLM prompt-engineering best practices (role framing, outcome-first design, explicit triggers).
3. **Is provider-naive** — The same 3 questions are used regardless of whether the target is OpenCode, Claude Code, or a future provider. Each platform has different agent-file formats and triggering mechanisms.
4. **No proactive delegation signals** — Claude Code's most critical field (`description` with `<example>` trigger blocks) is entirely missing.
5. **No runtime projection insight** — Users cannot see the generated agent file, token count, or how it will be projected at `agk p start` time.
6. **No archetype templates** — Every user starts from a blank slate. Industry tools (Claude `/agents`, OpenClaw `onboard`) offer role-based templates.
7. **No vault tracking for dependencies** — Skills and MCPs in a profile are stored as flat strings. If a profile is started on a workspace where those assets are not installed, AGK has no way to know which vault to pull them from.
8. **No tool/permission configuration** — Providers like Claude Code and OpenCode support explicit tool permissions, but the wizard offers no way to configure them.

---

## 2. Research Findings

### 2.1 How OpenClaw Creates Agents

OpenClaw (`openclaw onboard` / `openclaw agents add`) takes a **security-first, identity-centric** approach:

- **Model selection:** Recommends the strongest/latest-generation model. Weaker models are easier to prompt-inject.
- **Tool policy:** Defaults to strict tool profiles (`tools.profile: "coding"`).
- **Credential storage:** Uses `SecretRef` (env-backed references) over plaintext API keys.
- **Workspace-as-memory:** Treats `~/.openclaw/workspace` as the agent's memory. Auto-generates `AGENTS.md`, `SOUL.md`, `TOOLS.md`, `IDENTITY.md`. Recommends git backup.
- **Conservative proactive behavior:** Heartbeat (proactive agent loops) starts at **disabled** (`0m`). Default is `30m` once trusted.
- **Bootstrap isolation:** `session.dmScope: "per-channel-peer"` isolates conversations by sender.

**Key takeaway:** Agent creation = identity + security posture + memory architecture, not just "what do you want it to do?"

Sources: [OpenClaw Onboarding Reference](https://documentation.openclaw.ai/reference/wizard), [CLI Setup Reference](https://docs.openclaw.ai/start/wizard-cli-reference)

### 2.2 How Claude Code Creates Agents

Claude Code has two creation paths:

#### A. Interactive Wizard (`/agents`)
1. **Scope:** Personal (`~/.claude/agents/`) vs Project (`.claude/agents/`)
2. **Auto-generation:** Describe the agent; Claude generates name, description, system prompt.
3. **Tool selection:** Least-privilege access (Read-only, Full, or custom).
4. **Model:** `sonnet` / `opus` / `haiku` / `inherit`.
5. **Color:** Visual identifier (blue=analysis, green=creation, yellow=validation, red=security).
6. **Memory:** `user` (cross-project), `project` (shareable via git), `local` (git-ignored).
7. **Save:** Immediately available without restart.

#### B. Manual Markdown with YAML Frontmatter
```markdown
---
name: code-reviewer
description: |
  Expert code review specialist. Use proactively after writing or modifying code.
  <example>
  Context: User has just written a new function
  user: "Please write a function that checks if a number is prime"
  assistant: "[Writes function]"
  <commentary>A logical chunk of code was written. Trigger code-reviewer proactively.</commentary>
  assistant: "Now let me review this code with the code-reviewer agent."
  </example>
tools: Read, Glob, Grep, Bash
model: sonnet
color: blue
memory: project
---

You are a senior code reviewer...
```

#### Critical Insight: The `description` Field
The `description` field is **the single most important field** — it determines when Claude *delegates* to this agent. Best practices:
- Start with **"Use this agent when..."**
- Include **2–4 concrete `<example>` blocks** showing triggering context, user message, assistant response.
- Add **"use proactively"** to encourage automatic delegation.
- Include `<commentary>` explaining *why* the agent triggers.

Sources: [Claude Code Sub-Agents](https://code.claude.com/docs/en/sub-agents.md), [Agent Creation System Prompt](https://github.com/anthropics/claude-code/blob/main/plugins/plugin-dev/skills/agent-development/references/agent-creation-system-prompt.md)

### 2.3 What Makes Agents Run Better (2025 Best Practices)

From OpenAI, Anthropic, ElevenLabs, and leading prompt engineering resources:

| Principle | Why It Matters |
|-----------|---------------|
| **Specific Role + Domain + Audience** | "You are a Postgres 17 migration reviewer for a Python service team" beats "You are a helpful assistant." Acts as an ambiguity filter. |
| **Structured Sections** | `# Identity`, `# Capabilities`, `# Constraints`, `# Collaboration Style`, `# Output Format`, `# Scope Boundaries` |
| **Separate Personality from Capability** | Models treat these differently. Don't mix. |
| **Triggering Examples** | Agents that wait for explicit invocation are underutilized. Show proactive + reactive triggers. |
| **Confidence Calibration** | "When you do not know, say so. Do not invent function names or file paths." |
| **Least Privilege Tools** | Only grant tools the agent strictly needs. |
| **Outcome-First Design** | "Resolve the customer's issue end to end. Success means..." beats step-by-step process lists. |
| **Concrete Bounds** | "Max 5 bullets, 12 words each" beats "be concise." |
| **Anti-Sycophancy Rules** | "Do not flatter the user. If the user is wrong, say so plainly." Restate at end for recency bias. |
| **Prompt Length Cap** | Target 200–800 tokens. Max 1,000–2,000. Middle of long prompts gets less attention. |

Sources: [OpenAI Prompt Guidance](https://developers.openai.com/api/docs/guides/prompt-guidance), [Field Guide to System Prompt Design](https://fieldguidetoai.com/guides/system-prompt-design), [LLM Best Practices — Role Framing](https://llmbestpractices.com/ai-agents/role-framing), [LLM Best Practices — System Prompts](https://llmbestpractices.com/ai-agents/system-prompts)

---

## 3. AGK's Current Architecture

### 3.1 Profile Domain Model

```rust
// src/domain/profile.rs
pub struct Profile {
    pub id: ProfileId,
    pub scope: Scope,
    pub provider_id: ProviderId,
    pub skill_refs: Vec<SkillId>,
    pub mcp_refs: Vec<McpServerId>,
    pub instruction_refs: Vec<InstructionId>,
    pub prompt_overlay_path: Option<PathBuf>,   // ← currently unused in wizard
    pub launch_policy: LaunchPolicy,
}
```

### 3.2 Profile Storage

Profiles are stored in `config.toml` (scoped, shareable):
```toml
[[profiles]]
name = "bazel-build-optimization"
provider_id = "opencode"
skills = ["bazel-graph", "logs", "java-code-review"]
mcps = ["github-mcp", "aws-mcp"]
```

**Current gap:** Skills and MCPs are stored as flat name strings. There is no vault provenance. If `rust-patterns` exists in both the `clawhub` vault and a team vault, AGK cannot determine which one the profile intended.

### 3.3 Agent File Projection

At `agk p start <profile>`:
1. The provider's `ProfileRuntimePort::build_launch_plan()` reads `.agk/profiles/<profile>/agent.md`.
2. If `agent.md` is missing, it **auto-generates** a minimal stub: `# {name}\n\nProfile agent for {name}.`
3. The plan patches provider config (skill permissions, MCP enablement) and spawns the provider CLI.
4. On session end, surgical cleanup removes only the temporary agent entry.

**Current gap:** The wizard does NOT generate a rich `agent.md`. It passes raw Q&A to `opencode agent create --description`, letting OpenCode generate the file. This means:
- AGK loses control over prompt quality.
- The generated file is not inspectable or editable by the user.
- No token count estimation.
- No structured frontmatter (name, mode, description, triggers).

### 3.4 Wizard Flow (Current)

```
F2 → TextInput(name) → Q&A(task) → Q&A(tone) → Q&A(constraints)
     → Checklist(skills) → Checklist(MCPs) → Review → Save config.toml
     → Run: opencode agent create --path .agk/profiles/<name> --description <raw QA>
```

---

## 4. Proposed Enhancements

### 4.1 Design Philosophy

1. **AGK composes, provider materializes.** The wizard collects structured answers and composes a rich markdown description. For providers with a native wizard CLI (e.g., OpenCode `opencode agent create`), AGK feeds this composed markdown into the provider's wizard via `--description`. For providers without a native wizard (e.g., Claude Code), AGK writes the agent file directly to `.agk/profiles/<name>/agent.md` and the provider's `build_launch_plan()` copies it into its runtime directory.
2. **Provider-aware steps.** The wizard step list is generated by the active provider's `profile_wizard_steps()`, but AGK overlays universal questions (role, domain, triggers) before provider-specific ones (model, color, tools, permissions).
3. **Structured > Free-text.** Replace 3 generic Q&A questions with 6–8 structured prompts that map to system prompt sections.
4. **Templates > Blank slate.** Offer agent archetypes that pre-fill the structured questions.
5. **Live preview.** Show token count and a preview of the composed markdown during the Review step.
6. **Vault-aware dependencies.** Every skill and MCP in a profile stores its originating vault, enabling auto-install on `agk p start`.
7. **Least-privilege tools.** If a provider exposes configurable tools or permissions, the wizard presents them as a checklist.

### 4.2 New Wizard Questions

| # | Step | Question / Prompt | Maps To |
|---|------|-------------------|---------|
| 1 | **TextInput** | Profile name | `name` field, directory name |
| 2 | **ScopeSelect** | Scope (Workspace / Global) | `scope` in config.toml |
| 3 | **TemplateSelect** | Choose archetype: Code Reviewer, Feature Implementer, Security Auditor, Documentation Writer, Test Generator, Custom | Pre-fills steps 4–9 |
| 4 | **TextInput** | Agent role identity: "Who is this agent? e.g., 'Senior Rust CLI engineer'" | `# Identity` header |
| 5 | **TextInput** | Primary domain: "What stack or domain? e.g., 'Rust + async ecosystems'" | Domain specialization |
| 6 | **TextInput** | Target audience: "Who does this agent help? e.g., 'Junior devs on my team'" | Explanation depth calibration |
| 7 | **Textarea** | Core responsibilities: "What are the 1–3 main jobs?" | `# Core Responsibilities` |
| 8 | **TextInput** | Collaboration style: "How should it behave? Direct? Socratic? Proactive?" | `# Collaboration Style` |
| 9 | **TextInput** | Output format: "How should responses be structured? Bullets? Code blocks?" | `# Output Format` |
| 10 | **Textarea** | Scope boundaries: "What should it NEVER do?" | `# Scope Boundaries` |
| 11 | **Textarea** | Proactive triggers: "When should it act automatically? Include an example." | `description` frontmatter + `<example>` blocks |
| 12 | **Textarea** | Constraints & rules: "Any hard rules? e.g., 'Always run cargo fmt'" | `# Constraints` |
| 13 | **Checklist** | Select Skills (with vault shown) | `skill_refs` + `skill_vault_refs` |
| 14 | **Checklist** | Select MCP Servers (with vault shown) | `mcp_refs` + `mcp_vault_refs` |
| 15 | **Checklist** *(provider-opt-in)* | Select Tools / Permissions | `tool_refs` / `permission_policy` |
| 16 | **Review** | Preview composed markdown with token count, scope, provider | Final confirmation |

**Note:** Steps 4–12 are the *universal* overlay. Providers can inject their own steps (model, color, tools, permissions, memory) at appropriate positions via `profile_wizard_steps()`. Step 15 is gated by the provider advertising available tools/permissions via `ProviderPort`.

### 4.3 Composed Structured Markdown (The "AGK Prompt Contract")

Regardless of whether the provider has a native wizard, AGK composes a canonical structured markdown string from the wizard answers. This string is:
- **Fed to provider wizard** (OpenCode: `--description`; future providers: equivalent flag)
- **Or written to `.agk/profiles/<name>/agent.md`** (for providers without native wizard, e.g., Claude Code)

```markdown
# Identity
You are a {role} specializing in {domain}.
You work with {audience}.

# Core Responsibilities
{numbered_responsibilities}

# Collaboration Style
{tone_and_style}

# Output Format
{output_format}

# Scope Boundaries
IN SCOPE:
{in_scope_items}

OUT OF SCOPE:
{out_of_scope_items}

# Constraints
{constraints}
```

**Frontmatter injection (provider-dependent):**
- **OpenCode:** AGK does NOT write frontmatter. It passes the body as `--description` to `opencode agent create`. OpenCode generates its own frontmatter (`name`, `mode`, `description`).
- **Claude Code:** AGK writes full frontmatter (`name`, `description` with `<example>` blocks, `tools`, `model`, `color`, `memory`) + body to `.agk/profiles/<name>/agent.md`. The `description` field includes the proactive triggers.
- **Future providers:** Each provider's `build_launch_plan()` decides whether to consume `.agk/profiles/<name>/agent.md` or patch its own config.

### 4.4 Token Count Estimation

Add a `tokens` field to the frontmatter (or compute on the fly). The TUI Review step and Editor should display:

```
[Est. Tokens: 342]   [Target: < 800 for optimal performance]
```

Warn if >800 tokens (sweet spot per 2025 best practices). Warn if >1,500 tokens (hard cap).

### 4.5 Agent Archetype Templates

Pre-defined templates that pre-fill steps 4–12:

| Template | Pre-filled Identity | Style | Proactive Trigger |
|----------|---------------------|-------|-------------------|
| **Code Reviewer** | Senior code reviewer | Direct & critical | After any code change |
| **Feature Implementer** | Senior engineer | Pragmatic & thorough | When user asks for implementation |
| **Security Auditor** | Security engineer | Cautious & explicit | When security keywords detected |
| **Documentation Writer** | Technical writer | Clear & structured | After public API changes |
| **Test Generator** | QA engineer | Systematic | When source files lack tests |
| **Architecture Advisor** | Staff engineer | Analytical & nuanced | When new modules/packages created |
| **DevOps Assistant** | Platform engineer | Pragmatic & safe | When infra/config files modified |
| **Custom** | (blank) | (blank) | (blank) |

### 4.6 Vault-Aware Profile Storage

#### The Problem

A profile stored in `config.toml` currently looks like:
```toml
[[profiles]]
name = "dev"
provider_id = "opencode"
skills = ["rust-patterns", "docker"]
mcps = ["github-mcp"]
```

If `rust-patterns` exists in both the `clawhub` vault and a team vault, AGK cannot resolve which one the profile intended. When `agk p start dev` runs on a fresh workspace where these skills are not installed, the start fails or picks the wrong asset.

#### Proposed Solutions

**Option A: Structured Array with Vault Field (Recommended)**

Change the config schema to store skills and MCPs as inline tables:

```toml
[[profiles]]
name = "dev"
provider_id = "opencode"

[[profiles.skills]]
name = "rust-patterns"
vault = "clawhub"

[[profiles.skills]]
name = "docker"
vault = "ecc"

[[profiles.mcps]]
name = "github-mcp"
vault = "workspace"
```

**Pros:**
- Explicit, self-documenting, extensible (can add `version`, `sha` later).
- Maps cleanly to Rust structs.
- No ambiguity: every asset has a source vault.

**Cons:**
- More verbose than flat arrays.
- Requires schema migration.

**Option B: Vault Source Maps (Backward-Compatible)**

Keep the flat arrays and add optional source maps:

```toml
[[profiles]]
name = "dev"
provider_id = "opencode"
skills = ["rust-patterns", "docker"]
mcps = ["github-mcp"]

[profiles.skill_sources]
rust-patterns = "clawhub"
docker = "ecc"

[profiles.mcp_sources]
github-mcp = "workspace"
```

**Pros:**
- Backward compatible: old profiles without maps still work.
- Human-readable for hand-editing.

**Cons:**
- Two places to maintain; risk of drift.
- Less structured for programmatic access.

**Option C: Vault-Qualified Strings (Compact)**

Use `vault/name` notation in the flat array:

```toml
[[profiles]]
name = "dev"
provider_id = "opencode"
skills = ["clawhub/rust-patterns", "ecc/docker"]
mcps = ["workspace/github-mcp"]
```

**Pros:**
- Compact, one line per asset.
- Easy to read at a glance.

**Cons:**
- Vault IDs with `/` characters require escaping rules.
- Harder to extend with additional metadata.
- Loses the simplicity of "just a name" for hand-editing.

#### Recommendation

Adopt **Option A (Structured Array)** as the canonical format, with **custom serde** that also accepts the old flat-string format for backward compatibility:

```rust
// Deserializes both formats:
// skills = ["rust-patterns"]          → vault = "auto" (resolved at runtime)
// skills = [{ name = "rust-patterns", vault = "clawhub" }]
```

At `agk p start`, the launch logic:
1. Reads the profile's skill/mcp list.
2. Checks which assets are already installed in the current workspace config.
3. For any missing assets, resolves the vault from `skill_sources` / `mcp_sources`.
4. Auto-installs missing assets from the identified vault before launching the provider.
5. Fails with a clear error if a vault is unavailable or the asset is not found in the specified vault.

This makes profiles **truly portable** across workspaces: a profile checked into git will self-heal its dependencies on first run.

### 4.7 Provider Tool & Permission Selection

#### Problem

Providers expose different tool/permission models:
- **Claude Code:** `tools: Read, Glob, Grep, Bash, Write, Edit, LSP` + `permissionMode: default | acceptEdits | auto | dontAsk | bypassPermissions | plan`
- **OpenCode:** Per-agent skill permissions (`allow`/`deny`) + MCP enablement (`enabled: true`)
- **Future providers:** May have completely different permission vocabularies.

The wizard currently has no step for this.

#### Proposed Extension to `ProviderPort`

Add two optional trait methods:

```rust
/// Return the list of configurable tools/permissions this provider supports.
/// Each entry is (id, description, default_state).
fn available_profile_tools(&self) -> Vec<(String, String, bool)> {
    vec![] // default: no configurable tools
}

/// Return the list of permission modes this provider supports.
fn available_permission_modes(&self) -> Vec<(String, String)> {
    vec![] // default: no configurable permission modes
}
```

#### Wizard Integration

If `available_profile_tools()` returns non-empty, the wizard injects a **Checklist** step:
- Title: "Select Tools / Permissions"
- Options: tool IDs with descriptions
- Default state: pre-checked based on the provider's `default_state`

If `available_permission_modes()` returns non-empty, the wizard injects a **Select** step:
- Title: "Permission Mode"
- Options: mode IDs with descriptions
- Default: the first mode or the provider's default

#### Storage

Add to the domain `Profile` struct:

```rust
pub struct Profile {
    pub id: ProfileId,
    pub scope: Scope,
    pub provider_id: ProviderId,
    pub skill_refs: Vec<SkillId>,
    pub mcp_refs: Vec<McpServerId>,
    pub instruction_refs: Vec<InstructionId>,
    pub skill_vault_refs: Vec<(SkillId, VaultId)>,      // ← NEW
    pub mcp_vault_refs: Vec<(McpServerId, VaultId)>,     // ← NEW
    pub tool_refs: Vec<ToolId>,                          // ← NEW
    pub permission_mode: Option<String>,               // ← NEW
    pub prompt_overlay_path: Option<PathBuf>,
    pub launch_policy: LaunchPolicy,
}
```

And to `ConfigFile::Profile`:

```rust
pub struct Profile {
    pub name: String,
    pub provider_id: String,
    #[serde(default)]
    pub skills: Vec<ProfileAssetRef>,        // ← CHANGED from Vec<String>
    #[serde(default)]
    pub mcps: Vec<ProfileAssetRef>,         // ← CHANGED from Vec<String>
    #[serde(default)]
    pub tools: Vec<String>,                 // ← NEW
    #[serde(default)]
    pub permission_mode: Option<String>,    // ← NEW
}

pub struct ProfileAssetRef {
    pub name: String,
    #[serde(default = "default_auto_vault")]
    pub vault: String,
}
```

#### Runtime Behavior

At `agk p start`:
1. `build_launch_plan()` reads `profile.tool_refs` and `profile.permission_mode`.
2. For OpenCode: skill permissions are already auto-generated from `skill_refs` (allow listed, deny all others). MCP enablement is auto-generated from `mcp_refs`. Tool selection from the wizard could map to OpenCode's per-agent tool config if it exists.
3. For Claude Code: `tools` are written into the agent markdown frontmatter. `permission_mode` is written into the frontmatter if Claude Code supports it.
4. For future providers: each provider maps `tool_refs` and `permission_mode` to its native config format.

---

## 5. UI/UX Proposal (TUI Simulation)

The following HTML simulation demonstrates the proposed TUI experience for the Profiles tab, including the enhanced wizard, editor, and launch flow. It maps directly to the ratatui implementation.

**Key UI additions in this proposal:**
1. **Profile Inspection Panel (Right Pane):** Shows token count, provider, scope, skills (with vault), MCPs (with vault), and the rendered agent prompt overview.
2. **Wizard Overlay:** Step-by-step structured questions with template selection, token estimation, tool/permission checklists, and live preview.
3. **Editor Overlay (F3):** Post-creation modification of skills (with vault), MCPs (with vault), tools, permissions, and the raw composed markdown with token tracking.
4. **Launch Simulation (Enter):** Visual feedback showing dependency resolution → install → projection → provider runtime.

> The full simulation source is attached below. In the real TUI (ratatui), these map to:
> - `modal_long.rs` — checklist and review modals (extended for wizard steps)
> - `profiles/controller.rs` — `handle_profile_wizard_input()` extended with new step types
> - `app/ports/provider.rs` — new `WizardStep` variants: `TemplateSelect`, `ScopeSelect`, `Textarea`, `ToolSelect`, `PermissionSelect`
> - New `render_wizard_step()` in `tui/render/modals.rs`

---

## 6. Implementation Phases

### Phase 1: Core Wizard Restructure (High Impact, Low Effort)
1. Add `WizardStep` variants: `TemplateSelect`, `ScopeSelect`, `Textarea`.
2. Rewrite `OpenCodeProvider::profile_wizard_steps()` to return the 16-step sequence (universal + provider + tool/permissions).
3. Rewrite `WizardState::composed_description()` to generate structured markdown content instead of raw Q&A.
4. Update `handle_profile_wizard_input()` to handle new step types.
5. Keep the existing `opencode agent create` invocation, but pass the **composed structured markdown** as `--description` instead of raw Q&A.
6. Add `.agk/profiles/<name>/` directory creation for AGK metadata storage (even when provider generates the agent file).

### Phase 2: Vault-Aware Storage & Auto-Install (Medium Effort)
7. Define `ProfileAssetRef` struct with `name` + `vault` fields.
8. Update `ConfigFile::Profile` to use `Vec<ProfileAssetRef>` for skills and MCPs, with backward-compatible serde.
9. Update `CreateProfileInput` and domain `Profile` to include vault references.
10. Update `apply_enter_add_profile()` to capture vault IDs when skills/MCPs are selected.
11. Update `agk p start` flow: before `build_launch_plan()`, resolve missing skills/MCPs and auto-install from specified vaults.
12. Add clear error messages when a specified vault is unavailable.

### Phase 3: Provider Tool/Permission Selection (Medium Effort)
13. Add `available_profile_tools()` and `available_permission_modes()` to `ProviderPort` with default empty implementations.
14. Implement for OpenCode: return per-agent configurable tools (if any) and permission modes.
15. Implement for Claude Code: return tool list (Read, Glob, Grep, Bash, Write, Edit, LSP) and permission modes.
16. Update wizard to inject `ToolSelect` / `PermissionSelect` steps when provider returns non-empty lists.
17. Store selections in `config.toml` and project them at `build_launch_plan()` time.

### Phase 4: Token Estimation, Templates & Editor (Medium Effort)
18. Add `estimate_tokens()` utility (word-count heuristic: `words * 1.35`).
19. Inject `tokens:` into frontmatter for direct-write providers (Claude Code).
20. Render token count in Review step and Editor.
21. Define archetype template data structures.
22. Pre-fill wizard answers from template selection.
23. Extend F3 Editor to support raw composed markdown editing with live token updates.

### Phase 5: Provider Extensibility (Higher Effort)
24. Implement `ClaudeCodeProvider::profile_wizard_steps()` with model/color/memory/tool steps.
25. Implement `ClaudeCodeProvider::build_launch_plan()` to write `.agk/profiles/<name>/agent.md` to `.claude/agents/<name>.md`.
26. Support `prompt_overlay_path` for custom agent markdown files.

---

## 7. Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| More steps = longer wizard = user fatigue | Templates pre-fill 80% of fields. "Custom" is the only path that shows all questions. |
| Token estimation is inaccurate | Use a simple heuristic and label it "Est." The actual provider may count differently. |
| Provider-specific frontmatter divergence | AGK generates a canonical body. Provider-specific frontmatter is handled by the provider's `build_launch_plan()` or native wizard. |
| Backward compatibility with existing profiles | Old flat-string `skills = ["name"]` deserializes into `ProfileAssetRef { name, vault: "auto" }`. "Auto" resolves at runtime by scanning all vaults. |
| OpenCode `agent create` CLI behavior change | We continue shelling out to `opencode agent create`, but feed structured markdown instead of raw Q&A. AGK does not try to own OpenCode's file format. |
| Vault unavailable at `agk p start` | Emit a clear error: "Profile 'dev' requires skill 'rust-patterns' from vault 'clawhub', but vault 'clawhub' is not attached. Run `agk vault attach clawhub` or edit the profile." |
| Tool/permission list diverges across provider versions | `available_profile_tools()` is a runtime query. If a provider adds new tools, the wizard automatically surfaces them on next profile creation. |

---

## 8. Acceptance Criteria

- [ ] Wizard generates structured markdown from user answers (not raw Q&A).
- [ ] OpenCode provider receives structured markdown via `opencode agent create --description`.
- [ ] Profile skills/MCPs stored with vault provenance in `config.toml`.
- [ ] `agk p start <profile>` auto-installs missing skills/MCPs from their specified vaults before launching.
- [ ] Provider tool/permission selection appears in wizard when provider advertises options.
- [ ] Selected tools/permissions are stored in `config.toml` and projected at runtime.
- [ ] TUI Review step shows a scrollable preview of the composed markdown.
- [ ] TUI shows estimated token count for the composed prompt.
- [ ] At least 5 archetype templates are available in the wizard.
- [ ] F3 Editor allows editing skills (with vault), MCPs (with vault), tools, permissions, and raw markdown.
- [ ] Existing profiles without vault info continue to work (backward compatibility with "auto" vault resolution).
- [ ] Wizard step count is ≤ 10 when using a template (excluding checklist/review).

---

## 9. Appendix: HTML TUI Simulation

The attached HTML file (`profile-tui-simulation.html`) demonstrates the proposed UX. It is a self-contained, vanilla-JS simulation of the AGK TUI Profiles tab with:
- Tab [5] Profiles list + inspection panel (with vault labels)
- F2 Wizard with template selection, structured questions, skill/MCP checklists (with vault), tool/permission selection, review
- F3 Editor with skills/MCPs/tools/raw-file editing
- Enter Launch simulation with dependency resolution → install → log output
- Real-time token estimation

> **To view:** Open `profile-tui-simulation.html` in any browser. Use keyboard shortcuts: `[0-5]` tabs, `↑/↓` navigate, `F2` wizard, `F3` editor, `Enter` launch, `Tab` scope toggle, `Esc` back/cancel.

---

*End of Proposal*
