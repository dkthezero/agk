# Technical Design: Profile Wizard (v0.3)

**Status:** Draft
**Epic:** [v0.3 Team-Ready Profiles](../../../epics/v03-team-ready-profiles.md)
**Related PRD:** [Profile Wizard PRD](prd.md)

---

## Architecture

The enhanced wizard reuses the existing `WizardStep` / `WizardState` stack pattern but replaces the shallow Q&A composer with a **structured markdown generator** driven by archetype templates.

### New WizardStep Variants

```rust
pub enum WizardStep {
    // ... existing variants ...
    TemplateSelect {
        title: String,
        templates: Vec<ArchetypeTemplate>,
    },
    ScopeSelect {
        title: String,
    },
    Textarea {
        title: String,
        placeholder: String,
        rows: usize,
    },
    // Conditional, injected by provider
    ToolSelect {
        title: String,
        tools: Vec<(String, String, bool)>, // (id, description, default)
    },
    PermissionSelect {
        title: String,
        modes: Vec<(String, String)>, // (id, description)
    },
}
```

### Wizard State Extensions

```rust
pub struct WizardState {
    // ... existing fields ...
    /// Selected archetype template ID (if any).
    pub selected_template: Option<String>,
    /// Scope selection (workspace / global).
    pub scope: Option<Scope>,
    /// Structured answers: key -> value.
    pub structured_answers: HashMap<String, String>,
    /// Selected tool IDs.
    pub selected_tools: Vec<String>,
    /// Selected permission mode.
    pub selected_permission_mode: Option<String>,
}
```

### Structured Markdown Composer

```rust
// app/features/profile/wizard_description.rs
pub fn compose_description(answers: &HashMap<String, String>) -> String {
    let mut lines = Vec::new();
    lines.push(format!("# Identity"));
    lines.push(format!("You are a {} specializing in {}.",
        answers.get("role").unwrap_or_default(),
        answers.get("domain").unwrap_or_default()));
    lines.push(format!("You work with {}.",
        answers.get("audience").unwrap_or_default()));
    lines.push(String::new());
    // ... Core Responsibilities, Style, Format, Boundaries, Constraints ...
    lines.join("\n")
}
```

### Archetype Template Data

```rust
// app/features/profile/template.rs
pub struct ArchetypeTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub defaults: HashMap<String, String>,
    pub default_tools: Vec<String>,       // for providers with tool selection
    pub default_permission_mode: Option<String>,
}

pub const TEMPLATES: &[ArchetypeTemplate] = &[
    ArchetypeTemplate {
        id: "code-reviewer",
        name: "Code Reviewer",
        defaults: HashMap::from([
            ("role", "Senior code reviewer"),
            ("style", "Direct and critical"),
            ("triggers", "After any code change"),
        ]),
        default_tools: vec!["Read", "Glob", "Grep", "LSP"],
        default_permission_mode: Some("default"),
    },
    // ... Feature Implementer, Security Auditor, Documentation Writer, Test Generator, Custom ...
];
```

### Token Estimation

```rust
// app/features/profile/token_estimate.rs
pub fn estimate_tokens(text: &str) -> usize {
    let word_count = text.split_whitespace().count();
    (word_count as f32 * 1.35) as usize
}
```

- Heuristic: average English word ≈ 1.35 tokens.
- Labeled "Est." in UI; not a hard limit.

---

## Provider Integration

### OpenCode Path

1. Wizard collects structured answers.
2. `compose_description()` generates canonical body.
3. AGK runs `opencode agent create --name <name> --description <composed_body>`.
4. OpenCode generates its own frontmatter. AGK does NOT write frontmatter.
5. On success, AGK copies `.opencode/agents/<name>.md` → `.agk/profiles/<name>/agent.md`.

### Claude Code Path

1. Wizard collects structured answers.
2. AGK composes frontmatter + body:
   - `name` from Step 1.
   - `description` with `<example>` blocks from Step 11 (triggers).
   - `tools` from ToolSelect step.
   - `model`, `color`, `memory` from provider defaults or wizard answers.
   - Body from `compose_description()`.
3. AGK writes directly to `.agk/profiles/<name>/agent.md`.
4. At `agk p start`, `build_launch_plan()` copies this file to `.claude/agents/<name>.md`.

---

## TUI Integration

### New Render Functions

- `render_template_select()` — horizontal or vertical list of templates with preview pane.
- `render_scope_select()` — two-option toggle.
- `render_textarea()` — multi-line input with scroll, word wrap, and cursor.
- `render_token_badge()` — colored badge (green/yellow/red) based on count.
- `render_markdown_preview()` — scrollable composed markdown in Review step.

### Controller Updates

- `handle_profile_wizard_input()` gains branches for `TemplateSelect`, `ScopeSelect`, `Textarea`, `ToolSelect`, `PermissionSelect`.
- `WizardState::sync_checklist_state()` extended to handle `ToolSelect`.

---

## Testing Strategy

| Layer | What | How |
|-------|------|-----|
| Domain | Token estimation | Unit: `estimate_tokens("hello world") == 3` |
| Domain | Template pre-fill | Unit: `apply_template("code-reviewer")["role"] == "Senior code reviewer"` |
| App | Composer output | Unit: `compose_description()` contains `# Identity` |
| Integration | Full wizard flow | TUI `TestBackend`: template select → fill answers → review → save |
| Contract | `--json` output | `assert_cmd`: `profile create --template code-reviewer --json` validates schema |

---

*Technical Design v0.1 — 2026-05-30*
