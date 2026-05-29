use anyhow::Result;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// Compute a 10-char SHA hash over the *contents* of the given files.
///
/// The path is used purely for deterministic ordering — only the bytes
/// contribute to the digest. Callers in `app/features/` and `infra/` perform
/// the `std::fs::read` so this function stays pure (ADR-001 Commit 1).
pub fn compute_sha10(files: &[(PathBuf, Vec<u8>)]) -> Result<String> {
    if files.is_empty() {
        return Ok("0000000000".to_string());
    }

    let mut sorted: Vec<&(PathBuf, Vec<u8>)> = files.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    let mut hasher = Sha256::new();
    for (_path, bytes) in &sorted {
        let content = String::from_utf8_lossy(bytes);
        let normalized = content.replace("\r\n", "\n");
        hasher.update(normalized.as_bytes());
    }

    let digest = hasher.finalize();
    let hex_str = hex::encode(digest);
    Ok(hex_str[..10].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, content: &str) -> (PathBuf, Vec<u8>) {
        (PathBuf::from(name), content.as_bytes().to_vec())
    }

    #[test]
    fn sha10_is_ten_chars() {
        let result = compute_sha10(&[entry("SKILL.md", "hello world")]).unwrap();
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn sha10_normalizes_crlf() {
        let sha_unix = compute_sha10(&[entry("a.md", "hello\nworld")]).unwrap();
        let sha_windows = compute_sha10(&[entry("a.md", "hello\r\nworld")]).unwrap();
        assert_eq!(sha_unix, sha_windows);
    }

    #[test]
    fn sha10_empty_files_returns_fixed_value() {
        let result = compute_sha10(&[]).unwrap();
        assert_eq!(result, "0000000000");
    }

    #[test]
    fn sha10_is_deterministic() {
        let one = [entry("test.md", "deterministic content")];
        let a = compute_sha10(&one).unwrap();
        let b = compute_sha10(&one).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn sha10_different_content_differs() {
        let sha_a = compute_sha10(&[entry("a.md", "content alpha")]).unwrap();
        let sha_b = compute_sha10(&[entry("b.md", "content beta")]).unwrap();
        assert_ne!(sha_a, sha_b);
    }

    #[test]
    fn sha10_ordering_is_path_dependent() {
        // Identical file *bodies* in different path orderings still hash to the
        // same value (paths only drive the sort).
        let a = compute_sha10(&[entry("a.md", "x"), entry("b.md", "y")]).unwrap();
        let b = compute_sha10(&[entry("b.md", "y"), entry("a.md", "x")]).unwrap();
        assert_eq!(a, b);
    }
}
