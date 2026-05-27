use crate::app::event::LaunchPlan;
use crate::app::ports::{ProfileRuntimePort, ProfileSession};
use crate::domain::profile::Profile;
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
        // This can happen when a profile was created via the config-only
        // headless path (e.g. Phase-B fallback) before agent generation was wired.
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
                name.to_string(),
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
        let session_key = format!(
            "{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let agent_name = format!("{}_{}", plan.profile_id.as_str(), session_key);
        let agents_dir = self.workspace_root.join(".opencode").join("agents");
        let session_agent_path = agents_dir.join(format!("{}.md", agent_name));

        // 1. Write patched agent markdown
        let mut agent_content = std::fs::read_to_string(&plan.agent_markdown_source)?;
        agent_content = super::util::patch_agent_frontmatter(&agent_content, &agent_name);
        std::fs::create_dir_all(&agents_dir)?;
        std::fs::write(&session_agent_path, agent_content)?;

        // 2. Write patched opencode.json (or restore if plan fails)
        let original_bytes = plan.original_provider_config_bytes.clone();
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
        let cleanup_original = original_bytes;

        let cleanup = Box::new(move || {
            let _ = std::fs::remove_file(&cleanup_path);
            if let Some(bytes) = cleanup_original {
                let path = self_workspace.join("opencode.json");
                let _ = std::fs::write(&path, bytes);
            } else {
                let path = self_workspace.join("opencode.json");
                if path.exists() {
                    let _ = std::fs::remove_file(&path);
                }
            }
            let agents = self_workspace.join(".opencode").join("agents");
            if agents.exists() && agents.read_dir()?.next().is_none() {
                let _ = std::fs::remove_dir(&agents);
            }
            let opencode = self_workspace.join(".opencode");
            if opencode.exists() && opencode.read_dir()?.next().is_none() {
                let _ = std::fs::remove_dir(&opencode);
            }
            Ok(())
        });

        Ok(ProfileSession::new(process, cleanup))
    }
}

impl OpenCodeProvider {
    /// Start an OpenCode session for a profile.
    pub fn start_opencode_session(
        &self,
        profile: &crate::domain::config::Profile,
        session_key: &str,
    ) -> Result<ProfileSession> {
        super::config::validate_profile_name(&profile.name)?;

        let base_agent_path = self.profile_agent_path(&profile.name);

        // Auto-generate a minimal agent markdown if the file is missing.
        // This can happen when a profile was created via the config-only
        // headless path before agent generation was wired.
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

        let agent_name = format!("{}_{}", profile.name, session_key);
        let agents_dir = self.workspace_root.join(".opencode").join("agents");
        let session_agent_path = agents_dir.join(format!("{}.md", agent_name));

        // 1. Read base agent markdown and patch frontmatter
        let mut agent_content = std::fs::read_to_string(&base_agent_path)?;
        agent_content = super::util::patch_agent_frontmatter(&agent_content, &agent_name);
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

        Ok(ProfileSession::new(process, cleanup))
    }
}
