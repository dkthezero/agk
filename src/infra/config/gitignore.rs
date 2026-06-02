use anyhow::Result;
use std::path::PathBuf;

/// Manages .agk/.gitignore to ensure config.toml is ignored by version control.
pub struct GitignoreManager;

impl GitignoreManager {
    /// Ensures `.agk/.gitignore` exists and contains the `config.toml` entry.
    /// Creates the file if missing, appends the entry if absent.
    /// Idempotent — calling multiple times produces the same result.
    pub fn ensure_config_gitignore(workspace_root: &std::path::Path) -> Result<()> {
        let gitignore_path = workspace_root.join(".agk").join(".gitignore");
        Self::ensure_entry(&gitignore_path, "config.toml")
    }

    /// Ensures the given entry exists in the gitignore file.
    /// Creates the file and parent directories if they don't exist.
    /// Does not duplicate the entry if it already exists.
    fn ensure_entry(gitignore_path: &PathBuf, entry: &str) -> Result<()> {
        if let Some(parent) = gitignore_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if !gitignore_path.exists() {
            std::fs::write(gitignore_path, format!("{}\n", entry))?;
            return Ok(());
        }

        let content = std::fs::read_to_string(gitignore_path)?;
        let lines: Vec<&str> = content.lines().collect();

        // Check if the entry already exists as its own line
        if lines.iter().any(|line| line.trim() == entry) {
            return Ok(());
        }

        // Append the entry, ensuring a trailing newline on existing content
        let mut new_content = content;
        if !new_content.ends_with('\n') && !new_content.is_empty() {
            new_content.push('\n');
        }
        new_content.push_str(entry);
        new_content.push('\n');
        std::fs::write(gitignore_path, new_content)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_gitignore_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        GitignoreManager::ensure_config_gitignore(dir.path()).unwrap();

        let gitignore = dir.path().join(".agk").join(".gitignore");
        assert!(gitignore.exists());
        let content = std::fs::read_to_string(gitignore).unwrap();
        assert_eq!(content, "config.toml\n");
    }

    #[test]
    fn does_not_duplicate_existing_entry() {
        let dir = tempfile::tempdir().unwrap();
        let agk_dir = dir.path().join(".agk");
        std::fs::create_dir_all(&agk_dir).unwrap();
        std::fs::write(agk_dir.join(".gitignore"), "config.toml\n").unwrap();

        GitignoreManager::ensure_config_gitignore(dir.path()).unwrap();

        let content = std::fs::read_to_string(agk_dir.join(".gitignore")).unwrap();
        assert_eq!(content, "config.toml\n");
    }

    #[test]
    fn appends_to_existing_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        let agk_dir = dir.path().join(".agk");
        std::fs::create_dir_all(&agk_dir).unwrap();
        std::fs::write(agk_dir.join(".gitignore"), "*.log\n").unwrap();

        GitignoreManager::ensure_config_gitignore(dir.path()).unwrap();

        let content = std::fs::read_to_string(agk_dir.join(".gitignore")).unwrap();
        assert!(content.contains("*.log"));
        assert!(content.contains("config.toml"));
    }

    #[test]
    fn idempotent_multiple_calls() {
        let dir = tempfile::tempdir().unwrap();

        GitignoreManager::ensure_config_gitignore(dir.path()).unwrap();
        GitignoreManager::ensure_config_gitignore(dir.path()).unwrap();
        GitignoreManager::ensure_config_gitignore(dir.path()).unwrap();

        let content = std::fs::read_to_string(dir.path().join(".agk").join(".gitignore")).unwrap();
        let count = content.matches("config.toml").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn creates_agk_directory_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!dir.path().join(".agk").exists());

        GitignoreManager::ensure_config_gitignore(dir.path()).unwrap();

        assert!(dir.path().join(".agk").exists());
        assert!(dir.path().join(".agk").join(".gitignore").exists());
    }

    #[test]
    fn handles_existing_file_without_trailing_newline() {
        let dir = tempfile::tempdir().unwrap();
        let agk_dir = dir.path().join(".agk");
        std::fs::create_dir_all(&agk_dir).unwrap();
        // Write without trailing newline
        std::fs::write(agk_dir.join(".gitignore"), "*.log").unwrap();

        GitignoreManager::ensure_config_gitignore(dir.path()).unwrap();

        let content = std::fs::read_to_string(agk_dir.join(".gitignore")).unwrap();
        assert!(content.contains("*.log\nconfig.toml\n"));
    }

    #[test]
    fn does_not_match_partial_entry() {
        let dir = tempfile::tempdir().unwrap();
        let agk_dir = dir.path().join(".agk");
        std::fs::create_dir_all(&agk_dir).unwrap();
        // "config.toml.bak" should NOT prevent adding "config.toml"
        std::fs::write(agk_dir.join(".gitignore"), "config.toml.bak\n").unwrap();

        GitignoreManager::ensure_config_gitignore(dir.path()).unwrap();

        let content = std::fs::read_to_string(agk_dir.join(".gitignore")).unwrap();
        assert!(content.contains("config.toml.bak"));
        assert!(content.contains("config.toml"));
        // Verify "config.toml" is on its own line
        let lines: Vec<&str> = content.lines().collect();
        assert!(lines.contains(&"config.toml"));
    }
}
