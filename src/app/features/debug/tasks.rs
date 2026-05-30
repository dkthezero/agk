use crate::app::core::AgkCore;
use crate::app::event::CoreEvent;
use crate::app::outcome::{CoreEventSink, CoreOutcome, CoreResult};

/// List all active and recent tracked tasks.
pub fn run(core: &AgkCore, sink: &mut dyn CoreEventSink) -> CoreResult {
    let active = core.task_tracker.list_active();
    let recent = core.task_tracker.list_recent();

    sink.on_event(CoreEvent::Info(format!(
        "Active tasks: {} | Recent tasks: {}",
        active.len(),
        recent.len()
    )));

    for task in &active {
        sink.on_event(CoreEvent::Info(format!(
            "[active] {} — {:?} ({}s)",
            task.name,
            task.phase,
            task.started_at.map(|s| s.elapsed().as_secs()).unwrap_or(0)
        )));
    }

    for task in &recent {
        sink.on_event(CoreEvent::Info(format!(
            "[recent] {} — {:?}",
            task.name, task.phase
        )));
    }

    let mut all = active;
    all.extend(recent.into_iter().rev());
    Ok(CoreOutcome::DebugTaskList(all))
}
