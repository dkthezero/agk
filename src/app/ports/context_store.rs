use anyhow::Result;

/// Port for reading / writing the global contexts file.
pub trait ContextStorePort: Send + Sync {
    fn load_contexts(&self) -> Result<crate::domain::context::ContextFile>;
    fn save_contexts(&self, contexts: &crate::domain::context::ContextFile) -> Result<()>;
    fn current_context(&self) -> Result<crate::domain::context::ContextId>;
    fn switch_context(&self, id: &crate::domain::context::ContextId) -> Result<()>;
}
