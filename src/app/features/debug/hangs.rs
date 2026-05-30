use crate::app::core::AgkCore;
use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};
use std::time::Duration;

/// Detect hung tasks (running longer than 30 seconds) and emit warnings.
pub fn run(core: &AgkCore, sink: &mut dyn CoreEventSink) -> CoreResult {
    let hung = core.task_tracker.detect_hung(Duration::from_secs(30));

    if hung.is_empty() {
        sink.on_event(CoreEvent::Info("No hung tasks detected.".into()));
    } else {
        sink.on_event(CoreEvent::Info(format!(
            "Detected {} hung task(s):",
            hung.len()
        )));
        for task in &hung {
            let elapsed = task
                .started_at
                .map(|s| s.elapsed().as_secs())
                .unwrap_or_else(|| task.created_at.elapsed().as_secs());
            let id = task.id.strip_prefix("task-").and_then(|s| s.parse().ok());
            match id {
                Some(id) => {
                    sink.on_event(CoreEvent::TaskHungWarning {
                        id,
                        name: task.name.clone(),
                        elapsed_sec: elapsed,
                    });
                }
                None => {
                    sink.on_event(CoreEvent::Info(format!(
                        "Skipping hung task with unparseable id: {}",
                        task.id
                    )));
                }
            }
        }
    }

    Ok(CoreOutcome::DebugTaskList(hung))
}
