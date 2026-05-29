use crate::domain::config::ConfigFile;
use anyhow::Result;

/// Port for building and executing profile launch plans.
/// Implemented by provider adapters that support profile sessions.
pub trait ProfileRuntimePort: Send + Sync {
    fn provider_id(&self) -> &str;

    /// Build a deterministic launch plan without modifying filesystem state.
    fn build_launch_plan(
        &self,
        profile: &crate::domain::profile::Profile,
        config: Option<&ConfigFile>,
    ) -> Result<crate::app::event::LaunchPlan>;

    /// Execute a previously-built launch plan, returning a handle that
    /// includes a cleanup closure for restoring provider state.
    fn run_plan(&self, plan: &crate::app::event::LaunchPlan) -> Result<ProfileSession>;
}

/// Handle for a running profile session.
pub struct ProfileSession {
    pub process: std::process::Child,
    cleanup: Option<Box<dyn FnOnce() -> Result<()> + Send>>,
}

impl ProfileSession {
    pub fn new(
        process: std::process::Child,
        cleanup: Box<dyn FnOnce() -> Result<()> + Send>,
    ) -> Self {
        Self {
            process,
            cleanup: Some(cleanup),
        }
    }

    /// Block until the child process exits, then run the cleanup closure.
    pub fn wait_and_cleanup(mut self) -> Result<std::process::ExitStatus> {
        let status = self.process.wait()?;
        if let Some(cleanup) = self.cleanup.take() {
            cleanup()?;
        }
        Ok(status)
    }
}
