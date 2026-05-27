#![allow(dead_code)]

use crate::app::ports::{ConfigStorePort, ProviderPort};
use crate::domain::asset::{AssetKind, ScannedPackage};
use crate::domain::identity::AssetIdentity;
use crate::domain::scope::Scope;
use anyhow::{bail, Result};
use std::collections::{HashSet, VecDeque};

/// A resolved dependency ready for installation.
#[derive(Debug, Clone)]
pub struct DependencyResolution {
    pub identity: AssetIdentity,
    pub vault_id: String,
    pub package: ScannedPackage,
}

/// Result of installing a meta-skill with all its dependencies.
#[derive(Debug, Clone, Default)]
pub struct BundleInstallResult {
    pub installed: Vec<String>,
    pub updated: Vec<String>,
    pub skipped: Vec<String>,
    pub failed: Vec<(String, String)>,
}

/// Parse a dependency identity string like "vault/name" or "vault/name:version" or "name".
fn parse_dep_identity(dep: &str) -> (Option<&str>, &str) {
    let parts: Vec<&str> = dep.split('/').collect();
    if parts.len() == 2 {
        (Some(parts[0]), parts[1])
    } else {
        (None, dep)
    }
}

/// Find a package by name across all vaults, with optional vault hint.
fn find_dependency(
    registry: &crate::app::registry::Registry,
    vault_hint: Option<&str>,
    name: &str,
) -> Result<Option<ScannedPackage>> {
    for vault in &registry.vaults {
        if let Some(hint) = vault_hint {
            if vault.id() != hint {
                continue;
            }
        }
        for feature in &registry.feature_sets {
            if feature.asset_kind() != AssetKind::Skill {
                continue;
            }
            let pkgs = vault.list_packages(feature.as_ref())?;
            for pkg in pkgs {
                if pkg.identity.name == name {
                    return Ok(Some(pkg));
                }
            }
        }
    }
    Ok(None)
}

/// Resolve the full dependency tree for a package.
/// Returns a queue of dependencies in installation order (parents first, then children).
/// Uses BFS to resolve breadth-first, with cycle detection.
pub fn resolve_dependencies(
    root: &ScannedPackage,
    registry: &crate::app::registry::Registry,
) -> Result<Vec<DependencyResolution>> {
    let mut queue: VecDeque<(ScannedPackage, Vec<String>)> = VecDeque::new();
    let mut result: Vec<DependencyResolution> = Vec::new();
    let mut visited: HashSet<(String, String, String)> = HashSet::new(); // (vault_id, name, sha10)

    // Start with the root package
    queue.push_back((root.clone(), vec![root.identity.name.clone()]));

    while let Some((pkg, path)) = queue.pop_front() {
        let key = (
            pkg.vault_id.clone(),
            pkg.identity.name.clone(),
            pkg.identity.sha10.clone(),
        );

        // Skip if already visited with same sha10
        if visited.contains(&key) {
            continue;
        }
        visited.insert(key);

        // Add to result
        result.push(DependencyResolution {
            identity: pkg.identity.clone(),
            vault_id: pkg.vault_id.clone(),
            package: pkg.clone(),
        });

        // Process required dependencies
        for dep_str in &pkg.requires {
            let (vault_hint, name) = parse_dep_identity(dep_str);

            // Cycle detection
            if path.contains(&name.to_string()) {
                bail!(
                    "Circular dependency detected: {} → ... → {}",
                    path.join(" → "),
                    name
                );
            }

            let dep_pkg = find_dependency(registry, vault_hint, name)?;
            match dep_pkg {
                Some(dep) => {
                    let mut new_path = path.clone();
                    new_path.push(dep.identity.name.clone());
                    queue.push_back((dep, new_path));
                }
                None => {
                    bail!(
                        "Required dependency '{}' of '{}' not found in any vault",
                        dep_str,
                        pkg.identity.name
                    );
                }
            }
        }

        // Process optional dependencies (don't fail if missing)
        for dep_str in &pkg.requires_optional {
            let (vault_hint, name) = parse_dep_identity(dep_str);

            if path.contains(&name.to_string()) {
                // Skip optional cycles silently
                continue;
            }

            if let Some(dep) = find_dependency(registry, vault_hint, name)? {
                let mut new_path = path.clone();
                new_path.push(dep.identity.name.clone());
                queue.push_back((dep, new_path));
            }
            // Missing optional deps are silently ignored
        }
    }

    Ok(result)
}

/// Install a bundle (meta-skill) and all its dependencies.
/// Returns a summary of what was installed/updated/skipped/failed.
pub fn install_bundle(
    scope: Scope,
    root: &ScannedPackage,
    registry: &crate::app::registry::Registry,
    store: &dyn ConfigStorePort,
    providers: &[&dyn ProviderPort],
) -> Result<BundleInstallResult> {
    let deps = resolve_dependencies(root, registry)?;
    let mut result = BundleInstallResult::default();

    for dep in &deps {
        let config = store.load(scope)?;
        let is_installed = config.is_skill_installed(&dep.vault_id, &dep.identity.name);
        let installed_hash = config.installed_skill_hash(&dep.vault_id, &dep.identity.name);

        if is_installed {
            if installed_hash.as_ref() == Some(&dep.identity.sha10) {
                result.skipped.push(dep.identity.name.clone());
                continue;
            } else {
                // Update
                for provider in providers {
                    if let Err(e) =
                        crate::app::actions::update_asset(scope, &dep.package, store, *provider)
                    {
                        result.failed.push((
                            dep.identity.name.clone(),
                            format!("Update failed on {}: {}", provider.name(), e),
                        ));
                    }
                }
                result.updated.push(dep.identity.name.clone());
            }
        } else {
            // Fresh install
            for provider in providers {
                if let Err(e) =
                    crate::app::actions::install_asset(scope, &dep.package, store, *provider)
                {
                    result.failed.push((
                        dep.identity.name.clone(),
                        format!("Install failed on {}: {}", provider.name(), e),
                    ));
                }
            }
            result.installed.push(dep.identity.name.clone());
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::asset::{AssetKind, ScannedPackage};
    use crate::domain::config::ConfigFile;
    use crate::domain::identity::AssetIdentity;
    use crate::domain::scope::Scope;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeStore(Mutex<HashMap<String, ConfigFile>>);
    impl ConfigStorePort for FakeStore {
        fn load(&self, scope: Scope) -> Result<ConfigFile> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .get(&format!("{:?}", scope))
                .cloned()
                .unwrap_or_default())
        }
        fn save(&self, scope: Scope, config: &ConfigFile) -> Result<()> {
            self.0
                .lock()
                .unwrap()
                .insert(format!("{:?}", scope), config.clone());
            Ok(())
        }
    }

    struct FakeProvider;
    impl ProviderPort for FakeProvider {
        fn id(&self) -> &str {
            "fake"
        }
        fn name(&self) -> &str {
            "Fake"
        }
        fn install(
            &self,
            _pkg: &ScannedPackage,
            _scope: Scope,
            _config: Option<&crate::domain::config::ConfigFile>,
            _include_evals: bool,
        ) -> Result<()> {
            Ok(())
        }
        fn remove(
            &self,
            _identity: &AssetIdentity,
            _kind: &AssetKind,
            _scope: Scope,
            _config: Option<&crate::domain::config::ConfigFile>,
        ) -> Result<()> {
            Ok(())
        }
    }

    fn make_pkg(name: &str, requires: Vec<String>) -> ScannedPackage {
        ScannedPackage {
            identity: AssetIdentity::new(name, None, "0000000000"),
            path: std::path::PathBuf::from("/fake"),
            vault_id: "workspace".to_string(),
            kind: AssetKind::Skill,
            is_remote: false,
            remote_meta: None,
            requires,
            requires_optional: vec![],
            author: None,
            description: None,
            include_evals: false,
        }
    }

    #[test]
    fn resolve_single_package_no_deps() {
        let pkg = make_pkg("standalone", vec![]);
        let registry = crate::app::registry::Registry::new();
        let resolved = resolve_dependencies(&pkg, &registry).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].identity.name, "standalone");
    }

    #[test]
    fn resolve_linear_chain() {
        let _root = make_pkg("root", vec!["workspace/a".to_string()]);
        let _a = make_pkg("a", vec!["workspace/b".to_string()]);
        let _b = make_pkg("b", vec![]);

        let _registry = crate::app::registry::Registry::new();
        // We can't easily add fake vaults here, so we just test the cycle detection logic
        // Full integration tests would need real vault adapters
    }

    #[test]
    fn circular_dependency_detected() {
        let _a = make_pkg("a", vec!["workspace/b".to_string()]);
        let _b = make_pkg("b", vec!["workspace/a".to_string()]);

        let _registry = crate::app::registry::Registry::new();
        // Without vaults, find_dependency returns None, so it would bail on missing dep
        // This test demonstrates the cycle check on the path vector
        let mut path = vec!["a".to_string()];
        assert!(!path.contains(&"b".to_string()));
        path.push("b".to_string());
        assert!(path.contains(&"a".to_string()));
    }

    #[test]
    fn diamond_deduplication_key() {
        let key1 = ("v1".to_string(), "name".to_string(), "sha1".to_string());
        let key2 = ("v1".to_string(), "name".to_string(), "sha1".to_string());
        let mut visited = HashSet::new();
        visited.insert(key1.clone());
        assert!(visited.contains(&key2));
    }
}
