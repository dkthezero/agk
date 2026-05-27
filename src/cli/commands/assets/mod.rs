use super::*;
use crate::cli::entry::{Cli, PackTarget};
use crate::domain::asset::{AssetKind, ScannedPackage};
use anyhow::Result;

pub use install::cmd_install;
pub use sync::cmd_sync;
pub use validate::cmd_validate;

mod install;
mod remove;
mod search;
mod sync;
mod validate;

// -- Pack helpers (shared) --

pub fn cmd_pack(
    cli: &Cli,
    identity_str: &str,
    target: PackTarget,
    stdout_flag: bool,
    workspace: &std::path::Path,
) -> Result<i32> {
    let mode = OutputMode::from_cli(cli);

    let (registry, _scan, _store) = crate::app::bootstrap::build(workspace.to_path_buf())?;
    let pkg = match find_package_by_full_identity(&registry, identity_str)? {
        Some(p) => p,
        None => {
            eprintln_if_not_quiet(
                &mode,
                &format!("Asset '{}' not found in any vault", identity_str),
            );
            return Ok(EXIT_GENERAL_FAILURE);
        }
    };

    if pkg.kind != AssetKind::Skill {
        eprintln_if_not_quiet(
            &mode,
            "Packing is only supported for Skills (not Instructions)",
        );
        return Ok(EXIT_GENERAL_FAILURE);
    }

    match target {
        PackTarget::ClaudeDesktop => {
            pack_claude_desktop(&mode, &pkg, stdout_flag, workspace)?;
        }
        PackTarget::Firebender => {
            eprintln_if_not_quiet(
                &mode,
                "Firebender pack target not yet implemented. Use --target claude-desktop.",
            );
            return Ok(EXIT_GENERAL_FAILURE);
        }
        PackTarget::Tarball => {
            pack_tarball(&mode, &pkg, stdout_flag, workspace)?;
        }
    }

    Ok(EXIT_SUCCESS)
}

pub fn pack_claude_desktop(
    mode: &OutputMode,
    pkg: &ScannedPackage,
    stdout_flag: bool,
    workspace: &std::path::Path,
) -> Result<()> {
    use std::io::Write;

    let out_dir = workspace.join(".agk").join("pack");
    std::fs::create_dir_all(&out_dir)?;
    let out_path = out_dir.join(format!("{}-claude-desktop.zip", pkg.identity.name));

    let file = std::fs::File::create(&out_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    fn add_dir_to_zip(
        zip: &mut zip::ZipWriter<std::fs::File>,
        base_path: &std::path::Path,
        prefix: &str,
        options: zip::write::SimpleFileOptions,
    ) -> Result<()> {
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

    add_dir_to_zip(&mut zip, &pkg.path, &pkg.identity.name, options)?;
    zip.finish()?;

    if stdout_flag {
        let bytes = std::fs::read(&out_path)?;
        std::io::stdout().write_all(&bytes)?;
    } else {
        println_if_not_quiet(
            mode,
            &format!("Packed '{}' to {}", pkg.identity.name, out_path.display()),
        );
    }
    Ok(())
}

pub fn pack_tarball(
    mode: &OutputMode,
    pkg: &ScannedPackage,
    stdout_flag: bool,
    workspace: &std::path::Path,
) -> Result<()> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    use tar::Builder;

    let out_dir = workspace.join(".agk").join("pack");
    std::fs::create_dir_all(&out_dir)?;
    let out_path = out_dir.join(format!(
        "{}-{}.tar.gz",
        pkg.identity.name, pkg.identity.sha10
    ));

    let file = std::fs::File::create(&out_path)?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(enc);
    tar.append_dir_all(&pkg.identity.name, &pkg.path)?;
    let enc = tar.into_inner()?;
    enc.finish()?;

    if stdout_flag {
        let bytes = std::fs::read(&out_path)?;
        std::io::stdout().write_all(&bytes)?;
    } else {
        println_if_not_quiet(
            mode,
            &format!("Packed '{}' to {}", pkg.identity.name, out_path.display()),
        );
    }
    Ok(())
}
