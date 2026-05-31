//! Claude Code `ProfileRuntimePort` implementation.
//!
//! Generates an `agent.md` file with YAML frontmatter containing profile
//! metadata (role, tools, permission mode, MCPs) and spawns the CLI.

use crate::app::event::LaunchPlan;
use crate::app::ports::{ProfileRuntimePort, ProfileSession};
use crate::domain::profile::Profile;
use crate::domain::scope::Scope;
use crate::infra::provider::claude_code::ClaudeCodeProvider;
use anyhow::{Context, Result};

impl ProfileRuntimePort for ClaudeCodeProvider {
    fn provider_id(&self) -> &str {
        "claude-code"
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
            let content = compose_agent_markdown(profile);
            std::fs::write(&base_agent_path, content)?;
        }

        Ok(LaunchPlan {
            profile_id: profile.id.clone(),
            provider_id: crate::domain::profile::ProviderId::new("claude-code"),
            agent_markdown_source: base_agent_path,
            patched_provider_config: None,
            original_provider_config_bytes: None,
            session_agent_name: None,
            tool_refs: profile.tool_refs.clone(),
            permission_mode: profile.permission_mode.clone(),
            ..LaunchPlan::default()
        })
    }

    fn run_plan(&self, plan: &LaunchPlan) -> Result<ProfileSession> {
        let agent_path = &plan.agent_markdown_source;

        // Ensure the agent file exists (may have been written by build_launch_plan
        // or by a previous session).
        if !agent_path.exists() {
            anyhow::bail!("Agent file not found: {}", agent_path.display());
        }

        // Spawn claude CLI with the agent file.
        let agent_path_str = agent_path.display().to_string();
        let process = std::process::Command::new("claude")
            .current_dir(&self.workspace_root)
            .arg("--agent")
            .arg(&agent_path_str)
            .spawn()
            .with_context(|| "Failed to start claude CLI")?;

        // No provider config to patch/restore — Claude Code reads agent.md directly.
        let self_workspace = self.workspace_root.clone();
        let cleanup = Box::new(move || -> Result<()> {
            // Prune .claude/agents if empty
            let agents = self_workspace.join(".claude").join("agents");
            if agents.exists() {
                let is_empty = agents.read_dir()?.next().is_none();
                if is_empty {
                    let _ = std::fs::remove_dir(&agents);
                }
            }
            Ok(())
        });

        Ok(ProfileSession::new(process, cleanup))
    }
}

/// Compose an `agent.md` file for Claude Code from profile fields.
///
/// The file uses YAML frontmatter for structured metadata and a Markdown
/// body for the natural-language prompt.  Claude Code reads this file when
/// launched with `--agent <path>`.
pub fn compose_agent_markdown(profile: &Profile) -> String {
    let mut frontmatter = String::from("---\n");

    // Core identity — values are YAML-escaped to handle special characters.
    frontmatter.push_str(&format!("name: {}\n", yaml_escape(profile.id.as_str())));
    frontmatter.push_str(&format!(
        "provider: {}\n",
        yaml_escape(profile.provider_id.as_str())
    ));
    if !profile.tool_refs.is_empty() {
        frontmatter.push_str("tools:\n");
        for tool in &profile.tool_refs {
            frontmatter.push_str(&format!("  - {}\n", yaml_escape(tool)));
        }
    }
    if let Some(ref mode) = profile.permission_mode {
        frontmatter.push_str(&format!("permission_mode: {}\n", yaml_escape(mode)));
    }
    if !profile.skill_refs.is_empty() {
        frontmatter.push_str("skills:\n");
        for skill in &profile.skill_refs {
            frontmatter.push_str(&format!("  - {}\n", yaml_escape(&skill.name)));
        }
    }
    if !profile.mcp_refs.is_empty() {
        frontmatter.push_str("mcps:\n");
        for mcp in &profile.mcp_refs {
            frontmatter.push_str(&format!("  - {}\n", yaml_escape(&mcp.name)));
        }
    }

    frontmatter.push_str("---\n\n");

    // Compose a natural-language body from structured answers.
    // If a prompt_overlay_path is set, that file's content supersedes
    // the composed body — but we still write the frontmatter above.
    let scope_label = match profile.scope {
        Scope::Global => "global",
        Scope::Workspace => "workspace",
    };
    frontmatter.push_str(&format!(
        "# {}\n\nProfile agent for {} (scope: {}).\n",
        profile.id.as_str(),
        profile.id.as_str(),
        scope_label,
    ));

    frontmatter
}

/// Escape a string for safe inclusion in a YAML value.
///
/// Wraps the value in double quotes if it contains characters that would
/// break YAML parsing (colons, hashes, quotes, newlines, brackets, etc.)
/// and escapes any internal double quotes.
fn yaml_escape(s: &str) -> String {
    let needs_quoting = s.contains(':')
        || s.contains('#')
        || s.contains('"')
        || s.contains('\'')
        || s.contains('\n')
        || s.contains('[')
        || s.contains(']')
        || s.contains('{')
        || s.contains('}')
        || s.contains(',')
        || s.contains('&')
        || s.contains('*')
        || s.contains('!')
        || s.contains('|')
        || s.contains('>')
        || s.contains('%')
        || s.contains('@')
        || s.contains('`')
        || s.is_empty();
    if needs_quoting {
        format!("\"{}\"", s.replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::profile::{LaunchPolicy, ProfileAssetRef, ProfileId, ProviderId};
    use crate::domain::scope::Scope;

    fn test_profile() -> Profile {
        Profile {
            id: ProfileId::new("test-agent"),
            scope: Scope::Workspace,
            provider_id: ProviderId::new("claude-code"),
            skill_refs: vec![ProfileAssetRef::new("rust", "auto")],
            mcp_refs: vec![ProfileAssetRef::new("github", "auto")],
            instruction_refs: vec![],
            tool_refs: vec!["Read".into(), "Glob".into(), "Grep".into()],
            permission_mode: Some("auto".into()),
            prompt_overlay_path: None,
            launch_policy: LaunchPolicy::AutoRestore,
        }
    }

    #[test]
    fn compose_agent_markdown_includes_frontmatter() {
        let profile = test_profile();
        let md = compose_agent_markdown(&profile);
        assert!(md.starts_with("---\n"));
        assert!(md.contains("name: test-agent"));
        assert!(md.contains("provider: claude-code"));
        assert!(md.contains("  - Read"));
        assert!(md.contains("permission_mode: auto"));
        assert!(md.contains("  - rust"));
        assert!(md.contains("  - github"));
    }

    #[test]
    fn compose_agent_markdown_empty_profile() {
        let profile = Profile {
            id: ProfileId::new("minimal"),
            scope: Scope::Workspace,
            provider_id: ProviderId::new("claude-code"),
            skill_refs: vec![],
            mcp_refs: vec![],
            instruction_refs: vec![],
            tool_refs: vec![],
            permission_mode: None,
            prompt_overlay_path: None,
            launch_policy: LaunchPolicy::AutoRestore,
        };
        let md = compose_agent_markdown(&profile);
        assert!(md.contains("name: minimal"));
        assert!(!md.contains("tools:"));
        assert!(!md.contains("permission_mode:"));
    }

    #[test]
    fn build_launch_plan_creates_agent_file() {
        let dir = tempfile::tempdir().unwrap();
        let provider = ClaudeCodeProvider::new(dir.path().to_path_buf());
        let profile = test_profile();
        let plan = provider
            .build_launch_plan(&profile, None)
            .expect("plan should succeed");
        assert!(plan.agent_markdown_source.exists());
        assert!(plan.tool_refs.contains(&"Read".to_string()));
    }

    #[test]
    fn yaml_escape_plain_value() {
        assert_eq!(yaml_escape("hello"), "hello");
    }

    #[test]
    fn yaml_escape_colon() {
        assert_eq!(yaml_escape("foo:bar"), "\"foo:bar\"");
    }

    #[test]
    fn yaml_escape_hash() {
        assert_eq!(yaml_escape("foo#bar"), "\"foo#bar\"");
    }

    #[test]
    fn yaml_escape_newline() {
        // Newlines trigger quoting; the actual \n stays inside the quotes.
        assert!(yaml_escape("line1\nline2").starts_with('"'));
    }

    #[test]
    fn yaml_escape_quotes() {
        // Single quotes trigger quoting but aren't escaped
        assert_eq!(yaml_escape("it's"), "\"it's\"");
        // Double quotes are escaped
        assert_eq!(yaml_escape("say \"hi\""), "\"say \\\"hi\\\"\"");
    }

    #[test]
    fn yaml_escape_empty() {
        assert_eq!(yaml_escape(""), "\"\"");
    }

    #[test]
    fn yaml_escape_brackets() {
        assert_eq!(yaml_escape("[1,2]"), "\"[1,2]\"");
    }

    #[test]
    fn yaml_escape_compose_with_special_chars() {
        let profile = Profile {
            id: ProfileId::new("my:agent"),
            scope: Scope::Workspace,
            provider_id: ProviderId::new("claude-code"),
            skill_refs: vec![],
            mcp_refs: vec![],
            instruction_refs: vec![],
            tool_refs: vec![],
            permission_mode: Some("auto-accept".into()),
            prompt_overlay_path: None,
            launch_policy: LaunchPolicy::AutoRestore,
        };
        let md = compose_agent_markdown(&profile);
        // Colon in id should be quoted
        assert!(md.contains("name: \"my:agent\""));
        // Hyphenated mode should not be quoted (no special chars)
        assert!(md.contains("permission_mode: auto-accept"));
    }
}
