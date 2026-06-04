use crate::app::ports::llm_provider::LlmProviderAdapter;
use crate::domain::llm_provider::LlmProviderKind;

pub struct AnthropicProvider {
    pub endpoint: String,
    pub api_key: Option<String>,
}

impl AnthropicProvider {
    pub fn new(e: impl Into<String>, api_key: Option<String>) -> Self {
        Self {
            endpoint: e.into(),
            api_key,
        }
    }
}

impl LlmProviderAdapter for AnthropicProvider {
    fn kind(&self) -> LlmProviderKind {
        LlmProviderKind::Anthropic
    }
    fn health_url(&self) -> String {
        self.endpoint.trim_end_matches('/').to_string()
    }
    fn auth_header(&self) -> Option<(&'static str, String)> {
        self.api_key
            .as_deref()
            .map(|k| ("x-api-key", k.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::llm_provider::LlmProviderAdapter;

    #[test]
    fn anthropic_health_url_passes_through_endpoint() {
        let p = AnthropicProvider::new("https://api.anthropic.com", None);
        // HttpLlmHealthCheck will OPTIONS this URL.
        assert_eq!(p.health_url(), "https://api.anthropic.com");
        assert_eq!(p.kind(), LlmProviderKind::Anthropic);
    }

    #[test]
    fn anthropic_emits_x_api_key_when_key_present() {
        let p = AnthropicProvider::new("https://api.anthropic.com", Some("secret".into()));
        let (name, value) = p.auth_header().expect("auth header");
        assert_eq!(name, "x-api-key");
        assert_eq!(value, "secret");
    }

    #[test]
    fn anthropic_omits_auth_header_when_no_key() {
        let p = AnthropicProvider::new("https://api.anthropic.com", None);
        assert!(p.auth_header().is_none());
    }
}
