//! Concurrency tests for `LlmHealthCheckPort`.
//!
//! These tests lock in the contract that:
//!   * `LlmHealthCheckPort` is `Send + Sync` and can be stored in a
//!     `Vec<Box<dyn ...>>` and dispatched concurrently via
//!     `futures::future::join_all` without panics, race conditions, or
//!     interleaved mutable state.
//!   * The real HTTP implementation is also `Send + Sync`, so the same
//!     parallel-dispatch pattern works in production.
//!
//! The use-case `app::features::llm::health::run` is invoked once per
//! configured provider. If the dispatch accidentally serialised (e.g.
//! held a `!Sync` lock across an `.await`), the test below would take
//! ~400ms instead of ~100ms and trip the 300ms ceiling.
//!
//! The real-HTTP compile check is feature-gated: `HttpLlmHealthCheck`
//! only exists when at least one `llm-*` feature is enabled.

use agk::app::ports::llm_provider::LlmHealthCheckPort;
use agk::app::test_support::fake_llm_provider::{FakeAdapter, FakeLlmHealthCheck};
use agk::domain::llm_provider::LlmProviderKind;
use futures::future::join_all;
use std::time::{Duration, Instant};

/// Per-check delay chosen so that the parallel test is clearly under
/// the serial baseline (4 × 100ms = 400ms serial) but well above the
/// scheduler wakeup noise floor. Generous slack is built into the upper
/// bound to keep the test stable on slow CI.
const PER_CHECK_DELAY: Duration = Duration::from_millis(100);
const PARALLEL_CEILING: Duration = Duration::from_millis(300);
const N_CHECKS: usize = 4;

fn stub_adapter() -> FakeAdapter {
    FakeAdapter {
        kind: LlmProviderKind::Ollama,
        url: "http://localhost:11434/api/tags".into(),
    }
}

#[tokio::test]
async fn health_checks_run_in_parallel() {
    // 4 fakes, each sleeping for 100ms inside `check`. If the trait were
    // not `Send + Sync`, this would not compile (Box<dyn Trait> with the
    // auto-trait bounds). If dispatch serialised, this would take ~400ms.
    let checks: Vec<Box<dyn LlmHealthCheckPort>> = vec![
        Box::new(FakeLlmHealthCheck::with_delay(PER_CHECK_DELAY)),
        Box::new(FakeLlmHealthCheck::with_delay(PER_CHECK_DELAY)),
        Box::new(FakeLlmHealthCheck::with_delay(PER_CHECK_DELAY)),
        Box::new(FakeLlmHealthCheck::with_delay(PER_CHECK_DELAY)),
    ];

    let adapter = stub_adapter();
    let start = Instant::now();
    let results = join_all(
        checks
            .iter()
            .map(|c| c.check(&adapter, Duration::from_secs(5))),
    )
    .await;
    let elapsed = start.elapsed();

    assert_eq!(
        results.len(),
        N_CHECKS,
        "expected one result per health check"
    );
    for (i, r) in results.iter().enumerate() {
        assert!(r.is_ok(), "health check #{i} failed: {r:?}");
    }

    // Serial baseline is 4 × 100ms = 400ms. Allow up to 300ms for
    // parallel dispatch with CI slack. Any value <400ms proves at
    // least some overlap; <300ms is a comfortable margin.
    assert!(
        elapsed < PARALLEL_CEILING,
        "parallel health checks took {elapsed:?} (expected ~{PER_CHECK_DELAY:?}, max {PARALLEL_CEILING:?}) — \
         dispatch may be serialising on a !Send/!Sync lock or missing join"
    );
    // And we should have actually waited roughly the per-check delay —
    // a near-zero elapsed would suggest the fake didn't sleep.
    assert!(
        elapsed >= PER_CHECK_DELAY,
        "elapsed {elapsed:?} is shorter than the per-check delay {PER_CHECK_DELAY:?} — \
         the fake's sleep was skipped"
    );
}

#[tokio::test]
async fn parallel_health_checks_return_independent_results() {
    // Three fakes with distinct model lists and reachability outcomes.
    // Concurrency must not scramble their results — each futures combinator
    // preserves the input order, and the fakes hold no shared state, so
    // we expect to see the same status object we put in.
    let a = FakeLlmHealthCheck {
        models: vec!["model-a".into()],
        ..FakeLlmHealthCheck::default()
    };
    let b = FakeLlmHealthCheck {
        models: vec!["model-b".into()],
        reachable: false,
        error: Some("down".into()),
        ..FakeLlmHealthCheck::default()
    };
    let c = FakeLlmHealthCheck {
        models: vec!["model-c".into()],
        ..FakeLlmHealthCheck::default()
    };
    let checks: Vec<Box<dyn LlmHealthCheckPort>> = vec![Box::new(a), Box::new(b), Box::new(c)];

    let adapter = stub_adapter();
    let results = join_all(
        checks
            .iter()
            .map(|h| h.check(&adapter, Duration::from_secs(1))),
    )
    .await;

    assert_eq!(results.len(), 3);
    let r0 = results[0].as_ref().expect("a ok").clone();
    let r1 = results[1].as_ref().expect("b ok").clone();
    let r2 = results[2].as_ref().expect("c ok").clone();

    assert!(r0.reachable);
    assert_eq!(r0.models, vec!["model-a".to_string()]);
    assert!(!r1.reachable);
    assert_eq!(r1.error.as_deref(), Some("down"));
    assert!(r2.reachable);
    assert_eq!(r2.models, vec!["model-c".to_string()]);
}

#[cfg(any(
    feature = "llm-ollama",
    feature = "llm-lmstudio",
    feature = "llm-anthropic",
    feature = "llm-openai"
))]
#[test]
fn http_health_check_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    use agk::infra::llm::health::HttpLlmHealthCheck;
    assert_send_sync::<HttpLlmHealthCheck>();
}

#[cfg(any(
    feature = "llm-ollama",
    feature = "llm-lmstudio",
    feature = "llm-anthropic",
    feature = "llm-openai"
))]
#[tokio::test]
async fn http_health_check_dispatches_concurrently() {
    // Pins two contracts on the real `HttpLlmHealthCheck`:
    //   1. The type is `Send + Sync` (the macro-form Send+Sync assertion
    //      in `http_health_check_is_send_sync` covers this; here we
    //      exercise it through actual use).
    //   2. A single `HttpLlmHealthCheck` can drive two parallel
    //      `check` calls without serialising — exercised by sharing the
    //      instance behind an `Arc`.
    use agk::app::ports::llm_provider::LlmProviderAdapter;
    use agk::infra::llm::health::HttpLlmHealthCheck;
    use agk::infra::llm::ollama::OllamaProvider;
    use std::sync::Arc;

    let hc = Arc::new(HttpLlmHealthCheck::new());
    let hc2 = Arc::clone(&hc);
    let adapter: Box<dyn LlmProviderAdapter> = Box::new(OllamaProvider::new("http://127.0.0.1:1"));

    // Point both calls at a closed port; both should report unreachable
    // and neither should panic. The closed port is what makes the test
    // fast (no real network round-trip; reqwest's 50ms timeout kicks in).
    let a_ref: &dyn LlmProviderAdapter = adapter.as_ref();
    let (a, b) = tokio::join!(
        async move { hc.check(a_ref, Duration::from_millis(50)).await },
        async move { hc2.check(a_ref, Duration::from_millis(50)).await },
    );
    let a = a.expect("a ok");
    let b = b.expect("b ok");
    assert!(!a.reachable, "a should be unreachable on a closed port");
    assert!(!b.reachable, "b should be unreachable on a closed port");
}
