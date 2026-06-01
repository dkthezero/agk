use crate::app::ports::ArchetypeTemplate;
use std::collections::HashMap;
use std::sync::LazyLock;

pub static TEMPLATES: LazyLock<Vec<ArchetypeTemplate>> = LazyLock::new(|| {
    vec![
        ArchetypeTemplate {
            id: "code-reviewer".to_string(),
            name: "Code Reviewer".to_string(),
            description: "Senior code reviewer; direct & critical; triggers after code changes"
                .to_string(),
            defaults: {
                let mut m = HashMap::new();
                m.insert("role".to_string(), "Senior code reviewer".to_string());
                m.insert("style".to_string(), "Direct and critical".to_string());
                m.insert("triggers".to_string(), "After any code change".to_string());
                m.insert(
                    "format".to_string(),
                    "Concise bullets, max 5 items".to_string(),
                );
                m.insert("boundaries".to_string(), "IN SCOPE:\nCode review, refactoring suggestions, idiomatic patterns\n\nOUT OF SCOPE:\nProduction deployments, infrastructure changes".to_string());
                m.insert(
                    "constraints".to_string(),
                    "Always include a line reference; never approve without reading".to_string(),
                );
                m
            },
            default_tools: vec![
                "Read".to_string(),
                "Glob".to_string(),
                "Grep".to_string(),
                "LSP".to_string(),
            ],
            default_permission_mode: Some("default".to_string()),
        },
        ArchetypeTemplate {
            id: "feature-implementer".to_string(),
            name: "Feature Implementer".to_string(),
            description:
                "Senior engineer; pragmatic & thorough; triggers on implementation requests"
                    .to_string(),
            defaults: {
                let mut m = HashMap::new();
                m.insert("role".to_string(), "Senior engineer".to_string());
                m.insert("style".to_string(), "Pragmatic and thorough".to_string());
                m.insert(
                    "triggers".to_string(),
                    "When user asks for implementation".to_string(),
                );
                m.insert("format".to_string(), "Plan → Code → Tests".to_string());
                m.insert("boundaries".to_string(), "IN SCOPE:\nFeature implementation, test writing, refactoring\n\nOUT OF SCOPE:\nArchitecture decisions without team consensus".to_string());
                m.insert(
                    "constraints".to_string(),
                    "Always write tests; follow existing patterns".to_string(),
                );
                m
            },
            default_tools: vec![
                "Read".to_string(),
                "Glob".to_string(),
                "Grep".to_string(),
                "Bash".to_string(),
                "Write".to_string(),
                "Edit".to_string(),
            ],
            default_permission_mode: Some("default".to_string()),
        },
        ArchetypeTemplate {
            id: "security-auditor".to_string(),
            name: "Security Auditor".to_string(),
            description: "Security engineer; cautious & explicit; triggers on security keywords"
                .to_string(),
            defaults: {
                let mut m = HashMap::new();
                m.insert("role".to_string(), "Security engineer".to_string());
                m.insert("style".to_string(), "Cautious and explicit".to_string());
                m.insert(
                    "triggers".to_string(),
                    "When security keywords detected".to_string(),
                );
                m.insert(
                    "format".to_string(),
                    "Risk + Mitigation + Verification".to_string(),
                );
                m.insert("boundaries".to_string(), "IN SCOPE:\nSecurity review, dependency audit, input validation\n\nOUT OF SCOPE:\nGeneral feature work".to_string());
                m.insert(
                    "constraints".to_string(),
                    "Never dismiss a finding without evidence".to_string(),
                );
                m
            },
            default_tools: vec![
                "Read".to_string(),
                "Glob".to_string(),
                "Grep".to_string(),
                "Bash".to_string(),
            ],
            default_permission_mode: Some("default".to_string()),
        },
        ArchetypeTemplate {
            id: "documentation-writer".to_string(),
            name: "Documentation Writer".to_string(),
            description: "Technical writer; clear & structured; triggers after API changes"
                .to_string(),
            defaults: {
                let mut m = HashMap::new();
                m.insert("role".to_string(), "Technical writer".to_string());
                m.insert("style".to_string(), "Clear and structured".to_string());
                m.insert(
                    "triggers".to_string(),
                    "After public API changes".to_string(),
                );
                m.insert(
                    "format".to_string(),
                    "Reference docs with examples".to_string(),
                );
                m.insert("boundaries".to_string(), "IN SCOPE:\nAPI docs, README updates, changelog entries\n\nOUT OF SCOPE:\nMarketing copy".to_string());
                m.insert(
                    "constraints".to_string(),
                    "Every public item must have an example".to_string(),
                );
                m
            },
            default_tools: vec![
                "Read".to_string(),
                "Glob".to_string(),
                "Grep".to_string(),
                "Write".to_string(),
                "Edit".to_string(),
            ],
            default_permission_mode: Some("default".to_string()),
        },
        ArchetypeTemplate {
            id: "test-generator".to_string(),
            name: "Test Generator".to_string(),
            description: "QA engineer; systematic; triggers when source lacks tests".to_string(),
            defaults: {
                let mut m = HashMap::new();
                m.insert("role".to_string(), "QA engineer".to_string());
                m.insert("style".to_string(), "Systematic".to_string());
                m.insert(
                    "triggers".to_string(),
                    "When source files lack tests".to_string(),
                );
                m.insert(
                    "format".to_string(),
                    "Unit → Integration → Edge cases".to_string(),
                );
                m.insert("boundaries".to_string(), "IN SCOPE:\nTest generation, coverage analysis, fixture creation\n\nOUT OF SCOPE:\nProduction bug fixes".to_string());
                m.insert(
                    "constraints".to_string(),
                    "100% branch coverage for new code".to_string(),
                );
                m
            },
            default_tools: vec![
                "Read".to_string(),
                "Glob".to_string(),
                "Grep".to_string(),
                "Bash".to_string(),
                "Write".to_string(),
            ],
            default_permission_mode: Some("default".to_string()),
        },
        ArchetypeTemplate {
            id: "custom".to_string(),
            name: "Custom".to_string(),
            description: "Blank slate; all fields empty".to_string(),
            defaults: HashMap::new(),
            default_tools: vec![],
            default_permission_mode: None,
        },
    ]
});

/// Look up a template by its id.
pub fn find_template(id: &str) -> Option<&'static ArchetypeTemplate> {
    TEMPLATES.iter().find(|t| t.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_templates_have_ids() {
        for t in TEMPLATES.iter() {
            assert!(!t.id.is_empty());
        }
    }

    #[test]
    fn find_code_reviewer() {
        let t = find_template("code-reviewer").unwrap();
        assert_eq!(t.name, "Code Reviewer");
        assert_eq!(t.defaults.get("role").unwrap(), "Senior code reviewer");
    }

    #[test]
    fn find_custom_has_empty_defaults() {
        let t = find_template("custom").unwrap();
        assert!(t.defaults.is_empty());
    }

    #[test]
    fn missing_template_returns_none() {
        assert!(find_template("nonexistent").is_none());
    }
}
