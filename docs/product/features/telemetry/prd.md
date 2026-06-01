# PRD: Telemetry & Skill Usage Analytics

**Status:** Implemented (v0.2, updated v0.3.1)

> **Product Mindset:** `agk` is the agent kit for teams to share the way they work with AI agents together. Telemetry helps teams prune dead weight and invest in high-value skills — but only if it is strictly local, opt-out-capable, and resilient.

---

## Overview

Users and team leads lack visibility into which installed skills are actually being leveraged by their AI providers. This PRD proposes a passive, local-only analytics layer that scans provider log directories to infer skill invocation patterns. The data never leaves the machine.

> **Privacy-first policy:** All analytics are stored locally in `~/.config/agk/analytics.toml`. No network transmission. No cloud aggregation. Enabled by default but can be disabled via `agk telemetry disable`.

---

## Functional Requirements

### Passive Log Tailing
- `agk` does not hook network requests or invade provider processes.
- It implements lightweight string-matching parsers for known provider log directories:
  - **Claude Code:** `~/Library/Logs/Claude/` (macOS), `%APPDATA%/Claude/logs/` (Windows), `~/.local/share/Claude/logs/` (Linux)
  - **GitHub Copilot:** `%APPDATA%/GitHub Copilot/logs/`, `~/Library/Logs/GitHub Copilot/`
  - **OpenCode:** `~/.config/opencode/logs/` (if applicable; TBD based on OpenCode log conventions)
- Parsers look for skill-name execution patterns (e.g., `"executed tool `web-browsing-tool'`", `"skill `react-parser' invoked"`).

### v0.3.1: Template & Profile Tracking
- The wizard controller writes a telemetry event when a template is selected: `template_selected: "feature-implementer"`.
- The profile start use case writes a telemetry event when a profile is launched: `profile_launched: "web-app-team"`.
- Both are stored in `analytics.toml` under `[templates]` and `[profiles]` sections.

### v0.3.1: CSV Export
- `agk telemetry export --csv --file ./agk-usage.csv` generates a shareable report with columns:
  - `category` (skill / template / profile / mcp)
  - `name`
  - `total_invocations` or `selections` or `launches`
  - `last_used`
  - `providers` (if applicable)
- No PII. No usernames. No machine names. Safe to share with managers.

### Data Structure
- `~/.config/agk/analytics.toml` stores:
  ```toml
  [settings]
  enabled = true
  last_scan = "2026-05-01T14:32:00Z"

  [skills."web-browsing-tool"]
  total_invocations = 42
  last_used = "2026-05-01T14:32:00Z"
  providers = ["claude-code"]

  # v0.3.1: Template tracking
  [templates."feature-implementer"]
  selections = 15
  last_selected = "2026-05-10T09:00:00Z"

  [templates."code-reviewer"]
  selections = 8

  # v0.3.1: Profile tracking
  [profiles."web-app-team"]
  launches = 23
  last_launched = "2026-05-12T16:45:00Z"
  provider = "opencode"
  ```

### TUI Dashboard (Tab 5 — "Telemetry")
- **Skills tab:** Displays a table with columns: Skill name, Total invocations, Last used timestamp, Providers.
- **Templates tab (v0.3.1):** Displays template name, Selections count, Last selected.
- **Profiles tab (v0.3.1):** Displays profile name, Launches count, Last launched, Provider.
- Stale skills/templates (no activity in last 30 days) are dimmed.
- Toggle to enable/disable analytics collection from the TUI not yet implemented — use CLI for now.
- **Tab spacing design:** At least 3 spaces between adjacent tab labels for readability.

### Background Processing
- Log scanning runs in a low-priority background `tokio` task.
- It wakes every 60 seconds or on explicit user request (`F5` in Telemetry tab).
- Never blocks the TUI render loop.

---

## User Personas & Expected UX

### 👤 Human User

| Scenario | Expected UX |
|----------|-------------|
| Team lead audits skill value | Opens Tab 5 (Telemetry). Sees `web-browsing-tool` with 150 invocations and `arxiv-researcher` with 0. Decides to remove the dead skill from the team pack. |
| User disables analytics | `agk telemetry disable` stops all scanning. `agk telemetry enable` resumes. |
| User checks analytics status | `agk telemetry status` shows whether collection is active and how many skills/templates/profiles are tracked. `--json` returns structured output. |
| Missing log directory | If a provider log directory doesn't exist (e.g., Copilot not installed), the row for that provider shows "No logs found" in gray. No error modal. |

### 🤖 AI Agent User

| Scenario | Expected UX |
|----------|-------------|
| Agent checks skill popularity | Not a primary use case. Agents do not need usage analytics; they need installation and execution. |
| Agent queries telemetry status | `agk telemetry status --json` returns `{"enabled": true, "skills_tracked": 12, "templates_tracked": 5, "profiles_tracked": 3, "last_scan": "2026-05-01T14:32:00Z"}` |

### 🏭 CI/CD User

| Scenario | Expected UX |
|----------|-------------|
| Not applicable | Telemetry is an observability feature for human decision-making, not a pipeline gate. CI/CD does not consume analytics data. |

---

## Non-Goals
- Network transmission of any data. This is strictly local.
- Real-time provider hooking or monkey-patching.
- Cross-machine aggregation or dashboards.
- Analytics for Instructions (only Skills, since Instructions are passive context, not invoked tools).
- Provider log parsing for providers that do not write structured logs (e.g., AMP, Firebender) unless their log format is formally documented.

---

## Acceptance Criteria
- [x] Enabled by default (opt-out via `agk telemetry disable`).
- [x] Passive log parsers for Claude Code and GitHub Copilot (minimum viable set).
- [x] Data stored only in `~/.config/agk/analytics.toml`.
- [x] TUI Tab 5 displays usage stats with sortable columns.
- [x] Background task scans logs every 60s without blocking the render loop.
- [x] Missing log directories are silently skipped (no panics, no modals).
- [x] `agk telemetry enable|disable|status` CLI subcommands.
- [x] `--json` support for `agk telemetry status`.
- [x] v0.3.1: Template and profile tracking in analytics.
- [x] v0.3.1: Telemetry CSV export (`agk telemetry export --csv`).
- [ ] Toggle to enable/disable from TUI Tab 5 (future enhancement).

---

*End of PRD.*
