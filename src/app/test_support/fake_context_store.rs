use crate::app::ports::ContextStorePort;
use crate::domain::context::{ContextFile, ContextId};
use anyhow::Result;

/// In-memory [`ContextStorePort`] that always returns default contexts.
///
/// Tests that need custom context state can wrap this or build a more
/// sophisticated fake. This covers the 90 % case where context switching
/// is not the subject under test.
#[derive(Debug, Default)]
pub struct FakeContextStore;

impl FakeContextStore {
    pub fn new() -> Self {
        Self
    }
}

impl ContextStorePort for FakeContextStore {
    fn load_contexts(&self) -> Result<ContextFile> {
        Ok(ContextFile::default())
    }

    fn save_contexts(&self, _contexts: &ContextFile) -> Result<()> {
        Ok(())
    }

    fn current_context(&self) -> Result<ContextId> {
        Ok(ContextId::default())
    }

    fn switch_context(&self, _id: &ContextId) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_context_store_defaults() {
        let store = FakeContextStore::new();
        let ctx = store.current_context().unwrap();
        assert_eq!(ctx.as_str(), "default");
        let file = store.load_contexts().unwrap();
        assert!(file.contexts.is_empty());
    }
}
