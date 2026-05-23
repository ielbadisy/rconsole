use std::{
    fs::{self, OpenOptions},
    io::Write,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    thread,
};

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct CodexRunSummary {
    pub task: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub final_message: Option<String>,
    pub log_path: PathBuf,
}

pub fn run_codex_task(
    codex_binary: &Path,
    project_root: &Path,
    workspace_root: &Path,
    log_path: &Path,
    task: &str,
) -> Result<CodexRunSummary> {
    let last_message_path = workspace_root.join("codex-last-message.txt");
    if last_message_path.exists() {
        fs::remove_file(&last_message_path)?;
    }

    let mut child = Command::new(codex_binary)
        .arg("exec")
        .arg("--cd")
        .arg(project_root)
        .arg("--skip-git-repo-check")
        .arg("--output-last-message")
        .arg(&last_message_path)
        .arg(task)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to start {}", codex_binary.display()))?;

    let stdout = child
        .stdout
        .take()
        .context("failed to capture Codex stdout")?;
    let stderr = child
        .stderr
        .take()
        .context("failed to capture Codex stderr")?;
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .with_context(|| format!("failed to open {}", log_path.display()))?;

    let stdout_thread = thread::spawn(move || -> Result<()> {
        let mut log = log_file;
        for line in BufReader::new(stdout).lines() {
            let line = line?;
            writeln!(log, "{line}")?;
        }
        Ok(())
    });

    let stderr_log_path = log_path.to_path_buf();
    let stderr_thread = thread::spawn(move || -> Result<()> {
        let mut log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&stderr_log_path)
            .with_context(|| format!("failed to open {}", stderr_log_path.display()))?;
        for line in BufReader::new(stderr).lines() {
            let line = line?;
            writeln!(log, "{line}")?;
        }
        Ok(())
    });

    stdout_thread.join().expect("stdout thread")?;
    stderr_thread.join().expect("stderr thread")?;
    let status = child.wait()?;
    let final_message = read_final_message(&last_message_path)?;

    Ok(build_summary(
        task,
        status,
        final_message,
        log_path.to_path_buf(),
    ))
}

fn build_summary(
    task: &str,
    status: ExitStatus,
    final_message: Option<String>,
    log_path: PathBuf,
) -> CodexRunSummary {
    CodexRunSummary {
        task: task.to_string(),
        success: status.success(),
        exit_code: status.code(),
        final_message,
        log_path,
    }
}

fn read_final_message(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, process::Command};

    use super::build_summary;

    #[test]
    fn summary_tracks_exit_status() {
        let status = Command::new("sh")
            .arg("-c")
            .arg("exit 3")
            .status()
            .expect("status");
        let summary = build_summary(
            "test",
            status,
            Some("done".to_string()),
            PathBuf::from("codex.log"),
        );
        assert!(!summary.success);
        assert_eq!(summary.exit_code, Some(3));
        assert_eq!(summary.final_message.as_deref(), Some("done"));
    }
}
