use super::*;
use anyhow::Result;

// ---------------------------------------------------------------------------
// Command: profile start
// ---------------------------------------------------------------------------

pub fn run_profile_start(name: &str, workspace: &std::path::Path) -> Result<i32> {
    let (registry, _scan, store) = crate::app::bootstrap::build(workspace.to_path_buf())?;
    let config = store.load(crate::domain::scope::Scope::Workspace)?;
    let profile = config
        .find_profile(name)
        .cloned()
        .or_else(|| {
            store
                .load(crate::domain::scope::Scope::Global)
                .ok()?
                .find_profile(name)
                .cloned()
        })
        .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", name))?;

    let provider = registry
        .providers
        .iter()
        .find(|p| p.id() == profile.provider_id)
        .and_then(|p| {
            if p.supports_profiles() {
                Some(p.as_ref())
            } else {
                None
            }
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Provider '{}' does not support profiles",
                profile.provider_id
            )
        })?;

    let session_key = generate_profile_session_key();
    let mut session = provider.start_profile_session(&profile, &session_key, workspace)?;

    let exit_status = session.wait_and_cleanup()?;

    Ok(if exit_status.success() { 0 } else { 1 })
}

// ---------------------------------------------------------------------------
// Command: profile create
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn run_profile_create(
    name: &str,
    provider_id: &str,
    skills: &[String],
    mcps: &[String],
    description: Option<&str>,
    description_file: Option<&str>,
    scope: crate::domain::scope::Scope,
    workspace: &std::path::Path,
) -> Result<i32> {
    if provider_id == "opencode" {
        // 1. Resolve description: file content wins over raw flag.
        let desc = if let Some(path) = description_file {
            std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read description file: {}", path))?
        } else if let Some(text) = description {
            text.to_string()
        } else {
            String::new()
        };

        // 2. Validate provider is active and supports profiles.
        let (registry, _scan, store) = crate::app::bootstrap::build(workspace.to_path_buf())?;
        let _provider = registry
            .providers
            .iter()
            .find(|p| p.id() == provider_id)
            .and_then(|p| {
                if p.supports_profiles() {
                    Some(p.as_ref())
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Provider '{}' not active or does not support profiles",
                    provider_id
                )
            })?;

        // 3. Ensure profile name is available.
        let mut config = store.load(scope)?;
        if config.find_profile(name).is_some() {
            anyhow::bail!("Profile '{}' already exists in {:?} scope", name, scope);
        }

        // 4. Write config with the new profile (agent markdown will be written after opencode runs).
        config.profiles.push(crate::domain::config::Profile {
            name: name.to_string(),
            provider_id: provider_id.to_string(),
            skills: skills.to_vec(),
            mcps: mcps.to_vec(),
        });
        store.save(scope, &config)?;

        // 5. Invoke opencode agent create headlessly.
        let mut cmd = std::process::Command::new("opencode");
        cmd.args([
            "agent",
            "create",
            "--path",
            workspace.display().to_string().as_str(),
            "--mode",
            "primary",
            "--name",
            name,
        ]);
        if !desc.is_empty() {
            cmd.args(["--description", &desc]);
        }

        let output = cmd
            .output()
            .with_context(|| "Failed to run 'opencode agent create'")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("opencode agent create failed: {}", stderr);
        }

        // 6. Find the generated .opencode/agents/<name>.md and move it to .agk/profiles/<name>/agent.md.
        let agents_dir = workspace.join(".opencode").join("agents");
        let source = agents_dir.join(format!("{}.md", name));
        if !source.exists() {
            // The exact filename may vary; scan the agents dir for a file created during this run.
            let newest = std::fs::read_dir(&agents_dir).ok().and_then(|mut entries| {
                let mut latest: Option<(std::fs::DirEntry, std::time::SystemTime)> = None;
                while let Some(Ok(e)) = entries.next() {
                    let meta = e.metadata().ok()?;
                    if meta.is_file() {
                        let modified = meta.modified().ok()?;
                        if latest.as_ref().is_none_or(|(_, t)| modified > *t) {
                            latest = Some((e, modified));
                        }
                    }
                }
                latest.map(|(e, _)| e.path())
            });
            if let Some(path) = newest {
                std::fs::copy(&path, &source)?;
            }
        }

        let profile_dir = workspace.join(".agk").join("profiles").join(name);
        std::fs::create_dir_all(&profile_dir)?;
        let target = profile_dir.join("agent.md");

        if source.exists() {
            std::fs::copy(&source, &target)?;
            println!(
                "Profile '{}' created. Agent markdown saved to {}",
                name,
                target.display()
            );
        } else {
            println!(
                "Profile '{}' created. Agent markdown not found at expected path ({}). \
                 You may need to run `opencode agent create` manually.",
                name,
                source.display()
            );
        }

        Ok(EXIT_SUCCESS)
    } else {
        anyhow::bail!(
            "Headless creation for provider '{}' is not yet supported",
            provider_id
        )
    }
}
