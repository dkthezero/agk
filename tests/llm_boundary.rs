//! Architecture-enforcement test for the LLM adapter boundary.
//!
//! The LLM adapter implementations under `src/infra/llm/` are leaf modules.
//! They are not part of the public surface for `app/`, `tui/`, `cli/`, or
//! `domain/` — those layers must depend only on the `LlmProviderPort` /
//! `LlmProviderStorePort` traits in `app::ports::llm_provider` (and the
//! `LlmProviderFactoryPort` for wiring).
//!
//! This test fails the build if any `.rs` file outside `src/infra/llm/`
//! contains a `use crate::infra::llm::` import. That guarantees:
//!   * Wiring code in `cli/` or `app/` references the factory port, not
//!     a concrete adapter.
//!   * `tui/` cannot accidentally call into an adapter directly.
//!   * `domain/` cannot reach into LLM infrastructure at all.
//!
//! The grep is intentionally substring-based and looks for
//! `crate::infra::llm::` — a stable, easy-to-recognise signature that
//! covers all `use` statements and inline path references.
//!
//! The single carve-out is `src/cli/core_dispatcher/llm.rs` — the
//! composition root for the `agk llm` subcommand.  It is the only place
//! outside `infra/llm/` that legitimately constructs concrete adapters
//! and passes them as trait objects into the use-case layer.  Treat the
//! allowlist as a small, named exception: a follow-up that re-exports a
//! `build_*` factory from `app::ports::llm_provider` would let us delete
//! the allowlist entirely.

use std::fs;
use std::path::{Path, PathBuf};

/// Walk `dir` recursively and collect every `.rs` file path.
fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// The boundary we are enforcing: any file whose path starts with this
/// directory is allowed to reference `crate::infra::llm::*`.
fn infra_llm_dir(manifest: &Path) -> PathBuf {
    manifest.join("src").join("infra").join("llm")
}

/// The single composition-root carve-out.  Centralised here so the rule
/// is documented in one place and a reviewer can audit the exception
/// without scanning the test body.
fn is_composition_root(path: &Path) -> bool {
    let rel = path.to_string_lossy();
    rel.ends_with("/src/cli/core_dispatcher/llm.rs")
        || rel.ends_with("src\\cli\\core_dispatcher\\llm.rs")
}

#[test]
fn llm_adapters_not_imported_outside_infra_llm() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.join("src");
    assert!(src.is_dir(), "src/ not found at {}", src.display());

    let allowed = infra_llm_dir(manifest);
    assert!(
        allowed.is_dir(),
        "infra/llm/ not found at {} — has the LLM module moved?",
        allowed.display()
    );

    let mut all_rs = Vec::new();
    collect_rs_files(&src, &mut all_rs);

    let mut violations = Vec::new();
    for path in &all_rs {
        // Allow references from inside the boundary itself.
        if path.starts_with(&allowed) {
            continue;
        }
        // Allow the single composition-root carve-out.
        if is_composition_root(path) {
            continue;
        }
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        // Match the canonical import signature: `crate::infra::llm::`.
        // We skip doc-comment lines so the explanatory references in
        // `app/ports/llm_provider.rs` do not count as violations.
        for (idx, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("*") {
                continue;
            }
            if trimmed.contains("crate::infra::llm::") {
                let rel = path
                    .strip_prefix(manifest)
                    .unwrap_or(path)
                    .display()
                    .to_string();
                violations.push(format!("  {}:{}: {}", rel, idx + 1, trimmed));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Architecture violation: LLM adapters in `src/infra/llm/` must not be \
         imported outside the boundary. The domain / app / tui / cli layers must \
         depend only on the trait ports in `app::ports::llm_provider` \
         (LlmProviderPort, LlmProviderStorePort, LlmProviderFactoryPort).\n\
         Violations:\n{}",
        violations.join("\n")
    );
}
