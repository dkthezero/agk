# PRD: Profile Export / Import

**Status:** Draft
**Epic:** [v0.3.1 Enterprise Bridge & Profile Portability](../../../epics/v031-enterprise-bridge.md)
**Related:** [Profiles PRD](../profiles/prd.md) (parent feature)

---

## Overview

v0.3 makes profiles vault-discoverable, but teams still need a way to share profiles **outside of a vault** — with contractors, across air-gapped environments, or as a quick backup/restore mechanism. Profile Export/Import serializes a profile (including its composed agent markdown, structured answers, skill/MCP references, and tool selections) to a portable JSON file that can be re-imported on any machine running AGK.

---

## User-Facing Behavior

### CLI

```bash
# Export a profile to JSON
agk profile export web-app-team --file ./web-app-team.agk.json

# Export with resolved vaults (replace "auto" with actual vault names)
agk profile export web-app-team --file ./web-app-team.agk.json --resolve-vaults

# Import a profile from JSON
agk profile import ./web-app-team.agk.json

# Import with custom name (avoids collision)
agk profile import ./web-app-team.agk.json --name web-app-team-contractor
```

### TUI

- **Profiles tab `[5]`:**
  - `Ctrl+E` — Export modal: file path input, scope toggle (export structured answers yes/no), `Enter` to confirm.
  - `Ctrl+I` — Import modal: file path input, preview pane showing profile name, provider, skills count, MCP count. `Enter` to confirm.
- **Import preview:** Shows warnings for missing vaults (e.g., "Vault 'acme-private' is not attached. Skills will be set to 'auto' resolution.").

---

## Functional Requirements

1. **Export serialization:** `ExportProfile` shall produce a JSON file containing:
   - `agk_version` (for future compatibility)
   - `exported_at` timestamp
   - Profile name, provider_id, scope
   - `structured_answers` (wizard inputs: role, domain, style, triggers, etc.)
   - Skills as `Vec<ProfileAssetRef>` (name + vault)
   - MCPs as `Vec<ProfileAssetRef>`
   - Tools + permission_mode
   - `agent_markdown` (the composed / custom markdown body)
2. **Import deserialization:** `ImportProfile` shall:
   - Validate `agk_version` compatibility (warn if > minor version ahead; error if major version mismatch).
   - Create a new profile entry in `config.toml`.
   - Write `agent_markdown` to `.agk/profiles/<name>/agent.md`.
   - Set missing vaults to `"auto"` with a warning.
   - Does NOT auto-install dependencies; user runs `agk p start` for that.
3. **Scope handling:** Exported scope is advisory. Import defaults to `workspace` but can be overridden with `--scope global`.
4. **Name collision:** If a profile with the same name exists in the target scope, import fails with a clear error. User can use `--name` to rename.
5. **Backward compatibility:** Future AGK versions must be able to import v0.3.1 export files (via `agk_version` gate).

---

## Export JSON Schema

```json
{
  "$schema": "https://agk.dev/schemas/profile-export-v1.json",
  "agk_version": "0.3.1",
  "exported_at": "2026-06-15T10:00:00Z",
  "profile": {
    "name": "web-app-team",
    "provider_id": "opencode",
    "scope": "workspace",
    "structured_answers": {
      "role": "Senior full-stack engineer",
      "domain": "React + Node.js",
      "audience": "Frontend platform team",
      "responsibilities": "Review PRs, suggest idioms, enforce standards",
      "style": "Pragmatic and thorough",
      "format": "Bullets, max 5 items",
      "triggers": "After any component file change",
      "constraints": "Always run tests before suggesting changes"
    },
    "skills": [
      { "name": "react-patterns", "vault": "clawhub" },
      { "name": "node-testing", "vault": "acme-private" }
    ],
    "mcps": [
      { "name": "filesystem", "vault": "auto" }
    ],
    "instructions": [
      { "name": "web-app-guidelines", "vault": "acme-private" }
    ],
    "tools": ["Read", "Glob", "Grep"],
    "permission_mode": "acceptEdits",
    "agent_markdown": "# Identity\nYou are a Senior full-stack engineer..."
  }
}
```

---

## Non-Goals

- Encrypted export files (fast-follow: `--encrypt` with `age` or GPG).
- Cloud-hosted profile sharing (out of scope; vaults are the canonical sharing mechanism).
- Export of installed skill files themselves (export contains references only, not file contents).
- Import auto-install of dependencies (intentionally separate step for user confirmation).

## Security Considerations

- Export JSON may contain sensitive internal prompts. Documentation shall warn users to share `.agk.json` files over secure channels only.
- No secrets (tokens, API keys) shall ever be included in the export. `agent_markdown` is user-composed text only.

## Acceptance Criteria

- [ ] `agk profile export` produces valid JSON matching the schema.
- [ ] `agk profile import` creates profile entry + writes `agent.md`.
- [ ] Export/import roundtrip produces equivalent profile config.
- [ ] Major version mismatch blocks import with clear error.
- [ ] Missing vaults in import set to `"auto"` with warning.
- [ ] Name collision fails with clear error; `--name` override works.
- [ ] TUI export/import modals show preview and confirmation.
- [ ] Architecture tests pass with zero allowlists.

---

*PRD v0.1 — 2026-05-30*
