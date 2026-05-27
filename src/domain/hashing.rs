use anyhow::Result;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub fn compute_sha10(files: &[PathBuf]) -> Result<String> {
    if files.is_empty() {
        return Ok("0000000000".to_string());
    }

    let mut sorted = files.to_vec();
    sorted.sort();

    let mut hasher = Sha256::new();
    for path in &sorted {
        let bytes = std::fs::read(path)?;
        let content = String::from_utf8_lossy(&bytes);
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

    fn write_temp_file(dir: &std::path::Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn sha10_is_ten_chars() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_temp_file(dir.path(), "SKILL.md", "hello world");
        let result = compute_sha10(&[f]).unwrap();
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn sha10_normalizes_crlf() {
        let dir = tempfile::tempdir().unwrap();
        let unix = write_temp_file(dir.path(), "unix.md", "hello\nworld");
        let windows = write_temp_file(dir.path(), "windows.md", "hello\r\nworld");
        let sha_unix = compute_sha10(&[unix]).unwrap();
        let sha_windows = compute_sha10(&[windows]).unwrap();
        assert_eq!(sha_unix, sha_windows);
    }

    #[test]
    fn sha10_empty_files_returns_fixed_value() {
        let result = compute_sha10(&[]).unwrap();
        assert_eq!(result, "0000000000");
    }

    #[test]
    fn sha10_is_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        let f = write_temp_file(dir.path(), "test.md", "deterministic content");
        let a = compute_sha10(std::slice::from_ref(&f)).unwrap();
        let b = compute_sha10(std::slice::from_ref(&f)).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn sha10_different_content_differs() {
        let dir = tempfile::tempdir().unwrap();
        let f1 = write_temp_file(dir.path(), "a.md", "content alpha");
        let f2 = write_temp_file(dir.path(), "b.md", "content beta");
        let sha_a = compute_sha10(&[f1]).unwrap();
        let sha_b = compute_sha10(&[f2]).unwrap();
        assert_ne!(sha_a, sha_b);
    }
}
