#[cfg(any(
    feature = "llm-ollama",
    feature = "llm-lmstudio",
    feature = "llm-anthropic",
    feature = "llm-openai"
))]
pub mod health;
