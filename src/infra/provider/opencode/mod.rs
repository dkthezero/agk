use crate::app::ports::ProviderPort;
use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::identity::AssetIdentity;
use crate::domain::scope::Scope;
use crate::infra::provider::common;
use crate::infra::provider::common::{copy_dir, copy_dir_filtered};
use anyhow::Result;
use std::path::PathBuf;

pub mod config;
pub mod mcp;
pub mod session;
pub mod util;

pub struct OpenCodeProvider {
    workspace_root: PathBuf,
}

impl OpenCodeProvider {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
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
            copy_dir_filtered(&pkg.path, &dest, common::is_not_evals)?;
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

impl OpenCodeProvider {
    /// Remove a stale `"skills": [...]` array that earlier versions of agk
    /// wrote into opencode.json. OpenCode rejects this key, so we strip it.
    fn drop_stale_skills_array(&self, scope: &Scope) -> Result<()> {
        let path = self.config_path(scope);
        if !path.exists() {
            return Ok(());
        }
        let content = std::fs::read_to_string(&path)?;
        let cleaned = util::strip_jsonc_comments(&content);
        let mut config: serde_json::Value = match serde_json::from_str(&cleaned) {
            Ok(v) => v,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to parse opencode.json after stripping comments: {}",
                    e
                ));
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::ProfileRuntimePort;
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
        let cleaned = util::strip_jsonc_comments(input);
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
        let cleaned = util::strip_jsonc_comments(input);
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

    // -----------------------------------------------------------------------
    // ProfileRuntimePort tests
    // -----------------------------------------------------------------------
    #[test]
    fn profile_runtime_builds_launch_plan_with_skills() {
        let dir = tempfile::tempdir().unwrap();
        let provider = OpenCodeProvider::new(dir.path().to_path_buf());

        // Create profile agent markdown
        let profile_name = "dev";
        let agent_dir = dir.path().join(".agk").join("profiles").join(profile_name);
        std::fs::create_dir_all(&agent_dir).unwrap();
        std::fs::write(
            agent_dir.join("agent.md"),
            "---\nname: dev\n---\nTest agent",
        )
        .unwrap();

        let profile = crate::domain::profile::Profile {
            id: crate::domain::profile::ProfileId::new(profile_name),
            scope: Scope::Workspace,
            provider_id: crate::domain::profile::ProviderId::new("opencode"),
            skill_refs: vec![crate::domain::profile::SkillId::new("rust")],
            mcp_refs: vec![],
            instruction_refs: vec![],
            prompt_overlay_path: None,
            launch_policy: crate::domain::profile::LaunchPolicy::DryRun,
        };

        let plan = provider.build_launch_plan(&profile, None).unwrap();
        assert_eq!(plan.profile_id.as_str(), "dev");
        assert!(plan.patched_provider_config.is_some());

        let config = plan.patched_provider_config.unwrap();
        let permission = config.get("permission").unwrap();
        let skill_perm = permission.get("skill").unwrap();
        assert_eq!(skill_perm.get("rust").unwrap(), "allow");
        assert_eq!(skill_perm.get("*").unwrap(), "deny");
    }

    #[test]
    fn profile_runtime_auto_generates_agent_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let provider = OpenCodeProvider::new(dir.path().to_path_buf());

        let profile = crate::domain::profile::Profile {
            id: crate::domain::profile::ProfileId::new("nonexistent"),
            scope: Scope::Workspace,
            provider_id: crate::domain::profile::ProviderId::new("opencode"),
            skill_refs: vec![],
            mcp_refs: vec![],
            instruction_refs: vec![],
            prompt_overlay_path: None,
            launch_policy: crate::domain::profile::LaunchPolicy::DryRun,
        };

        let result = provider.build_launch_plan(&profile, None);
        assert!(result.is_ok(), "Should auto-generate agent.md when missing");
        let plan = result.unwrap();
        assert_eq!(plan.profile_id.as_str(), "nonexistent");
        assert!(plan.agent_markdown_source.exists());
    }
}
