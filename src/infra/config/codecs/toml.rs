use crate::app::ports::ManifestCodecPort;
use crate::domain::config::ConfigFile;

#[derive(Debug)]
pub struct TomlCodec;

impl Default for TomlCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl TomlCodec {
    pub fn new() -> Self {
        Self
    }
}

impl ManifestCodecPort for TomlCodec {
    fn id(&self) -> &'static str {
        "toml"
    }
    fn supports_ext(&self, ext: &str) -> bool {
        ext == "toml"
    }
    fn decode_config(&self, text: &str) -> anyhow::Result<ConfigFile> {
        let config: ConfigFile = toml::from_str(text)?;
        Ok(config)
    }
    fn encode_config(&self, config: &ConfigFile) -> anyhow::Result<String> {
        Ok(toml::to_string_pretty(config)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_empty_config() {
        let codec = TomlCodec::new();
        let config = ConfigFile::default();
        let encoded = codec.encode_config(&config).unwrap();
        let decoded = codec.decode_config(&encoded).unwrap();
        assert_eq!(config, decoded);
    }

    #[test]
    fn round_trip_with_profile() {
        let codec = TomlCodec::new();
        let mut config = ConfigFile::default();
        config.profiles.push(crate::domain::config::Profile {
            name: "dev".into(),
            provider_id: "opencode".into(),
            scope: "workspace".into(),
            skills: vec![crate::domain::profile::ProfileAssetRef::new("rust", "auto")],
            mcps: vec![],
            instructions: vec![],
            tool_refs: vec![],
            permission_mode: None,
            prompt_overlay_path: None,
        });
        let encoded = codec.encode_config(&config).unwrap();
        let decoded = codec.decode_config(&encoded).unwrap();
        assert_eq!(config, decoded);
    }
}
