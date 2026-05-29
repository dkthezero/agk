//! Architecture-enforcement tests.
//!
//! These tests scan the `src/` directory and assert that the hexagonal
//! dependency rules are never violated:
//!
//! 1. `domain/` must not import `app/`, `infra/`, `tui/`, or `cli/`.
//! 2. `app/` must not import `tui/` or `cli/`.
//! 3. `infra/` must not import `tui/` or `cli/`.
//! 4. `tui/` must not import `infra/`.
//! 5. `cli/` must not import `tui/`.
//! 6. No direct `std::process::Command` outside `infra/process/`.
//! 7. No non-test `.rs` source file exceeds ~300 lines (temporary allowlist).
//!
//! Phase A: These tests are now part of the CI architecture gate.
//! Run explicitly: `cargo test --test architecture -- --ignored`
//!
//! Temporary allowlists are annotated with their assigned convergence phase
//! (docs/proposals/architectural-convergence-plan.md). New violations must
//! not be added.

use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Collect every `.rs` file under `src/<dir>/` (recursively).
fn collect_rust_files(dir: &str) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let root = Path::new("src").join(dir);
    if !root.exists() {
        return files;
    }
    for entry in walkdir::WalkDir::new(&root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
    {
        files.push(entry.path().to_path_buf());
    }
    files
}

/// Read file contents to a String.
fn read_file(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

/// Return the set of forbidden crate imports found in `text`.
fn forbidden_imports(text: &str, forbidden: &[&str]) -> Vec<String> {
    let mut violations = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("use ") && !trimmed.starts_with("pub use ") {
            continue;
        }
        for target in forbidden {
            let needle = format!("crate::{}", target);
            if trimmed.contains(&needle) {
                violations.push(format!("  {}", trimmed));
            }
        }
    }
    violations
}

/// Return the set of forbidden `crate::<target>` references (not just imports).
fn forbidden_refs(text: &str, forbidden: &[&str]) -> Vec<String> {
    let mut violations = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        for target in forbidden {
            let needle = format!("crate::{}", target);
            // Skip comments and doc strings
            if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("*") {
                continue;
            }
            if trimmed.contains(&needle) && !trimmed.starts_with("use ") {
                violations.push(format!("  {}", trimmed));
            }
        }
    }
    violations
}

/// Return the set of forbidden `use crate::infra` or inline `crate::infra::`
/// references found in `text`.
fn contains_infra_refs(text: &str) -> Vec<String> {
    forbidden_imports(text, &["infra"])
        .into_iter()
        .chain(forbidden_refs(text, &["infra"]))
        .collect()
}

/// Return any line referencing `std::process::Command::new` outside the
/// allowed directory `src/infra/process/`.
fn contains_direct_process_spawn(text: &str, path: &Path) -> Vec<String> {
    let allowed_prefix = Path::new("src").join("infra/process");
    if path.starts_with(&allowed_prefix) {
        return Vec::new();
    }
    let mut violations = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("*") {
            continue;
        }
        if trimmed.contains("std::process::Command::new") {
            violations.push(format!("  {}", trimmed));
        }
    }
    violations
}

/// Strip `#[cfg(test)] mod ... { ... }` blocks from `text`.
///
/// Test modules legitimately contain `std::fs` / `std::process` calls (temp
/// dirs, fixtures). Domain purity scans must ignore them. This is a brace-
/// depth scanner — sufficient for well-formed Rust where `{` / `}` inside
/// strings or comments are not expected in domain test fixtures.
fn strip_cfg_test_modules(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_cfg_test = false;
    let mut in_test_mod = false;
    let mut depth: i32 = 0;

    for line in text.lines() {
        if in_test_mod {
            for ch in line.chars() {
                if ch == '{' {
                    depth += 1;
                } else if ch == '}' {
                    depth -= 1;
                    if depth == 0 {
                        in_test_mod = false;
                        break;
                    }
                }
            }
            continue;
        }
        let trimmed = line.trim();
        if pending_cfg_test {
            if trimmed.starts_with("mod ") || trimmed.starts_with("pub mod ") {
                let opens = line.chars().filter(|c| *c == '{').count() as i32;
                let closes = line.chars().filter(|c| *c == '}').count() as i32;
                let line_depth = opens - closes;
                if line_depth > 0 {
                    in_test_mod = true;
                    depth = line_depth;
                }
                pending_cfg_test = false;
                continue;
            }
            pending_cfg_test = false;
        }
        if trimmed.starts_with("#[cfg(test)]") {
            pending_cfg_test = true;
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Return non-test lines under `path` that match `needle`. Honors single-line
/// comment skipping.
fn matching_non_test_lines(text: &str, needle: &str) -> Vec<String> {
    let stripped = strip_cfg_test_modules(text);
    let mut violations = Vec::new();
    for line in stripped.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("*") {
            continue;
        }
        if trimmed.contains(needle) {
            violations.push(format!("  {}", trimmed));
        }
    }
    violations
}

/// Return true if any non-test `.rs` file under `src/` exceeds `limit` lines.
/// Strips `#[cfg(test)] mod tests { ... }` blocks before counting so test
/// code does not inflate the business-logic line count.
fn files_exceeding_line_limit(dir: &str, limit: usize) -> Vec<(PathBuf, usize)> {
    let mut offenders = Vec::new();
    let root = Path::new(dir);
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
    {
        let path = entry.path();
        // Skip test files (files ending in _tests.rs or inside a tests/ directory)
        if path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|n| n.ends_with("_tests.rs"))
            .unwrap_or(false)
        {
            continue;
        }
        if path
            .parent()
            .and_then(|p| p.file_name())
            .map(|f| f == "tests")
            .unwrap_or(false)
        {
            continue;
        }
        let text = read_file(path);
        let stripped = strip_cfg_test_modules(&text);
        let lines = stripped.lines().count();
        if lines > limit {
            offenders.push((path.to_path_buf(), lines));
        }
    }
    offenders
}

/// Scan all `.rs` files under `src/<dir>/` and assert none contain any of the
/// forbidden crate references.
fn assert_dir_has_no_refs(dir: &str, forbidden: &[&str], label: &str) {
    let files = collect_rust_files(dir);
    let mut all_violations = Vec::new();

    for path in &files {
        let text = read_file(path);
        let mut violations = forbidden_imports(&text, forbidden);
        violations.extend(forbidden_refs(&text, forbidden));
        if !violations.is_empty() {
            all_violations.push(format!(
                "{}\n{}",
                path.strip_prefix("src/").unwrap_or(path).display(),
                violations.join("\n")
            ));
        }
    }

    assert!(
        all_violations.is_empty(),
        "Architecture violation: {} code contains forbidden imports/refs:\n{}",
        label,
        all_violations.join("\n")
    );
}

// ---------------------------------------------------------------------------
// Tests (run with `cargo test -- --ignored`)
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn domain_must_not_import_app() {
    assert_dir_has_no_refs("domain", &["app"], "domain → app");
}

#[test]
#[ignore]
fn domain_must_not_import_infra() {
    assert_dir_has_no_refs("domain", &["infra"], "domain → infra");
}

#[test]
#[ignore]
fn domain_must_not_import_tui() {
    assert_dir_has_no_refs("domain", &["tui"], "domain → tui");
}

#[test]
#[ignore]
fn domain_must_not_import_cli() {
    assert_dir_has_no_refs("domain", &["cli"], "domain → cli");
}

#[test]
#[ignore]
fn app_must_not_import_tui() {
    assert_dir_has_no_refs("app", &["tui"], "app → tui");
}

#[test]
#[ignore]
fn app_must_not_import_cli() {
    assert_dir_has_no_refs("app", &["cli"], "app → cli");
}

#[test]
#[ignore]
fn infra_must_not_import_tui() {
    assert_dir_has_no_refs("infra", &["tui"], "infra → tui");
}

#[test]
#[ignore]
fn infra_must_not_import_cli() {
    assert_dir_has_no_refs("infra", &["cli"], "infra → cli");
}

#[test]
#[ignore]
fn tui_must_not_import_infra() {
    // The TUI must not call infrastructure directly — all I/O goes through ports.
    let files = collect_rust_files("tui");
    let mut all_violations = Vec::new();
    for path in &files {
        let text = read_file(path);
        let refs = contains_infra_refs(&text);
        if !refs.is_empty() {
            all_violations.push(format!(
                "{}\n{}",
                path.strip_prefix("src/").unwrap_or(path).display(),
                refs.join("\n")
            ));
        }
    }
    assert!(
        all_violations.is_empty(),
        "Architecture violation: TUI code contains illegal `crate::infra` references.\n\
         The TUI must communicate with infrastructure only via port traits in `app::ports`.\n{}",
        all_violations.join("\n")
    );
}

#[test]
#[ignore]
fn cli_must_not_import_tui() {
    assert_dir_has_no_refs("cli", &["tui"], "cli → tui");
}

// ---------------------------------------------------------------------------
// Phase A allowlist helpers – files known to violate rules until their
// assigned convergence phase completes.  New violations must not be added.
// ---------------------------------------------------------------------------

/// Out-of-scope-by-design exemptions from Rule 6.
///
/// Both entries are explicitly Phase E (out of ADR-001 scope): provider and
/// vault adapters still spawn `git`, `npx`, `brew`, and provider binaries
/// directly. A follow-up will route them through `ProcessRunnerPort`; until
/// then the allowlist documents the boundary so new violations elsewhere are
/// still caught.
fn is_phase_e_provider_vault_debt(path: &Path) -> bool {
    let s = path.strip_prefix("src/").unwrap_or(path).to_string_lossy();
    s.starts_with("infra/provider/") || s.starts_with("infra/vault/")
}

/// Rule 6: No direct std::process::Command outside infra/process/.
/// This protects testability and ensures all child-process execution is
/// concentrated in one infra module where it can be audited and replaced.
#[test]
#[ignore]
fn process_spawn_must_be_in_infra_process() {
    let mut violations = Vec::new();
    let root = Path::new("src");
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
    {
        let path = entry.path();
        // Tests are allowed to reference Command directly.
        if path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|n| n == "architecture.rs" || n.ends_with("_tests.rs"))
            .unwrap_or(false)
        {
            continue;
        }
        // Phase E follow-up: provider + vault adapters still spawn directly.
        if is_phase_e_provider_vault_debt(path) {
            continue;
        }
        let text = read_file(path);
        let refs = contains_direct_process_spawn(&text, path);
        if !refs.is_empty() {
            violations.push(format!(
                "{}\n{}",
                path.strip_prefix("src/").unwrap_or(path).display(),
                refs.join("\n")
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "Architecture violation: direct `std::process::Command::new` found outside `infra/process/`:\n{}",
        violations.join("\n")
    );
}

/// Domain purity (process): no `std::process::Command` may appear in
/// `src/domain/` outside `#[cfg(test)]` modules. Side effects belong in
/// `infra/`, exposed via a port.
///
/// Added by ADR-001 Commit 0. Will fail until Commit 1 lands
/// `FileOpenerPort` and moves `domain/paths.rs` open helpers.
#[test]
#[ignore]
fn domain_must_not_spawn_processes() {
    let files = collect_rust_files("domain");
    let mut all_violations = Vec::new();
    for path in &files {
        let text = read_file(path);
        let refs = matching_non_test_lines(&text, "std::process::Command");
        if !refs.is_empty() {
            all_violations.push(format!(
                "{}\n{}",
                path.strip_prefix("src/").unwrap_or(path).display(),
                refs.join("\n")
            ));
        }
    }
    assert!(
        all_violations.is_empty(),
        "Domain purity violation: `std::process::Command` found in domain/ outside #[cfg(test)].\n\
         Domain must be pure: extract the side effect to a port in app/ports/ \
         with an impl in infra/.\n{}",
        all_violations.join("\n")
    );
}

/// Domain purity (fs): no `std::fs::` may appear in `src/domain/` outside
/// `#[cfg(test)]` modules.
///
/// Added by ADR-001 Commit 0. Will fail until Commit 1 lands
/// `TelemetryStorePort` and removes file I/O from `domain/telemetry.rs`,
/// `domain/mcp.rs`, and `domain/hashing.rs`.
#[test]
#[ignore]
fn domain_must_not_use_fs() {
    let files = collect_rust_files("domain");
    let mut all_violations = Vec::new();
    for path in &files {
        let text = read_file(path);
        let refs = matching_non_test_lines(&text, "std::fs::");
        if !refs.is_empty() {
            all_violations.push(format!(
                "{}\n{}",
                path.strip_prefix("src/").unwrap_or(path).display(),
                refs.join("\n")
            ));
        }
    }
    assert!(
        all_violations.is_empty(),
        "Domain purity violation: `std::fs::` found in domain/ outside #[cfg(test)].\n\
         Domain must be pure: file I/O belongs in infra/, behind a port.\n{}",
        all_violations.join("\n")
    );
}

/// Rule 7: File size lint — no non-test `.rs` source file should exceed
/// ~300 lines of business logic.  This prevents "god files".
///
/// Known large files are temporarily allow-listed with tracking references
/// to the phases that will break them up.
#[test]
#[ignore]
fn file_size_lint() {
    const LIMIT: usize = 300;

    let offenders = files_exceeding_line_limit("src", LIMIT);
    let violations: Vec<String> = offenders
        .into_iter()
        .map(|(path, lines)| {
            let relative = path
                .strip_prefix("src/")
                .unwrap_or(&path)
                .display()
                .to_string();
            format!("  {} ({} lines)", relative, lines)
        })
        .collect();

    assert!(
        violations.is_empty(),
        "File-size lint: the following source files exceed {} non-test lines:\n{}\n\
         Split into smaller modules per AGENTS.md guidelines.",
        LIMIT,
        violations.join("\n")
    );
}
