#![cfg(feature = "pack")]

use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use crate::app::registry::Registry;
use crate::domain::asset::{AssetKind, PackTarget, ScannedPackage};
use std::io::Write;
use std::path::Path;

/// Pack a skill into a provider-specific distributable.
///
/// Emits [`CoreEvent::Info`] with the output path on success.
pub fn run(
    identity_str: &str,
    target: PackTarget,
    stdout_flag: bool,
    _scope: crate::domain::scope::Scope,
    registry: &Registry,
    workspace: &Path,
    sink: &mut dyn CoreEventSink,
) -> CoreResult {
    let pkg = match registry.find_package_by_identity(identity_str)? {
        Some(p) => p,
        None => {
            return Err(anyhow::anyhow!(
                "Asset '{}' not found in any vault",
                identity_str
            ));
        }
    };

    if pkg.kind != AssetKind::Skill {
        return Err(anyhow::anyhow!(
            "Packing is only supported for Skills (not Instructions)"
        ));
    }

    match target {
        PackTarget::ClaudeDesktop => {
            pack_claude_desktop(&pkg, stdout_flag, workspace, sink)?;
        }
        PackTarget::Firebender => {
            pack_firebender(&pkg, stdout_flag, workspace, sink)?;
        }
        PackTarget::Tarball => {
            pack_tarball(&pkg, stdout_flag, workspace, sink)?;
        }
    }

    Ok(CoreOutcome::Ok)
}

/// Pack a skill into a Firebender JSON manifest.
///
/// Emits a JSON document describing the skill identity, metadata, and the
/// full file tree (each file embedded as base64) so a Firebender-compatible
/// runtime can materialize the skill under `.firebender/skills/{name}/`.
fn pack_firebender(
    pkg: &ScannedPackage,
    stdout_flag: bool,
    workspace: &Path,
    sink: &mut dyn CoreEventSink,
) -> anyhow::Result<()> {
    let out_dir = workspace.join(".agk").join("pack");
    std::fs::create_dir_all(&out_dir)?;
    let out_path = out_dir.join(format!("{}-firebender.json", pkg.identity.name));

    let manifest = build_firebender_manifest(pkg)?;

    let json = serde_json::to_string_pretty(&manifest)?;

    if stdout_flag {
        let bytes = json.into_bytes();
        std::io::stdout().write_all(&bytes)?;
    } else {
        std::fs::write(&out_path, &json)?;
        sink.on_event(CoreEvent::Info(format!(
            "Packed '{}' to {}",
            pkg.identity.name,
            out_path.display()
        )));
    }
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct FirebenderManifest {
    schema_version: u32,
    kind: String,
    identity: FirebenderIdentity,
    vault_id: String,
    author: Option<String>,
    description: Option<String>,
    requires: Vec<String>,
    requires_optional: Vec<String>,
    files: Vec<FirebenderFile>,
}

#[derive(Debug, serde::Serialize)]
struct FirebenderIdentity {
    name: String,
    version: Option<String>,
    sha10: String,
}

#[derive(Debug, serde::Serialize)]
struct FirebenderFile {
    path: String,
    content_base64: String,
}

fn build_firebender_manifest(pkg: &ScannedPackage) -> anyhow::Result<FirebenderManifest> {
    let kind = match pkg.kind {
        AssetKind::Skill => "skill",
        AssetKind::Instruction => "instruction",
        AssetKind::McpServer => "mcp_server",
        AssetKind::Profile => "profile",
    };

    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(&pkg.path) {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let relative = path.strip_prefix(&pkg.path)?;
        let bytes = std::fs::read(path)?;
        files.push(FirebenderFile {
            path: relative.display().to_string(),
            content_base64: base64_encode(&bytes),
        });
    }

    Ok(FirebenderManifest {
        schema_version: 1,
        kind: kind.to_string(),
        identity: FirebenderIdentity {
            name: pkg.identity.name.clone(),
            version: pkg.identity.version.clone(),
            sha10: pkg.identity.sha10.clone(),
        },
        vault_id: pkg.vault_id.clone(),
        author: pkg.author.clone(),
        description: pkg.description.clone(),
        requires: pkg.requires.clone(),
        requires_optional: pkg.requires_optional.clone(),
        files,
    })
}

/// Standard base64 encoder (RFC 4648) with padding, no line wrapping.
fn base64_encode(input: &[u8]) -> String {
    const ALPHA: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    // Each 3-byte chunk encodes to 4 chars; a remainder of 1→4 (2 data + "==")
    // or 2→4 (3 data + "="). So the padded output length is 4 * ceil(len/3).
    let mut out = String::with_capacity(4 * input.len().div_ceil(3));
    let mut chunks = input.chunks_exact(3);
    for chunk in &mut chunks {
        let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        out.push(ALPHA[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPHA[((n >> 12) & 0x3f) as usize] as char);
        out.push(ALPHA[((n >> 6) & 0x3f) as usize] as char);
        out.push(ALPHA[(n & 0x3f) as usize] as char);
    }
    let rem = chunks.remainder();
    match rem.len() {
        1 => {
            let n = (rem[0] as u32) << 16;
            out.push(ALPHA[((n >> 18) & 0x3f) as usize] as char);
            out.push(ALPHA[((n >> 12) & 0x3f) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let n = ((rem[0] as u32) << 16) | ((rem[1] as u32) << 8);
            out.push(ALPHA[((n >> 18) & 0x3f) as usize] as char);
            out.push(ALPHA[((n >> 12) & 0x3f) as usize] as char);
            out.push(ALPHA[((n >> 6) & 0x3f) as usize] as char);
            out.push('=');
        }
        _ => {}
    }
    out
}

#[cfg(test)]
mod base64_tests {
    use super::base64_encode;

    #[test]
    fn base64_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }
}

#[cfg(test)]
mod firebender_tests {
    use super::*;
    use crate::app::outcome::NullSink;
    use crate::domain::identity::AssetIdentity;

    fn make_skill_pkg(dir: &std::path::Path) -> ScannedPackage {
        let skill_dir = dir.join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# My Skill\nbody").unwrap();
        std::fs::create_dir_all(skill_dir.join("scripts")).unwrap();
        std::fs::write(skill_dir.join("scripts/run.sh"), b"#!/bin/sh\necho hi\n").unwrap();

        ScannedPackage {
            identity: AssetIdentity::new("my-skill", Some("1.0.0".to_string()), "abc1234567"),
            path: skill_dir,
            vault_id: "workspace".to_string(),
            kind: AssetKind::Skill,
            is_remote: false,
            remote_meta: None,
            requires: vec!["other-skill".to_string()],
            requires_optional: vec![],
            author: Some("tester".to_string()),
            description: Some("a skill".to_string()),
            include_evals: false,
        }
    }

    #[test]
    fn firebender_manifest_includes_identity_metadata_and_files() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg = make_skill_pkg(tmp.path());

        let manifest = build_firebender_manifest(&pkg).unwrap();

        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.kind, "skill");
        assert_eq!(manifest.identity.name, "my-skill");
        assert_eq!(manifest.identity.version.as_deref(), Some("1.0.0"));
        assert_eq!(manifest.identity.sha10, "abc1234567");
        assert_eq!(manifest.vault_id, "workspace");
        assert_eq!(manifest.author.as_deref(), Some("tester"));
        assert_eq!(manifest.description.as_deref(), Some("a skill"));
        assert_eq!(manifest.requires, vec!["other-skill".to_string()]);
        assert!(manifest.requires_optional.is_empty());

        let paths: Vec<_> = manifest.files.iter().map(|f| f.path.clone()).collect();
        assert!(paths.contains(&"SKILL.md".to_string()));
        assert!(paths.contains(&"scripts/run.sh".to_string()));
        assert_eq!(manifest.files.len(), 2);

        let skill_file = manifest
            .files
            .iter()
            .find(|f| f.path == "SKILL.md")
            .unwrap();
        assert_eq!(
            skill_file.content_base64,
            base64_encode(b"# My Skill\nbody")
        );
    }

    #[test]
    fn firebender_manifest_serializes_to_valid_json() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg = make_skill_pkg(tmp.path());
        let manifest = build_firebender_manifest(&pkg).unwrap();
        let json = serde_json::to_string(&manifest).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["schema_version"], 1);
        assert_eq!(v["kind"], "skill");
        assert_eq!(v["identity"]["name"], "my-skill");
        assert!(v["files"].is_array());
        assert_eq!(v["files"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn pack_firebender_writes_json_file_and_emits_info() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg = make_skill_pkg(tmp.path());
        let workspace = tmp.path();

        let mut sink = NullSink;
        pack_firebender(&pkg, false, workspace, &mut sink).unwrap();

        let out = workspace
            .join(".agk")
            .join("pack")
            .join("my-skill-firebender.json");
        assert!(out.exists(), "json output file should exist");
        let content = std::fs::read_to_string(&out).unwrap();
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(v["identity"]["name"], "my-skill");
    }

    #[test]
    fn pack_firebender_stdout_emits_json_to_stdout() {
        // Cannot easily capture stdout in a unit test without redirecting;
        // exercise build_firebender_manifest path used by stdout mode.
        let tmp = tempfile::tempdir().unwrap();
        let pkg = make_skill_pkg(tmp.path());
        let manifest = build_firebender_manifest(&pkg).unwrap();
        let json = serde_json::to_string_pretty(&manifest).unwrap();
        assert!(json.contains("\"schema_version\": 1"));
        assert!(json.contains("\"my-skill\""));
    }

    #[test]
    fn run_firebender_target_succeeds_via_registry() {
        // Seed a FakeVault with a ScannedPackage pointing at a real tempdir skill
        // so the pack use case can walk real files on disk.
        use crate::app::test_support::fake_vault::FakeVault;

        let tmp = tempfile::tempdir().unwrap();
        let skill_dir = tmp.path().join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), "# My Skill").unwrap();

        let mut registry = Registry::new();
        let vault = FakeVault::new("workspace");
        vault.seed(ScannedPackage {
            identity: AssetIdentity::new("my-skill", None, "abc1234567"),
            path: skill_dir,
            vault_id: "workspace".to_string(),
            kind: AssetKind::Skill,
            is_remote: false,
            remote_meta: None,
            requires: vec![],
            requires_optional: vec![],
            author: None,
            description: None,
            include_evals: false,
        });
        registry.register_vault(Box::new(vault));
        registry.register_feature_set(Box::new(crate::infra::feature::skill::SkillFeatureSet));

        let workspace = tmp.path().join("ws");
        std::fs::create_dir_all(&workspace).unwrap();

        let mut sink = NullSink;
        let result = run(
            "workspace/my-skill",
            PackTarget::Firebender,
            false,
            crate::domain::scope::Scope::Workspace,
            &registry,
            &workspace,
            &mut sink,
        );
        assert!(result.is_ok(), "pack run should succeed: {:?}", result);
        let out = workspace
            .join(".agk")
            .join("pack")
            .join("my-skill-firebender.json");
        assert!(out.exists(), "firebender json pack file should be written");
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
        assert_eq!(v["kind"], "skill");
        assert_eq!(v["identity"]["name"], "my-skill");
    }
}

fn pack_claude_desktop(
    pkg: &ScannedPackage,
    stdout_flag: bool,
    workspace: &Path,
    sink: &mut dyn CoreEventSink,
) -> anyhow::Result<()> {
    let out_dir = workspace.join(".agk").join("pack");
    std::fs::create_dir_all(&out_dir)?;
    let out_path = out_dir.join(format!("{}-claude-desktop.zip", pkg.identity.name));

    let file = std::fs::File::create(&out_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    add_dir_to_zip(&mut zip, &pkg.path, &pkg.identity.name, options)?;
    zip.finish()?;

    if stdout_flag {
        let bytes = std::fs::read(&out_path)?;
        std::io::stdout().write_all(&bytes)?;
    } else {
        sink.on_event(CoreEvent::Info(format!(
            "Packed '{}' to {}",
            pkg.identity.name,
            out_path.display()
        )));
    }
    Ok(())
}

fn pack_tarball(
    pkg: &ScannedPackage,
    stdout_flag: bool,
    workspace: &Path,
    sink: &mut dyn CoreEventSink,
) -> anyhow::Result<()> {
    let out_dir = workspace.join(".agk").join("pack");
    std::fs::create_dir_all(&out_dir)?;
    let out_path = out_dir.join(format!(
        "{}-{}.tar.gz",
        pkg.identity.name, pkg.identity.sha10
    ));

    let file = std::fs::File::create(&out_path)?;
    let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.append_dir_all(&pkg.identity.name, &pkg.path)?;
    let enc = tar.into_inner()?;
    enc.finish()?;

    if stdout_flag {
        let bytes = std::fs::read(&out_path)?;
        std::io::stdout().write_all(&bytes)?;
    } else {
        sink.on_event(CoreEvent::Info(format!(
            "Packed '{}' to {}",
            pkg.identity.name,
            out_path.display()
        )));
    }
    Ok(())
}

fn add_dir_to_zip(
    zip: &mut zip::ZipWriter<std::fs::File>,
    base_path: &Path,
    prefix: &str,
    options: zip::write::SimpleFileOptions,
) -> anyhow::Result<()> {
    for entry in walkdir::WalkDir::new(base_path) {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(base_path)?;
        let zip_path = format!("{}/{}", prefix, relative.display());

        if path.is_file() {
            zip.start_file(&zip_path, options)?;
            let content = std::fs::read(path)?;
            zip.write_all(&content)?;
        }
    }
    Ok(())
}
