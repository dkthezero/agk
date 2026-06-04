# agk

The agent kit for teams to share how they work with AI — together.

Manage vaults of reusable skills, instructions, MCP servers, and **profiles**. Install them to Claude Code, GitHub Copilot, Gemini, OpenCode, and more — all from an interactive TUI.

![License](https://img.shields.io/github/license/dkthezero/agk)
![Crates.io](https://img.shields.io/crates/v/agk)
![GitHub release](https://img.shields.io/github/v/release/dkthezero/agk)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/dkthezero/agk)

## What's new in v0.3.0

**Claude Code agent profiles + multi-provider LLM support.** v0.3.0 extends the
profile system with a wizard that emits native Claude Code agent markdown, and
adds a sibling port for LLM model servers (Ollama, LM Studio, Anthropic, OpenAI).

- **Claude Code agent profiles** — `agk profile create` now supports `claude-code` as a provider, generating `.claude/agents/<name>.md` agent markdown with full frontmatter (name, description, tools, model, permission_mode, skills, mcp_servers, hooks, etc.)
- **Multi-provider LLM support** — configure Ollama, LM Studio, Anthropic, and OpenAI as model servers. `agk llm list/add/remove/health` manages providers. `LlmProviderPort` is a sibling to the existing `ProviderPort` (different role: model server vs agent harness)
- **Hexagonal provider boundaries** — agent providers (claude-code, opencode) and LLM providers are isolated; architecture test enforces that concrete LLM adapters are only imported from the composition root
- **Multi-stage Docker** — slim runtime image (~50MB) for production, full CI image for the test matrix
- **Slim-build regression** — the binary still builds with `--no-default-features --features tui` only

## What's new in v0.3

**Team-ready profiles** — the capstone feature. A profile is a portable, versioned environment blueprint that bundles an AI agent, its skills, MCP servers, and a structured prompt. Teams distribute profiles through vaults; new developers install one profile and are ready to code in 30 seconds.

- **Profile wizard** with 6 archetype templates (Code Reviewer, Feature Implementer, Security Auditor, Documentation Writer, Test Generator, Custom) — structured prompts replace raw Q&A
- **Token estimation** with green/yellow/red badges — know your prompt cost before launching
- **Vault-discoverable MCPs & profiles** — `mcps/` and `profiles/` directories in vaults are scanned automatically
- **5 MCP providers** — Claude Code, OpenCode, GitHub Copilot, Gemini CLI, and AMP
- **Auto-install on launch** — `agk p start` resolves missing skills and MCPs from vaults before starting
- **Profile export/import** — share profiles as JSON files across machines or with contractors
- **GHES vault support** — attach GitHub Enterprise Server vaults with SSO token resolution
- **MCP security scorecard** — heuristic flags for broad filesystem access, network egress, arbitrary execution
- **Profile diff** — detect drift between your local profile and its vault source with `agk profile diff`
- **Parallel vault scanning** — rayon-powered concurrent scan across `skills/`, `instructions/`, `mcps/`, `profiles/`
- **Telemetry** — track template selections and profile launches; export as CSV

## Features

- **Multi-provider support** — Install to Claude Code, GitHub Copilot, Gemini, Letta, Snowflake, Firebender, AMP, and OpenCode
- **Local, GitHub & ClawHub vaults** — Source skills from local directories, GitHub repositories (including GHES), or the [ClawHub](https://clawhub.ai) community marketplace
- **Interactive TUI** — Browse, search, install, and update assets with keyboard navigation
- **Headless mode** — All operations available as CLI commands with `--json`, `--quiet`, and deterministic exit codes for CI/CD
- **Team-ready profiles** — Create, share, and launch complete AI agent environments from vaults or JSON export files
- **MCP server registry** — Register, test, and enable MCP servers for 5 providers with a JSON-RPC handshake and security assessment
- **Skill bundling** — Meta-skills with `requires:` in `SKILL.md` frontmatter auto-install dependency trees
- **Change detection** — SHA-based hashing detects when vault assets have been updated
- **Scoped configuration** — Global settings for vaults, workspace-level settings for providers and installed assets
- **Batch operations** — Update all installed assets at once with `F5`
- **Smart provider lifecycle** — Deactivating the last provider prompts before wiping installed assets; re-activating a provider re-opens the config-folder selector
- **Clean config management** — Empty vault sections and arrays are auto-pruned from TOML; empty configs are removed entirely

## Installation

### Homebrew (macOS & Linux)

```bash
brew tap dkthezero/agk
brew install agk
```

### Cargo (from source)

```bash
cargo install agk
```

### From releases (macOS, Linux, Windows)

Download pre-built binaries from the [Releases](https://github.com/dkthezero/agk/releases) page.

## Quick start

Launch the TUI:

```bash
agk
```

### Attach a vault

1. Press `0` to switch to the Vaults tab
2. Press `F2` to attach a new vault
3. Enter a local path (`./my-vault`) or GitHub URL (`owner/repo`)
4. Follow the prompts for branch, subfolder, and vault name
   - GitHub URLs ask for branch and subfolder first
   - GHES URLs are auto-detected and use the enterprise API endpoint
   - All paths ask for a custom vault name (defaults to folder or repo name)

### Install a team profile

1. Press `5` to switch to the Profiles tab
2. Vault-discovered profiles appear with a `[Vault]` badge
3. Press `Space` on a vault profile — agk batch-installs all referenced skills, instructions, and MCPs
4. Press `Enter` to start the profile — agk auto-resolves any remaining missing dependencies, then launches the provider

That's it. One profile, one keypress, fully configured AI agent.

### Create a profile

1. Press `5` to switch to the Profiles tab, then `F2`
2. Pick a template — Code Reviewer, Feature Implementer, Security Auditor, Documentation Writer, Test Generator, or Custom
3. Fill in the structured prompts (template pre-fills most fields)
4. Select skills, MCPs, tools, and permissions
5. Review — see the composed prompt with token count estimate
6. Save and launch with `Enter`

### Browse & install from ClawHub

[ClawHub](https://clawhub.ai) is a community marketplace for agent skills. agk integrates with it out of the box:

1. Press `0` to switch to the Vaults tab — ClawHub appears as an inactive vault by default
2. Press `Space` on ClawHub to activate it (agk will help you install the `clawhub` CLI via Homebrew if needed)
3. Press `1` to switch to the Skills tab and start typing to search
4. agk searches your local vaults and ClawHub in parallel — remote results appear in gray with owner, downloads, and star counts
5. Press `Space` on a remote skill to install it

### Install a skill

1. Press `1` to switch to the Skills tab
2. Navigate with arrow keys, press `Space` to install
3. The skill is copied to all active providers

### Configure providers

1. Press `4` to switch to the Providers tab
2. Press `Space` to toggle providers on/off
3. When deactivating the **last** provider with installed assets, agk shows a confirmation popup to prevent accidental data loss
4. Re-activating a provider always re-opens the config-folder selector

### Register an MCP server

1. Press `2` to switch to the MCP Servers tab
2. Press `F2` to register a new MCP server
3. Fill in: Name, Command, Arguments, Transport (`stdio` or `sse`), Description
4. Confirm the security warning — agk will tell you exactly what it'll execute on your machine
5. agk auto-runs the MCP `initialize` handshake test. If it passes, the server appears in the list with `[✓]`
6. Security flags appear automatically — `[!]` for high-risk patterns (broad filesystem, network egress, arbitrary execution)

### Share a profile

```bash
# Export a profile to JSON
agk profile export web-app-team --file ./web-app-team.agk.json

# Import on another machine
agk profile import ./web-app-team.agk.json
```

### Check for profile drift

```bash
# Compare your local profile against the vault source
agk profile diff web-app-team
# Output: added/removed skills, MCPs, tools, permission mode differences
```

In the TUI, profiles that differ from their vault source show a `[⇄]` drift badge.

## Keybindings

| Key       | Action                                                           |
| --------- | ---------------------------------------------------------------- |
| `1`–`5`   | Skills, MCP Servers, Instructions, Providers, Profiles          |
| `0`       | Vaults tab                                                       |
| `Up/Down` | Navigate list                                                    |
| `Space`   | Install/uninstall asset, toggle provider/vault/MCP, install profile |
| `Enter`   | Update selected asset, start profile                             |
| `F2`      | Attach vault / register MCP / create profile (context-aware)     |
| `F3`      | Edit selected profile                                            |
| `F4`      | Refresh all vaults from source                                   |
| `F5`      | Update all installed assets                                      |
| `Ctrl+E`  | Export profile                                                    |
| `Ctrl+I`  | Import profile                                                   |
| `Tab`     | Toggle between Global and Workspace scope                        |
| `Type`    | Search/filter by name (searches ClawHub in parallel when active) |
| `Esc`     | Clear search / cancel / quit                                     |

## Vault structure

agk expects vaults to follow this layout:

```
my-vault/
├── skills/
│   ├── my-skill/
│   │   ├── SKILL.md        # Required
│   │   └── ...              # Supporting files
│   └── another-skill/
│       └── SKILL.md
├── instructions/
│   └── my-instruction/
│       ├── AGENTS.md        # Required
│       └── ...
├── mcps/                     # NEW in v0.3
│   └── filesystem/
│       └── MCP.md           # MCP server definition
├── profiles/                 # NEW in v0.3
│   └── web-app-team/
│       └── PROFILE.md      # Team profile blueprint
└── README.md
```

Skills and instructions support optional YAML frontmatter for metadata:

```markdown
---
name: my-skill
version: 1.0.0
---

# My Skill

...
```

Skills can declare dependencies with `requires:`:

```yaml
---
name: acme-company-pack
version: 1.0.0
requires:
  - clawhub/react-parser
  - clawhub/css-linter
requires_optional:
  - clawhub/storybook-scaffold
---
```

When someone installs `acme-company-pack`, agk recursively resolves and installs all dependencies. Circular dependencies are detected and rejected. Diamond dependencies are deduplicated.

### MCP.md format

```markdown
---
name: filesystem
version: 1.0.0
command: npx
args:
  - "-y"
  - "@modelcontextprotocol/server-filesystem"
  - "."
transport: stdio
description: File system access via MCP
---
```

### PROFILE.md format

```markdown
---
name: web-app-team
version: 1.2.0
provider: opencode
description: Full-stack web development profile
skills:
  - acme-org/react-skills
  - acme-org/typescript-linter
instructions:
  - acme-org/web-app-guidelines
mcps:
  - filesystem
  - github-api
---
```

Installing a vault profile is atomic — all referenced skills, instructions, and MCPs are installed together, or none are.

## Configuration

agk uses two configuration scopes:

| Scope     | Path                        | Purpose                        |
| --------- | --------------------------- | ------------------------------ |
| Global    | `~/.config/agk/config.toml` | Vaults, enabled providers      |
| Workspace | `.agk/config.toml`          | Installed assets per workspace |

MCP servers are stored separately:

| File                     | Purpose                                                   |
| ------------------------ | --------------------------------------------------------- |
| `~/.config/agk/mcp.toml` | Registered MCP servers with activation state per provider |

Profile sessions write to provider-specific directories:

| Provider    | Agent Path                      |
| ----------- | ------------------------------- |
| Claude Code | `.claude/agents/<name>.md`      |
| OpenCode    | `.opencode/agents/<name>.md`   |

### Clean up

```bash
agk clean            # Remove workspace config
agk clean --global   # Remove global config
```

## Supported providers

| Provider                                                                                   | Skills                       | Instructions              | MCP                                  | Profiles |
| ------------------------------------------------------------------------------------------ | ---------------------------- | ------------------------- | ------------------------------------ | -------- |
| [Claude Code](https://docs.anthropic.com/en/docs/claude-code/overview)                     | `~/.claude/skills/`          | `.claude/instructions/`   | `.claude/mcp.json`                   | ✅       |
| [OpenCode](https://github.com/anomalyco/opencode)                                          | `~/.config/opencode/skills/` | (Claude-compatible paths) | Flat `mcp.<name>` in `opencode.json` | ✅       |
| [GitHub Copilot](https://docs.github.com/en/copilot/how-tos/configure-custom-instructions) | `~/.copilot/`                | `.github/`                | `~/.copilot/mcp-config.json`         | —        |
| [Gemini CLI](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/gemini-md.md)  | `.gemini/`                   | `.ai/`                    | `~/.gemini/settings.json`            | —        |
| [AMP](https://ampcode.com/manual)                                                          | `.amp/`                      | —                         | `.amp/settings.json`                 | —        |
| [Letta](https://docs.letta.com/introduction)                                               | Provider-specific            | Provider-specific         | —                                    | —        |
| [Snowflake](https://docs.snowflake.com/en/user-guide/snowflake-cortex/cortex-agents)       | Provider-specific            | Provider-specific         | —                                    | —        |
| [Firebender](https://docs.firebender.com/get-started/agent)                                | Provider-specific            | Provider-specific         | —                                    | —        |

## Headless mode

agk works in CI/CD and automation without a terminal:

```bash
# Sync all configured assets (team onboarding)
agk sync

# Install a specific skill
agk install clawhub/web-browser

# Validate installed assets against source vaults
agk validate

# Pack a skill for distribution
agk pack my-skill --target claude-desktop

# Register an MCP server
agk mcp add --name fs --command npx \
  --args "@modelcontextprotocol/server-filesystem /tmp" \
  --transport stdio

# Enable MCP for a provider
agk mcp enable fs --provider claude-code --scope workspace

# Create a profile from a template
agk profile create my-reviewer \
  --provider claude-code \
  --template code-reviewer \
  --skills rust-patterns:clawhub,docker:ecc \
  --scope workspace

# Start a profile session
agk p start my-reviewer

# Export/import profiles
agk profile export my-reviewer --file ./reviewer.agk.json
agk profile import ./reviewer.agk.json

# Check drift between local and vault
agk profile diff my-reviewer
```

All commands support `--quiet`, `--verbose`, and `--json`.

## ClawHub integration

agk uses the [`clawhub` CLI](https://clawhub.ai) for all remote operations. When you activate the ClawHub vault, agk will:

1. Check if `clawhub` is on your `$PATH`
2. Offer to install it via Homebrew (`brew install clawhub`) if available
3. Display a manual install link if Homebrew is not available

Skills installed from ClawHub are cached in `~/.config/agk/clawhub/` and treated like any other vault source.

## Development

```bash
# Build
cargo build

# Run TUI
cargo run

# Test
cargo test

# Format (CI-enforced)
cargo fmt

# Lint
cargo clippy -- -D warnings
```

## Support

If you find agk useful, consider supporting its development:

[![Patreon](https://img.shields.io/badge/Patreon-Support-f96854?logo=patreon&logoColor=white)](https://www.patreon.com/dkthezero)

## License

[MIT](LICENSE)