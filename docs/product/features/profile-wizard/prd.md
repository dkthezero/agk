# Profile Wizard Feature – Product Requirements

**Status:** Draft  
**Epic:** [v0.3 Team-Ready Profiles](../../../epics/v03-team-ready-profiles.md)  
**Related:** [Profiles PRD](../profiles/prd.md) (parent feature)

---

## Overview

The Profile Wizard is the interactive flow for creating new agent profiles in AGK. In v0.3, it is completely restructured: instead of 3 shallow free-text Q&A questions that produce a raw description blob, the wizard collects **structured, prompt-engineered inputs** that map to system prompt sections (Identity, Responsibilities, Style, Triggers, Constraints). It offers **agent archetype templates** to pre-fill most fields, estimates **token count** in real time, and generates a **preview** of the composed markdown before saving.

---

## User-Facing Behavior

### TUI Flow (F2 in Profiles Tab)

#### Step 1: Profile Name
- **Input:** TextInput
- **Prompt:** "Profile name (e.g., `rust-reviewer`)"
- **Validation:** Alphanumeric + hyphens only. Must be unique in scope.

#### Step 2: Scope
- **Input:** ScopeSelect (new step type)
- **Options:** `Workspace` / `Global`
- **Default:** Workspace

#### Step 3: Archetype Template
- **Input:** TemplateSelect (new step type)
- **Options:**
  - **Code Reviewer** — Senior code reviewer; direct & critical; triggers after code changes
  - **Feature Implementer** — Senior engineer; pragmatic & thorough; triggers on implementation requests
  - **Security Auditor** — Security engineer; cautious & explicit; triggers on security keywords
  - **Documentation Writer** — Technical writer; clear & structured; triggers after API changes
  - **Test Generator** — QA engineer; systematic; triggers when source lacks tests
  - **Custom** — Blank slate; all fields empty
- **Behavior:** Selecting a template pre-fills Steps 4–11. "Custom" shows all fields empty. User can edit any pre-filled field.

#### Steps 4–11: Structured Identity Questions

| Step | Input Type | Question / Prompt | Maps To |
|------|------------|-------------------|---------|
| 4 | TextInput | "Who is this agent? e.g., 'Senior Rust CLI engineer'" | `# Identity` |
| 5 | TextInput | "What stack or domain? e.g., 'Rust + async ecosystems'" | Domain specialization |
| 6 | TextInput | "Who does this agent help? e.g., 'Junior devs on my team'" | Audience |
| 7 | Textarea | "What are the 1–3 main jobs?" | `# Core Responsibilities` |
| 8 | TextInput | "How should it behave? Direct? Socratic? Proactive?" | `# Collaboration Style` |
| 9 | TextInput | "How should responses be structured? Bullets? Code blocks?" | `# Output Format` |
| 10 | Textarea | "What should it NEVER do?" | `# Scope Boundaries` |
| 11 | Textarea | "When should it act automatically? Include an example." | `description` frontmatter + `<example>` blocks |
| 12 | Textarea | "Any hard rules? e.g., 'Always run cargo fmt'" | `# Constraints` |

#### Step 13: Select Skills (with vault shown)
- **Input:** Checklist
- **Options:** All available skills across active vaults, formatted as `skill-name [vault-id]`.
- **Filter:** Type-to-filter search box.

#### Step 14: Select MCP Servers (with vault shown)
- **Input:** Checklist
- **Options:** All registered + vault-discovered MCPs, formatted as `mcp-name [vault-id]` or `mcp-name [registered]`.

#### Step 15: Select Tools / Permissions *(provider-opt-in)*
- **Input:** Checklist + Select (conditional)
- **Shown only if** the active provider returns non-empty from `available_profile_tools()` or `available_permission_modes()`.
- **Example (Claude Code):**
  - Tools checklist: `Read`, `Glob`, `Grep`, `Bash`, `Write`, `Edit`, `LSP`
  - Permission mode select: `default`, `acceptEdits`, `auto`, `dontAsk`

#### Step 16: Review
- **Content:**
  - Profile name, scope, provider
  - **Token count badge:** `[Est. Tokens: 342]   [Target: < 800]`
  - **Warning if >800:** "Prompt exceeds recommended length. Consider shortening constraints or triggers."
  - **Scrollable preview** of the composed markdown (frontmatter + body)
  - Skills count + MCPs count
- **Actions:** `Enter` to confirm; `Esc` to go back.

---

### CLI Flow (Headless)

```bash
# Template-driven creation
agk profile create <name> \
  --provider opencode \
  --template code-reviewer \
  --skills rust-patterns:clawhub,docker:ecc \
  --mcps filesystem:workspace \
  --scope workspace

# Custom creation (all fields explicit)
agk profile create <name> \
  --provider claude-code \
  --role "Senior Rust CLI engineer" \
  --domain "Rust + async ecosystems" \
  --audience "Junior devs on my team" \
  --responsibilities "Review PRs, suggest idioms, enforce fmt" \
  --style "Direct and critical" \
  --format "Bullets, max 5 items" \
  --triggers "After any code change; include example" \
  --constraints "Always run cargo fmt; never suggest unsafe" \
  --tools Read,Glob,Grep \
  --permission-mode acceptEdits \
  --description-file ./my-agent.md
```

**Behavior:**
- `--template` pre-fills all structured fields. Individual flags can override template defaults.
- `--description-file` bypasses the wizard entirely and uses the provided markdown file as `agent.md`.
- Without `--template` or `--description-file`, the CLI falls back to the old `--description` string (backward compatible).

---

## Functional Requirements

1. **Structured markdown composition:** The wizard shall compose a canonical structured markdown body from user answers, not concatenate raw Q&A pairs.
2. **Template pre-fill:** Selecting an archetype shall pre-fill Steps 4–11 with sensible defaults. User may edit any pre-filled field.
3. **Token estimation:** The Review step and Editor shall display `[Est. Tokens: N]` using a `words * 1.35` heuristic. Warn if >800. Hard-cap warning at >1500.
4. **Provider-aware step injection:** The wizard step list is generated by `provider.profile_wizard_steps()`, with AGK overlaying universal questions before provider-specific ones.
5. **Skill/MCP vault display:** Checklist options shall show the originating vault for vault-sourced assets.
6. **Tool/permission conditional step:** If `available_profile_tools()` returns non-empty, inject a Checklist step. If `available_permission_modes()` returns non-empty, inject a Select step.
7. **OpenCode path:** AGK passes the composed structured markdown body as `--description` to `opencode agent create`. AGK does NOT write frontmatter for OpenCode.
8. **Claude Code path:** AGK writes full frontmatter + body to `.agk/profiles/<name>/agent.md`. The `description` field includes proactive triggers with `<example>` blocks.
9. **Template count:** At least 5 archetype templates must be available.
10. **Wizard brevity:** Template path shall require ≤ 10 steps (excluding checklist/review).

---

## The "AGK Prompt Contract"

Regardless of provider, AGK composes this canonical body:

```markdown
# Identity
You are a {role} specializing in {domain}.
You work with {audience}.

# Core Responsibilities
1. {responsibility_1}
2. {responsibility_2}
3. {responsibility_3}

# Collaboration Style
{style}

# Output Format
{format}

# Scope Boundaries
IN SCOPE:
{in_scope_items}

OUT OF SCOPE:
{out_of_scope_items}

# Constraints
{constraints}
```

**Provider-specific frontmatter:**
- **OpenCode:** None — passed as `--description` to `opencode agent create`.
- **Claude Code:** AGK writes:
  ```yaml
  ---
  name: {profile_name}
  description: |
    {one_line_summary}
    <example>
    Context: {trigger_context}
    user: "{user_message}"
    assistant: "{assistant_response}"
    <commentary>{why_triggered}</commentary>
    assistant: "Now let me review this with the {profile_name} agent."
    </example>
  tools: {tool_refs}
  model: sonnet
  color: blue
  memory: project
  ---
  ```

---

## Archetype Templates

| Template | Identity | Style | Trigger | Default Tools (Claude) |
|----------|----------|-------|---------|------------------------|
| **Code Reviewer** | Senior code reviewer | Direct & critical | After any code change | Read, Glob, Grep, LSP |
| **Feature Implementer** | Senior engineer | Pragmatic & thorough | When user asks for implementation | Read, Glob, Grep, Bash, Write, Edit |
| **Security Auditor** | Security engineer | Cautious & explicit | When security keywords detected | Read, Glob, Grep, Bash |
| **Documentation Writer** | Technical writer | Clear & structured | After public API changes | Read, Glob, Grep, Write, Edit |
| **Test Generator** | QA engineer | Systematic | When source files lack tests | Read, Glob, Grep, Bash, Write |
| **Custom** | (blank) | (blank) | (blank) | (provider default) |

---

## UI/UX Specifications

### Token Count Badge
```
[Est. Tokens: 342]   [Target: < 800 for optimal performance]
```
- Color: green if <500, yellow if 500–800, red if >800.
- Position: top-right of Review step and Editor Overview tab.

### Review Step Layout
```
┌─────────────────────────────────────────┐
│ Profile: rust-reviewer  [Est: 342 ✓]   │
│ Provider: opencode | Scope: workspace   │
├─────────────────────────────────────────┤
│ # Identity                              │
│ You are a Senior Rust CLI engineer...   │
│ (scrollable preview)                    │
├─────────────────────────────────────────┤
│ Skills: 3  |  MCPs: 2  |  Tools: 3      │
│                                         │
│ [Enter] Confirm    [Esc] Back           │
└─────────────────────────────────────────┘
```

---

## Non-Goals

- Natural-language template generation ("describe what you want and AI fills everything"). Templates are static, hand-curated data structures.
- Real-time provider-side token counting. The heuristic is approximate and labeled "Est.".
- Multi-language wizard localization. English only for v0.3.

## Success Criteria

- [ ] Wizard generates structured markdown from 6–8 structured prompts (not raw Q&A).
- [ ] OpenCode provider receives structured markdown via `opencode agent create --description`.
- [ ] TUI Review step shows scrollable preview of composed markdown.
- [ ] TUI shows estimated token count for composed prompt.
- [ ] At least 5 archetype templates available in wizard.
- [ ] Template path completes in ≤ 10 steps (excluding checklist/review).
- [ ] Provider tool/permission selection appears when provider advertises options.
- [ ] Skill/MCP checklist shows originating vault.
- [ ] F3 Editor allows raw markdown editing with live token updates.
- [ ] `cargo test` passes; architecture tests pass with zero allowlists.

---

*PRD v0.1 — 2026-05-30*
