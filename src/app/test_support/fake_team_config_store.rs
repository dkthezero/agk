//! Fake TeamConfigStorePort for testing.

use crate::app::ports::TeamConfigStorePort;
use crate::domain::scope::Scope;
use crate::domain::team::TeamConfig;
use anyhow::Result;
use std::sync::Mutex;

/// In-memory fake that stores TeamConfig in a Mutex.
pub struct FakeTeamConfigStore {
    workspace: Mutex<TeamConfig>,
    global: Mutex<TeamConfig>,
}

impl Default for FakeTeamConfigStore {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeTeamConfigStore {
    pub fn new() -> Self {
        Self {
            workspace: Mutex::new(TeamConfig::default()),
            global: Mutex::new(TeamConfig::default()),
        }
    }
}

impl TeamConfigStorePort for FakeTeamConfigStore {
    fn load(&self, scope: Scope) -> Result<TeamConfig> {
        match scope {
            Scope::Workspace => Ok(self.workspace.lock().unwrap().clone()),
            Scope::Global => Ok(self.global.lock().unwrap().clone()),
        }
    }

    fn save(&self, scope: Scope, config: &TeamConfig) -> Result<()> {
        match scope {
            Scope::Workspace => *self.workspace.lock().unwrap() = config.clone(),
            Scope::Global => *self.global.lock().unwrap() = config.clone(),
        }
        Ok(())
    }

    fn exists(&self, scope: Scope) -> bool {
        match scope {
            Scope::Workspace => !self.workspace.lock().unwrap().name.is_empty(),
            Scope::Global => !self.global.lock().unwrap().name.is_empty(),
        }
    }
}
