# PRD: Skill Bundling & Meta-Skills

**Status:** Implemented (v0.2)

> **Product Mindset:** `agk` is the agent kit for teams to share the way they work with AI agents together. Meta-skills are the ultimate team onboarding tool — one identifier deploys an entire standardized workflow.

---

## Overview

Currently, users must install individual skills one by one. A meta-skill (or "skill pack") is a single `SKILL.md` that declares a list of dependencies in its YAML frontmatter. When installed, `agk` recursively resolves and installs the entire dependency tree. This enables tech leads to create standardized "way of work" packages that junior engineers can deploy with a single command.

---

## Functional Requirements

### Dependency Declaration
- `SKILL.md` YAML frontmatter supports `requires:` array.
- Each entry is an asset identity: `vault/name` (version optional; agk resolves latest).
- Optional: `requires_optional:` for soft dependencies that are installed only if available (no failure if missing).
- Example:
  ```yaml
  ---
  name: frontend-engineer-pack
  version: "1.0.0"
  description: "A meta-bundle of skills optimized for React and CSS workflows."
  requires:
    - clawhub/react-parser
    - clawhub/css-linter
    - internal-vault/component-generator
  requires_optional:
    - clawhub/storybook-scaffold
  ---
  ```

### Dependency Resolution
- **BFS (Breadth-First Search)** traversal of the dependency tree.
- Returns a queue in installation order (parents first, then children breadth-first).
- Queue items contain the resolved `ScannedPackage` with full identity info.

### Circular Dependency Guard
- Tracks the active resolution path (`Vec<String>` of names).
- If a dependency resolves to a name already in the branch, aborts with a clear error:
  ```
  Circular dependency detected: frontend-engineer-pack → react-parser → component-generator → frontend-engineer-pack
  ```

### Diamond Dependency Deduplication
- If two sibling skills both require the same grandchild, the grandchild is queued only once.
- Deduplication is keyed by `(vault_id, name, sha10)`.
- If the target sha10 matches the already-queued version, skip.

### Scope Inheritance
- A meta-skill installed in **Global** scope installs its children in **Global** scope.
- A meta-skill installed in **Workspace** scope installs its children in **Workspace** scope.
- No cross-scope leakage.

---

## User Personas & Expected UX

### 👤 Human User

| Scenario | Expected UX |
|----------|-------------|
| Tech lead creates a pack | Writes a `SKILL.md` with `requires:` and pushes it to the team vault. |
| Junior engineer onboarding | TUI Skills tab → finds `Acme-Company-Pack` → presses `Space`. Background tasks show progress for each dependency. |
| Circular dependency encountered | TUI detail pane or CLI shows a red error with the exact cycle path. |
| Diamond dependency | User installs `frontend-engineer-pack`. `react-parser` and `css-linter` both require `dom-utils`. `dom-utils` installs once. |

### 🤖 AI Agent User

| Scenario | Expected UX |
|----------|-------------|
| Agent installs a pack | `agk install acme/frontend-pack --json` returns structured install results per asset. |
| Agent validates a pack | `agk validate` checks that installed assets match their source vault hashes. |

### 🏭 CI/CD User

| Scenario | Expected UX |
|----------|-------------|
| Team pack is versioned | `agk validate` in CI ensures installed skills match source vault hashes. |
| Onboarding script | `agk install acme/onboarding-pack --global --quiet` in a team setup script. Exit `0` = new machine is ready. |

---

## Non-Goals
- Semver range resolution (`^1.0.0`, `>=2.0.0`). agk uses `sha10` as the source of truth, not version ranges. The `requires:` array specifies exact identities or relies on vault default resolution.
- Nested meta-skills beyond depth 3 (defensible limit to prevent runaway installs).
- Auto-update of child dependencies when the parent pack is updated. Updating a pack reinstalls children based on the pack's current `requires:` list.

---

## Acceptance Criteria
- [x] `SKILL.md` frontmatter parser reads `requires:` and `requires_optional:`.
- [x] BFS resolution with cycle detection.
- [x] Diamond deduplication by `(vault, name, sha10)`.
- [ ] TUI macro-progress bar for pack installation with child task increments (background tasks cover basic progress; dedicated pack progress UI is future work).
- [ ] `--json` output for `agk install` includes the full dependency tree (basic install results returned; tree serialization is future work).
- [ ] `agk validate` detects unresolvable `requires:` entries (validate checks installed assets, not frontmatter dependencies — future work).

---

*End of PRD.*
