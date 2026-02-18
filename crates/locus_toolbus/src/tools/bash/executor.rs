use super::args::BashArgs;
use super::error::BashError;
use crate::tools::ToolOutput;
use std::time::{Duration, Instant};
use tokio::process::Command;

#[derive(Default)]
pub struct BashExecutor {
    working_dir: Option<String>,
}

impl BashExecutor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timeout(self, _timeout: Duration) -> Self {
        self
    }

    pub fn with_working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    pub async fn run(&self, args: &BashArgs) -> Result<ToolOutput, BashError> {
        let timeout = Duration::from_secs(args.timeout);
        let start = Instant::now();

        let shell = self.get_shell();
        let mut cmd = Command::new(shell);

        cmd.arg("-c").arg(&args.command).kill_on_drop(true);

        if let Some(ref dir) = args.working_dir {
            cmd.current_dir(dir);
        } else if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        let output = tokio::time::timeout(timeout, cmd.output())
            .await
            .map_err(|_| BashError::Timeout(args.timeout))?
            .map_err(|e| BashError::SpawnFailed(e.to_string()))?;

        let stdout = String::from_utf8(output.stdout)?;
        let stderr = String::from_utf8(output.stderr)?;
        let exit_code = output.status.code().unwrap_or(-1);
        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ToolOutput {
            stdout,
            stderr,
            exit_code,
            duration_ms,
        })
    }

    #[cfg(unix)]
    fn get_shell(&self) -> &'static str {
        "/bin/bash"
    }

    #[cfg(windows)]
    fn get_shell(&self) -> &'static str {
        "cmd"
    }
}
