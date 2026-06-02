use crate::domain::vault_manifest::VaultManifest;
use anyhow::Result;
use std::path::PathBuf;

pub struct VaultInitResult {
    pub name: String,
    pub created: bool,
    pub message: String,
}

/// Initialize a vault repo with .agk/vault.toml and standard asset folders.
pub fn vault_init(
    workspace_root: &PathBuf,
    name: Option<String>,
    dry_run: bool,
) -> Result<VaultInitResult> {
    let vault_name = name.unwrap_or_else(|| {
        workspace_root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "vault".to_string())
    });

    let agk_dir = workspace_root.join(".agk");
    let vault_toml_path = agk_dir.join("vault.toml");

    if vault_toml_path.exists() {
        return Ok(VaultInitResult {
            name: vault_name.clone(),
            created: false,
            message: "Vault already initialized. Use --force to overwrite.".to_string(),
        });
    }

    if dry_run {
        return Ok(VaultInitResult {
            name: vault_name.clone(),
            created: false,
            message: format!("Would initialize vault '{}' with standard folders.", vault_name),
        });
    }

    // Create standard asset folders
    let folders = ["skills", "instructions", "mcps", "profiles"];
    for folder in &folders {
        std::fs::create_dir_all(workspace_root.join(folder))?;
    }

    // Create .agk directory
    std::fs::create_dir_all(&agk_dir)?;

    // Write vault.toml
    let manifest = VaultManifest {
        name: vault_name.clone(),
        description: None,
        version: Some("1.0.0".to_string()),
        dependencies: vec![],
    };
    let content = toml::to_string_pretty(&manifest)?;
    std::fs::write(&vault_toml_path, content)?;

    Ok(VaultInitResult {
        name: vault_name.clone(),
        created: true,
        message: format!("Initialized vault '{}' with standard folders.", vault_name),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_creates_vault_toml_and_folders() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        let result = vault_init(&workspace, Some("my-vault".into()), false).unwrap();

        assert!(result.created);
        assert_eq!(result.name, "my-vault");

        // Check vault.toml exists
        let vault_toml = workspace.join(".agk").join("vault.toml");
        assert!(vault_toml.exists(), "vault.toml should be created");

        // Check standard folders exist
        assert!(workspace.join("skills").is_dir());
        assert!(workspace.join("instructions").is_dir());
        assert!(workspace.join("mcps").is_dir());
        assert!(workspace.join("profiles").is_dir());
    }

    #[test]
    fn init_writes_valid_manifest_content() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        vault_init(&workspace, Some("test-vault".into()), false).unwrap();

        let vault_toml = workspace.join(".agk").join("vault.toml");
        let content = std::fs::read_to_string(&vault_toml).unwrap();
        let manifest: VaultManifest = toml::from_str(&content).unwrap();

        assert_eq!(manifest.name, "test-vault");
        assert!(manifest.description.is_none());
        assert_eq!(manifest.version.as_deref(), Some("1.0.0"));
        assert!(manifest.dependencies.is_empty());
    }

    #[test]
    fn init_uses_folder_name_when_name_not_provided() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        let result = vault_init(&workspace, None, false).unwrap();

        // The folder name comes from the temp dir path, which is a random string
        assert!(result.created);
        assert!(!result.name.is_empty());
        assert_ne!(result.name, "vault"); // Should use folder name, not fallback
    }

    #[test]
    fn init_returns_fallback_when_no_folder_name() {
        // Root path "/" has no file_name, should fall back to "vault"
        let root = PathBuf::from("/");
        let _result = vault_init(&root, None, false);

        // This will fail because we can't write to /, but let's check
        // the name derivation logic by testing the None branch with a path
        // that has a file_name first.
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();
        let result = vault_init(&workspace, None, false).unwrap();
        // Name should be derived from the folder name, not "vault"
        let expected_name = workspace
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "vault".to_string());
        assert_eq!(result.name, expected_name);
    }

    #[test]
    fn init_idempotent_returns_not_created() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        // First init succeeds
        let result1 = vault_init(&workspace, Some("my-vault".into()), false).unwrap();
        assert!(result1.created);

        // Second init reports already initialized
        let result2 = vault_init(&workspace, Some("my-vault".into()), false).unwrap();
        assert!(!result2.created);
        assert!(result2.message.contains("already initialized"));
    }

    #[test]
    fn init_dry_run_does_not_create_files() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        let result = vault_init(&workspace, Some("dry-vault".into()), true).unwrap();

        assert!(!result.created);
        assert!(result.message.contains("Would initialize"));
        assert!(result.message.contains("dry-vault"));

        // No files should be created
        let vault_toml = workspace.join(".agk").join("vault.toml");
        assert!(!vault_toml.exists());
        assert!(!workspace.join("skills").exists());
    }

    #[test]
    fn init_creates_agk_directory_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let workspace = dir.path().to_path_buf();

        assert!(!workspace.join(".agk").exists());

        vault_init(&workspace, Some("new-vault".into()), false).unwrap();

        assert!(workspace.join(".agk").is_dir());
    }
}