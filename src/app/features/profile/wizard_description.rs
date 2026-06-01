use std::collections::HashMap;

/// Compose the canonical structured markdown body from wizard answers.
///
/// Expected keys: role, domain, audience, responsibilities, style, format,
/// boundaries, triggers, constraints.
pub fn compose_description(answers: &HashMap<String, String>) -> String {
    let mut lines: Vec<String> = Vec::new();

    let role = answers.get("role").map(|s| s.as_str()).unwrap_or("");
    let domain = answers.get("domain").map(|s| s.as_str()).unwrap_or("");
    let audience = answers.get("audience").map(|s| s.as_str()).unwrap_or("");

    if !role.is_empty() || !domain.is_empty() || !audience.is_empty() {
        lines.push("# Identity".to_string());
        if !role.is_empty() && !domain.is_empty() {
            lines.push(format!("You are a {} specializing in {}.", role, domain));
        } else if !role.is_empty() {
            lines.push(format!("You are a {}.", role));
        } else if !domain.is_empty() {
            lines.push(format!("You are an expert in {}.", domain));
        }
        if !audience.is_empty() {
            lines.push(format!("You work with {}.", audience));
        }
        lines.push(String::new());
    }

    let responsibilities = answers
        .get("responsibilities")
        .map(|s| s.as_str())
        .unwrap_or("");
    if !responsibilities.is_empty() {
        lines.push("# Core Responsibilities".to_string());
        for (i, line) in responsibilities.lines().enumerate() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                lines.push(format!("{}. {}", i + 1, trimmed));
            }
        }
        lines.push(String::new());
    }

    let style = answers.get("style").map(|s| s.as_str()).unwrap_or("");
    if !style.is_empty() {
        lines.push("# Collaboration Style".to_string());
        lines.push(style.to_string());
        lines.push(String::new());
    }

    let format = answers.get("format").map(|s| s.as_str()).unwrap_or("");
    if !format.is_empty() {
        lines.push("# Output Format".to_string());
        lines.push(format.to_string());
        lines.push(String::new());
    }

    let boundaries = answers.get("boundaries").map(|s| s.as_str()).unwrap_or("");
    if !boundaries.is_empty() {
        lines.push("# Scope Boundaries".to_string());
        lines.push(boundaries.to_string());
        lines.push(String::new());
    }

    let triggers = answers.get("triggers").map(|s| s.as_str()).unwrap_or("");
    if !triggers.is_empty() {
        lines.push("# Activation Triggers".to_string());
        lines.push(triggers.to_string());
        lines.push(String::new());
    }

    let constraints = answers.get("constraints").map(|s| s.as_str()).unwrap_or("");
    if !constraints.is_empty() {
        lines.push("# Constraints".to_string());
        lines.push(constraints.to_string());
        lines.push(String::new());
    }

    lines.join("\n").trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_with_all_fields() {
        let mut answers = HashMap::new();
        answers.insert("role".into(), "Senior Rust engineer".into());
        answers.insert("domain".into(), "async CLI tooling".into());
        answers.insert("audience".into(), "my team".into());
        answers.insert(
            "responsibilities".into(),
            "Review PRs\nSuggest idioms".into(),
        );
        answers.insert("style".into(), "Direct".into());
        answers.insert("format".into(), "Bullets".into());
        answers.insert("boundaries".into(), "No prod deploys".into());
        answers.insert("triggers".into(), "After code changes".into());
        answers.insert("constraints".into(), "Always run cargo fmt".into());

        let md = compose_description(&answers);
        assert!(md.contains("# Identity"));
        assert!(md.contains("Senior Rust engineer"));
        assert!(md.contains("# Core Responsibilities"));
        assert!(md.contains("1. Review PRs"));
        assert!(md.contains("# Collaboration Style"));
        assert!(md.contains("# Output Format"));
        assert!(md.contains("# Scope Boundaries"));
        assert!(md.contains("# Activation Triggers"));
        assert!(md.contains("# Constraints"));
    }

    #[test]
    fn compose_empty_returns_empty() {
        let answers = HashMap::new();
        let md = compose_description(&answers);
        assert!(md.is_empty());
    }

    #[test]
    fn compose_partial_skips_missing_sections() {
        let mut answers = HashMap::new();
        answers.insert("role".into(), "Reviewer".into());
        let md = compose_description(&answers);
        assert!(md.contains("# Identity"));
        assert!(!md.contains("# Core Responsibilities"));
    }
}
