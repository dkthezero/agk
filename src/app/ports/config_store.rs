use crate::domain::config::ConfigFile;
use crate::domain::scope::Scope;
use anyhow::Result;

pub trait ConfigStorePort: Send + Sync {
    fn load(&self, scope: Scope) -> Result<ConfigFile>;
    fn save(&self, scope: Scope, config: &ConfigFile) -> Result<()>;
    /// Delete the backing file for the scope if it exists.
    /// Default no-op for stores that don't have a file.
    fn delete_file(&self, _scope: Scope) -> Result<()> {
        Ok(())
    }
}
