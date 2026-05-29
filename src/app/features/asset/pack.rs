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
            sink.on_error(format!("Asset '{}' not found in any vault", identity_str));
            return Ok(CoreOutcome::Ok);
        }
    };

    if pkg.kind != AssetKind::Skill {
        sink.on_error("Packing is only supported for Skills (not Instructions)".to_string());
        return Ok(CoreOutcome::Ok);
    }

    match target {
        PackTarget::ClaudeDesktop => {
            pack_claude_desktop(&pkg, stdout_flag, workspace, sink)?;
        }
        PackTarget::Firebender => {
            sink.on_error(
                "Firebender pack target not yet implemented. Use --target claude-desktop."
                    .to_string(),
            );
            return Ok(CoreOutcome::Ok);
        }
        PackTarget::Tarball => {
            pack_tarball(&pkg, stdout_flag, workspace, sink)?;
        }
    }

    Ok(CoreOutcome::Ok)
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
