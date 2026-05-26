use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a context (display name acts as the key).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContextId(pub String);

impl ContextId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ContextId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl Default for ContextId {
    fn default() -> Self {
        Self("default".to_string())
    }
}

/// Deployment environment for filtering assets and profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Environment {
    #[default]
    Local,
    Dev,
    Staging,
    Prod,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Dev => "dev",
            Environment::Staging => "staging",
            Environment::Prod => "prod",
        }
    }
}

impl From<&str> for Environment {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "dev" => Environment::Dev,
            "staging" => Environment::Staging,
            "prod" => Environment::Prod,
            _ => Environment::Local,
        }
    }
}

/// Per-context configuration stored in global scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ContextConfig {
    /// Human-readable display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// Vault IDs that are always active in this context (merged with personal).
    #[serde(default)]
    pub vaults: Vec<String>,
    /// Profile names that belong to this context.
    #[serde(default)]
    pub profiles: Vec<String>,
    /// Provider IDs active in this context.
    #[serde(default)]
    pub providers: Vec<String>,
    /// Environment filter for this context (defaults to Local if absent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<Environment>,
    /// Arbitrary metadata tags (e.g. team, region).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub tags: HashMap<String, String>,
}

/// The full contexts.toml schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextFile {
    #[serde(default = "default_context")]
    pub current_context: String,
    #[serde(default)]
    pub contexts: HashMap<String, ContextConfig>,
}

impl Default for ContextFile {
    fn default() -> Self {
        Self {
            current_context: default_context(),
            contexts: HashMap::new(),
        }
    }
}

fn default_context() -> String {
    "default".to_string()
}

impl ContextFile {
    pub fn current_id(&self) -> ContextId {
        ContextId::new(&self.current_context)
    }

    pub fn get(&self, id: &ContextId) -> Option<&ContextConfig> {
        self.contexts.get(id.as_str())
    }

    pub fn get_mut(&mut self, id: &ContextId) -> Option<&mut ContextConfig> {
        self.contexts.get_mut(id.as_str())
    }

    pub fn ensure_default(&mut self) {
        if !self.contexts.contains_key("default") {
            self.contexts.insert(
                "default".to_string(),
                ContextConfig {
                    display_name: Some("Personal".to_string()),
                    ..ContextConfig::default()
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_id_from_str() {
        let id = ContextId::from("company-x");
        assert_eq!(id.as_str(), "company-x");
    }

    #[test]
    fn default_context_id_is_default() {
        let id = ContextId::default();
        assert_eq!(id.as_str(), "default");
    }

    #[test]
    fn environment_round_trip() {
        assert_eq!(Environment::from("dev"), Environment::Dev);
        assert_eq!(Environment::from("staging"), Environment::Staging);
        assert_eq!(Environment::from("prod"), Environment::Prod);
        assert_eq!(Environment::from("local"), Environment::Local);
        assert_eq!(Environment::from("unknown"), Environment::Local);
    }

    #[test]
    fn environment_as_str() {
        assert_eq!(Environment::Dev.as_str(), "dev");
        assert_eq!(Environment::Staging.as_str(), "staging");
        assert_eq!(Environment::Prod.as_str(), "prod");
        assert_eq!(Environment::Local.as_str(), "local");
    }

    #[test]
    fn context_file_ensure_default() {
        let mut file = ContextFile::default();
        assert!(file.contexts.is_empty());
        file.ensure_default();
        assert!(file.contexts.contains_key("default"));
        let default = file.contexts.get("default").unwrap();
        assert_eq!(default.display_name, Some("Personal".to_string()));
    }

    #[test]
    fn context_file_current_id() {
        let file = ContextFile {
            current_context: "team-a".to_string(),
            contexts: HashMap::new(),
        };
        assert_eq!(file.current_id().as_str(), "team-a");
    }

    #[test]
    fn context_config_defaults() {
        let cfg = ContextConfig::default();
        assert!(cfg.vaults.is_empty());
        assert!(cfg.profiles.is_empty());
        assert!(cfg.providers.is_empty());
        assert_eq!(cfg.environment, None);
        assert!(cfg.tags.is_empty());
    }

    #[test]
    fn context_file_toml_round_trip() {
        let mut file = ContextFile::default();
        file.ensure_default();
        file.contexts.insert(
            "company-x".to_string(),
            ContextConfig {
                display_name: Some("Company X".to_string()),
                vaults: vec!["team-skills".to_string()],
                profiles: vec!["backend".to_string()],
                environment: Some(Environment::Prod),
                tags: {
                    let mut m = HashMap::new();
                    m.insert("team".to_string(), "platform".to_string());
                    m
                },
                ..ContextConfig::default()
            },
        );

        let serialized = toml::to_string(&file).unwrap();
        let deserialized: ContextFile = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.current_context, "default");
        assert_eq!(deserialized.contexts.len(), 2);
        let company = deserialized.contexts.get("company-x").unwrap();
        assert_eq!(company.display_name, Some("Company X".to_string()));
        assert_eq!(company.environment, Some(Environment::Prod));
        assert_eq!(company.tags.get("team"), Some(&"platform".to_string()));
    }
}
