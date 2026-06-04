//! Pure renderer: takes a fully-resolved `LaunchPlan` and emits the markdown
//! the Claude Code CLI expects at `.claude/agents/<name>.md`.
//!
//! Zero I/O, zero port calls, zero global state — given the same `LaunchPlan`
//! the function always returns the same string. Tested via golden fixtures in
//! `tests/agent_markdown_renderer.rs`.

use crate::domain::launch_plan::LaunchPlan;

pub fn render_agent_markdown(plan: &LaunchPlan) -> String {
    let mut yaml = String::new();
    yaml.push_str(&format!("name: {}\n", yaml_scalar(&plan.frontmatter.name)));
    yaml.push_str(&format!(
        "description: {}\n",
        yaml_scalar(&plan.frontmatter.description)
    ));
    if !plan.frontmatter.tools.is_empty() {
        yaml.push_str("tools:\n");
        for t in &plan.frontmatter.tools {
            yaml.push_str(&format!("  - {}\n", yaml_scalar(t)));
        }
    }
    if !plan.frontmatter.disallowed_tools.is_empty() {
        yaml.push_str("disallowedTools:\n");
        for t in &plan.frontmatter.disallowed_tools {
            yaml.push_str(&format!("  - {}\n", yaml_scalar(t)));
        }
    }
    yaml.push_str(&format!(
        "model: {}\n",
        yaml_scalar(&plan.frontmatter.model)
    ));
    if let Some(pm) = &plan.frontmatter.permission_mode {
        yaml.push_str(&format!("permissionMode: {}\n", yaml_scalar(pm)));
    }
    if let Some(mt) = plan.frontmatter.max_turns {
        yaml.push_str(&format!("maxTurns: {}\n", mt));
    }
    if !plan.frontmatter.skills.is_empty() {
        yaml.push_str("skills:\n");
        for s in &plan.frontmatter.skills {
            yaml.push_str(&format!("  - {}\n", yaml_scalar(s)));
        }
    }
    if !plan.resolved_mcp_servers.is_empty() {
        yaml.push_str("mcpServers:\n");
        for server in &plan.resolved_mcp_servers {
            yaml.push_str(&format!("  {}:\n", yaml_scalar(&server.name)));
            yaml.push_str(&format!("    command: {}\n", yaml_scalar(&server.command)));
            if !server.args.is_empty() {
                yaml.push_str("    args:\n");
                for a in &server.args {
                    yaml.push_str(&format!("      - {}\n", yaml_scalar(a)));
                }
            }
            if !server.env.is_empty() {
                yaml.push_str("    env:\n");
                for e in &server.env {
                    yaml.push_str(&format!("      {}\n", yaml_scalar(e)));
                }
            }
        }
    }
    if !plan.frontmatter.hooks.is_empty() {
        yaml.push_str("hooks:\n");
        for h in &plan.frontmatter.hooks {
            yaml.push_str(&format!("  - {}\n", yaml_scalar(h)));
        }
    }
    if let Some(m) = &plan.frontmatter.memory {
        yaml.push_str(&format!("memory: {}\n", yaml_scalar(m)));
    }
    if plan.frontmatter.background {
        yaml.push_str("background: true\n");
    }
    if let Some(effort) = &plan.frontmatter.effort {
        yaml.push_str(&format!("effort: {}\n", yaml_scalar(effort)));
    }
    if let Some(iso) = &plan.frontmatter.isolation {
        yaml.push_str(&format!("isolation: {}\n", yaml_scalar(iso)));
    }
    if let Some(color) = &plan.frontmatter.color {
        yaml.push_str(&format!("color: {}\n", yaml_scalar(color)));
    }
    format!("---\n{}---\n\n{}", yaml, plan.prompt_body)
}

fn yaml_scalar(s: &str) -> String {
    if s.contains(':')
        || s.contains('#')
        || s.contains('"')
        || s.contains('\n')
        || s.starts_with(' ')
        || s.ends_with(' ')
    {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agent_markdown::{AgentFrontmatter, AgentMcpServer};
    use crate::domain::launch_plan::LaunchPlan;

    fn sample_plan() -> LaunchPlan {
        LaunchPlan {
            profile_id: "reviewer".into(),
            provider_id: "claude-code".into(),
            frontmatter: AgentFrontmatter {
                name: "reviewer".into(),
                description: "PR reviewer".into(),
                tools: vec!["Read".into(), "Grep".into()],
                disallowed_tools: vec![],
                model: "sonnet".into(),
                permission_mode: Some("acceptEdits".into()),
                max_turns: None,
                skills: vec!["code-review".into()],
                mcp_servers: vec!["github".into()],
                hooks: vec![],
                memory: None,
                background: false,
                effort: None,
                isolation: None,
                color: None,
            },
            prompt_body: "Review staged changes carefully.".into(),
            resolved_mcp_servers: vec![AgentMcpServer {
                name: "github".into(),
                command: "docker".into(),
                args: vec!["run".into(), "-i".into(), "mcp/github".into()],
                env: vec![],
            }],
            llm_provider_id: Some("local-ollama".into()),
        }
    }

    #[test]
    fn render_minimal_no_mcp() {
        let mut p = sample_plan();
        p.resolved_mcp_servers.clear();
        p.frontmatter.mcp_servers.clear();
        let out = render_agent_markdown(&p);
        assert!(out.starts_with("---\nname: reviewer\n"));
        assert!(out.contains("model: sonnet\n"));
        assert!(!out.contains("mcpServers:"));
    }

    #[test]
    fn render_full_with_mcp_servers() {
        let out = render_agent_markdown(&sample_plan());
        assert!(out.contains("mcpServers:"));
        assert!(out.contains("  github:"));
        assert!(out.contains("    command: docker"));
    }

    #[test]
    fn render_yaml_escapes_double_quotes_in_description() {
        let mut p = sample_plan();
        p.frontmatter.description = "Quote: \"yes\"".into();
        let out = render_agent_markdown(&p);
        assert!(out.contains("description: \"Quote: \\\"yes\\\"\""));
    }
}
