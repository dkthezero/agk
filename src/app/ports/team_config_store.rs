use crate::domain::scope::Scope;
use crate::domain::team::TeamConfig;
use anyhow::Result;

/// Port for reading/writing team configuration.
/// Concrete implementation: TeamTomlStore in infra/config/team_store.rs
pub trait TeamConfigStorePort: Send + Sync {
    fn load(&self, scope: Scope) -> Result<TeamConfig>;
    fn save(&self, scope: Scope, config: &TeamConfig) -> Result<()>;
    fn exists(&self, scope: Scope) -> bool;
}