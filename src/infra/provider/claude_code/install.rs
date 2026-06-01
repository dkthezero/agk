use crate::app::ports::ProviderPort;
use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::identity::AssetIdentity;
use crate::domain::scope::Scope;
use crate::infra::provider::claude_code::ClaudeCodeProvider;
use crate::infra::provider::common;
use crate::infra::provider::common::copy_dir;
use std::path::PathBuf;

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
    ) -> anyhow::Result<()> {
        let dest = self.asset_dir(&scope, &pkg.kind, &pkg.identity.name, config);
        copy_dir(&pkg.path, &dest)
    }

    fn remove(
        &self,
        identity: &AssetIdentity,
        kind: &AssetKind,
        scope: Scope,
        config: Option<&crate::domain::config::ConfigFile>,
    ) -> anyhow::Result<()> {
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

    /// Bridge from the TUI's `Profile` config type to `ProfileRuntimePort`.
    ///
    /// `session_key` and `workspace_root` are currently unused because
    /// `ProfileRuntimePort::run_plan` already has access to `workspace_root`
    /// via `self.workspace_root`. They are kept in the trait signature for
    /// future use when providers may need an explicit session key or an
    /// overridden workspace root.
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
