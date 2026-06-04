use crate::app::ports::llm_provider::LlmProviderAdapter;
use crate::domain::llm_provider::LlmProviderKind;

pub struct OllamaProvider {
    pub endpoint: String,
}

impl OllamaProvider {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }
}

impl LlmProviderAdapter for OllamaProvider {
    fn kind(&self) -> LlmProviderKind {
        LlmProviderKind::Ollama
    }
    fn health_url(&self) -> String {
        format!("{}/api/tags", self.endpoint.trim_end_matches('/'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::llm_provider::LlmProviderAdapter;
    use crate::domain::llm_provider::LlmProviderKind;

    #[test]
    fn ollama_health_url_uses_api_tags() {
        let p = OllamaProvider::new("http://127.0.0.1:11434");
        assert_eq!(p.health_url(), "http://127.0.0.1:11434/api/tags");
        assert_eq!(p.kind(), LlmProviderKind::Ollama);
    }

    #[test]
    fn ollama_strips_trailing_slash() {
        let p = OllamaProvider::new("http://127.0.0.1:11434/");
        assert_eq!(p.health_url(), "http://127.0.0.1:11434/api/tags");
    }
}
