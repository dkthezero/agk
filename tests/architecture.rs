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
//!
//! All tests are marked `#[ignore]` so they do NOT run as part of the normal
//! test suite (`cargo test`). They are intended to be run explicitly in CI via
//! `cargo test -- --ignored`.
//!
//! This prevents regressions during refactors where logic accidentally leaks
//! across layer boundaries.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

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
        .chain(forbidden_refs(text, &["infra"]).into_iter())
        .collect()
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
