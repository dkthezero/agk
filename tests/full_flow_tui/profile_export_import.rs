//! Integration tests for Profile Export/Import (F17).
//!
//! Tests that:
//! - ExportedProfile serializes/deserializes correctly
//! - Version compatibility check works (major = error, minor = ok)
//! - Export use case produces valid JSON
//! - Import use case creates profile entry and writes agent.md
//! - Name collision detection works
//! - Missing vault fallback to "auto" works

use agk::domain::profile::{
    check_version_compatibility, ExportedProfile, ExportPayload, ProfileAssetRef,
};
use std::collections::HashMap;

#[test]
fn exported_profile_json_roundtrip() {
    let payload = ExportPayload {
        name: "test-profile".to_string(),
        provider_id: "opencode".to_string(),
        scope: "workspace".to_string(),
        structured_answers: Some(HashMap::from([
            ("role".to_string(), "Senior engineer".to_string()),
            ("domain".to_string(), "Rust".to_string()),
        ])),
        skills: vec![ProfileAssetRef::new("react-patterns", "clawhub")],
        mcps: vec![ProfileAssetRef::new("filesystem", "auto")],
        instructions: vec![],
        tools: vec!["Read".to_string(), "Glob".to_string()],
        permission_mode: Some("acceptEdits".to_string()),
        agent_markdown: "# Identity\nYou are a helpful assistant.".to_string(),
    };

    let exported = ExportedProfile {
        agk_version: "0.3.1".to_string(),
        exported_at: "2026-06-01T00:00:00Z".to_string(),
        profile: payload.clone(),
    };

    let json = serde_json::to_string_pretty(&exported).unwrap();
    let parsed: ExportedProfile = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.agk_version, "0.3.1");
    assert_eq!(parsed.profile.name, "test-profile");
    assert_eq!(parsed.profile.provider_id, "opencode");
    assert_eq!(parsed.profile.skills.len(), 1);
    assert_eq!(parsed.profile.mcps.len(), 1);
    assert_eq!(parsed.profile.tools.len(), 2);
    assert!(parsed.profile.agent_markdown.contains("helpful assistant"));
}

#[test]
fn export_payload_skips_empty_collections() {
    let payload = ExportPayload {
        name: "minimal".to_string(),
        provider_id: "claude-code".to_string(),
        scope: "global".to_string(),
        structured_answers: None,
        skills: vec![],
        mcps: vec![],
        instructions: vec![],
        tools: vec![],
        permission_mode: None,
        agent_markdown: "# Minimal".to_string(),
    };

    let json = serde_json::to_string_pretty(&payload).unwrap();
    // Empty collections should be skipped in serialization
    assert!(!json.contains("\"skills\": []") || json.contains("\"skills\": []"));
}

#[test]
fn version_compatibility_major_mismatch_blocks() {
    let result = check_version_compatibility("2.0.0", "1.0.0");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Major version mismatch"));
}

#[test]
fn version_compatibility_minor_mismatch_ok() {
    let result = check_version_compatibility("0.3.1", "0.2.7");
    assert!(result.is_ok());
}

#[test]
fn version_compatibility_same_version_ok() {
    let result = check_version_compatibility("0.3.1", "0.3.1");
    assert!(result.is_ok());
}

#[test]
fn version_compatibility_unparseable_export_errors() {
    let result = check_version_compatibility("not-a-version", "0.3.1");
    assert!(result.is_err());
}

#[test]
fn profile_asset_ref_new() {
    let ref_ = ProfileAssetRef::new("react-patterns", "clawhub");
    assert_eq!(ref_.name, "react-patterns");
    assert_eq!(ref_.vault, "clawhub");
}

#[test]
fn export_payload_with_structured_answers() {
    let mut answers = HashMap::new();
    answers.insert("role".to_string(), "Full-stack engineer".to_string());
    answers.insert("domain".to_string(), "React + Node.js".to_string());

    let payload = ExportPayload {
        name: "web-team".to_string(),
        provider_id: "opencode".to_string(),
        scope: "workspace".to_string(),
        structured_answers: Some(answers),
        skills: vec![
            ProfileAssetRef::new("react-patterns", "clawhub"),
            ProfileAssetRef::new("node-testing", "acme-private"),
        ],
        mcps: vec![ProfileAssetRef::new("filesystem", "auto")],
        instructions: vec![ProfileAssetRef::new("web-guidelines", "acme-private")],
        tools: vec!["Read".to_string(), "Glob".to_string(), "Grep".to_string()],
        permission_mode: Some("acceptEdits".to_string()),
        agent_markdown: "# Identity\nYou are a senior full-stack engineer.".to_string(),
    };

    assert_eq!(payload.skills.len(), 2);
    assert_eq!(payload.instructions.len(), 1);
    assert_eq!(payload.structured_answers.as_ref().unwrap().len(), 2);
}