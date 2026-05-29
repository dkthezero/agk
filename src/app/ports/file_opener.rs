use anyhow::Result;

/// Port for OS-level file/folder open operations (Finder, file managers,
/// terminal emulators). Implemented by `infra/process/opener.rs`.
pub trait FileOpenerPort: Send + Sync {
    fn open_file_manager(&self, path: &std::path::Path) -> Result<()>;
    fn open_terminal(&self, path: &std::path::Path) -> Result<()>;
}
