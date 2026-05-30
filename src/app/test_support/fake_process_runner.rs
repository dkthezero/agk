use crate::app::ports::ProcessRunnerPort;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::sync::Mutex;

/// In-memory [`ProcessRunnerPort`] that records every command invocation
/// and returns canned stdout / status from a lookup table.
///
/// This is the primary fake for asserting "dry-run must not spawn clawhub"
/// or "install should have called git clone".
#[derive(Debug)]
pub struct FakeProcessRunner {
    pub log: Mutex<Vec<RecordedCommand>>,
    pub canned_stdout: Mutex<HashMap<String, String>>,
    pub canned_status: Mutex<HashMap<String, ExitStatus>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordedCommand {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
}

impl Default for FakeProcessRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeProcessRunner {
    pub fn new() -> Self {
        Self {
            log: Mutex::new(Vec::new()),
            canned_stdout: Mutex::new(HashMap::new()),
            canned_status: Mutex::new(HashMap::new()),
        }
    }

    /// Pre-configure stdout for a given command key (`"npx -y foo"`).
    pub fn set_stdout(&self, key: &str, output: &str) {
        self.canned_stdout
            .lock()
            .unwrap()
            .insert(key.to_string(), output.to_string());
    }

    /// Pre-configure an exit status for a given command key.
    pub fn set_status(&self, key: &str, status: ExitStatus) {
        self.canned_status
            .lock()
            .unwrap()
            .insert(key.to_string(), status);
    }

    fn lookup_key(command: &str, args: &[String]) -> String {
        format!("{} {}", command, args.join(" "))
    }
}

impl ProcessRunnerPort for FakeProcessRunner {
    fn run(
        &self,
        command: &str,
        args: &[&str],
        cwd: Option<&Path>,
        _env: Option<&[(String, String)]>,
    ) -> Result<String> {
        let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let key = Self::lookup_key(command, &args_owned);
        self.log.lock().unwrap().push(RecordedCommand {
            command: command.to_string(),
            args: args_owned,
            cwd: cwd.map(|p| p.to_path_buf()),
        });
        self.canned_stdout
            .lock()
            .unwrap()
            .get(&key)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("FakeProcessRunner: no canned stdout for '{}'", key))
    }

    fn run_interactive(&self, command: &str, args: &[String], cwd: &Path) -> Result<ExitStatus> {
        let key = Self::lookup_key(command, args);
        self.log.lock().unwrap().push(RecordedCommand {
            command: command.to_string(),
            args: args.to_vec(),
            cwd: Some(cwd.to_path_buf()),
        });
        self.canned_status
            .lock()
            .unwrap()
            .get(&key)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("FakeProcessRunner: no canned status for '{}'", key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_process_runner_records_and_returns_canned() {
        let runner = FakeProcessRunner::new();
        runner.set_stdout("npx --version", "9.0.0");

        let out = runner.run("npx", &["--version"], None, None).unwrap();
        assert_eq!(out, "9.0.0");

        let log = runner.log.lock().unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].command, "npx");
    }

    #[test]
    fn fake_process_runner_missing_canned_errors() {
        let runner = FakeProcessRunner::new();
        let result = runner.run("npx", &["--version"], None, None);
        assert!(result.is_err());
    }
}
