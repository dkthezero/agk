/// Payload for [`CoreCommand::RegisterMcp`].
#[derive(Debug, Clone, PartialEq)]
pub struct RegisterMcpInput {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub transport: crate::domain::mcp::McpTransport,
    pub description: Option<String>,
    pub test_after: bool,
}
