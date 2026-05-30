//! In-memory task tracker implementation.

use crate::app::ports::{TaskPhase, TaskTrackerPort, TrackedTask};
use anyhow::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const HISTORY_CAP: usize = 100;

/// Concrete [`TaskTrackerPort`] backed by a mutex-guarded in-memory vector.
pub struct InMemoryTaskTracker {
    tasks: Mutex<Vec<TrackedTask>>,
    counter: AtomicU64,
}

impl InMemoryTaskTracker {
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(Vec::new()),
            counter: AtomicU64::new(0),
        }
    }

    fn next_id(&self) -> String {
        format!("task-{:010}", self.counter.fetch_add(1, Ordering::SeqCst))
    }

    fn is_terminal(phase: &TaskPhase) -> bool {
        matches!(
            phase,
            TaskPhase::Completed | TaskPhase::Failed | TaskPhase::Cancelled
        )
    }

    fn prune_completed(tasks: &mut Vec<TrackedTask>) {
        let completed_count = tasks.iter().filter(|t| Self::is_terminal(&t.phase)).count();
        if completed_count > HISTORY_CAP {
            let to_remove = completed_count - HISTORY_CAP;
            let mut terminal_indices: Vec<(usize, Instant, Instant)> = tasks
                .iter()
                .enumerate()
                .filter(|(_, t)| Self::is_terminal(&t.phase))
                .map(|(i, t)| (i, t.completed_at.unwrap_or(t.created_at), t.created_at))
                .collect();
            terminal_indices.sort_by(|a, b| {
                a.1.cmp(&b.1)
                    .then_with(|| a.2.cmp(&b.2))
                    .then_with(|| a.0.cmp(&b.0))
            });
            let mut indices_to_remove: Vec<usize> = terminal_indices
                .into_iter()
                .take(to_remove)
                .map(|(i, _, _)| i)
                .collect();
            indices_to_remove.sort_by(|a, b| b.cmp(a)); // reverse order for stable removal
            for idx in indices_to_remove {
                tasks.remove(idx);
            }
        }
    }
}

impl Default for InMemoryTaskTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskTrackerPort for InMemoryTaskTracker {
    fn register(&self, name: &str) -> String {
        let id = self.next_id();
        let task = TrackedTask {
            id: id.clone(),
            name: name.to_string(),
            phase: TaskPhase::Pending,
            created_at: Instant::now(),
            started_at: None,
            completed_at: None,
        };
        let mut tasks = self.tasks.lock().unwrap();
        tasks.push(task);
        id
    }

    fn transition(&self, id: &str, phase: TaskPhase) -> Result<()> {
        let mut tasks = self.tasks.lock().unwrap();
        let task = tasks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| anyhow::anyhow!("task {} not found", id))?;

        if phase == TaskPhase::Running && task.started_at.is_none() {
            task.started_at = Some(Instant::now());
        }
        if Self::is_terminal(&phase) && task.completed_at.is_none() {
            task.completed_at = Some(Instant::now());
        }
        task.phase = phase;

        if Self::is_terminal(&task.phase) {
            Self::prune_completed(&mut tasks);
        }

        Ok(())
    }

    fn complete(&self, id: &str) -> Result<()> {
        self.transition(id, TaskPhase::Completed)
    }

    fn list_active(&self) -> Vec<TrackedTask> {
        let tasks = self.tasks.lock().unwrap();
        tasks
            .iter()
            .filter(|t| !Self::is_terminal(&t.phase))
            .cloned()
            .collect()
    }

    fn list_recent(&self) -> Vec<TrackedTask> {
        let tasks = self.tasks.lock().unwrap();
        let mut completed: Vec<TrackedTask> = tasks
            .iter()
            .filter(|t| Self::is_terminal(&t.phase))
            .cloned()
            .collect();
        completed.sort_by(|a, b| {
            b.completed_at
                .unwrap_or(b.created_at)
                .cmp(&a.completed_at.unwrap_or(a.created_at))
        });
        completed
    }

    fn detect_hung(&self, threshold: Duration) -> Vec<TrackedTask> {
        let now = Instant::now();
        let tasks = self.tasks.lock().unwrap();
        tasks
            .iter()
            .filter(|t| !Self::is_terminal(&t.phase))
            .filter(|t| {
                let elapsed = t
                    .started_at
                    .map(|s| now.duration_since(s))
                    .unwrap_or_else(|| now.duration_since(t.created_at));
                elapsed > threshold
            })
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_complete() {
        let tracker = InMemoryTaskTracker::new();
        let id = tracker.register("test-task");
        assert!(id.starts_with("task-"));

        tracker.complete(&id).unwrap();

        let active = tracker.list_active();
        assert!(active.is_empty());

        let recent = tracker.list_recent();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].name, "test-task");
        assert_eq!(recent[0].phase, TaskPhase::Completed);
    }

    #[test]
    fn transition_updates_timestamps() {
        let tracker = InMemoryTaskTracker::new();
        let id = tracker.register("timestamp-task");

        tracker.transition(&id, TaskPhase::Running).unwrap();
        let active = tracker.list_active();
        assert!(active[0].started_at.is_some());

        tracker.transition(&id, TaskPhase::Failed).unwrap();
        let recent = tracker.list_recent();
        assert_eq!(recent[0].phase, TaskPhase::Failed);
        assert!(recent[0].completed_at.is_some());
    }

    #[test]
    fn detect_hung_tasks() {
        let tracker = InMemoryTaskTracker::new();
        let id = tracker.register("slow-task");
        tracker.transition(&id, TaskPhase::Running).unwrap();

        // A task just started should not be hung with a 1s threshold.
        let hung = tracker.detect_hung(Duration::from_secs(1));
        assert!(hung.is_empty());

        // With a zero threshold the same task is considered hung.
        let hung = tracker.detect_hung(Duration::from_secs(0));
        assert_eq!(hung.len(), 1);
        assert_eq!(hung[0].name, "slow-task");
    }

    #[test]
    fn history_cap_drops_oldest() {
        let tracker = InMemoryTaskTracker::new();
        for i in 0..105 {
            let id = tracker.register(&format!("task-{}", i));
            tracker.complete(&id).unwrap();
        }
        let recent = tracker.list_recent();
        assert_eq!(recent.len(), 100);
    }
}
