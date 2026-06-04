//! All the data the agent-markdown renderer needs, pre-resolved by the
//! use-case layer so the renderer itself has zero I/O and zero port calls.

use crate::domain::agent_markdown::{AgentFrontmatter, AgentMcpServer};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LaunchPlan {
    pub profile_id: String,
    pub provider_id: String,
    pub frontmatter: AgentFrontmatter,
    pub prompt_body: String,
    /// MCP servers already resolved from the registry (name -> command/args/env).
    /// The renderer embeds these into the `mcpServers` block of the frontmatter.
    pub resolved_mcp_servers: Vec<AgentMcpServer>,
    /// Optional LLM provider id to record in the launch plan for the
    /// downstream exec layer to consume (AGK does not probe the server).
    pub llm_provider_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::agent_markdown::{AgentFrontmatter, AgentMcpServer};

    #[test]
    fn launch_plan_carries_resolved_mcp_servers() {
        let servers = vec![AgentMcpServer {
            name: "github".into(),
            command: "docker".into(),
            args: vec!["run".into(), "-i".into(), "mcp/github".into()],
            env: vec![],
        }];
        let plan = LaunchPlan {
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
            prompt_body: "Review the staged diff carefully.".into(),
            resolved_mcp_servers: servers.clone(),
            llm_provider_id: Some("local-ollama".into()),
        };
        assert_eq!(plan.resolved_mcp_servers.len(), 1);
        assert_eq!(plan.frontmatter.name, "reviewer");
        assert_eq!(plan.llm_provider_id.as_deref(), Some("local-ollama"));
    }
}
