# Rust Security (AGK Edition)

> Adapted for AGK from `ECC/rules/rust/security.md`. SQL-injection content dropped (AGK has no SQL); secrets and process-spawning rules tightened.

## Secrets Management

AGK handles provider API keys, GitHub tokens for vault refresh, and MCP server credentials. These must never appear in source or default config.

- Never hardcode API keys, tokens, or credentials.
- Read from environment variables: `std::env::var("...")`.
- Fail fast at startup if a required secret is missing — don't fall back to a placeholder.
- `.env` files and provider config files containing tokens MUST be in `.gitignore`.
- Provider adapters in `infra/provider/` are the only places allowed to read credential env vars.

```rust
// GOOD — fails at the boundary, message names the variable
fn load_anthropic_key() -> anyhow::Result<String> {
    std::env::var("ANTHROPIC_API_KEY")
        .with_context(|| "ANTHROPIC_API_KEY must be set in environment")
}

// BAD — silently uses a placeholder
fn load_anthropic_key_bad() -> String {
    std::env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| "missing".into())
}
```

## Input Validation at Boundaries

Validate **once** at the adapter boundary — by the time data reaches `app/features/`, it must be typed. This is the "parse, don't validate" rule.

**AGK boundaries:**
- CLI args → use Clap's parser; convert raw strings to domain types in `cli/features/<f>.rs`.
- TUI input → controller validates before emitting `CoreCommand`.
- Manifest files (TOML/YAML) → `infra/config/codecs/` parses into typed structs; invalid manifests are rejected here, not in features.
- Vault URLs and MCP commands → validate format before constructing the `CoreCommand`.

```rust
// GOOD — parse at the boundary, illegal states unrepresentable downstream
pub struct VaultRef(String);

impl VaultRef {
    pub fn parse(input: &str) -> anyhow::Result<Self> {
        let trimmed = input.trim();
        anyhow::ensure!(!trimmed.is_empty(), "vault ref cannot be empty");
        anyhow::ensure!(
            trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '/'),
            "vault ref contains invalid characters: {trimmed}"
        );
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str { &self.0 }
}
```

## Process Spawning

`std::process::Command` is restricted by architecture rule to `infra/process/` and `main.rs`. This protects against:
- Command injection (passing untrusted strings to a shell).
- Domain code accidentally depending on host state.

**Rules for `infra/process/`:**
- Always pass args as `&[String]` to `Command::args(...)` — never use `Command::new("sh").arg("-c").arg(format!("{cmd} {user_input}"))`.
- Never invoke a shell to interpret user input.
- Validate the command name against an allowlist if the source is user input (e.g., MCP transport types).
- Set `current_dir` explicitly; don't rely on the CWD.

```rust
// GOOD — args as a typed slice, no shell interpretation
impl ProcessRunnerPort for StdProcessRunner {
    fn run(&self, command: &str, args: &[String], cwd: &Path)
        -> anyhow::Result<ExitStatus>
    {
        let status = std::process::Command::new(command)
            .args(args)
            .current_dir(cwd)
            .status()
            .with_context(|| format!("failed to spawn {command}"))?;
        Ok(status)
    }
}

// BAD — shell injection via format!
let status = std::process::Command::new("sh")
    .arg("-c")
    .arg(format!("git clone {user_supplied_url}"))  // injection point
    .status()?;
```

## Unsafe Code

AGK should have **zero `unsafe`** blocks. If you find yourself needing one:

- Stop and check: is there a safe abstraction in a crate already?
- If you truly need `unsafe`, the block MUST have a `// SAFETY:` comment naming every required invariant.
- All `unsafe` blocks are reviewed by the architecture maintainer before merge.

```rust
// GOOD — SAFETY comment documents every invariant
// SAFETY: `ptr` is non-null (checked above), aligned, points to an initialized
// Widget, and no mutable references exist for the duration of this borrow.
let widget: &Widget = unsafe { &*ptr };

// BAD — no justification
unsafe { &*ptr }
```

## Dependency Security

Run periodically (CI should also do this):

```bash
cargo audit       # known CVEs
cargo deny check  # advisories, license violations, version duplicates
cargo tree -d     # duplicate transitive deps
```

When adding a crate to `Cargo.toml`:
- Prefer crates with > 1M downloads or well-known maintainers.
- Check the crate's last release date — abandoned crates are a supply chain risk.
- Minimize feature flags — opt out of features you don't need (`default-features = false`).
- Heavy subsystems (HTTP clients, YAML, TUI rendering) should be behind a Cargo feature flag so headless builds stay small.

## File and Path Safety

- Never trust paths from manifests — they may be relative escapes (`../../etc/passwd`).
- Canonicalize and verify paths stay within the expected root before reading/writing.
- Domain code receives `&Path`/`&PathBuf` but does not read or write — `std::fs` lives in `infra/` only.

```rust
// GOOD — confine to a base directory
fn resolve_inside(base: &Path, user_relative: &Path) -> anyhow::Result<PathBuf> {
    let joined = base.join(user_relative);
    let canonical = joined.canonicalize()
        .with_context(|| format!("failed to canonicalize {}", joined.display()))?;
    anyhow::ensure!(
        canonical.starts_with(base),
        "path {} escapes base {}",
        canonical.display(),
        base.display()
    );
    Ok(canonical)
}
```

## Error Messages to Users

CLI / TUI error output is user-facing. Don't leak internals.

- ✅ `"failed to load workspace config: file not found at .../agk.toml"` — actionable.
- ❌ `"Os { code: 2, kind: NotFound, message: \"No such file or directory\" }"` — exposes internals.

Use `anyhow::Context` to add a user-friendly prefix; the underlying error is preserved in the chain for `--verbose` output.

## What NOT to Do

- ❌ `Command::new("sh").arg("-c").arg(user_input)` — shell injection.
- ❌ Hardcoded tokens, even "for testing" — they end up in git history.
- ❌ Silent fallbacks for missing secrets — fail loudly at startup.
- ❌ `unsafe` without `// SAFETY:` — automatic review blocker.
- ❌ `.unwrap()` on `std::env::var()` — print the variable name in the error.
