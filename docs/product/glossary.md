# Glossary & Domain Schema

The purpose of this document is to catalog the ubiquitous domain terminology and encoding formats strictly enforced across the `agk` codebase.

## Concepts

### 1. Scopes
`agk` enables toggling configurations between two primary scope contexts directly in the TUI (typically with the `Tab` key):

- **Global Scope:** Affects assets and configurations stored universally for the active system user, usually resolving to `~/.config/agk/config.toml` and injecting assets into global provider configuration paths.
- **Workspace Scope:** Project-specific targeting. Saves and overrides the active provider selections in a local directory (`./.agk/config.toml`). Usually injects assets into the current folder root (like `.github/skills/`). The Workspace scopes can override and merge state visually in the TUI alongside global scopes to act dynamically depending on the current active directory.

### 2. Provider
A target AI framework or agent ecosystem that executes instructions or uses skills (e.g., GitHub Copilot, Claude Desktop, Firebender, OpenCode). Providers act as endpoints where `agk` syncs or "projects" assets into when they are marked as installed/active.

### 3. Vault
A remote or local directory mapping containing managed raw tools, instructions, and schemas. Vaults act as the canonical source of truth for library fetching/discovery before being assigned to target active providers.

### 4. AssetKind
Determines what the asset is used for contextually:
- `Skill`: A functional tool script, module, or logical wrapper that allows an AI agent to execute tasks or make changes mechanically. 
- `Instruction`: Custom behavior prompts or context files the AI agent should keep in consideration while operating. 
- `McpServer`: A Model Context Protocol server that provides tools to AI agents. Registered in agk's global MCP registry and activated per-provider. (NEW)

### 5. Meta-Skill
A `Skill` whose `SKILL.md` frontmatter contains a `requires:` array. Installing a meta-skill recursively installs all listed dependencies. Used for team onboarding packs. (NEW)

### 6. MCP Server
A Model Context Protocol server registered in agk's global MCP registry (`~/.config/agk/mcp.toml`). MCP servers are configured with a command, arguments, and transport (stdio or SSE), then activated per-provider per-scope. (NEW)

### 7. Headless CLI
The non-interactive command-line interface for `agk` (`agk sync`, `agk install`, `agk validate`, `agk pack`). Used by CI/CD pipelines, onboarding scripts, and AI agents. Supports `--quiet`, `--verbose`, and `--json` output. (NEW)

## Identity and Addressing Rules

### AssetIdentity

The canonical identification structure for items within `agk` configuration systems relies uniformly on a tripartite string model bracketed to assure deterministic storage keys and UI tracking.

Format:
```text
[<name>:<version>:<sha10>]
```

Example:
```text
[web-browsing-tool:1.2.0:a13c9ef042]
```

If version tags are unavailable or inherently non-applicable:
```text
[local-script-v1:--:9ac00ff113]
```

### Dependency Identity Format (Meta-Skills)
Dependencies in `requires:` use a looser identity format:
```text
vault/name          # latest version from vault
vault/name:version  # specific version
name                # search all vaults for name
```

## Freshness and Hashing

### Single Source of Truth (`sha10`)
While semantic version strings exist for display mapping, code divergence freshness **is always** determined deterministically by `sha10`. The tool inherently calculates code differences rather than trusting manually maintained version string headers.

#### Comparison Rule
An asset in `agk` is considered `Up to Date` globally if:
```text
current := installed.sha10 == scanned.sha10
```

#### Hash Scope Rules
When `agk` assesses packages across vaults to map them, it generates its `sha10` marker over the primary files associated with the object to generate a fingerprint.

- **For Skills:** `agk` computes the hashes spanning the canonical package tree which generally includes: `SKILL.md` along with adjacent `scripts/**`, `references/**`, and `assets/**`.
- **For Instructions:** Hashes the source document alongside metadata sidecar files if present.
- **For MCP Servers:** No sha10 hashing. MCP servers are runtime configurations (command + args), not static files.

### Hash Generation
For each package discovered:
1. Collect strictly canonical associated files
2. Sort strictly by relative path
3. Normalize line endings natively (CRL/LF → unified logic)
4. Compute standard `sha256` buffer
5. Store string of the first 10 hex characters natively as `sha10` attribute

---

*End of Glossary.*
