use crate::domain::profile::ProfileAssetRef;

/// Result of comparing a local profile's refs against a vault-discovered version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileDiff {
    /// Skills present in local but not in vault.
    pub added_skills: Vec<ProfileAssetRef>,
    /// Skills present in vault but not in local.
    pub removed_skills: Vec<ProfileAssetRef>,
    /// MCPs present in local but not in vault.
    pub added_mcps: Vec<ProfileAssetRef>,
    /// MCPs present in vault but not in local.
    pub removed_mcps: Vec<ProfileAssetRef>,
    /// Instructions present in local but not in vault.
    pub added_instructions: Vec<ProfileAssetRef>,
    /// Instructions present in vault but not in local.
    pub removed_instructions: Vec<ProfileAssetRef>,
    /// Tools present in local but not in vault.
    pub added_tools: Vec<String>,
    /// Tools present in vault but not in local.
    pub removed_tools: Vec<String>,
    /// Permission mode differs.
    pub permission_mode_differs: bool,
}

impl ProfileDiff {
    /// Returns true if any differences were found.
    pub fn has_drift(&self) -> bool {
        !self.added_skills.is_empty()
            || !self.removed_skills.is_empty()
            || !self.added_mcps.is_empty()
            || !self.removed_mcps.is_empty()
            || !self.added_instructions.is_empty()
            || !self.removed_instructions.is_empty()
            || !self.added_tools.is_empty()
            || !self.removed_tools.is_empty()
            || self.permission_mode_differs
    }

    /// Returns a human-readable summary of the diff.
    pub fn summary(&self) -> String {
        if !self.has_drift() {
            return "No drift — local profile matches vault source.".to_string();
        }
        let mut lines = Vec::new();
        if !self.added_skills.is_empty() {
            let names: Vec<&str> = self.added_skills.iter().map(|r| r.name.as_str()).collect();
            lines.push(format!("  + skills: {}", names.join(", ")));
        }
        if !self.removed_skills.is_empty() {
            let names: Vec<&str> = self
                .removed_skills
                .iter()
                .map(|r| r.name.as_str())
                .collect();
            lines.push(format!("  - skills: {}", names.join(", ")));
        }
        if !self.added_mcps.is_empty() {
            let names: Vec<&str> = self.added_mcps.iter().map(|r| r.name.as_str()).collect();
            lines.push(format!("  + mcps: {}", names.join(", ")));
        }
        if !self.removed_mcps.is_empty() {
            let names: Vec<&str> = self.removed_mcps.iter().map(|r| r.name.as_str()).collect();
            lines.push(format!("  - mcps: {}", names.join(", ")));
        }
        if !self.added_instructions.is_empty() {
            let names: Vec<&str> = self
                .added_instructions
                .iter()
                .map(|r| r.name.as_str())
                .collect();
            lines.push(format!("  + instructions: {}", names.join(", ")));
        }
        if !self.removed_instructions.is_empty() {
            let names: Vec<&str> = self
                .removed_instructions
                .iter()
                .map(|r| r.name.as_str())
                .collect();
            lines.push(format!("  - instructions: {}", names.join(", ")));
        }
        if !self.added_tools.is_empty() {
            lines.push(format!("  + tools: {}", self.added_tools.join(", ")));
        }
        if !self.removed_tools.is_empty() {
            lines.push(format!("  - tools: {}", self.removed_tools.join(", ")));
        }
        if self.permission_mode_differs {
            lines.push("  ~ permission_mode differs".to_string());
        }
        format!("Profile drift detected:\n{}", lines.join("\n"))
    }
}

/// Compare two sets of `ProfileAssetRef` by name (ignoring vault ID).
///
/// Vault resolution is runtime-dependent, so we compare by identity name only.
fn diff_refs(
    local: &[ProfileAssetRef],
    vault: &[ProfileAssetRef],
) -> (Vec<ProfileAssetRef>, Vec<ProfileAssetRef>) {
    let local_names: std::collections::HashSet<&str> =
        local.iter().map(|r| r.name.as_str()).collect();
    let vault_names: std::collections::HashSet<&str> =
        vault.iter().map(|r| r.name.as_str()).collect();

    let added = local
        .iter()
        .filter(|r| !vault_names.contains(r.name.as_str()))
        .cloned()
        .collect();
    let removed = vault
        .iter()
        .filter(|r| !local_names.contains(r.name.as_str()))
        .cloned()
        .collect();

    (added, removed)
}

/// Compare two sets of tool strings by name.
fn diff_tools(local: &[String], vault: &[String]) -> (Vec<String>, Vec<String>) {
    let local_set: std::collections::HashSet<&str> = local.iter().map(|s| s.as_str()).collect();
    let vault_set: std::collections::HashSet<&str> = vault.iter().map(|s| s.as_str()).collect();

    let added = local
        .iter()
        .filter(|s| !vault_set.contains(s.as_str()))
        .cloned()
        .collect();
    let removed = vault
        .iter()
        .filter(|s| !local_set.contains(s.as_str()))
        .cloned()
        .collect();

    (added, removed)
}

/// Pure function: compute the diff between a local profile and a vault source.
///
/// Compares by identity name, ignoring vault ID (vault resolution is runtime).
#[allow(clippy::too_many_arguments)]
pub fn compute_diff(
    local_skills: &[ProfileAssetRef],
    vault_skills: &[ProfileAssetRef],
    local_mcps: &[ProfileAssetRef],
    vault_mcps: &[ProfileAssetRef],
    local_instructions: &[ProfileAssetRef],
    vault_instructions: &[ProfileAssetRef],
    local_tools: &[String],
    vault_tools: &[String],
    local_permission_mode: Option<&str>,
    vault_permission_mode: Option<&str>,
) -> ProfileDiff {
    let (added_skills, removed_skills) = diff_refs(local_skills, vault_skills);
    let (added_mcps, removed_mcps) = diff_refs(local_mcps, vault_mcps);
    let (added_instructions, removed_instructions) =
        diff_refs(local_instructions, vault_instructions);
    let (added_tools, removed_tools) = diff_tools(local_tools, vault_tools);
    let permission_mode_differs = local_permission_mode != vault_permission_mode;

    ProfileDiff {
        added_skills,
        removed_skills,
        added_mcps,
        removed_mcps,
        added_instructions,
        removed_instructions,
        added_tools,
        removed_tools,
        permission_mode_differs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_drift_when_identical() {
        let skills = vec![ProfileAssetRef::new("rust-patterns", "auto")];
        let diff = compute_diff(&skills, &skills, &[], &[], &[], &[], &[], &[], None, None);
        assert!(!diff.has_drift());
        assert!(diff.summary().contains("No drift"));
    }

    #[test]
    fn added_skill_detected() {
        let local = vec![ProfileAssetRef::new("rust-patterns", "auto")];
        let vault = vec![];
        let diff = compute_diff(&local, &vault, &[], &[], &[], &[], &[], &[], None, None);
        assert!(diff.has_drift());
        assert_eq!(diff.added_skills.len(), 1);
        assert_eq!(diff.added_skills[0].name, "rust-patterns");
        assert!(diff.removed_skills.is_empty());
    }

    #[test]
    fn removed_skill_detected() {
        let local = vec![];
        let vault = vec![ProfileAssetRef::new("docker", "clawhub")];
        let diff = compute_diff(&local, &vault, &[], &[], &[], &[], &[], &[], None, None);
        assert!(diff.has_drift());
        assert!(diff.added_skills.is_empty());
        assert_eq!(diff.removed_skills.len(), 1);
        assert_eq!(diff.removed_skills[0].name, "docker");
    }

    #[test]
    fn same_name_different_vault_is_no_drift() {
        let local = vec![ProfileAssetRef::new("rust-patterns", "auto")];
        let vault = vec![ProfileAssetRef::new("rust-patterns", "clawhub")];
        let diff = compute_diff(&local, &vault, &[], &[], &[], &[], &[], &[], None, None);
        assert!(
            !diff.has_drift(),
            "same name with different vault should not count as drift"
        );
    }

    #[test]
    fn permission_mode_diff_detected() {
        let diff = compute_diff(
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            Some("auto"),
            Some("plan"),
        );
        assert!(diff.has_drift());
        assert!(diff.permission_mode_differs);
    }

    #[test]
    fn tools_diff_detected() {
        let local = vec!["Read".to_string(), "Write".to_string()];
        let vault = vec!["Read".to_string(), "Glob".to_string()];
        let diff = compute_diff(&[], &[], &[], &[], &[], &[], &local, &vault, None, None);
        assert!(diff.has_drift());
        assert_eq!(diff.added_tools, vec!["Write"]);
        assert_eq!(diff.removed_tools, vec!["Glob"]);
    }

    #[test]
    fn summary_format() {
        let local_skills = vec![ProfileAssetRef::new("new-skill", "auto")];
        let vault_mcps = vec![ProfileAssetRef::new("old-mcp", "auto")];
        let diff = compute_diff(
            &local_skills,
            &[],
            &[],
            &vault_mcps,
            &[],
            &[],
            &[],
            &[],
            Some("plan"),
            None,
        );
        let summary = diff.summary();
        assert!(summary.contains("+ skills: new-skill"));
        assert!(summary.contains("- mcps: old-mcp"));
        assert!(summary.contains("permission_mode differs"));
    }
}
