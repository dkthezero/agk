# AGK Support Guide

> **AGK** (Agent Kit) is a terminal tool that manages AI agent skills, instructions, and MCP server configurations across multiple AI coding assistants — from one place, to all of them.

`[Personal]` `[Team]` `[Org]` badges show which user sets each section is most relevant for. Every user can use every feature; the badges highlight the primary audience.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Installation](#2-installation)
3. [Concepts](#3-concepts)
4. [Getting Started](#4-getting-started)
5. [Step-by-Step Guides](#5-step-by-step-guides)
6. [Team & Organization Guides](#6-team--organization-guides)
7. [TUI Reference](#7-tui-reference)
8. [CLI Reference](#8-cli-reference)
9. [Configuration Reference](#9-configuration-reference)
10. [Troubleshooting](#10-troubleshooting)
11. [Provider-Specific Guides](#11-provider-specific-guides)

---

## 1. Overview

You use several AI coding tools — Claude Code, GitHub Copilot, Gemini CLI, OpenCode, and others. Each one has its own folder structure, its own config format, its own way of adding skills and instructions. Keeping them all in sync is manual and error-prone.

**AGK solves this.** Think of it as a control center that broadcasts your AI agent configuration to every provider at once:

```
  ┌─────────┐     ┌─────────┐     ┌─────────┐
  │  Local   │     │ GitHub  │     │ ClawHub │
  │  Vault   │     │  Vault  │     │ Market  │
  └────┬─────┘     └────┬────┘     └────┬─────┘
       │                │                │
       └────────────────┼────────────────┘
                        │
                   ┌────┴────┐
                   │   AGK    │
                   └────┬────┘
        ┌──────┬──────┬──────┼──────┬──────┬──────┬──────┐
        │      │      │      │      │      │      │      │
    Claude  Open  GitHub  Gemini  AMP  Fire-  Letta  Snow-
    Code    Code  Copilot  CLI        bender         flake
```

**What AGK does:**
- **Skills** — Reusable tools that make your AI agent smarter (like installing apps on your phone)
- **Instructions** — Behavioral rules that shape how your AI agent responds (like system preferences)
- **MCP Servers** — Bridges that connect your AI to external services (like browser extensions)
- **Profiles** — Named configurations that bundle a provider, skills, MCPs, and permissions together (like a home screen layout)
- **Vaults** — Sources of skills, instructions, and profiles (like an app store)
- **Contexts** — Switchable workspaces for different teams or projects (like user accounts on your computer)

**Supported providers:** Claude Code · OpenCode · GitHub Copilot · Gemini CLI · AMP Code · Firebender · Letta · Snowflake Cortex

`[Personal]` `[Team]` `[Org]`

---

## 2. Installation

### Homebrew (macOS and Linux)

```bash
brew tap agk/tap
brew install agk
```

### Cargo (build from source)

```bash
cargo install agk
```

Requires a recent Rust toolchain. Install Rust from [rustup.rs](https://rustup.rs/) if needed.

### Pre-built binaries

Download from [GitHub Releases](https://github.com/agk-cli/agk/releases) and add to your `PATH`.

### Verify installation

```bash
agk --version
```

If you see a version number, you are ready to go.

`[Personal]` `[Team]`

---

## 3. Concepts

### 3.1 Skills

A **skill** is a tool your AI agent can use. It is a folder containing a `SKILL.md` file and optional subfolders (`scripts/`, `references/`, `assets/`).

Think of a skill as an **app** you install on your phone — it adds a capability.

```
my-vault/
  skills/
    web-browser/
      SKILL.md          # Required — describes the skill
      scripts/           # Optional — executable scripts
      references/        # Optional — reference documents
      assets/            # Optional — additional files
```

When you install a skill, AGK copies it to the provider's skill directory (for example, `~/.claude/skills/web-browser/` for Claude Code).

`[Personal]` `[Team]`

### 3.2 Instructions

An **instruction** is a behavioral rule for your AI agent. It is a folder containing an `AGENTS.md` file.

Think of an instruction as a **system preference** — it shapes behavior rather than adding a tool. For example, an instruction might say "always write tests first" or "respond in bullet points."

Instructions are installed to provider-specific directories (for example, `.claude/instructions/my-rule/` for Claude Code in workspace scope).

`[Personal]` `[Team]`

### 3.3 MCP Servers

An **MCP server** (Model Context Protocol) is a bridge between your AI agent and an external service — a database, the filesystem, a browser, an API.

Think of an MCP server as a **browser extension** — it plugs in and gives your AI new capabilities it did not have before.

MCP servers can use two transport types:
- **stdio** — The server runs as a local process. AGK launches it and communicates over standard input/output.
- **SSE** — The server runs as a remote HTTP service. AGK connects to a URL.

When you register an MCP server, AGK stores it in `~/.config/agk/mcp.toml` and can enable it per-provider per-scope.

> **Tip:** After registering an MCP server, AGK automatically runs a handshake test to verify the connection. A `[✓]` badge means the test passed.

`[Personal]` `[Team]`

### 3.4 Providers

A **provider** is the AI platform where your skills and instructions land. Think of it as your **phone** — AGK installs your apps on whichever phones you choose.

| Provider | ID | Skills | Instructions | MCP | Profiles | Config roots |
|---|---|---|---|---|---|---|
| Claude Code | `claude-code` | ✓ | ✓ | ✓ | ✓ | `.claude`, `.agents` |
| OpenCode | `opencode` | ✓ | ✓ | ✓ | ✓ | `.opencode`, `.agents` |
| GitHub Copilot | `github-copilot` | ✓ | ✓ | ✓ (global only) | — | — |
| Gemini CLI | `gemini-cli` | ✓ | ✓ | ✓ (global only) | — | `.gemini`, `.ai` |
| AMP Code | `amp` | ✓ | ✓ | ✓ | — | — |
| Firebender | `firebender` | ✓ | ✓ | — | — | — |
| Letta | `letta` | ✓ | ✓ | — | — | — |
| Snowflake Cortex | `snowflake` | ✓ | ✓ | — | — | — |

Some providers (Claude Code, OpenCode, Gemini CLI) let you choose a **config root** — the folder name where skills and instructions are stored. For example, OpenCode can use `.opencode` (default) or `.agents` (Claude-compatible). You select this the first time you activate the provider.

`[Personal]` `[Team]`

### 3.5 Vaults

A **vault** is where skills come from. Think of it as an **app store** — you attach a vault, browse what is inside, and install what you need.

AGK supports three vault types:

| Vault type | How it works | Example |
|---|---|---|
| **Local** | A directory on your disk | `./my-vault` |
| **GitHub** | A GitHub repository (sparse checkout) | `owner/repo` |
| **ClawHub** | The ClawHub community marketplace | Built-in, toggle to activate |

For GitHub vaults, AGK uses `git sparse-checkout` to fetch only the subfolder you need, keeping things fast. You specify the branch (default: `main`) and subfolder path (default: `skills/`).

For ClawHub, AGK uses the `clawhub` CLI to search and install community packages. If the CLI is not installed, AGK offers to install it via Homebrew or give you a manual download link.

`[Personal]` `[Team]` `[Org]`

### 3.6 Profiles

A **profile** is a named, self-contained configuration that bundles a provider with selected skills, MCPs, instructions, and permission settings. Think of it as a **home screen layout** — same phone, different arrangement of apps for work versus personal use.

When you start a profile, AGK:
1. Generates the agent markdown file with your selected tools and permissions
2. Patches the provider's config with your MCP servers and skill permissions
3. Launches the provider CLI
4. Cleans everything up when the session ends

Profiles are created through the TUI wizard (press `F2` on the Profiles tab) or the CLI (`agk profile create`).

**Profile wizard archetypes:**

| Archetype | Role | Default Tools | Permission Mode |
|---|---|---|---|
| Code Reviewer | Senior code reviewer | Read, Glob, Grep, LSP | default |
| Feature Implementer | Senior engineer | Read, Glob, Grep, Bash, Write, Edit | default |
| Security Auditor | Security engineer | Read, Glob, Grep, Bash | default |
| Documentation Writer | Technical writer | Read, Glob, Grep, Write, Edit | default |
| Test Generator | QA engineer | Read, Glob, Grep, Bash, Write | default |
| Custom | Blank slate | — | — |

`[Personal]` `[Team]`

**Permission modes:**

| Mode | Behavior |
|---|---|
| `default` | Ask for confirmation on edits |
| `acceptEdits` | Accept edits automatically |
| `auto` | Auto-approve safe operations |
| `dontAsk` | Never ask for confirmation |
| `plan` | Plan mode — suggest only, do not execute |

`[Personal]` `[Team]`

### 3.7 Contexts

A **context** is a named switchable workspace. Think of it as a **user account** on your computer — one for personal projects, one for your company, one for a specific client.

Each context carries its own:
- Display name (for example, "Personal", "Acme Corp", "Client X")
- Vault list
- Provider list
- Profile list
- Environment label (local, dev, staging, prod)
- Tags (key-value pairs)

The default context is named `default` with the display name "Personal". Switching contexts merges the context's vaults and providers into your active configuration.

```bash
agk context list               # Show all contexts
agk context switch acme-corp   # Switch to the acme-corp context
agk context create client-x --display-name "Client X"  # Create a new context
```

`[Team]` `[Org]`

### 3.8 Scope (Global vs Workspace)

**Scope** determines where AGK stores configuration and installed assets.

| Scope | Config path | What goes here |
|---|---|---|
| **Global** | `~/.config/agk/config.toml` | Vault definitions, provider activations, global profiles |
| **Workspace** | `.agk/config.toml` | Installed assets for this project |

Think of it as **System Settings vs App Settings** — global scope applies everywhere on your machine, workspace scope applies only inside one project folder.

In the TUI, press `Tab` to toggle between scopes. In the CLI, use `--scope global` or `--scope workspace`.

> **Note:** Vaults and providers are typically configured in global scope. Installed assets (skills, instructions) are typically tracked in workspace scope.

`[Personal]` `[Team]`

### 3.9 SHA10 Change Detection

AGK tracks whether your installed assets are up to date using **SHA10** — a content fingerprint of each asset. It hashes the skill's `SKILL.md` plus its `scripts/`, `references/`, and `assets/` folders and takes the first 10 characters.

An asset shows as **up to date** when the installed SHA10 matches the scanned SHA10. If someone updates a skill in the vault, the SHA10 changes even if the version number does not, so AGK always knows when you need an update.

In the TUI, press `Enter` on an outdated asset to update it, or `F5` to update everything at once.

`[Personal]` `[Team]`

### 3.10 Meta-skills and Dependencies

A **meta-skill** is a skill whose `SKILL.md` frontmatter lists other skills as dependencies. Think of it as a **bundle** or **metapackage** — installing it installs everything it needs.

```yaml
# SKILL.md frontmatter
---
name: company-onboarding-pack
version: "1.0.0"
requires:
  - clawhub/git-workflow
  - clawhub/code-review
requires_optional:
  - clawhub/security-audit
---
```

- `requires` — Dependencies that are always installed.
- `requires_optional` — Dependencies that the user can choose to skip.

AGK resolves dependencies recursively. If two meta-skills depend on the same skill, it is installed only once (diamond deduplication). Circular dependencies are detected and rejected with an error.

`[Team]`

---

## 4. Getting Started

Walk through this section to go from zero to a working setup in under 5 minutes.

### 4.1 Launch the TUI

```bash
agk
```

You see a full-screen terminal interface with tabs across the top and a list of keybindings at the bottom.

### 4.2 Attach a vault

1. Press `0` to switch to the **Vaults** tab.
2. Press `F2` to attach a new vault.
3. Enter a local path (for example, `./my-vault`) or a GitHub URL (for example, `my-org/team-skills`).
4. For GitHub vaults: confirm the branch (default `main`) and subfolder path (default `skills/`).
5. Enter a name for the vault (defaults to the folder or repo name).

Alternatively, activate the built-in ClawHub vault by pressing `Space` on the `clawhub` entry in the Vaults tab.

### 4.3 Activate a provider

1. Press `4` to switch to the **Providers** tab.
2. Press `Space` on the provider you want to activate (for example, `claude-code`).
3. If the provider supports multiple config roots, select one from the dialog.

### 4.4 Install your first skill

1. Press `1` to switch to the **Skills** tab.
2. Type to search for a skill by name.
3. Press `Space` on the skill you want to install.
4. AGK copies the skill files to the provider's skill directory and records the install in `config.toml`.

### 4.5 Register an MCP server

1. Press `2` to switch to the **MCP** tab.
2. Press `F2` to start the registration wizard.
3. Fill in the 5 steps: **Name**, **Command**, **Arguments**, **Transport** (stdio or SSE), **Description**.
4. AGK automatically runs a handshake test. If successful, you see a `[✓]` badge.

### 4.6 Create a profile

1. Press `5` to switch to the **Profiles** tab.
2. Press `F2` to start the profile wizard.
3. Follow the steps: name, scope, archetype template, identity questions, skill checklist, MCP checklist, tool/permission selection, review.
4. To launch the profile:

```bash
agk profile start my-profile
```

`[Personal]`

---

## 5. Step-by-Step Guides

### 5.1 Managing Vaults

**Attach a local vault:**

```bash
# Via TUI: Press 0 → F2 → enter path → enter name
# Or via config file:
```

```toml
# ~/.config/agk/config.toml
[my-vault.vault]
type = "local"
path = "/path/to/my-vault"
```

**Attach a GitHub vault:**

```bash
# Via TUI: Press 0 → F2 → enter "owner/repo" → confirm branch → confirm path → enter name
# Or via config file:
```

```toml
[team-skills.vault]
type = "github"
repo = "my-org/team-skills"
ref = "main"
path = "skills/"
```

**Activate ClawHub:**
- In the TUI, navigate to the Vaults tab and press `Space` on the `clawhub` entry.
- If the `clawhub` CLI is not installed, AGK offers to install it via Homebrew or provide a manual download link.

**Detach a vault:**
- In the TUI, navigate to the Vaults tab, select the vault, and press `Space` to toggle it off. A confirmation dialog appears.

**Refresh vaults:**
- Press `F4` in any tab to refresh all vaults from their sources.

`[Personal]` `[Team]`

### 5.2 Installing and Updating Skills

**Install a skill:**

```bash
# TUI: Press 1 → type to search → Space to install
# CLI:
agk install web-browser
agk install my-vault/web-browser       # from a specific vault
agk install web-browser:1.2.0          # specific version
```

**Update a single skill:**
- In the TUI, select the skill and press `Enter`.

**Update all skills:**
- Press `F5` in any tab.

**Include evals when installing:**

```bash
agk install web-browser --evals
```

The `--evals` flag includes the `evals/` subfolder (test cases) in the installation.

`[Personal]` `[Team]`

### 5.3 Working with Instructions

Instructions follow the same mechanics as skills — `Space` to install, `Enter` to update, `F5` for bulk update. The difference is what they contain: `AGENTS.md` behavioral prompts instead of `SKILL.md` tool definitions.

See [Section 11](#11-provider-specific-guides) for where instructions land per provider.

`[Personal]` `[Team]`

### 5.4 MCP Server Management

**Register an MCP server:**

```bash
# TUI: Press 2 → F2 → fill in 5 steps
# CLI:
agk mcp add \
  --name my-server \
  --command "npx" \
  --args "-y,@modelcontextprotocol/server-filesystem,/tmp" \
  --transport stdio \
  --description "Filesystem access server"
```

**Enable an MCP server for a provider:**

```bash
agk mcp enable my-server --provider claude-code
agk mcp enable my-server --provider claude-code --scope global
```

**Disable an MCP server:**

```bash
agk mcp disable my-server --provider claude-code
```

**List registered MCP servers:**

```bash
agk mcp list
agk mcp list --provider claude-code
```

**Test an MCP server connection:**

```bash
agk mcp test my-server
```

> **Warning:** The MCP handshake test runs the server command on your machine. Only register MCP servers you trust.

> **Note:** The CLI `agk mcp add` command does not support specifying an SSE URL directly. To register an SSE server, use the TUI wizard (MCP tab → `F2`) or edit `~/.config/agk/mcp.toml` directly and set `transport = "sse"` with the `url` field.

`[Personal]` `[Team]`

### 5.5 Creating and Launching Profiles

**Create via TUI wizard (recommended):**
- Press `5` → `F2` → follow the multi-step wizard.

**Create via CLI:**

```bash
agk profile create my-reviewer \
  --provider claude-code \
  --skills "code-reviewer,security-audit" \
  --mcps "my-server" \
  --description "Reviews code for quality and security" \
  --scope workspace
```

**Launch a profile:**

```bash
agk profile start my-reviewer
```

**Preview without running:**

```bash
agk profile start my-reviewer --dry-run
```

This shows the launch plan (what files will be created, what config will be patched) without actually starting the session.

`[Personal]` `[Team]`

### 5.6 Switching Contexts

```bash
# List all contexts
agk context list

# Switch to a context
agk context switch acme-corp

# Create a new context
agk context create client-x --display-name "Client X"
```

When you switch contexts, AGK merges the context's vaults and providers into your active global configuration. The previous context's additions are removed first.

Contexts are stored in `~/.config/agk/contexts.toml`.

`[Team]` `[Org]`

### 5.7 Applying Declarative Config (Team Onboarding)

`agk apply` reads a configuration source (URL or file) and reconciles your local setup to match it. Think of it as `docker compose up` for your AI tooling — you describe what you want, and `apply` makes it so.

```bash
# Apply from a URL
agk apply https://raw.githubusercontent.com/my-org/configs/main/team.toml

# Apply from a local file
agk apply ./team-config.toml

# Preview without making changes
agk apply ./team-config.toml --dry-run

# Apply to a specific context and environment
agk apply ./team-config.toml --context acme-corp --environment prod
```

The configuration source can specify vaults, providers, profiles, and MCP servers. `agk apply` adds missing entries, updates changed entries, and removes entries that are no longer in the source.

`[Team]` `[Org]`

### 5.8 Syncing Assets

```bash
# Sync all configured assets (install missing, update outdated)
agk sync

# Sync in global scope
agk sync --global

# Preview without making changes
agk sync --dry-run
```

`[Personal]` `[Team]`

### 5.9 Packing Skills for Distribution

```bash
# Pack for Claude Desktop
agk pack web-browser --target claude-desktop

# Pack as a tarball
agk pack web-browser --target tarball

# Write to stdout (pipe-friendly)
agk pack web-browser --target tarball --stdout > my-skill.tar.gz
```

Pack targets: `claude-desktop`, `firebender`, `tarball`.

`[Team]`

### 5.10 Telemetry and Usage Insights

AGK collects telemetry locally only — nothing is sent externally. Data is stored in `~/.config/agk/analytics.toml`.

```bash
agk telemetry status            # Check if telemetry is enabled
agk telemetry enable            # Enable telemetry
agk telemetry disable           # Disable telemetry
agk telemetry export            # Export as JSON (default)
agk telemetry export --format csv   # Export as CSV
agk telemetry export --output ~/analytics.json  # Write to file
```

`[Personal]` `[Team]`

### 5.11 Cleaning Up

```bash
# Remove workspace config
agk clean

# Remove global config
agk clean --global
```

> **Warning:** `agk clean` removes configuration files. Installed skill files in provider directories are not removed — only the AGK config is deleted.

`[Personal]`

---

## 6. Team & Organization Guides

### 6.1 Team Onboarding with Apply

The fastest way to get a new team member set up is with `agk apply`. A team lead creates a declarative configuration file and commits it to the team repository. New hires run one command:

```bash
agk apply https://raw.githubusercontent.com/my-org/configs/main/team.toml --dry-run
agk apply https://raw.githubusercontent.com/my-org/configs/main/team.toml
```

The configuration file specifies which vaults to attach, which providers to activate, and which profiles to create. Everyone on the team ends up with the same setup.

Combine this with context switching for teams that work on multiple projects:

```bash
agk context create project-alpha --display-name "Project Alpha"
agk context switch project-alpha
agk apply https://internal.configs/alpha.toml
```

`[Team]`

### 6.2 Sharing Vaults via GitHub

1. Create a repository with a `skills/` directory following the vault structure.
2. Each skill is a folder under `skills/` with a `SKILL.md` file.
3. Team members attach the repo as a GitHub vault:

```bash
# In the TUI: Press 0 → F2 → enter "my-org/team-skills"
# Or configure directly:
```

```toml
# ~/.config/agk/config.toml
[team-skills.vault]
type = "github"
repo = "my-org/team-skills"
ref = "main"
path = "skills/"
```

GitHub vaults use sparse checkout, so only the specified subfolder is downloaded — not the entire repository.

**Branch strategy:** Use different branches for different environments (for example, `main` for stable, `dev` for experimental). Change the `ref` field to point to the desired branch.

`[Team]`

### 6.3 Distributing Profiles

Profiles can be stored in vaults under a `profiles/` directory with a `PROFILE.md` file. Team members install profiles from the vault just like skills.

A profile in a vault specifies the provider, skills, MCPs, and permissions. When a team member activates the profile, AGK resolves dependencies automatically.

`[Team]`

### 6.4 Context Management for Multi-Project Work

Teams that work across multiple projects or clients use contexts to switch between different configurations:

```bash
# Create contexts for each project
agk context create project-alpha --display-name "Project Alpha"
agk context create project-beta --display-name "Project Beta"

# Switch to a project
agk context switch project-alpha

# Each context can have its own environment label
# (local, dev, staging, prod) for filtering
```

When you switch contexts, AGK replaces the previous context's vaults and providers with the new context's. This prevents conflicts between projects.

`[Team]` `[Org]`

---

## 7. TUI Reference

### 7.1 Navigation

| Key | Action |
|---|---|
| `1` | Switch to Skills tab |
| `2` | Switch to MCP tab |
| `3` | Switch to Instructions tab |
| `4` | Switch to Providers tab |
| `5` | Switch to Profiles tab |
| `0` | Switch to Vaults tab |
| `Up` / `Down` | Navigate list |
| `Tab` | Toggle Global / Workspace scope |
| `Esc` (twice) | Quit |
| `Ctrl+C` | Force quit |

### 7.2 Asset Tab (Skills, Instructions)

| Key | Action |
|---|---|
| `Space` | Install / Uninstall |
| `Enter` | Update selected asset |
| `F5` | Update all installed assets |
| `F4` | Refresh vaults from source |
| `Ctrl+O` | Open asset folder in file manager |
| `Ctrl+T` | Open terminal at asset folder |
| Type | Filter / search (also searches ClawHub when active) |

### 7.3 MCP Tab

| Key | Action |
|---|---|
| `F2` | Register new MCP server (5-step wizard) |
| `Space` | Enable / Disable MCP server for current scope |
| `Enter` | Test MCP server connection |

### 7.4 Providers Tab

| Key | Action |
|---|---|
| `Space` | Activate / Deactivate provider |
| `Enter` | Update selected provider |
| `F4` | Refresh provider list |

> **Warning:** Deactivating the last provider with installed assets shows a confirmation dialog. Confirming will remove installed skill files from that provider's directories.

### 7.5 Profiles Tab

| Key | Action |
|---|---|
| `F2` | Create new profile (wizard) |
| `F3` | Edit selected profile |
| `Delete` | Delete selected profile (with confirmation) |

### 7.6 Vaults Tab

| Key | Action |
|---|---|
| `F2` | Attach new vault (local path, GitHub URL, or ClawHub) |
| `Space` | Toggle vault active/inactive |
| `F4` | Refresh vaults from source |

### 7.7 Profile Wizard Steps

The profile wizard walks through 16 steps. The order depends on the provider, but the general flow is:

1. **Archetype template** — choose from predefined templates or Custom
2. **Profile name** — any characters except `/`, `\`, `:`, and null; must be unique
3. **Scope selection** — Workspace or Global
4. **Role** — what role the agent plays (for example, "Senior code reviewer")
5. **Domain / Specialty** — the agent's area of expertise
6. **Collaboration Style** — how the agent communicates (for example, "Direct and critical")
7. **Scope Boundaries** — what is in and out of scope for the agent
8. **Activation Triggers** — when the agent should activate (for example, "After any code change")
9. **Constraints** — rules the agent must follow (for example, "Always include a line reference")
10. **Output Format** — preferred output format (for example, "Concise bullets, max 5 items")
11. **Core Responsibilities** — the agent's main duties
12. **Tool selection** — provider-specific tool allowlist
13. **Permission mode** — default, acceptEdits, auto, dontAsk, or plan
14. **Skill checklist** — select skills from vaults (searchable, with vault badges)
15. **MCP checklist** — select MCP servers (with vault/registered badges)
16. **Review** — scrollable markdown preview with token count badge

`[Personal]` `[Team]`

---

## 8. CLI Reference

All commands support `--quiet` / `-q`, `--verbose` / `-v`, and `--json` global flags.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | General failure |
| 2 | Validation failure |
| 3 | Partial success |

### 8.1 Core Commands

#### `agk`

Launch the TUI. No arguments needed.

#### `agk sync`

Synchronize installed assets with config (install missing, update outdated).

```bash
agk sync [--global] [--dry-run]
```

| Flag | Description |
|---|---|
| `--global` / `-g` | Force global scope |
| `--dry-run` / `-d` | Preview changes without modifying |

#### `agk install <IDENTITY>`

Install a specific asset by identity.

```bash
agk install web-browser                 # by name
agk install my-vault/web-browser        # from a specific vault
agk install web-browser:1.2.0           # specific version
```

| Flag | Description |
|---|---|
| `--scope <scope>` / `-s` | Target scope (`global` or `workspace`) |
| `--dry-run` / `-d` | Preview changes without modifying |
| `--provider <provider>` / `-p` | Limit to a specific provider |
| `--evals` | Include the `evals/` subfolder |

#### `agk validate`

Validate installed assets against source vaults.

```bash
agk validate [--scope <scope>]
```

| Flag | Description |
|---|---|
| `--scope <scope>` / `-s` | Target scope (`global` or `workspace`) |

#### `agk pack <IDENTITY>`

Pack a skill into a provider-specific distributable.

```bash
agk pack web-browser --target claude-desktop
agk pack web-browser --target tarball --stdout > my-skill.tar.gz
```

| Flag | Description |
|---|---|
| `--target <target>` / `-t` | Pack format: `claude-desktop`, `firebender`, or `tarball` |
| `--stdout` | Write to stdout instead of a file |

#### `agk clean`

Remove AGK configuration files.

```bash
agk clean [--global]
```

| Flag | Description |
|---|---|
| `--global` / `-g` | Remove global config instead of workspace config |

### 8.2 Context Commands

#### `agk context switch <NAME>`

Switch to a context and apply its defaults.

```bash
agk context switch acme-corp [--dry-run]
```

#### `agk context list`

List all configured contexts.

#### `agk context create <NAME>`

Create a new context.

```bash
agk context create client-x --display-name "Client X"
```

| Flag | Description |
|---|---|
| `--display-name <name>` / `-d` | Human-readable display name |

### 8.3 Apply Command

#### `agk apply <SOURCE>`

Apply a declarative configuration from a URL or local path.

```bash
agk apply https://example.com/team.toml
agk apply ./team-config.toml --dry-run
agk apply ./team.toml --context acme-corp --environment prod
```

| Flag | Description |
|---|---|
| `--scope <scope>` / `-s` | Target scope (default: `workspace`) |
| `--context <name>` / `-c` | Target context |
| `--environment <env>` / `-e` | Target environment: `local`, `dev`, `staging`, `prod` |
| `--dry-run` | Preview changes without modifying |

### 8.4 MCP Commands

#### `agk mcp add`

Register a new MCP server.

```bash
agk mcp add \
  --name my-server \
  --command "npx" \
  --args "-y,@modelcontextprotocol/server-filesystem,/tmp" \
  --transport stdio \
  --description "Filesystem access"
```

| Flag | Description |
|---|---|
| `--name <name>` / `-n` | Server name (required, unique) |
| `--command <cmd>` / `-c` | Command to run (required) |
| `--args <args>` / `-a` | Arguments (comma-separated) |
| `--env <env>` / `-e` | Environment variables (`KEY=VALUE`, comma-separated) |
| `--transport <type>` / `-t` | Transport type: `stdio` (default) or `sse` |
| `--description <desc>` / `-d` | Description |
| `--no-test` | Skip the connection test after registering |

#### `agk mcp enable <NAME>`

Enable an MCP server for a provider.

```bash
agk mcp enable my-server --provider claude-code [--scope global]
```

#### `agk mcp disable <NAME>`

Disable an MCP server for a provider.

```bash
agk mcp disable my-server --provider claude-code [--scope global]
```

#### `agk mcp list`

List all registered MCP servers.

```bash
agk mcp list [--provider <provider>]
```

#### `agk mcp test <NAME>`

Test an MCP server connection.

```bash
agk mcp test my-server
```

### 8.5 Profile Commands

> **Tip:** `agk profile` has a shorthand alias `agk p` — for example, `agk p start my-reviewer`.

#### `agk profile start <NAME>`

Start (launch) a profile session.

```bash
agk profile start my-reviewer [--dry-run]
```

#### `agk profile create <NAME>`

Create a new profile (headless, no TUI wizard).

```bash
agk profile create my-reviewer \
  --provider claude-code \
  --skills "code-reviewer,security-audit" \
  --mcps "my-server" \
  --description "Code review profile" \
  --scope workspace
```

| Flag | Description |
|---|---|
| `--provider <provider>` / `-p` | Provider to use (default: `opencode`) |
| `--skills <list>` / `-s` | Comma-separated skill names |
| `--mcps <list>` / `-m` | Comma-separated MCP server names |
| `--description <desc>` / `-d` | Agent description (or path to a markdown file) |
| `--description-file <path>` | Read description from a markdown file |
| `--scope <scope>` | Scope: `global` or `workspace` (default: `workspace`) |
| `--dry-run` | Preview changes without modifying |

### 8.6 Telemetry Commands

#### `agk telemetry enable`

Enable local telemetry collection.

#### `agk telemetry disable`

Disable local telemetry collection.

#### `agk telemetry status`

Show telemetry status (enabled/disabled).

#### `agk telemetry export`

Export telemetry data.

```bash
agk telemetry export                       # JSON to stdout
agk telemetry export --format csv          # CSV to stdout
agk telemetry export --output ~/data.json  # Write to file
```

| Flag | Description |
|---|---|
| `--format <fmt>` | Output format: `json` (default) or `csv` |
| `--output <path>` | Write to file (default: stdout) |

### 8.7 Debug Commands (Hidden)

These commands are not shown in help output.

#### `agk debug tasks`

List active and recent tracked tasks.

#### `agk debug hangs`

Detect hung tasks (running longer than 30 seconds).

#### `agk debug trace`

Dump current trace span tree (requires `observability` feature).

`[Personal]` `[Team]` `[Org]`

---

## 9. Configuration Reference

### 9.1 Global Config (`~/.config/agk/config.toml`)

```toml
version = 1

# Active vault IDs (must match vault section keys below)
vaults = ["my-vault", "team-skills"]

# Active providers (toggle with TUI or CLI)
providers = ["claude-code", "opencode"]

# Provider root overrides (which folder each provider uses in workspace)
[provider_roots]
claude-code = ".claude"     # Options: ".claude", ".agents"
opencode = ".opencode"     # Options: ".opencode", ".agents"
gemini-cli = ".gemini"     # Options: ".gemini", ".ai"

# Vault definitions
[my-vault.vault]
type = "local"
path = "/path/to/my-vault"

[team-skills.vault]
type = "github"
repo = "my-org/team-skills"
ref = "main"
path = "skills/"

# Installed assets per vault (managed by AGK, do not edit manually)
[my-vault.skills]
items = ["[web-browser:1.2.0:a13c9ef042]"]

[my-vault.instructions]
items = ["[code-style:--:9ac00ff113]"]

# Profiles
[[profiles]]
name = "my-reviewer"
provider_id = "claude-code"
scope = "workspace"
skills = ["code-reviewer", "security-audit"]
mcps = ["my-server"]
permission_mode = "default"
```

### 9.2 Workspace Config (`.agk/config.toml`)

Workspace config has the same structure as global config but is scoped to the current project directory. It inherits vaults and providers from the global config and adds workspace-specific installed assets.

```toml
version = 1
vaults = []
providers = ["claude-code"]

[my-vault.skills]
items = ["[web-browser:1.2.0:a13c9ef042]"]
```

### 9.3 MCP Registry (`~/.config/agk/mcp.toml`)

```toml
[servers.my-server]
name = "my-server"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
transport = "stdio"
description = "Filesystem access server"
tested = true
tested_at = "2024-01-15T10:30:00Z"

[servers.my-server.env]
API_KEY = "secret-value"

[servers.my-server.activation.claude-code]
global = true
workspace = true

# SSE transport example
[servers.remote-api]
name = "remote-api"
command = ""
transport = "sse"
url = "https://api.example.com/mcp"
```

### 9.4 Contexts (`~/.config/agk/contexts.toml`)

Contexts are stored in a single TOML file. The current context is tracked by the `current_context` field.

```toml
# ~/.config/agk/contexts.toml
current_context = "default"

[contexts.default]
display_name = "Personal"
vaults = ["my-vault"]
providers = ["claude-code"]
profiles = ["my-reviewer"]

[contexts.acme-corp]
display_name = "Acme Corp"
vaults = ["team-skills"]
providers = ["claude-code", "opencode"]
profiles = ["acme-reviewer"]
environment = "prod"

[contexts.acme-corp.tags]
team = "backend"
cost-center = "eng-001"
```

### 9.5 Telemetry (`~/.config/agk/analytics.toml`)

Telemetry is opt-in and stored locally. It tracks skill invocations per provider with timestamps.

```bash
agk telemetry status    # Check status
agk telemetry enable    # Start collecting
agk telemetry export    # Export as JSON or CSV
```

### 9.6 Vault Structure

A vault directory follows this structure:

```
my-vault/
  skills/
    web-browser/
      SKILL.md           # Required — describes the skill
      scripts/            # Optional — executable scripts
      references/         # Optional — reference documents
      assets/             # Optional — additional files
      evals/              # Optional — test cases (installed with --evals)
  instructions/
    code-style/
      AGENTS.md          # Required — behavioral prompt
  mcps/
    my-server/
      MCP.md             # Required — MCP server definition
  profiles/
    reviewer/
      PROFILE.md         # Required — profile definition
```

### 9.7 SKILL.md Frontmatter

```yaml
---
name: web-browser
version: "1.2.0"
author: "Jane Developer"
description: "Browse the web from your AI agent"
requires:
  - clawhub/http-client
  - clawhub/html-parser
requires_optional:
  - clawhub/cache
---
```

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | string | Yes | Skill identifier |
| `version` | string | Yes | Semantic version |
| `author` | string | No | Author name |
| `description` | string | No | Short description |
| `requires` | list | No | Dependencies that are always installed |
| `requires_optional` | list | No | Dependencies that can be skipped |

### 9.8 Profile Configuration in config.toml

```toml
[[profiles]]
name = "my-reviewer"
provider_id = "claude-code"
scope = "workspace"

# Skills can be plain strings or tables with vault references
skills = ["code-reviewer", { name = "security-audit", vault = "clawhub" }]

# Instructions follow the same format
instructions = ["code-style", { name = "security-rules", vault = "team-skills" }]

# MCPs follow the same format
mcps = ["my-server"]

# Tool restrictions (provider-specific)
tool_refs = ["Read", "Glob", "Grep"]

# Permission mode: "default", "auto", "acceptEdits", or "plan"
permission_mode = "default"

# Optional: path to a markdown file that overlays additional prompt content
# prompt_overlay_path = "./my-overlay.md"
```

`[Personal]` `[Team]` `[Org]`

---

## 10. Troubleshooting

### 10.1 Installation Issues

| Problem | Solution |
|---|---|
| `cargo install agk` fails | Ensure you have Rust 1.70+ installed. Run `rustup update` to update. |
| `agk` command not found | Add Cargo's bin directory to your `PATH`: `export PATH="$HOME/.cargo/bin:$PATH"` |
| Homebrew tap not found | Make sure you added the tap: `brew tap agk/tap` |

### 10.2 Vault Problems

| Problem | Solution |
|---|---|
| GitHub vault clone fails | Check your network connection. Ensure the repo is accessible. Try `git ls-remote owner/repo` to verify access. |
| Local vault not found | Verify the path is absolute or relative to your current directory. Use `pwd` to check. |
| ClawHub CLI not found | Install with `brew install clawhub` or download from [clawhub.ai](https://clawhub.ai). If Homebrew is unavailable, use the manual download link. |
| F4 refresh hangs | GitHub vaults use sparse checkout. Large repos may take time. Check your network connection. |
| Vault shows no skills | Ensure the vault has the correct structure: `skills/<name>/SKILL.md`. Check the `path` setting in your vault config — it should point to the folder containing the skills, not the repo root. |

### 10.3 Skill and Install Problems

| Problem | Solution |
|---|---|
| "No provider configured" error | Activate a provider first (TUI: press `4`, then `Space` on a provider). |
| Skill not found after vault attach | Press `F4` to refresh vaults. Check the vault path and structure. |
| SHA10 mismatch after update | This means the skill content changed. Press `Enter` on the asset or `F5` to update all. |
| Meta-skill dependency resolution failure | Check the `requires:` list in the skill's `SKILL.md`. Ensure all referenced vaults are attached. |
| Circular dependency error | A skill depends on itself through a chain. Remove the circular reference from the `requires:` list. |
| Skill files not appearing in provider | Check that the provider is activated (TUI: Providers tab). Check scope (Global vs Workspace — press `Tab`). |

### 10.4 MCP Server Problems

| Problem | Solution |
|---|---|
| Handshake test fails | Verify the command is correct and the server binary is on your `PATH`. Try running the command manually. |
| "Command not found" when registering | Ensure the MCP server's command is an absolute path or on your system `PATH`. |
| SSE server connection fails | Verify the URL is correct and the server is running. Check for firewall or proxy issues. |
| MCP enabled but provider does not see it | Check the scope — MCP servers can be enabled per-scope. Use `agk mcp list` to verify activation state. |

### 10.5 Profile Problems

| Problem | Solution |
|---|---|
| Profile wizard not appearing | Only providers that support profiles (Claude Code, OpenCode) show the wizard. Activate a profile-capable provider first. |
| "Profile already exists" error | Profile names must be unique within a scope. Choose a different name or use `--scope global`. |
| Missing skills when starting a profile | The profile references skills that are not installed. AGK warns but does not block. Install the referenced skills or remove them from the profile. |
| "Provider not active or does not support profiles" | Activate the provider first (TUI: Providers tab, `Space` to toggle). |

### 10.6 Context Problems

| Problem | Solution |
|---|---|
| Context switch does not activate expected vaults | Check the context's `vaults` list in `~/.config/agk/contexts.toml`. Ensure the vault names match your config. |
| "Context does not exist" error | Create the context first: `agk context create <name>`. List existing contexts with `agk context list`. |

### 10.7 TUI Problems

| Problem | Solution |
|---|---|
| Terminal rendering is broken | Ensure your terminal supports true color and is at least 80×24. Try `export TERM=xterm-256color`. |
| Keybindings not responding | Some terminals intercept function keys. Try a different terminal (iTerm2, Kitty, Alacritty). |
| Scope toggle not working | Press `Tab` to toggle between Global and Workspace scope. The current scope is shown in the footer. |
| Search not finding remote skills | Ensure ClawHub vault is activated (Vaults tab, `Space` on `clawhub`). |

### 10.8 CLI Exit Codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | General failure |
| 2 | Validation failure |
| 3 | Partial success (some operations succeeded, some failed) |

`[Personal]` `[Team]`

---

## 11. Provider-Specific Guides

### 11.1 Claude Code

**Provider ID:** `claude-code`

**Install paths (workspace):**

| Asset type | Path |
|---|---|
| Skills | `{workspace}/.claude/skills/{name}/` (or `{workspace}/{provider_root}/skills/{name}/`) |
| Instructions | `{workspace}/.claude/instructions/{name}/` |
| MCP config | `{workspace}/.claude/mcp.json` |
| Profiles | `{workspace}/.claude/agents/{name}.md` |

**Install paths (global):**

| Asset type | Path |
|---|---|
| Skills | `~/.claude/skills/{name}/` |
| Instructions | `~/.claude/instructions/{name}/` |
| MCP config | `~/.claude/mcp.json` |

**Config roots:** `.claude` (default) or `.agents` (shared agents folder)

**Profile launch:** AGK generates an `agents/{name}.md` file with YAML frontmatter (name, provider, tools, permission_mode, skills, mcps) and runs `claude --agent <path>`.

**Capabilities:** Skills ✓ · Instructions ✓ · MCP ✓ · Profiles ✓

### 11.2 OpenCode

**Provider ID:** `opencode`

**Install paths (workspace):**

| Asset type | Path |
|---|---|
| Skills | `{workspace}/.opencode/skills/{name}/` (or `{workspace}/{provider_root}/skills/{name}/`) |
| Instructions | `{workspace}/.opencode/instructions/{name}/` |
| MCP config | `{workspace}/opencode.json` (note: at workspace root, not inside `.opencode/`) |
| Profiles | `{workspace}/.agk/profiles/{name}/agents/{name}.md` |

**Install paths (global):**

| Asset type | Path |
|---|---|
| Skills | `~/.config/opencode/skills/{name}/` |
| Instructions | `~/.config/opencode/instructions/{name}/` |
| MCP config | `~/.config/opencode/opencode.json` |

**Config roots:** `.opencode` (default) or `.agents` (Claude-compatible)

**Profile launch:** AGK patches `opencode.json` with per-agent entries and runs `opencode --agent <name>`, then cleans up the session agent entry on exit.

**Capabilities:** Skills ✓ · Instructions ✓ · MCP ✓ · Profiles ✓

### 11.3 GitHub Copilot

**Provider ID:** `github-copilot`

**Install paths (workspace):**

| Asset type | Path |
|---|---|
| Skills | `{workspace}/.github/skills/{name}/` |
| Instructions | `{workspace}/.github/instructions/{name}/` |

**Install paths (global):**

| Asset type | Path |
|---|---|
| Skills | `~/.copilot/skills/{name}/` |
| Instructions | `~/.copilot/instructions/{name}/` |
| MCP config | `~/.copilot/mcp-config.json` |

> **Note:** GitHub Copilot does not support workspace-scope MCP configuration. MCP is global-only.

**Capabilities:** Skills ✓ · Instructions ✓ · MCP ✓ (global only) · Profiles ✗

### 11.4 Gemini CLI

**Provider ID:** `gemini-cli`

**Install paths (workspace):**

| Asset type | Path |
|---|---|
| Skills | `{workspace}/.gemini/skills/{name}/` (or `{workspace}/{provider_root}/skills/{name}/`) |
| Instructions | `{workspace}/.gemini/instructions/{name}/` |

**Install paths (global):**

| Asset type | Path |
|---|---|
| Skills | `~/.gemini/skills/{name}/` |
| Instructions | `~/.gemini/instructions/{name}/` |
| MCP config | `~/.gemini/settings.json` |

**Config roots:** `.gemini` (default) or `.ai` (legacy)

> **Note:** Gemini CLI does not support workspace-scope MCP configuration. MCP is global-only.

**Capabilities:** Skills ✓ · Instructions ✓ · MCP ✓ (global only) · Profiles ✗

### 11.5 AMP Code

**Provider ID:** `amp`

**Install paths (workspace):**

| Asset type | Path |
|---|---|
| Skills | `{workspace}/.amp/skills/{name}/` |
| Instructions | `{workspace}/.amp/instructions/{name}/` |
| MCP config | `{workspace}/.amp/settings.json` |

**Install paths (global):**

| Asset type | Path |
|---|---|
| Skills | `~/.amp/skills/{name}/` |
| Instructions | `~/.amp/instructions/{name}/` |
| MCP config | `~/.config/amp/settings.json` |

> **Note:** AMP's MCP entries are nested under `amp.mcpServers` (not top-level).

**Capabilities:** Skills ✓ · Instructions ✓ · MCP ✓ · Profiles ✗

### 11.6 Firebender

**Provider ID:** `firebender`

**Install paths (workspace):**

| Asset type | Path |
|---|---|
| Skills | `{workspace}/.firebender/skills/{name}/` |
| Instructions | `{workspace}/.firebender/instructions/{name}/` |

**Install paths (global):**

| Asset type | Path |
|---|---|
| Skills | `~/.firebender/skills/{name}/` |
| Instructions | `~/.firebender/instructions/{name}/` |

**Capabilities:** Skills ✓ · Instructions ✓ · MCP ✗ · Profiles ✗

### 11.7 Letta

**Provider ID:** `letta`

**Install paths (workspace):**

| Asset type | Path |
|---|---|
| Skills | `{workspace}/.letta/skills/{name}/` |
| Instructions | `{workspace}/.letta/instructions/{name}/` |

**Install paths (global):**

| Asset type | Path |
|---|---|
| Skills | `~/.letta/skills/{name}/` |
| Instructions | `~/.letta/instructions/{name}/` |

**Capabilities:** Skills ✓ · Instructions ✓ · MCP ✗ · Profiles ✗

### 11.8 Snowflake Cortex

**Provider ID:** `snowflake`

**Install paths (workspace):**

| Asset type | Path |
|---|---|
| Skills | `{workspace}/.cortex/skills/{name}/` |
| Instructions | `{workspace}/.cortex/instructions/{name}/` |

**Install paths (global):**

| Asset type | Path |
|---|---|
| Skills | `~/.cortex/skills/{name}/` |
| Instructions | `~/.cortex/instructions/{name}/` |

> **Note:** The folder name is `.cortex`, not `.snowflake`.

**Capabilities:** Skills ✓ · Instructions ✓ · MCP ✗ · Profiles ✗

`[Personal]` `[Team]`

---

*AGK Support Guide — version 0.2.x*