use crate::app::ports::llm_provider::LlmProviderAdapter;
use crate::domain::llm_provider::LlmProviderKind;

pub struct LmStudioProvider {
    pub endpoint: String,
}

impl LmStudioProvider {
    pub fn new(e: impl Into<String>) -> Self {
        Self {
            endpoint: e.into(),
        }
    }
}

impl LlmProviderAdapter for LmStudioProvider {
    fn kind(&self) -> LlmProviderKind {
        LlmProviderKind::LmStudio
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
    fn lmstudio_health_url_uses_v1_models() {
        let p = LmStudioProvider::new("http://127.0.0.1:1234");
        assert_eq!(p.health_url(), "http://127.0.0.1:1234/v1/models");
        assert_eq!(p.kind(), LlmProviderKind::LmStudio);
    }
}
