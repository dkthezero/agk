use crate::app::ports::{McpProvider, ProviderPort};
use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::identity::AssetIdentity;
use crate::domain::mcp::{McpServer, McpTransport};
use crate::domain::scope::Scope;
use crate::infra::provider::common;
use crate::infra::provider::common::{copy_dir, copy_dir_filtered};
use anyhow::{Context, Result};
use std::path::PathBuf;

pub struct OpenCodeProvider {
    workspace_root: PathBuf,
}

impl OpenCodeProvider {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    fn provider_root(
        &self,
        scope: &Scope,
        config: Option<&crate::domain::config::ConfigFile>,
    ) -> PathBuf {
        // provider_roots is workspace-only; global always uses hardcoded defaults
        match scope {
            Scope::Global => dirs_next::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
                .join("opencode"),
            Scope::Workspace => {
                let folder = config
                    .and_then(|c| c.provider_roots.get(self.id()))
                    .map(|s| s.as_str())
                    .unwrap_or(".opencode");
                self.workspace_root.join(folder)
            }
        }
    }

    fn asset_dir(
        &self,
        scope: &Scope,
        kind: &AssetKind,
        name: &str,
        config: Option<&crate::domain::config::ConfigFile>,
    ) -> PathBuf {
        let root = self.provider_root(scope, config);
        match kind {
            AssetKind::Skill => root.join("skills").join(name),
            AssetKind::Instruction => root.join("instructions").join(name),
            AssetKind::McpServer => PathBuf::new(),
        }
    }

    fn config_path(&self, scope: &Scope) -> PathBuf {
        match scope {
            Scope::Global => dirs_next::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
                .join("opencode")
                .join("opencode.json"),
            Scope::Workspace => self.workspace_root.join("opencode.json"),
        }
    }
}

impl ProviderPort for OpenCodeProvider {
    fn id(&self) -> &str {
        "opencode"
    }

    fn name(&self) -> &str {
        "OpenCode"
    }

    fn install_path_for(
        &self,
        identity: &AssetIdentity,
        kind: &AssetKind,
        scope: Scope,
    ) -> Option<PathBuf> {
        if *kind == AssetKind::McpServer {
            return None;
        }
        Some(self.asset_dir(&scope, kind, &identity.name, None))
    }

    fn install(
        &self,
        pkg: &ScannedPackage,
        scope: Scope,
        config: Option<&crate::domain::config::ConfigFile>,
        include_evals: bool,
    ) -> Result<()> {
        let dest = self.asset_dir(&scope, &pkg.kind, &pkg.identity.name, config);
        if include_evals {
            copy_dir(&pkg.path, &dest)?;
        } else {
            copy_dir_filtered(
                &pkg.path,
                &dest,
                crate::infra::provider::common::is_not_evals,
            )?;
        }

        // OpenCode does NOT accept a "skills" key in opencode.json.
        // Skills are auto-discovered from the .opencode/skills directory.
        // Self-heal: strip any stale "skills" array left by older agk versions
        // so users upgrading from the buggy build get a working config.
        self.drop_stale_skills_array(&scope)?;
        Ok(())
    }

    fn remove(
        &self,
        identity: &AssetIdentity,
        kind: &AssetKind,
        scope: Scope,
        config: Option<&crate::domain::config::ConfigFile>,
    ) -> Result<()> {
        let dest = self.asset_dir(&scope, kind, &identity.name, config);
        common::remove_dir_and_prune_empty_parents(&dest, 2)?;

        // Also remove any stale "skills" array that agk may have written in an
        // earlier version.  OpenCode rejects this key, so we quietly strip it.
        self.drop_stale_skills_array(&scope)?;
        Ok(())
    }

    fn available_config_roots(&self) -> Vec<(String, String)> {
        vec![
            (
                ".opencode".to_string(),
                "OpenCode native folder".to_string(),
            ),
            (
                ".agents".to_string(),
                "Shared agents folder (Claude-compatible)".to_string(),
            ),
        ]
    }
    fn supports_profiles(&self) -> bool {
        true
    }

    fn start_profile_session(
        &self,
        profile: &crate::domain::config::Profile,
        session_key: &str,
        _workspace_root: &std::path::Path,
    ) -> anyhow::Result<crate::app::ports::ProfileSession> {
        let session = self.start_opencode_session(profile, session_key)?;
        Ok(session)
    }

    fn profile_wizard_steps(&self) -> Vec<crate::app::ports::WizardStep> {
        use crate::app::ports::WizardStep;
        vec![
            WizardStep::TextInput {
                title: "Profile name".into(),
                placeholder: "e.g. opencode-dev".into(),
            },
            WizardStep::QuestionAnswer {
                question: "What is the primary task this agent should handle?".into(),
                placeholder: "e.g. Write Rust CLI tools".into(),
            },
            WizardStep::QuestionAnswer {
                question: "What tone or style should the agent use?".into(),
                placeholder: "e.g. Concise, professional".into(),
            },
            WizardStep::QuestionAnswer {
                question: "Are there any specific constraints or rules?".into(),
                placeholder: "e.g. Always run cargo fmt".into(),
            },
            WizardStep::Checklist {
                title: "Select Skills".into(),
                options: vec![],
            },
            WizardStep::Checklist {
                title: "Select MCP Servers".into(),
                options: vec![],
            },
            WizardStep::Review {
                title: "Review & Confirm".into(),
            },
        ]
    }
}
impl McpProvider for OpenCodeProvider {
    fn provider_id(&self) -> &str {
        "opencode"
    }

    fn supports_mcp(&self) -> bool {
        true
    }

    fn mcp_config_path(&self, scope: Scope) -> Option<PathBuf> {
        Some(self.config_path(&scope))
    }

    fn write_mcp_server(&self, server: &McpServer, scope: Scope) -> Result<()> {
        let path = self.config_path(&scope);
        let mut config: serde_json::Value = if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let cleaned = strip_jsonc_comments(&content);
            serde_json::from_str(&cleaned)?
        } else {
            serde_json::json!({})
        };

        if !config.is_object() {
            config = serde_json::json!({});
        }
        if config.get("mcp").is_none() {
            config["mcp"] = serde_json::json!({});
        }
        let mcp = config["mcp"]
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("opencode.json 'mcp' key is not an object"))?;

        // Migrate: drop old nested "servers" key if present so we don't leave a
        // stale empty object that OpenCode rejects.
        mcp.remove("servers");

        let entry = match &server.transport {
            McpTransport::Stdio => serde_json::json!({
                "type": "local",
                "command": server.command,
                "args": server.args,
                "env": server.env,
                "enabled": true,
            }),
            McpTransport::Sse { url } => serde_json::json!({
                "type": "remote",
                "url": url,
                "enabled": true,
            }),
        };
        mcp.insert(server.name.clone(), entry);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&config)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    fn remove_mcp_server(&self, name: &str, scope: Scope) -> Result<()> {
        let path = self.config_path(&scope);
        if !path.exists() {
            return Ok(());
        }
        let content = std::fs::read_to_string(&path)?;
        let cleaned = strip_jsonc_comments(&content);
        let mut config: serde_json::Value = serde_json::from_str(&cleaned)?;
        if let Some(mcp) = config.get_mut("mcp").and_then(|m| m.as_object_mut()) {
            mcp.remove(name);
            // If mcp is now empty, drop it entirely.
            if mcp.is_empty() {
                config.as_object_mut().unwrap().remove("mcp");
            }
        }
        let content = serde_json::to_string_pretty(&config)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Profile session helpers
// ---------------------------------------------------------------------------

impl OpenCodeProvider {
    /// Build the workspace path to the profile's base agent markdown.
    fn profile_agent_path(&self, profile_name: &str) -> PathBuf {
        self.workspace_root
            .join(".agk")
            .join("profiles")
            .join(profile_name)
            .join("agent.md")
    }

    /// Returns the workspace `opencode.json` path.
    fn workspace_opencode_json(&self) -> PathBuf {
        self.workspace_root.join("opencode.json")
    }

    /// Validate profile name is safe for filesystem use.
    fn validate_profile_name(name: &str) -> Result<()> {
        if name.is_empty()
            || name.contains('/')
            || name.contains('\\')
            || name.contains('\u{0000}')
            || name.contains(':')
            || name == "."
            || name == ".."
            || name.starts_with("..")
        {
            anyhow::bail!("Profile name contains invalid characters: {}", name);
        }
        Ok(())
    }

    /// Read workspace `opencode.json`.
    /// Returns (parsed value, original file bytes for lossless restore).
    /// Errors on parse failure or non-object root instead of silently defaulting.
    fn read_workspace_config(&self) -> Result<(serde_json::Value, Option<Vec<u8>>)> {
        let path = self.workspace_opencode_json();
        if !path.exists() {
            return Ok((serde_json::json!({}), None));
        }
        let content = std::fs::read_to_string(&path)?;
        let cleaned = strip_jsonc_comments(&content);
        let value: serde_json::Value = serde_json::from_str(&cleaned).with_context(|| {
            format!(
                "Failed to parse workspace opencode.json at {}",
                path.display()
            )
        })?;
        if !value.is_object() {
            anyhow::bail!(
                "Workspace opencode.json root must be an object, got {}",
                serde_json::to_string(&value).unwrap_or_else(|_| "non-serializable".into())
            );
        }
        // Preserve original bytes for lossless restore (preserves comments/formatting)
        let original_bytes = std::fs::read(&path).ok();
        Ok((value, original_bytes))
    }

    /// Write workspace `opencode.json` or delete it if empty.
    fn write_workspace_config(&self, value: &serde_json::Value) -> Result<()> {
        let path = self.workspace_opencode_json();
        if let Some(obj) = value.as_object() {
            if obj.is_empty() {
                if path.exists() {
                    std::fs::remove_file(&path)?;
                }
                return Ok(());
            }
        }
        let content = serde_json::to_string_pretty(value)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Start an OpenCode session for a profile.
    fn start_opencode_session(
        &self,
        profile: &crate::domain::config::Profile,
        session_key: &str,
    ) -> Result<crate::app::ports::ProfileSession> {
        Self::validate_profile_name(&profile.name)?;

        let base_agent_path = self.profile_agent_path(&profile.name);
        if !base_agent_path.exists() {
            anyhow::bail!(
                "Profile agent file not found at {}",
                base_agent_path.display()
            );
        }

        let agent_name = format!("{}_{}", profile.name, session_key);
        let agents_dir = self.workspace_root.join(".opencode").join("agents");
        let session_agent_path = agents_dir.join(format!("{}.md", agent_name));

        // 1. Read base agent markdown and patch frontmatter
        let mut agent_content = std::fs::read_to_string(&base_agent_path)?;
        agent_content = patch_agent_frontmatter(&agent_content, &agent_name);
        std::fs::create_dir_all(&agents_dir)?;
        std::fs::write(&session_agent_path, agent_content)?;

        // 2. Read workspace opencode.json with lossless restore capability
        let (mut config, original_bytes) = self.read_workspace_config()?;
        let original_config = config.clone();

        // 3. Patch `agent`
        if config.get("agent").is_none() {
            config["agent"] = serde_json::json!({});
        }
        if let Some(agent_obj) = config["agent"].as_object_mut() {
            agent_obj.insert(
                agent_name.clone(),
                serde_json::json!({
                    "mode": "primary",
                    "description": format!("Session agent for {} profile", profile.name),
                }),
            );
        }

        // 4. Patch `permission -> skill`
        if config.get("permission").is_none() {
            config["permission"] = serde_json::json!({});
        }
        if config["permission"].get("skill").is_none() {
            config["permission"]["skill"] = serde_json::json!({});
        }
        if let Some(skill_perm) = config["permission"]["skill"].as_object_mut() {
            for skill in &profile.skills {
                skill_perm.insert(skill.clone(), serde_json::json!("allow"));
            }
            skill_perm.insert("*".to_string(), serde_json::json!("deny"));
        }

        // 5. Patch `mcp` entries
        for mcp_name in &profile.mcps {
            if config.get("mcp").is_none() {
                config["mcp"] = serde_json::json!({});
            }
            if let Some(mcp_obj) = config["mcp"].as_object_mut() {
                if let Some(entry) = mcp_obj.get_mut(mcp_name) {
                    if let Some(e) = entry.as_object_mut() {
                        e.insert("enabled".to_string(), serde_json::json!(true));
                    }
                }
            }
        }

        self.write_workspace_config(&config)?;

        let process = match std::process::Command::new("opencode")
            .current_dir(&self.workspace_root)
            .arg("--agent")
            .arg(&agent_name)
            .spawn()
        {
            Ok(p) => p,
            Err(e) => {
                // Roll back patches before returning error (PRD #9)
                let _ = std::fs::remove_file(&session_agent_path);
                self.write_workspace_config(&original_config)?;
                let agents = self.workspace_root.join(".opencode").join("agents");
                if agents.exists() && agents.read_dir()?.next().is_none() {
                    let _ = std::fs::remove_dir(&agents);
                }
                let opencode = self.workspace_root.join(".opencode");
                if opencode.exists() && opencode.read_dir()?.next().is_none() {
                    let _ = std::fs::remove_dir(&opencode);
                }
                return Err(e).with_context(|| "Failed to start opencode CLI");
            }
        };

        let cleanup_path = session_agent_path;
        let self_workspace = self.workspace_root.clone();
        let cleanup_original = original_bytes;

        let cleanup = Box::new(move || {
            // Remove session agent file
            let _ = std::fs::remove_file(&cleanup_path);

            // Restore original opencode.json bytes (lossless: preserves comments/formatting)
            if let Some(bytes) = cleanup_original {
                let path = self_workspace.join("opencode.json");
                let _ = std::fs::write(&path, bytes);
            } else {
                // No original file existed; delete if present
                let path = self_workspace.join("opencode.json");
                if path.exists() {
                    let _ = std::fs::remove_file(&path);
                }
            }

            // Prune .opencode/agents if empty
            let agents = self_workspace.join(".opencode").join("agents");
            if agents.exists() && agents.read_dir()?.next().is_none() {
                let _ = std::fs::remove_dir(&agents);
            }

            // Prune .opencode if empty
            let opencode = self_workspace.join(".opencode");
            if opencode.exists() && opencode.read_dir()?.next().is_none() {
                let _ = std::fs::remove_dir(&opencode);
            }

            Ok(())
        });

        Ok(crate::app::ports::ProfileSession { process, cleanup })
    }
}

/// Patch frontmatter in an agent markdown to set the correct name and mode.
fn patch_agent_frontmatter(content: &str, agent_name: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut in_frontmatter = false;
    let mut frontmatter_started = false;
    let mut result = Vec::new();
    let mut name_set = false;
    let mut mode_set = false;

    for line in &lines {
        if *line == "---" {
            if !frontmatter_started {
                frontmatter_started = true;
                in_frontmatter = true;
                result.push(line.to_string());
                continue;
            } else if in_frontmatter {
                in_frontmatter = false;
                if !name_set {
                    result.push(format!("name: {}", agent_name));
                }
                if !mode_set {
                    result.push("mode: primary".to_string());
                }
                result.push(line.to_string());
                continue;
            }
        }

        if in_frontmatter {
            if line.starts_with("name:") {
                result.push(format!("name: {}", agent_name));
                name_set = true;
                continue;
            }
            if line.starts_with("mode:") {
                result.push("mode: primary".to_string());
                mode_set = true;
                continue;
            }
        }

        result.push(line.to_string());
    }

    // If no frontmatter at all, prepend one
    if !frontmatter_started {
        result.insert(0, "---".to_string());
        result.insert(1, format!("name: {}", agent_name));
        result.insert(2, "mode: primary".to_string());
        result.insert(3, "---".to_string());
    }

    result.join("\n") + "\n"
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

impl OpenCodeProvider {
    /// Remove a stale `"skills": [...]` array that earlier versions of agk
    /// wrote into opencode.json. OpenCode rejects this key, so we strip it.
    fn drop_stale_skills_array(&self, scope: &Scope) -> Result<()> {
        let path = self.config_path(scope);
        if !path.exists() {
            return Ok(());
        }
        let content = std::fs::read_to_string(&path)?;
        let cleaned = strip_jsonc_comments(&content);
        let mut config: serde_json::Value = match serde_json::from_str(&cleaned) {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };

        if let Some(obj) = config.as_object_mut() {
            if obj.remove("skills").is_some() {
                let content = serde_json::to_string_pretty(&config)?;
                std::fs::write(&path, content)?;
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// JSONC comment stripper (basic)
// ---------------------------------------------------------------------------

fn strip_jsonc_comments(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while let Some(ch) = chars.next() {
        if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
                result.push('\n');
            }
            continue;
        }

        if in_block_comment {
            if ch == '*' {
                if let Some(&'/') = chars.peek() {
                    chars.next();
                    in_block_comment = false;
                }
            }
            continue;
        }

        if in_string {
            result.push(ch);
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    result.push(next);
                }
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            result.push(ch);
            continue;
        }

        if ch == '/' {
            match chars.peek() {
                Some(&'/') => {
                    chars.next();
                    in_line_comment = true;
                    continue;
                }
                Some(&'*') => {
                    chars.next();
                    in_block_comment = true;
                    continue;
                }
                _ => {}
            }
        }

        result.push(ch);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::asset::AssetKind;
    use crate::domain::config::ConfigFile;

    fn make_pkg(
        dir: &std::path::Path,
        name: &str,
        kind: AssetKind,
        marker: &str,
    ) -> ScannedPackage {
        let pkg_dir = dir.join(name);
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(pkg_dir.join(marker), format!("# {}", name)).unwrap();
        ScannedPackage {
            identity: AssetIdentity::new(name, None, "0000000000"),
            path: pkg_dir,
            vault_id: "workspace".to_string(),
            kind,
            is_remote: false,
            remote_meta: None,
            requires: vec![],
            requires_optional: vec![],
            author: None,
            description: None,
            include_evals: false,
        }
    }

    #[test]
    fn install_skill_copies_to_workspace_opencode_skills() {
        let dir = tempfile::tempdir().unwrap();
        let src_dir = dir.path().join("source");
        std::fs::create_dir(&src_dir).unwrap();
        let pkg = make_pkg(&src_dir, "my-skill", AssetKind::Skill, "SKILL.md");
        let provider = OpenCodeProvider::new(dir.path().to_path_buf());
        provider
            .install(&pkg, Scope::Workspace, None, false)
            .unwrap();
        assert!(dir
            .path()
            .join(".opencode/skills/my-skill/SKILL.md")
            .exists());
    }

    #[test]
    fn install_instruction_copies_to_workspace_opencode_instructions() {
        let dir = tempfile::tempdir().unwrap();
        let src_dir = dir.path().join("source");
        std::fs::create_dir(&src_dir).unwrap();
        let pkg = make_pkg(&src_dir, "my-inst", AssetKind::Instruction, "AGENTS.md");
        let provider = OpenCodeProvider::new(dir.path().to_path_buf());
        provider
            .install(&pkg, Scope::Workspace, None, false)
            .unwrap();
        assert!(dir
            .path()
            .join(".opencode/instructions/my-inst/AGENTS.md")
            .exists());
    }

    #[test]
    fn install_does_not_add_skills_key() {
        let dir = tempfile::tempdir().unwrap();
        let src_dir = dir.path().join("source");
        std::fs::create_dir(&src_dir).unwrap();
        let pkg = make_pkg(&src_dir, "my-skill", AssetKind::Skill, "SKILL.md");
        let provider = OpenCodeProvider::new(dir.path().to_path_buf());
        provider
            .install(&pkg, Scope::Workspace, None, false)
            .unwrap();

        let config_path = dir.path().join("opencode.json");
        assert!(!config_path.exists());
    }

    #[test]
    fn remove_skill_deletes_directory_and_drops_stale_skills_key() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join(".opencode/skills/my-skill");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("SKILL.md"), "x").unwrap();

        // Pre-populate config with a stale skills array (old agk output)
        let config_path = dir.path().join("opencode.json");
        std::fs::write(
            &config_path,
            r#"{"skills":[{"name":"my-skill","path":".opencode/skills/my-skill"}]}"#,
        )
        .unwrap();

        let provider = OpenCodeProvider::new(dir.path().to_path_buf());
        let identity = AssetIdentity::new("my-skill", None, "0000000000");
        provider
            .remove(&identity, &AssetKind::Skill, Scope::Workspace, None)
            .unwrap();
        assert!(!dest.exists());

        let content = std::fs::read_to_string(config_path).unwrap();
        assert!(!content.contains("my-skill"));
        assert!(!content.contains("skills"));
    }

    #[test]
    fn remove_nonexistent_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        let provider = OpenCodeProvider::new(dir.path().to_path_buf());
        let identity = AssetIdentity::new("ghost", None, "0000000000");
        let result = provider.remove(&identity, &AssetKind::Skill, Scope::Workspace, None);
        assert!(result.is_ok());
    }

    #[test]
    fn strip_jsonc_line_comments() {
        let input = r#"{
            // This is a comment
            "key": "value"
        }"#;
        let cleaned = strip_jsonc_comments(input);
        assert!(!cleaned.contains("// This is a comment"));
        assert!(cleaned.contains("\"key\": \"value\""));
    }

    #[test]
    fn strip_jsonc_block_comments() {
        let input = r#"{
            /* This is a
               block comment */
            "key": "value"
        }"#;
        let cleaned = strip_jsonc_comments(input);
        assert!(!cleaned.contains("/* This is a"));
        assert!(cleaned.contains("\"key\": \"value\""));
    }

    #[test]
    fn opencode_provider_root_uses_config_override() {
        let dir = tempfile::tempdir().unwrap();
        let provider = OpenCodeProvider::new(dir.path().to_path_buf());
        let mut config = ConfigFile::default();
        config
            .provider_roots
            .insert("opencode".to_string(), ".agents".to_string());
        let root = provider.provider_root(&Scope::Workspace, Some(&config));
        assert_eq!(root, dir.path().join(".agents"));
    }

    #[test]
    fn install_heals_stale_skills_key() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("opencode.json");
        std::fs::write(
            &config_path,
            r#"{"customKey": "customValue", "skills": []}"#,
        )
        .unwrap();

        let src_dir = dir.path().join("source");
        std::fs::create_dir(&src_dir).unwrap();
        let pkg = make_pkg(&src_dir, "my-skill", AssetKind::Skill, "SKILL.md");
        let provider = OpenCodeProvider::new(dir.path().to_path_buf());
        provider
            .install(&pkg, Scope::Workspace, None, false)
            .unwrap();

        let content = std::fs::read_to_string(config_path).unwrap();
        assert!(content.contains("customKey"));
        assert!(!content.contains("skills"));
        assert!(!content.contains(".opencode/skills/my-skill"));
    }

    #[test]
    fn opencode_install_uses_agents_when_configured() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = ConfigFile::default();
        config
            .provider_roots
            .insert("opencode".to_string(), ".agents".to_string());

        let src_dir = dir.path().join("source");
        std::fs::create_dir(&src_dir).unwrap();
        let pkg = make_pkg(&src_dir, "my-skill", AssetKind::Skill, "SKILL.md");
        let provider = OpenCodeProvider::new(dir.path().to_path_buf());
        provider
            .install(&pkg, Scope::Workspace, Some(&config), false)
            .unwrap();

        // Should be in .agents, not .opencode
        assert!(dir.path().join(".agents/skills/my-skill/SKILL.md").exists());
        assert!(!dir
            .path()
            .join(".opencode/skills/my-skill/SKILL.md")
            .exists());
    }

    #[test]
    fn opencode_and_claude_share_agents_folder() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = ConfigFile::default();
        config
            .provider_roots
            .insert("opencode".to_string(), ".agents".to_string());
        config
            .provider_roots
            .insert("claude-code".to_string(), ".agents".to_string());

        let opencode = OpenCodeProvider::new(dir.path().to_path_buf());
        let claude =
            crate::infra::provider::claude_code::ClaudeCodeProvider::new(dir.path().to_path_buf());

        assert_eq!(
            opencode.provider_root(&Scope::Workspace, Some(&config)),
            dir.path().join(".agents")
        );
        assert_eq!(
            claude.provider_root(&Scope::Workspace, Some(&config)),
            dir.path().join(".agents")
        );
    }
}
