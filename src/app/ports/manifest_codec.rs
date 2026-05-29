use crate::domain::config::ConfigFile;
use anyhow::Result;

/// Port for configuration format codecs (TOML, YAML, JSON, etc.)
pub trait ManifestCodecPort: std::fmt::Debug {
    /// Codec identifier e.g. "toml", "yaml"
    fn id(&self) -> &'static str;
    /// Whether this codec can handle files with `ext` (e.g. "toml", "yaml")
    fn supports_ext(&self, ext: &str) -> bool;
    /// Decode config file from text
    fn decode_config(&self, text: &str) -> Result<ConfigFile>;
    /// Encode config file to text
    fn encode_config(&self, config: &ConfigFile) -> Result<String>;
}
