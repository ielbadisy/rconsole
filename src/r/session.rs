use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use anyhow::{anyhow, Context, Result};

use crate::r::protocol::{BEGIN_MARKER, END_MARKER, ERROR_MARKER, PLOT_MARKER};

#[derive(Debug)]
pub struct RSession {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    command_file: PathBuf,
    artifacts_dir: PathBuf,
    next_plot_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResult {
    pub output: String,
    pub error: Option<String>,
    pub plot_path: Option<PathBuf>,
}

impl RSession {
    pub fn start(r_binary: &str, workspace_root: &Path, artifacts_dir: &Path) -> Result<Self> {
        fs::create_dir_all(workspace_root)?;
        fs::create_dir_all(artifacts_dir)?;

        let mut child = Command::new(r_binary)
            .args(["--quiet", "--no-save", "--slave"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("failed to start R binary: {r_binary}"))?;

        let stdin = child.stdin.take().context("failed to open stdin for R")?;
        let stdout = child.stdout.take().context("failed to open stdout for R")?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            command_file: workspace_root.join("current-command.R"),
            artifacts_dir: artifacts_dir.to_path_buf(),
            next_plot_id: 1,
        })
    }

    pub fn execute(&mut self, code: &str) -> Result<ExecutionResult> {
        fs::write(&self.command_file, code)?;
        let plot_path = self
            .artifacts_dir
            .join(format!("plot-{:04}.png", self.next_plot_id));
        self.next_plot_id += 1;

        let command_file = escape_r_string(&self.command_file);
        let plot_file = escape_r_string(&plot_path);
        let wrapped = format!(
            "cat('{BEGIN_MARKER}\\n')\n\
             if (file.exists('{plot_file}')) file.remove('{plot_file}')\n\
             .rconsole_plot_opened <- FALSE\n\
             try({{ grDevices::png(filename = '{plot_file}'); .rconsole_plot_opened <- TRUE }}, silent = TRUE)\n\
             tryCatch({{\n\
               source('{command_file}', local = .GlobalEnv, echo = FALSE, print.eval = TRUE, max.deparse.length = Inf)\n\
             }}, error = function(e) {{\n\
               cat('{ERROR_MARKER}', conditionMessage(e), '\\n', sep = ' ')\n\
             }})\n\
             if (.rconsole_plot_opened) invisible(try(grDevices::dev.off(), silent = TRUE))\n\
             if (file.exists('{plot_file}') && isTRUE(file.info('{plot_file}')$size > 0)) cat('{PLOT_MARKER}', '{plot_file}', '\\n', sep = ' ')\n\
             cat('{END_MARKER}\\n')\n\
             flush.console()\n"
        );

        self.stdin.write_all(wrapped.as_bytes())?;
        self.stdin.flush()?;
        self.read_response(plot_path)
    }

    pub fn list_objects(&mut self) -> Result<ExecutionResult> {
        self.execute("base::ls()")
    }

    fn read_response(&mut self, expected_plot: PathBuf) -> Result<ExecutionResult> {
        let mut line = String::new();
        let mut output = Vec::new();
        let mut error = None;
        let mut plot_path = None;
        let mut seen_begin = false;

        loop {
            line.clear();
            let read = self.stdout.read_line(&mut line)?;
            if read == 0 {
                return Err(anyhow!("R session terminated unexpectedly"));
            }

            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed == BEGIN_MARKER {
                seen_begin = true;
                continue;
            }

            if !seen_begin {
                continue;
            }

            if trimmed == END_MARKER {
                break;
            }

            if let Some(rest) = trimmed.strip_prefix(&format!("{ERROR_MARKER} ")) {
                error = Some(rest.trim().to_string());
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix(&format!("{PLOT_MARKER} ")) {
                if !rest.is_empty() {
                    plot_path = Some(PathBuf::from(rest));
                } else {
                    plot_path = Some(expected_plot.clone());
                }
                continue;
            }

            output.push(trimmed.to_string());
        }

        Ok(ExecutionResult {
            output: output.join("\n").trim().to_string(),
            error,
            plot_path,
        })
    }
}

impl Drop for RSession {
    fn drop(&mut self) {
        let _ = self.stdin.write_all(b"quit(save = 'no')\n");
        let _ = self.stdin.flush();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn escape_r_string(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "/")
        .replace('\'', "\\'")
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        process::{Command, Stdio},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::RSession;

    #[test]
    fn startup_execute_and_persistence() {
        if !r_available() {
            return;
        }

        let root = test_dir("r-session");
        let artifacts = root.join("artifacts");
        let mut session = RSession::start("R", &root, &artifacts).expect("start R");

        let first = session.execute("x <- 41 + 1\nx").expect("first execution");
        assert!(first.error.is_none());
        assert!(first.output.contains("[1] 42"));

        let second = session.execute("x").expect("second execution");
        assert!(second.error.is_none());
        assert!(second.output.contains("[1] 42"));

        cleanup(&root);
    }

    #[test]
    fn surfaces_errors() {
        if !r_available() {
            return;
        }

        let root = test_dir("r-errors");
        let artifacts = root.join("artifacts");
        let mut session = RSession::start("R", &root, &artifacts).expect("start R");

        let result = session.execute("stop('boom')").expect("execution");
        assert_eq!(result.error.as_deref(), Some("boom"));

        cleanup(&root);
    }

    fn r_available() -> bool {
        Command::new("R")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    fn test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rconsole-{name}-{unique}"));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_dir_all(path);
    }
}
