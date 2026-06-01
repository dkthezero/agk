# PRD: GitHub Enterprise Server (GHES) Vault Support

**Status:** Draft
**Epic:** [v0.3.1 Enterprise Bridge & Profile Portability](../../../epics/v031-enterprise-bridge.md)

---

## Overview

AGK's GitHub vault adapter currently supports only `github.com`. Many enterprises run **GitHub Enterprise Server (GHES)** on private infrastructure and cannot expose their internal skill vaults to the public internet. This feature extends the GitHub vault backend to support GHES URLs, SSO authentication, and private repo access.

---

## User-Facing Behavior

### CLI

```bash
# Attach a GHES vault
agk vault attach https://github.acme.internal/acme-org/ai-workflows

# With explicit enterprise URL (if auto-detection fails)
agk vault attach https://github.acme.internal/acme-org/ai-workflows \
  --enterprise-url https://github.acme.internal

# Token resolution is automatic:
# 1. `gh auth token --hostname github.acme.internal`
# 2. `GITHUB_TOKEN` env var
# 3. `GITHUB_ENTERPRISE_TOKEN` env var
```

### TUI

- **Vault tab `[0]`:** Shows attached GHES vaults with `[GHES]` badge next to the vault name.
- **Attach flow (`F2`):** When a user enters a URL containing a non-`github.com` hostname, AGK auto-detects GHES and prompts for `enterprise_url` confirmation (pre-filled from hostname).
- **Detail view (`Enter`):** Shows `enterprise_url`, token source (`gh auth`, `GITHUB_TOKEN`, or `GITHUB_ENTERPRISE_TOKEN`), and last sync status.
- **Error handling:** If token lacks access, modal shows: "Token cannot access GHES vault. Ensure `gh auth` has `repo` scope for host `github.acme.internal`."

---

## Functional Requirements

1. **Auto-detection:** URLs with a hostname other than `github.com` shall be treated as GHES by default. User can override with `--type github` + `--enterprise-url`.
2. **API base URL customization:** The `GithubVaultAdapter` shall use `enterprise_url` as the API base (e.g., `https://github.acme.internal/api/v3/`) instead of `https://api.github.com/`.
3. **Token resolution order:**
   - `gh auth token --hostname <enterprise_host>` (if `gh` CLI ≥ 2.0 installed)
   - `GITHUB_TOKEN` env var
   - `GITHUB_ENTERPRISE_TOKEN` env var (GHES-specific fallback)
4. **Private repo support:** Same flow as public repos; authentication is token-scoped. No additional logic required beyond token handling.
5. **Sparse checkout:** GHES repos shall use the same `git sparse-checkout` pattern as github.com repos.
6. **Config persistence:** `enterprise_url` shall be stored in `config.toml` alongside other vault fields.
7. **Backward compatibility:** Vaults without `enterprise_url` continue to use `github.com` API.

---

## Config Schema

```toml
[[vaults]]
id = "acme-private"
type = "github"
url = "https://github.acme.internal/acme-org/ai-workflows"
enterprise_url = "https://github.acme.internal"
branch = "main"
path = "vault"
```

---

## Non-Goals

- GHES-specific rate-limit handling (use existing GitHub backoff; GHES admins configure their own limits).
- GHES SAML/SSO deep integration beyond `gh auth` token pass-through.
- GitHub Enterprise Cloud (GHEC) — separate SKU; may work with same adapter but not explicitly targeted.

## Security Considerations

- Tokens for GHES vaults may grant access to sensitive corporate repos. `config.toml` shall store vault URLs only, never tokens. Tokens are resolved at runtime from `gh auth` or env vars.
- `gh auth` tokens are typically stored in `~/.config/gh/hosts.yml` with OS-level encryption. AGK does not duplicate or persist them.

## Acceptance Criteria

- [ ] `agk vault attach` with GHES URL succeeds and lists skills/instructions/MCPs/profiles.
- [ ] `gh auth` token is used automatically when `gh` CLI is installed and authenticated.
- [ ] `GITHUB_ENTERPRISE_TOKEN` works as fallback when `gh` CLI is absent.
- [ ] TUI shows `[GHES]` badge for GHES vaults.
- [ ] Clear error message when token lacks `repo` scope.
- [ ] Old github.com vaults continue to work without changes.
- [ ] Architecture tests pass with zero allowlists.

---

*PRD v0.1 — 2026-05-30*
