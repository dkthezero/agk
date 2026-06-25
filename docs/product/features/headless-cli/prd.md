# PRD: Headless CLI Operations

**Status:** Implemented (v0.2)

> **Product Mindset:** `agk` is the agent kit for teams to share the way they work with AI agents together. The CLI is the contract; the TUI is the discoverability layer.

---

## Overview

While `agk`'s TUI excels at discovery and interactive management, team workflows, CI/CD pipelines, and autonomous AI agents require deterministic, non-interactive commands. This PRD defines the headless CLI surface that turns `agk` from a personal tool into a team infrastructure component.

---

## Functional Requirements

### Non-Interactive Execution
- Commands must execute and exit without spawning the Ratatui/Crossterm buffer.
- Exit codes:
  - `0` — Success (all operations completed)
  - `1` — General failure (network error, vault unreachable, I/O error)
  - `2` — Validation failure (config malformed, skill missing, hash mismatch)
  - `3` — Partial success (some assets installed/updated, others failed)

### Output Formatting
- **Default:** Human-readable plain text with progress spinners where applicable.
- **`--quiet` (`-q`):** Suppress all non-error output. Only exit codes matter.
- **`--verbose` (`-v`):** Full debug output: vault scanning, hash computation, provider path resolution.
- **`--json`:** Structured JSON output for every command, suitable for machine parsing.

### Subcommands

#### `agk sync [--global] [--dry-run]`
- **Behavior:** Reads the active `.agk/config.toml` (or `~/.config/agk/config.toml` with `--global`), sparse-clones required vaults if needed, and ensures all configured assets are installed to their target providers.
- **Use case:** Team onboarding script. A new hire runs `agk sync` and immediately has the same skills as the rest of the team.
- **Flags:**
  - `--global` — Force global scope
  - `--dry-run` — Output what would change without modifying filesystem or provider configs

#### `agk install <identity>`
- **Behavior:** Resolves `<identity>` against configured vaults, downloads the asset, and installs it to all active providers in the current scope.
- **Identity format:** `[vault/]name` or fully qualified `[vault/]name:version`.
- **Examples:**
  - `agk install clawhub/web-browser`
  - `agk install internal/react-parser:2.1.0`
- **Flags:** `--scope global|workspace`, `--dry-run`, `--provider <id>` (limit to single provider)

#### `agk validate`
- **Behavior:** Scans installed assets against their source vault representations. Verifies:
  - Parsing integrity (SKILL.md frontmatter, AGENTS.md structure)
  - `sha10` hash consistency
  - Provider path existence
- **Use case:** Git pre-commit hook. Ensures a developer hasn't accidentally corrupted their local skill configs.
- **Flags:** `--scope global|workspace`, `--json`

#### `agk pack <identity> [--target <provider>]`
- **Behavior:** Compiles the raw markdown and supporting files of a skill into a provider-specific distributable package.
- **Targets:**
  - `claude-desktop` — Claude Desktop zip bundle (✅ implemented)
  - `firebender` — JSON manifest for Firebender (✅ implemented; emits a JSON document describing the skill identity, metadata, and base64-encoded file tree)
  - `tarball` — Plain tarball for generic use (✅ implemented)
- **Output:** Writes to `./.agk/pack/<identity>-<target>.zip` (or prints to stdout with `--stdout`).

---

## User Personas & Expected UX

### 👤 Human User

| Scenario | Expected UX |
|----------|-------------|
| New team member onboarding | `agk sync --dry-run` first to preview, then `agk sync` to apply. Sees a concise summary: "Installed 7 skills, updated 2, skipped 0." |
| Installing a specific skill | `agk install clawhub/web-browser` shows a progress line: "Resolving… → Downloading… → Installing to Claude Code… → Done." |
| Pre-commit validation | `agk validate` runs in <500ms. If a skill is broken, prints the exact file and line. |
| Sharing a skill pack | `agk pack my-skill --target claude-desktop` creates a zip that can be emailed or uploaded to Slack. |

### 🤖 AI Agent User

| Scenario | Expected UX |
|----------|-------------|
| Agent discovers a new skill | `agk install clawhub/web-browser --json` returns `{"installed": true, "identity": "web-browser:1.2.0:a13c9ef042", "providers": ["claude-code"], "sha10": "a13c9ef042"}` |
| Agent checks environment health | `agk validate --json` returns structured pass/fail per asset so the agent can self-heal. |
| Agent needs to remain silent | `agk sync --quiet` — agent only checks exit code `0`. |

### 🏭 CI/CD User

| Scenario | Expected UX |
|----------|-------------|
| GitHub Actions ensures team consistency | `agk sync --global --quiet` in a workflow step. Exit `0` = green check, exit `1` or `2` = red fail. |
| Pre-commit hook blocks broken configs | `agk validate --scope workspace` in `.pre-commit-config.yaml`. Failures prevent the commit. |
| Pipeline reports skill drift | `agk sync --dry-run --json` output is parsed by the pipeline to post a PR comment: "3 skills out of date." |

---

## Non-Goals
- Real-time TUI streaming in headless mode. If `--json` is passed, all progress is batched and emitted at the end.
- Interactive prompts in headless mode. If a command requires input (e.g., vault auth), it fails with exit code `1` and a message to use the TUI.
- GUI or web interface. This PRD is strictly CLI/TUI.

---

## Acceptance Criteria
- [x] All four subcommands (`sync`, `install`, `validate`, `pack`) are implemented.
- [x] `--quiet`, `--verbose`, `--json` work consistently across subcommands.
- [x] Exit codes `0`, `1`, `2`, `3` are used correctly.
- [x] Headless commands never allocate a terminal alternate screen.
- [x] TUI still uses the same underlying pure async functions (no logic duplication).
- [x] `cargo test` includes at least one integration test per subcommand.
- [x] `pack` Firebender target fully wired (emits a JSON manifest with skill identity, metadata, and base64 file tree).

---

*End of PRD.*
