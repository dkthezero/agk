use crate::app::ports::ProviderPort;
use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::identity::AssetIdentity;
use crate::domain::scope::Scope;
use crate::infra::provider::common;
use crate::infra::provider::common::copy_dir;
use anyhow::Result;
use std::path::PathBuf;

pub mod mcp;
pub mod session;

pub struct ClaudeCodeProvider {
    pub(crate) workspace_root: PathBuf,
}

impl ClaudeCodeProvider {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self { workspace_root }
    }

    pub(crate) fn provider_root(
        &self,
        scope: &Scope,
        config: Option<&crate::domain::config::ConfigFile>,
    ) -> PathBuf {
        // provider_roots is workspace-only; global always uses the hardcoded default
        match scope {
            Scope::Global => dirs_next::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".claude"),
            Scope::Workspace => {
                let folder = config
                    .and_then(|c| c.provider_roots.get(self.id()))
                    .map(|s| s.as_str())
                    .unwrap_or(".claude");
                self.workspace_root.join(folder)
            }
        }
    }

    pub(crate) fn profile_agent_path(&self, profile_name: &str) -> PathBuf {
        // Claude Code stores agent markdown at .claude/agents/<name>.md
        self.workspace_root
            .join(".claude")
            .join("agents")
            .join(format!("{}.md", profile_name))
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
            AssetKind::Profile => PathBuf::new(),
        }
    }

    fn mcp_json_path(&self, scope: &Scope) -> PathBuf {
        self.provider_root(scope, None).join("mcp.json")
    }

    fn load_mcp_config(&self, scope: &Scope) -> Result<serde_json::Value> {
        let path = self.mcp_json_path(scope);
        if !path.exists() {
            return Ok(serde_json::json!({ "mcpServers": {} }));
        }
        let content = std::fs::read_to_string(&path)?;
        let config: serde_json::Value = serde_json::from_str(&content)?;
        Ok(config)
    }

    fn save_mcp_config(&self, scope: &Scope, config: &serde_json::Value) -> Result<()> {
        let path = self.mcp_json_path(scope);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(config)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}

impl ProviderPort for ClaudeCodeProvider {
    fn id(&self) -> &str {
        "claude-code"
    }

    fn name(&self) -> &str {
        "Claude Code"
    }

    fn install(
        &self,
        pkg: &ScannedPackage,
        scope: Scope,
        config: Option<&crate::domain::config::ConfigFile>,
        _include_evals: bool,
    ) -> Result<()> {
        let dest = self.asset_dir(&scope, &pkg.kind, &pkg.identity.name, config);
        copy_dir(&pkg.path, &dest)
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
        Ok(())
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

    fn available_config_roots(&self) -> Vec<(String, String)> {
        vec![
            (
                ".claude".to_string(),
                "Claude Code native folder".to_string(),
            ),
            (".agents".to_string(), "Shared agents folder".to_string()),
        ]
    }

    fn supports_profiles(&self) -> bool {
        true
    }

    fn profile_wizard_steps(&self) -> Vec<crate::app::ports::WizardStep> {
        use crate::app::ports::WizardStep;
        vec![
            WizardStep::TemplateSelect {
                title: "Select Archetype".into(),
                templates: crate::app::features::profile::template::TEMPLATES.to_vec(),
            },
            WizardStep::TextInput {
                title: "Profile name".into(),
                placeholder: "e.g. claude-dev".into(),
            },
            WizardStep::ScopeSelect {
                title: "Select Scope".into(),
            },
            WizardStep::Textarea {
                title: "Role".into(),
                placeholder: "e.g. Senior Rust engineer".into(),
                rows: 2,
                key: "role".into(),
            },
            WizardStep::Textarea {
                title: "Domain / Specialty".into(),
                placeholder: "e.g. async CLI tooling".into(),
                rows: 2,
                key: "domain".into(),
            },
            WizardStep::Textarea {
                title: "Collaboration Style".into(),
                placeholder: "e.g. Direct and thorough".into(),
                rows: 3,
                key: "style".into(),
            },
            WizardStep::Textarea {
                title: "Scope Boundaries".into(),
                placeholder: "IN SCOPE:\n...\n\nOUT OF SCOPE:\n...".into(),
                rows: 5,
                key: "boundaries".into(),
            },
            WizardStep::Textarea {
                title: "Activation Triggers".into(),
                placeholder: "e.g. After code changes, on explicit request".into(),
                rows: 3,
                key: "triggers".into(),
            },
            WizardStep::Textarea {
                title: "Constraints".into(),
                placeholder: "e.g. Always run cargo fmt before finishing".into(),
                rows: 3,
                key: "constraints".into(),
            },
            WizardStep::Textarea {
                title: "Output Format".into(),
                placeholder: "e.g. Concise bullets, max 5 items".into(),
                rows: 2,
                key: "format".into(),
            },
            WizardStep::Textarea {
                title: "Core Responsibilities".into(),
                placeholder: "e.g. Review PRs, suggest idioms, catch regressions".into(),
                rows: 4,
                key: "responsibilities".into(),
            },
            WizardStep::ToolSelect {
                title: "Select Tools".into(),
                tools: self
                    .available_profile_tools()
                    .into_iter()
                    .map(|t| (t.clone(), t, false))
                    .collect(),
            },
            WizardStep::PermissionSelect {
                title: "Select Permission Mode".into(),
                modes: self.available_permission_modes(),
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

    fn available_profile_tools(&self) -> Vec<String> {
        vec![
            "Read".into(),
            "Glob".into(),
            "Grep".into(),
            "Bash".into(),
            "Write".into(),
            "Edit".into(),
            "LSP".into(),
        ]
    }

    fn available_permission_modes(&self) -> Vec<(String, String)> {
        vec![
            ("default".into(), "Ask for confirmation on edits".into()),
            ("acceptEdits".into(), "Accept edits automatically".into()),
            ("auto".into(), "Auto-approve safe operations".into()),
            ("dontAsk".into(), "Never ask for confirmation".into()),
            ("plan".into(), "Plan mode — suggest only".into()),
        ]
    }

    fn supports_mcp(&self) -> bool {
        true
    }

    fn start_profile_session(
        &self,
        profile: &crate::domain::config::Profile,
        session_key: &str,
        workspace_root: &std::path::Path,
    ) -> anyhow::Result<crate::app::ports::ProfileSession> {
        use crate::app::ports::ProfileRuntimePort;
        let domain_profile = crate::domain::profile::Profile {
            id: crate::domain::profile::ProfileId::new(&profile.name),
            scope: if profile.scope == "global" {
                Scope::Global
            } else {
                Scope::Workspace
            },
            provider_id: crate::domain::profile::ProviderId::new(&profile.provider_id),
            skill_refs: profile.skills.clone(),
            mcp_refs: profile.mcps.clone(),
            instruction_refs: profile.instructions.clone(),
            tool_refs: profile.tool_refs.clone(),
            permission_mode: profile.permission_mode.clone(),
            prompt_overlay_path: profile
                .prompt_overlay_path
                .as_ref()
                .map(std::path::PathBuf::from),
            launch_policy: crate::domain::profile::LaunchPolicy::AutoRestore,
        };
        let plan = self.build_launch_plan(&domain_profile, None)?;
        let _ = (session_key, workspace_root);
        self.run_plan(&plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::asset::AssetKind;
    use std::path::Path;

    fn make_pkg(dir: &Path, name: &str, kind: AssetKind, marker: &str) -> ScannedPackage {
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
    fn install_skill_copies_to_workspace_claude_skills() {
        let dir = tempfile::tempdir().unwrap();
        let src_dir = dir.path().join("source");
        std::fs::create_dir(&src_dir).unwrap();
        let pkg = make_pkg(&src_dir, "my-skill", AssetKind::Skill, "SKILL.md");
        let provider = ClaudeCodeProvider::new(dir.path().to_path_buf());
        provider
            .install(&pkg, Scope::Workspace, None, false)
            .unwrap();
        assert!(dir.path().join(".claude/skills/my-skill/SKILL.md").exists());
    }

    #[test]
    fn install_instruction_copies_to_workspace_claude_instructions() {
        let dir = tempfile::tempdir().unwrap();
        let src_dir = dir.path().join("source");
        std::fs::create_dir(&src_dir).unwrap();
        let pkg = make_pkg(&src_dir, "my-inst", AssetKind::Instruction, "AGENTS.md");
        let provider = ClaudeCodeProvider::new(dir.path().to_path_buf());
        provider
            .install(&pkg, Scope::Workspace, None, false)
            .unwrap();
        assert!(dir
            .path()
            .join(".claude/instructions/my-inst/AGENTS.md")
            .exists());
    }

    #[test]
    fn remove_skill_deletes_directory() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join(".claude/skills/my-skill");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("SKILL.md"), "x").unwrap();
        let provider = ClaudeCodeProvider::new(dir.path().to_path_buf());
        let identity = AssetIdentity::new("my-skill", None, "0000000000");
        provider
            .remove(&identity, &AssetKind::Skill, Scope::Workspace, None)
            .unwrap();
        assert!(!dest.exists());
    }

    #[test]
    fn remove_nonexistent_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        let provider = ClaudeCodeProvider::new(dir.path().to_path_buf());
        let identity = AssetIdentity::new("ghost", None, "0000000000");
        let result = provider.remove(&identity, &AssetKind::Skill, Scope::Workspace, None);
        assert!(result.is_ok());
    }

    #[test]
    fn claude_provider_root_uses_config_override() {
        let dir = tempfile::tempdir().unwrap();
        let provider = ClaudeCodeProvider::new(dir.path().to_path_buf());
        let mut config = crate::domain::config::ConfigFile::default();
        config
            .provider_roots
            .insert("claude-code".to_string(), ".agents".to_string());
        let root = provider.provider_root(&Scope::Workspace, Some(&config));
        assert_eq!(root, dir.path().join(".agents"));
    }

    #[test]
    fn claude_install_uses_agents_when_configured() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = crate::domain::config::ConfigFile::default();
        config
            .provider_roots
            .insert("claude-code".to_string(), ".agents".to_string());

        let src_dir = dir.path().join("source");
        std::fs::create_dir(&src_dir).unwrap();
        let pkg = make_pkg(&src_dir, "my-skill", AssetKind::Skill, "SKILL.md");
        let provider = ClaudeCodeProvider::new(dir.path().to_path_buf());
        provider
            .install(&pkg, Scope::Workspace, Some(&config), false)
            .unwrap();

        assert!(dir.path().join(".agents/skills/my-skill/SKILL.md").exists());
        assert!(!dir.path().join(".claude/skills/my-skill/SKILL.md").exists());
    }
}
