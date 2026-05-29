use crate::app::event::LaunchPlan;
use crate::app::ports::{ProfileRuntimePort, ProfileSession};
use crate::domain::profile::Profile;
use crate::infra::provider::opencode::util::random_6_digits;
use crate::infra::provider::opencode::OpenCodeProvider;
use anyhow::{Context, Result};

impl ProfileRuntimePort for OpenCodeProvider {
    fn provider_id(&self) -> &str {
        "opencode"
    }

    fn build_launch_plan(
        &self,
        profile: &Profile,
        _config: Option<&crate::domain::config::ConfigFile>,
    ) -> Result<LaunchPlan> {
        let name = profile.id.as_str();
        let base_agent_path = self.profile_agent_path(name);

        // Auto-generate a minimal agent markdown if the file is missing.
        if !base_agent_path.exists() {
            if let Some(parent) = base_agent_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let content = format!("# {}\n\nProfile agent for {}.\n", name, name);
            std::fs::write(&base_agent_path, content)?;
        }

        let (current_config, original_bytes) = self.read_workspace_config()?;
        let mut plan_config = current_config.clone();

        // Patch agent entry
        if plan_config.get("agent").is_none() {
            plan_config["agent"] = serde_json::json!({});
        }
        if let Some(agent_obj) = plan_config["agent"].as_object_mut() {
            agent_obj.insert(
                format!("{}{}", name, random_6_digits()),
                serde_json::json!({
                    "mode": "primary",
                    "description": format!("Session agent for {} profile", name),
                }),
            );
        }

        // Patch permission -> skill
        if plan_config.get("permission").is_none() {
            plan_config["permission"] = serde_json::json!({});
        }
        if plan_config["permission"].get("skill").is_none() {
            plan_config["permission"]["skill"] = serde_json::json!({});
        }
        if let Some(skill_perm) = plan_config["permission"]["skill"].as_object_mut() {
            for skill in &profile.skill_refs {
                skill_perm.insert(skill.0.clone(), serde_json::json!("allow"));
            }
            skill_perm.insert("*".to_string(), serde_json::json!("deny"));
        }

        // Patch mcp entries
        for mcp in &profile.mcp_refs {
            if plan_config.get("mcp").is_none() {
                plan_config["mcp"] = serde_json::json!({});
            }
            if let Some(mcp_obj) = plan_config["mcp"].as_object_mut() {
                if let Some(entry) = mcp_obj.get_mut(&mcp.0) {
                    if let Some(e) = entry.as_object_mut() {
                        e.insert("enabled".to_string(), serde_json::json!(true));
                    }
                }
            }
        }

        Ok(LaunchPlan {
            profile_id: profile.id.clone(),
            provider_id: crate::domain::profile::ProviderId::new("opencode"),
            agent_markdown_source: base_agent_path,
            patched_provider_config: Some(plan_config),
            original_provider_config_bytes: original_bytes,
            ..LaunchPlan::default()
        })
    }

    fn run_plan(&self, plan: &LaunchPlan) -> Result<ProfileSession> {
        let agent_name = format!("{}{}", plan.profile_id.as_str(), random_6_digits());
        let agents_dir = self.workspace_root.join(".opencode").join("agents");
        let session_agent_path = agents_dir.join(format!("{}.md", agent_name));

        // 1. Copy base agent markdown to session agent with patched frontmatter
        let mut agent_content = std::fs::read_to_string(&plan.agent_markdown_source)?;
        agent_content = super::util::patch_agent_frontmatter(&agent_content, &agent_name);
        std::fs::create_dir_all(&agents_dir)?;
        std::fs::write(&session_agent_path, agent_content)?;

        // 2. Write patched opencode.json (or restore if plan fails)
        let _original_bytes = plan.original_provider_config_bytes.clone();
        if let Some(ref value) = plan.patched_provider_config {
            self.write_workspace_config(value)?;
        }

        // 3. Spawn opencode process
        let process = std::process::Command::new("opencode")
            .current_dir(&self.workspace_root)
            .arg("--agent")
            .arg(&agent_name)
            .spawn()
            .with_context(|| "Failed to start opencode CLI")?;

        let cleanup_path = session_agent_path.clone();
        let self_workspace = self.workspace_root.clone();

        let cleanup = Box::new(move || {
            // Remove session agent file
            let _ = std::fs::remove_file(&cleanup_path);

            // Surgical cleanup: remove only the agent entry from opencode.json
            // instead of wiping the whole file.
            let json_path = self_workspace.join("opencode.json");
            if json_path.exists() {
                let content = std::fs::read_to_string(&json_path).unwrap_or_default();
                let cleaned = super::util::strip_jsonc_comments(&content);
                if let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&cleaned) {
                    if let Some(obj) = config.as_object_mut() {
                        if let Some(agent_obj) =
                            obj.get_mut("agent").and_then(|v| v.as_object_mut())
                        {
                            agent_obj.remove(&agent_name);
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

// The legacy inherent `impl OpenCodeProvider { start_opencode_session(...) }`
// lives in `legacy_session.rs` so this file stays ≤300 LOC per ADR-001 §6.4.
