//! Task tracker port — underground system observability.
//!
//! Provides a port for tracking long-running operations (tasks) through their
//! lifecycle phases, enabling hang detection and operational visibility.

use anyhow::Result;
use std::time::{Duration, Instant};

/// Lifecycle phase of a tracked task.
#[derive(Debug, Clone, PartialEq)]
pub enum TaskPhase {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Snapshot of a tracked task at a point in time.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackedTask {
    pub id: String,
    pub name: String,
    pub phase: TaskPhase,
    pub created_at: Instant,
    pub started_at: Option<Instant>,
    pub completed_at: Option<Instant>,
}

/// Port for tracking task lifecycle and detecting hung operations.
pub trait TaskTrackerPort: Send + Sync {
    /// Register a new task and return its unique ID.
    fn register(&self, name: &str) -> String;

    /// Transition an existing task to a new phase.
    fn transition(&self, id: &str, phase: TaskPhase) -> Result<()>;

    /// Mark a task as completed.
    fn complete(&self, id: &str) -> Result<()>;

    /// List all tasks that are not in a terminal phase.
    fn list_active(&self) -> Vec<TrackedTask>;

    /// List recently completed tasks, newest first.
    fn list_recent(&self) -> Vec<TrackedTask>;

    /// Return tasks that have been active longer than the given threshold.
    fn detect_hung(&self, threshold: Duration) -> Vec<TrackedTask>;
}
