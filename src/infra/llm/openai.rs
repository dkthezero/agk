use crate::app::ports::llm_provider::LlmProviderAdapter;
use crate::domain::llm_provider::LlmProviderKind;

pub struct OpenAiProvider {
    pub endpoint: String,
    pub api_key: Option<String>,
}

impl OpenAiProvider {
    pub fn new(e: impl Into<String>, api_key: Option<String>) -> Self {
        Self {
            endpoint: e.into(),
            api_key,
        }
    }
}

impl LlmProviderAdapter for OpenAiProvider {
    fn kind(&self) -> LlmProviderKind {
        LlmProviderKind::OpenAi
    }
    fn health_url(&self) -> String {
        format!("{}/v1/models", self.endpoint.trim_end_matches('/'))
    }
    fn auth_header(&self) -> Option<(&'static str, String)> {
        self.api_key
            .as_deref()
            .map(|k| ("Authorization", format!("Bearer {k}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::llm_provider::LlmProviderAdapter;

    #[test]
    fn openai_health_url_uses_v1_models() {
        let p = OpenAiProvider::new("https://api.openai.com", None);
        assert_eq!(p.health_url(), "https://api.openai.com/v1/models");
        assert_eq!(p.kind(), LlmProviderKind::OpenAi);
    }

    #[test]
    fn openai_emits_bearer_header_when_key_present() {
        let p = OpenAiProvider::new("https://api.openai.com", Some("sk-test".into()));
        let (name, value) = p.auth_header().expect("auth header");
        assert_eq!(name, "Authorization");
        assert_eq!(value, "Bearer sk-test");
    }

    #[test]
    fn openai_omits_auth_header_when_no_key() {
        let p = OpenAiProvider::new("https://api.openai.com", None);
        assert!(p.auth_header().is_none());
    }
}
