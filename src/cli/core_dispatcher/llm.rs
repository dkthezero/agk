//! Dispatcher for the `agk llm` subcommand.  Thin bridge: parse args →
//! call the appropriate use case in `app::features::llm` → emit events via
//! the sink.  No business logic lives here.

use crate::app::features::llm;
use crate::app::outcome::{CoreEventSink, CoreOutcome};
use crate::app::ports::llm_provider::LlmProviderStorePort;
use crate::cli::llm::{LlmArgs, LlmCommand};
use crate::cli::presenter::CliPresenter;
use crate::domain::llm_provider::{LlmProviderConfig, LlmProviderKind};
use anyhow::{anyhow, Result};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

const HEALTH_TIMEOUT: Duration = Duration::from_secs(5);

/// Top-level entry point invoked from `core_dispatcher::dispatch`.
pub fn dispatch(args: &LlmArgs, workspace: &Path, presenter: &mut CliPresenter) -> Result<i32> {
    let store_path = llm_store_path(workspace, &args.config_dir);
    let mut sink = PresenterSink { presenter };
    match &args.command {
        LlmCommand::List => {
            let store = crate::infra::llm::store::FileLlmProviderStore::new(&store_path);
            let result = llm::list::run(&store, &mut sink);
            map_outcome(result)
        }
        LlmCommand::Add {
            id,
            kind,
            endpoint,
            api_key,
            model,
        } => {
            let parsed_kind = LlmProviderKind::from_str(kind)
                .map_err(|e| anyhow!("invalid LLM provider kind '{}': {}", kind, e))?;
            let cfg = LlmProviderConfig {
                id: id.clone(),
                kind: parsed_kind,
                endpoint: endpoint.clone(),
                api_key: api_key.clone(),
                default_model: model.clone(),
            };
            let store = crate::infra::llm::store::FileLlmProviderStore::new(&store_path);
            let result = llm::add::run(cfg, &store, &mut sink);
            map_outcome(result)
        }
        LlmCommand::Remove { id } => {
            let store = crate::infra::llm::store::FileLlmProviderStore::new(&store_path);
            let result = llm::remove::run(id, &store, &mut sink);
            map_outcome(result)
        }
        LlmCommand::Health { id } => {
            // Health checks are async (the HTTP probe is async) but the
            // top-level CLI dispatch is sync, and the binary is already
            // running on a tokio multi-thread runtime (see `#[tokio::main]`
            // in main.rs).  We use `block_in_place` to free the current
            // worker so the spawned tasks on the same runtime can run
            // while we block on the future.  `Runtime::new().block_on` is
            // not an option: building a new runtime from inside an
            // existing one panics.
            let presenter = sink.presenter;
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(run_health(
                    &store_path,
                    id.as_deref(),
                    presenter,
                ))
            })
        }
    }
}

/// Async health probe that fans out across the configured providers and
/// emits one `LlmProviderHealth` event per probe.
///
/// `health::run` emits a `LlmProviderHealth` event (rendered by the
/// presenter) for every probe and returns `Err` when the provider is
/// unreachable.  Two distinct failure shapes flow back here:
///
///   * an *unreachable* probe — the event already rendered
///     `"<id> unreachable: ..."` to stderr, so we must NOT let the error
///     propagate to `main.rs` (it would print a duplicate `Error: ...`
///     line); we surface it as a non-zero exit code instead.
///   * any other error (store/factory/probe failure) — no event was
///     emitted, so we propagate it so `main.rs` prints `Error: ...`.
///
/// `presenter.already_reported_task_failure()` distinguishes the two: it
/// returns true once an `LlmProviderHealth { reachable: false }` event has
/// been rendered, mirroring the `TaskFailed`/`McpTested` convention used by
/// the main dispatcher.
async fn run_health(
    store_path: &Path,
    only: Option<&str>,
    presenter: &mut CliPresenter,
) -> Result<i32> {
    let store = crate::infra::llm::store::FileLlmProviderStore::new(store_path);
    let cfgs = match only {
        Some(id) => {
            let cfg = store
                .get(id)?
                .ok_or_else(|| anyhow!("LLM provider '{}' not configured", id))?;
            vec![cfg]
        }
        None => store.list()?,
    };
    if cfgs.is_empty() {
        presenter.on_event(crate::app::event::CoreEvent::Info(
            "No LLM providers configured. Use `agk llm add` to add one.".into(),
        ));
        return Ok(crate::cli::EXIT_SUCCESS);
    }

    let factory = crate::infra::llm::factory::InfraLlmProviderFactory::new();
    let health = build_health_check();

    let mut had_unreachable = false;
    for cfg in &cfgs {
        match llm::health::run(
            &cfg.id,
            &store,
            &factory,
            health.as_ref(),
            HEALTH_TIMEOUT,
            presenter,
        )
        .await
        {
            Ok(_) => {}
            Err(e) => {
                if presenter.already_reported_task_failure() {
                    // The unreachable event already rendered the message
                    // to stderr; surface a non-zero exit code without a
                    // duplicate `Error:` line from `main.rs`.
                    had_unreachable = true;
                } else {
                    // No event rendered this failure — propagate so
                    // `main.rs` prints `Error: ...`.
                    return Err(e);
                }
            }
        }
    }
    if had_unreachable {
        Ok(crate::cli::EXIT_GENERAL_FAILURE)
    } else {
        Ok(crate::cli::EXIT_SUCCESS)
    }
}

/// Construct the concrete health check port.  We always use the
/// `HttpLlmHealthCheck` when at least one provider-feature is on; in the
/// no-feature build we fall back to a short-circuited no-op check that
/// marks every provider unreachable (so the CLI can still execute and
/// surface a useful error to the user).
#[cfg(any(
    feature = "llm-ollama",
    feature = "llm-lmstudio",
    feature = "llm-anthropic",
    feature = "llm-openai"
))]
fn build_health_check() -> Box<dyn crate::app::ports::llm_provider::LlmHealthCheckPort> {
    Box::new(crate::infra::llm::health::HttpLlmHealthCheck::new())
}

#[cfg(not(any(
    feature = "llm-ollama",
    feature = "llm-lmstudio",
    feature = "llm-anthropic",
    feature = "llm-openai"
)))]
fn build_health_check() -> Box<dyn crate::app::ports::llm_provider::LlmHealthCheckPort> {
    Box::new(NoopHealthCheck)
}

#[cfg(not(any(
    feature = "llm-ollama",
    feature = "llm-lmstudio",
    feature = "llm-anthropic",
    feature = "llm-openai"
)))]
struct NoopHealthCheck;

#[cfg(not(any(
    feature = "llm-ollama",
    feature = "llm-lmstudio",
    feature = "llm-anthropic",
    feature = "llm-openai"
)))]
#[async_trait::async_trait]
impl crate::app::ports::llm_provider::LlmHealthCheckPort for NoopHealthCheck {
    async fn check(
        &self,
        _adapter: &dyn crate::app::ports::llm_provider::LlmProviderAdapter,
        _timeout: Duration,
    ) -> Result<crate::domain::llm_provider::LlmHealthStatus> {
        Ok(crate::domain::llm_provider::LlmHealthStatus {
            reachable: false,
            latency_ms: None,
            models: vec![],
            error: Some(
                "no LLM provider features enabled; rebuild with --features llm-ollama,llm-lmstudio,llm-anthropic,llm-openai"
                    .to_string(),
            ),
        })
    }
}

fn llm_store_path(workspace: &Path, config_dir: &Path) -> std::path::PathBuf {
    let base = if config_dir == std::path::Path::new(".") {
        workspace.to_path_buf()
    } else {
        config_dir.to_path_buf()
    };
    base.join("llm_providers.toml")
}

fn map_outcome(result: Result<CoreOutcome>) -> Result<i32> {
    match result? {
        CoreOutcome::Ok => Ok(crate::cli::EXIT_SUCCESS),
        // `CoreOutcome` has a single variant today, but keep the match
        // exhaustive in case the enum grows.
        _ => Ok(crate::cli::EXIT_GENERAL_FAILURE),
    }
}

/// Tiny adapter so we can write `&mut sink` (a `CoreEventSink` trait object)
/// while reusing the [`CliPresenter`].  The presenter is the only thing the
/// CLI ever wants to talk to; it is itself a [`CoreEventSink`], so we
/// forward every event through it.  This keeps the LLM feature output
/// consistent with the rest of the CLI (text vs JSON, quiet mode, etc.).
struct PresenterSink<'a> {
    presenter: &'a mut CliPresenter,
}

impl<'a> CoreEventSink for PresenterSink<'a> {
    fn on_event(&mut self, event: crate::app::event::CoreEvent) {
        self.presenter.on_event(event);
    }

    fn on_error(&mut self, error: String) {
        self.presenter.eprint(&error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::llm::LlmCommand;

    fn args(command: LlmCommand) -> LlmArgs {
        LlmArgs {
            config_dir: std::path::PathBuf::from("."),
            command,
        }
    }

    #[test]
    fn llm_store_path_uses_workspace_when_default() {
        let p = llm_store_path(std::path::Path::new("/work"), std::path::Path::new("."));
        assert_eq!(p, std::path::PathBuf::from("/work/llm_providers.toml"));
    }

    #[test]
    fn llm_store_path_respects_explicit_config_dir() {
        let p = llm_store_path(std::path::Path::new("/work"), std::path::Path::new("/cfg"));
        assert_eq!(p, std::path::PathBuf::from("/cfg/llm_providers.toml"));
    }

    #[test]
    fn dispatch_list_with_no_providers_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        let mut presenter = CliPresenter::new(false, true);
        let rc = dispatch(&args(LlmCommand::List), dir.path(), &mut presenter).unwrap();
        assert_eq!(rc, crate::cli::EXIT_SUCCESS);
    }

    #[test]
    fn dispatch_add_then_list_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut presenter = CliPresenter::new(false, true);
        let rc = dispatch(
            &args(LlmCommand::Add {
                id: "local".into(),
                kind: "ollama".into(),
                endpoint: "http://127.0.0.1:11434".into(),
                api_key: None,
                model: Some("llama3.2".into()),
            }),
            dir.path(),
            &mut presenter,
        )
        .unwrap();
        assert_eq!(rc, crate::cli::EXIT_SUCCESS);
        let rc = dispatch(&args(LlmCommand::List), dir.path(), &mut presenter).unwrap();
        assert_eq!(rc, crate::cli::EXIT_SUCCESS);
    }

    #[test]
    fn dispatch_add_rejects_unknown_kind() {
        let dir = tempfile::tempdir().unwrap();
        let mut presenter = CliPresenter::new(false, true);
        let rc = dispatch(
            &args(LlmCommand::Add {
                id: "bad".into(),
                kind: "gpt-9".into(),
                endpoint: "http://x".into(),
                api_key: None,
                model: None,
            }),
            dir.path(),
            &mut presenter,
        );
        assert!(rc.is_err());
    }

    #[test]
    fn dispatch_remove_missing_provider_reports_error() {
        // Removing a provider that was never configured is *not* idempotent
        // from the user's perspective: `agk llm remove <id>` should tell them
        // the id was unknown rather than falsely reporting a successful
        // removal.  The use case now returns `Err`, so dispatch must surface
        // that error (non-zero exit) instead of `EXIT_SUCCESS`.
        let dir = tempfile::tempdir().unwrap();
        let mut presenter = CliPresenter::new(false, true);
        let result = dispatch(
            &args(LlmCommand::Remove {
                id: "missing".into(),
            }),
            dir.path(),
            &mut presenter,
        );
        assert!(result.is_err(), "expected error for missing provider");
    }

    /// Regression: `agk llm health <id>` must exit non-zero when the probe
    /// reports the provider unreachable.  Previously `health::run` returned
    /// `Ok(CoreOutcome::Ok)` on an unreachable probe and `run_health`
    /// propagated `Ok`, so the CLI exited 0 despite printing
    /// "<id> unreachable: ..." — a false success.
    ///
    /// The probe targets `127.0.0.1:1` (a privileged port that is reliably
    /// not bound), so it is unreachable in *both* feature configurations:
    /// the default (no-LLM-feature) build uses `NoopHealthCheck` which
    /// always reports unreachable, and an `--all-features` build uses the
    /// real `HttpLlmHealthCheck` which gets a connection refused on port 1.
    /// The test runs on a multi-thread Tokio runtime because the dispatch
    /// path uses `block_in_place`.
    #[tokio::test(flavor = "multi_thread")]
    async fn dispatch_health_unreachable_returns_failure_exit() {
        let dir = tempfile::tempdir().unwrap();
        let mut presenter = CliPresenter::new(false, false);
        // Add a provider so the health probe has something to probe.  Port 1
        // is reliably closed, so the probe is unreachable in every build.
        let rc = dispatch(
            &args(LlmCommand::Add {
                id: "dead".into(),
                kind: "ollama".into(),
                endpoint: "http://127.0.0.1:1".into(),
                api_key: None,
                model: Some("llama3.2".into()),
            }),
            dir.path(),
            &mut presenter,
        )
        .unwrap();
        assert_eq!(rc, crate::cli::EXIT_SUCCESS);

        // Probe it.  Both NoopHealthCheck (default) and HttpLlmHealthCheck
        // (all-features) report unreachable for port 1, so the dispatcher
        // must surface a non-zero exit code.
        let rc = dispatch(
            &args(LlmCommand::Health {
                id: Some("dead".into()),
            }),
            dir.path(),
            &mut presenter,
        )
        .unwrap();
        assert_eq!(
            rc,
            crate::cli::EXIT_GENERAL_FAILURE,
            "an unreachable health probe must surface as a non-zero exit code"
        );
    }

    // When a provider is reachable, `agk llm health <id>` must exit 0.
    // This requires a live HTTP endpoint, so it is only covered by the
    // use-case-level `run_reachable_*` test (which uses a fake health
    // check) rather than a dispatcher test.  The dispatcher path is
    // exercised end-to-end for the *unreachable* case above.
}
