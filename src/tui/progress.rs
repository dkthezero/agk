#[derive(Clone, Debug, PartialEq)]
pub enum ProgressStatus {
    Starting,
    Running(u8),
}

#[derive(Clone, Debug)]
pub struct Progress {
    pub name: String,
    pub status: ProgressStatus,
}

pub static NEXT_TASK_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
