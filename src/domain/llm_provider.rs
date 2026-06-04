//! LLM provider configuration and validation.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModelInput(String);

impl ModelInput {
    pub fn new(value: impl Into<String>) -> Result<Self, LlmDomainError> {
        let s: String = value.into();
        if s.is_empty() {
            return Err(LlmDomainError::EmptyModel);
        }
        if s.chars().count() > 256 {
            return Err(LlmDomainError::ModelTooLong);
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ModelInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LlmProviderKind {
    Ollama,
    LmStudio,
    Anthropic,
    OpenAi,
}

impl LlmProviderKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ollama" => Some(Self::Ollama),
            "lm-studio" => Some(Self::LmStudio),
            "anthropic" => Some(Self::Anthropic),
            "openai" => Some(Self::OpenAi),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::LmStudio => "lm-studio",
            Self::Anthropic => "anthropic",
            Self::OpenAi => "openai",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub id: String,
    pub kind: LlmProviderKind,
    pub endpoint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,
}

impl LlmProviderConfig {
    pub fn validate(&self) -> Result<(), LlmDomainError> {
        if self.id.trim().is_empty() {
            return Err(LlmDomainError::EmptyId);
        }
        let url = Url::parse(&self.endpoint)
            .map_err(|_| LlmDomainError::InvalidEndpoint(self.endpoint.clone()))?;
        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(LlmDomainError::InvalidEndpoint(self.endpoint.clone()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LlmHealthStatus {
    pub reachable: bool,
    pub latency_ms: Option<u64>,
    pub models: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Error, PartialEq)]
pub enum LlmDomainError {
    #[error("LLM provider id cannot be empty")]
    EmptyId,
    #[error("LLM provider endpoint is invalid: {0}")]
    InvalidEndpoint(String),
    #[error("model string cannot be empty")]
    EmptyModel,
    #[error("model string exceeds 256 characters")]
    ModelTooLong,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_provider_kind_from_str_canonical() {
        assert_eq!(LlmProviderKind::from_str("ollama"), Some(LlmProviderKind::Ollama));
        assert_eq!(LlmProviderKind::from_str("lm-studio"), Some(LlmProviderKind::LmStudio));
        assert_eq!(LlmProviderKind::from_str("anthropic"), Some(LlmProviderKind::Anthropic));
        assert_eq!(LlmProviderKind::from_str("openai"), Some(LlmProviderKind::OpenAi));
        assert_eq!(LlmProviderKind::from_str("unknown"), None);
    }

    #[test]
    fn llm_provider_config_validates_endpoint_url() {
        let cfg = LlmProviderConfig {
            id: "local-ollama".into(),
            kind: LlmProviderKind::Ollama,
            endpoint: "http://127.0.0.1:11434".into(),
            api_key: None,
            default_model: Some("llama3.2".into()),
        };
        assert!(cfg.validate().is_ok());

        let bad = LlmProviderConfig {
            id: "bad".into(),
            kind: LlmProviderKind::Ollama,
            endpoint: "not a url".into(),
            api_key: None,
            default_model: None,
        };
        assert!(cfg_validate_err_contains(&bad, "endpoint"));
    }

    fn cfg_validate_err_contains(cfg: &LlmProviderConfig, needle: &str) -> bool {
        match cfg.validate() {
            Ok(()) => false,
            Err(e) => e.to_string().contains(needle),
        }
    }

    #[test]
    fn model_string_capped_at_256_chars() {
        let long = "a".repeat(257);
        assert!(ModelInput::new(long.clone()).is_err());
        let ok = ModelInput::new("a".repeat(256));
        assert!(ok.is_ok());
    }
}
