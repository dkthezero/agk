//! Legacy inherent `OpenCodeProvider::start_opencode_session` entry point.
//!
//! Preserves the previous (pre-`ProfileRuntimePort`) path that some callers
//! still use directly. Mostly duplicates the patch-config + spawn logic in
//! `session.rs`; consolidation is tracked separately. Split out of
//! `session.rs` to keep that file under the 300-LOC ADR-001 §6.4 limit.

use crate::app::ports::ProfileSession;
use crate::infra::provider::opencode::util::random_6_digits;
use crate::infra::provider::opencode::OpenCodeProvider;
use anyhow::{Context, Result};

impl OpenCodeProvider {
    /// Start an OpenCode session for a profile.
    pub fn start_opencode_session(
        &self,
        profile: &crate::domain::config::Profile,
        _session_key: &str,
    ) -> Result<ProfileSession> {
        super::config::validate_profile_name(&profile.name)?;

        let base_agent_path = self.profile_agent_path(&profile.name);

        // Auto-generate a minimal agent markdown if the file is missing.
        if !base_agent_path.exists() {
            if let Some(parent) = base_agent_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let content = format!(
                "# {}\n\nProfile agent for {}.\n",
                profile.name, profile.name
            );
            std::fs::write(&base_agent_path, content)?;
        }

        let agent_name = format!("{}{}", profile.name, random_6_digits());
        let agents_dir = self.workspace_root.join(".opencode").join("agents");
        let session_agent_path = agents_dir.join(format!("{}.md", agent_name));

        // 1. Read base agent markdown and patch frontmatter
        let mut agent_content = std::fs::read_to_string(&base_agent_path)?;
        agent_content = super::util::patch_agent_frontmatter(&agent_content, &agent_name);
        std::fs::create_dir_all(&agents_dir)?;
        std::fs::write(&session_agent_path, agent_content)?;

        // 2. Read workspace opencode.json with lossless restore capability
        let (mut config, _original_bytes) = self.read_workspace_config()?;
        let original_config = config.clone();

        // 3. Patch agent entry (per-agent overrides for skills and MCPs)
        if config.get("agent").is_none() {
            config["agent"] = serde_json::json!({});
        }
        if let Some(agent_obj) = config["agent"].as_object_mut() {
            let mut agent_entry = serde_json::json!({
                "mode": "primary",
                "description": format!("Session agent for {} profile", profile.name),
            });

            // Per-agent skill permissions
            let mut skill_perm = serde_json::json!({});
            for skill in &profile.skills {
                skill_perm[skill.clone()] = serde_json::json!("allow");
            }
            skill_perm["*"] = serde_json::json!("deny");
            agent_entry["permission"] = serde_json::json!({ "skill": skill_perm });

            // Per-agent MCP enablement
            let mut mcp_obj = serde_json::json!({});
            for mcp_name in &profile.mcps {
                mcp_obj[mcp_name.clone()] = serde_json::json!({ "enabled": true });
            }
            if mcp_obj.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
                agent_entry["mcp"] = mcp_obj;
            }

            agent_obj.insert(agent_name.clone(), agent_entry);
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
                // Roll back patches before returning error
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
        let agent_name_cleanup = agent_name.clone();

        let cleanup = Box::new(move || {
            // Remove session agent file
            let _ = std::fs::remove_file(&cleanup_path);

            // Surgical cleanup: remove only the agent entry from opencode.json
            let json_path = self_workspace.join("opencode.json");
            if json_path.exists() {
                let content = std::fs::read_to_string(&json_path).unwrap_or_default();
                let cleaned = super::util::strip_jsonc_comments(&content);
                if let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&cleaned) {
                    if let Some(obj) = config.as_object_mut() {
                        if let Some(agent_obj) =
                            obj.get_mut("agent").and_then(|v| v.as_object_mut())
                        {
                            agent_obj.remove(&agent_name_cleanup);
                            if agent_obj.is_empty() {
                                obj.remove("agent");
                            }
                        }
                        let _ = super::OpenCodeProvider::new(self_workspace.clone())
                            .write_workspace_config(&config);
                    }
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

        Ok(ProfileSession::new(process, cleanup))
    }
}
