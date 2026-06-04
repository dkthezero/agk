use crate::app::ports::llm_provider::LlmProviderAdapter;
use crate::domain::llm_provider::LlmProviderKind;

pub struct OpenAiProvider {
    pub endpoint: String,
}

impl OpenAiProvider {
    pub fn new(e: impl Into<String>) -> Self {
        Self {
            endpoint: e.into(),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::llm_provider::LlmProviderAdapter;
    use crate::domain::llm_provider::LlmProviderKind;

    #[test]
    fn openai_health_url_uses_v1_models() {
        let p = OpenAiProvider::new("https://api.openai.com");
        assert_eq!(p.health_url(), "https://api.openai.com/v1/models");
        assert_eq!(p.kind(), LlmProviderKind::OpenAi);
    }
}
