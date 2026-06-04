//! Reusable fake port implementations for testing.
//!
//! Hand-written fakes (no mocking libraries) that can be wired into `AgkCore`
//! for use-case and full-flow tests. Promoted from `#[cfg(test)]` blocks
//! once reused across multiple test modules.

pub mod collecting_sink;
pub mod fake_claude_cli_probe;
pub mod fake_clawhub;
pub mod fake_context_store;
pub mod fake_llm_provider;
pub mod fake_mcp_registry;
pub mod fake_process_runner;
pub mod fake_store;
pub mod fake_team_config_store;
pub mod fake_vault;

#[allow(unused_imports)]
pub use collecting_sink::CollectingSink;
#[allow(unused_imports)]
pub use fake_claude_cli_probe::FakeClaudeCliProbe;
#[allow(unused_imports)]
pub use fake_clawhub::FakeClawHub;
#[allow(unused_imports)]
pub use fake_context_store::FakeContextStore;
#[allow(unused_imports)]
pub use fake_llm_provider::{
    FakeAdapter, FakeLlmHealthCheck, FakeLlmProviderFactory, FakeLlmProviderStore,
};
#[allow(unused_imports)]
pub use fake_mcp_registry::FakeMcpRegistry;
#[allow(unused_imports)]
pub use fake_process_runner::FakeProcessRunner;
#[allow(unused_imports)]
pub use fake_store::FakeStore;
#[allow(unused_imports)]
pub use fake_team_config_store::FakeTeamConfigStore;
#[allow(unused_imports)]
pub use fake_vault::FakeVault;
