use crate::app::bootstrap::build_provider_entries;
use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::ports::ConfigStorePort;
use crate::app::registry::Registry;
use crate::domain::scope::Scope;

/// List all registered providers with their capability flags
/// (`supports_mcp`, `supports_profiles`, `available_tools`,
/// `available_permission_modes`) and active state.
///
/// Emits a single [`CoreEvent::ProviderListed`] carrying one
/// [`crate::app::snapshot::ProviderEntry`] per registered provider.
/// A missing config file is treated as an empty provider set (the store
/// returns `Ok(default)` in that case), while a malformed config surfaces
/// as an error — per the AGENTS.md "Malformed Config: Surface Errors,
/// Don't Default" rule.
pub fn run(
    scope: Scope,
    store: &dyn ConfigStorePort,
    registry: &Registry,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let config = store.load(scope)?;
    let entries = build_provider_entries(&config, registry);
    sink.on_event(CoreEvent::ProviderListed(entries));
    Ok(CoreOutcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ports::ProviderPort;
    use crate::domain::asset::{AssetKind, ScannedPackage};
    use crate::domain::config::ConfigFile;
    use crate::domain::identity::AssetIdentity;
    use std::sync::Mutex;

    struct FakeStore {
        data: Mutex<ConfigFile>,
    }

    impl FakeStore {
        fn with(file: ConfigFile) -> Self {
            Self {
                data: Mutex::new(file),
            }
        }
    }

    impl ConfigStorePort for FakeStore {
        fn load(&self, _scope: Scope) -> anyhow::Result<ConfigFile> {
            Ok(self.data.lock().unwrap().clone())
        }
        fn save(&self, _scope: Scope, _config: &ConfigFile) -> anyhow::Result<()> {
            Ok(())
        }
    }

    struct CapabilityProvider {
        id: &'static str,
        name: &'static str,
        mcp: bool,
        profiles: bool,
        tools: Vec<String>,
        modes: Vec<(String, String)>,
    }

    impl ProviderPort for CapabilityProvider {
        fn id(&self) -> &str {
            self.id
        }
        fn name(&self) -> &str {
            self.name
        }
        fn install(
            &self,
            _: &ScannedPackage,
            _: Scope,
            _: Option<&ConfigFile>,
            _: bool,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        fn remove(
            &self,
            _: &AssetIdentity,
            _: &AssetKind,
            _: Scope,
            _: Option<&ConfigFile>,
        ) -> anyhow::Result<()> {
            Ok(())
        }
        fn supports_mcp(&self) -> bool {
            self.mcp
        }
        fn supports_profiles(&self) -> bool {
            self.profiles
        }
        fn available_profile_tools(&self) -> Vec<String> {
            self.tools.clone()
        }
        fn available_permission_modes(&self) -> Vec<(String, String)> {
            self.modes.clone()
        }
    }

    struct CollectingSink {
        events: Vec<CoreEvent>,
    }

    impl CoreEventSink for CollectingSink {
        fn on_event(&mut self, event: CoreEvent) {
            self.events.push(event);
        }
        fn on_error(&mut self, _error: String) {}
    }

    #[test]
    fn list_providers_empty_registry_emits_empty_list_event() {
        let store = FakeStore::with(ConfigFile::default());
        let registry = Registry::new();
        let mut sink = CollectingSink { events: vec![] };
        let result = run(Scope::Workspace, &store, &registry, &mut sink);
        assert!(result.is_ok());
        assert_eq!(sink.events.len(), 1);
        match &sink.events[0] {
            CoreEvent::ProviderListed(entries) => assert!(entries.is_empty()),
            other => panic!("expected ProviderListed, got {:?}", other),
        }
    }

    #[test]
    fn list_providers_emits_capability_flags_and_active_state() {
        let mut config = ConfigFile::default();
        config.providers.push("claude-code".into());
        let store = FakeStore::with(config);

        let mut registry = Registry::new();
        registry.register_provider(Box::new(CapabilityProvider {
            id: "claude-code",
            name: "Claude Code",
            mcp: true,
            profiles: true,
            tools: vec!["Read".into(), "Glob".into()],
            modes: vec![("default".into(), "Default".into())],
        }));
        registry.register_provider(Box::new(CapabilityProvider {
            id: "letta",
            name: "Letta",
            mcp: false,
            profiles: false,
            tools: vec![],
            modes: vec![],
        }));

        let mut sink = CollectingSink { events: vec![] };
        run(Scope::Workspace, &store, &registry, &mut sink).unwrap();

        let entries = match &sink.events[0] {
            CoreEvent::ProviderListed(e) => e,
            other => panic!("expected ProviderListed, got {:?}", other),
        };
        assert_eq!(entries.len(), 2);

        let claude = entries.iter().find(|e| e.id == "claude-code").unwrap();
        assert!(claude.active);
        assert!(claude.supports_mcp);
        assert!(claude.supports_profiles);
        assert_eq!(claude.available_tools, vec!["Read", "Glob"]);
        assert_eq!(
            claude.available_permission_modes,
            vec![("default".to_string(), "Default".to_string())]
        );

        let letta = entries.iter().find(|e| e.id == "letta").unwrap();
        assert!(!letta.active);
        assert!(!letta.supports_mcp);
        assert!(!letta.supports_profiles);
        assert!(letta.available_tools.is_empty());
    }

    #[test]
    fn list_providers_surfaces_malformed_config_error() {
        struct FailingStore;
        impl ConfigStorePort for FailingStore {
            fn load(&self, _scope: Scope) -> anyhow::Result<ConfigFile> {
                Err(anyhow::anyhow!("malformed config"))
            }
            fn save(&self, _scope: Scope, _config: &ConfigFile) -> anyhow::Result<()> {
                Ok(())
            }
        }
        let store = FailingStore;
        let registry = Registry::new();
        let mut sink = CollectingSink { events: vec![] };
        let result = run(Scope::Workspace, &store, &registry, &mut sink);
        assert!(result.is_err());
        assert!(sink.events.is_empty());
    }
}
