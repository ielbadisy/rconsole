use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::r::session::ExecutionResult;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RContextSnapshot {
    pub objects: Vec<String>,
    pub last_command: Option<String>,
    pub last_status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SessionContextPaths {
    pub root: PathBuf,
    pub r_objects_json: PathBuf,
    pub r_last_command_txt: PathBuf,
    pub r_last_output_txt: PathBuf,
    pub r_last_status_txt: PathBuf,
    pub r_last_glimpse_txt: PathBuf,
}

impl SessionContextPaths {
    pub fn initialize(workspace_root: &Path) -> Result<Self> {
        let root = workspace_root.join("session");
        fs::create_dir_all(&root)?;
        let paths = Self {
            root: root.clone(),
            r_objects_json: root.join("r-objects.json"),
            r_last_command_txt: root.join("r-last-command.txt"),
            r_last_output_txt: root.join("r-last-output.txt"),
            r_last_status_txt: root.join("r-last-status.txt"),
            r_last_glimpse_txt: root.join("r-last-glimpse.txt"),
        };
        touch(&paths.r_objects_json)?;
        touch(&paths.r_last_command_txt)?;
        touch(&paths.r_last_output_txt)?;
        touch(&paths.r_last_status_txt)?;
        touch(&paths.r_last_glimpse_txt)?;
        Ok(paths)
    }
}

pub fn write_r_snapshot(
    paths: &SessionContextPaths,
    objects: &[String],
    last_command: Option<&str>,
    last_status: Option<&str>,
    result: Option<&ExecutionResult>,
) -> Result<()> {
    let snapshot = RContextSnapshot {
        objects: objects.to_vec(),
        last_command: last_command.map(ToOwned::to_owned),
        last_status: last_status.map(ToOwned::to_owned),
    };
    fs::write(
        &paths.r_objects_json,
        serde_json::to_string_pretty(&snapshot)?,
    )?;
    fs::write(&paths.r_last_command_txt, last_command.unwrap_or(""))?;
    fs::write(&paths.r_last_status_txt, last_status.unwrap_or(""))?;

    let last_output = match result {
        Some(value) if value.output.is_empty() && value.error.is_none() => {
            "(no output)".to_string()
        }
        Some(value) => {
            let mut parts = Vec::new();
            if !value.output.is_empty() {
                parts.push(value.output.clone());
            }
            if let Some(error) = &value.error {
                parts.push(format!("error: {error}"));
            }
            if let Some(plot_path) = &value.plot_path {
                parts.push(format!("plot: {}", plot_path.display()));
            }
            parts.join("\n")
        }
        None => String::new(),
    };
    fs::write(&paths.r_last_output_txt, last_output)?;
    Ok(())
}

pub fn clear_r_snapshot(paths: &SessionContextPaths) -> Result<()> {
    write_r_snapshot(paths, &[], None, None, None)?;
    fs::write(&paths.r_last_glimpse_txt, [])?;
    Ok(())
}

pub fn write_r_glimpse(paths: &SessionContextPaths, expression: &str, output: &str) -> Result<()> {
    let content = format!("Expression: {expression}\n\n{output}");
    fs::write(&paths.r_last_glimpse_txt, content)?;
    Ok(())
}

pub fn build_codex_preamble(paths: &SessionContextPaths) -> String {
    format!(
        "Shared session context for this repo:\n- Read R context from `{}`\n- Read last R command from `{}`\n- Read last R output from `{}`\n- Read last R glimpse from `{}`\nUse that context if it is relevant, but do not assume live access to the R process.\n",
        paths.r_objects_json.display(),
        paths.r_last_command_txt.display(),
        paths.r_last_output_txt.display(),
        paths.r_last_glimpse_txt.display()
    )
}

pub fn render_context(paths: &SessionContextPaths) -> Result<String> {
    let objects = fs::read_to_string(&paths.r_objects_json)?;
    let last_command = fs::read_to_string(&paths.r_last_command_txt)?;
    let last_output = fs::read_to_string(&paths.r_last_output_txt)?;
    let last_status = fs::read_to_string(&paths.r_last_status_txt)?;
    let last_glimpse = fs::read_to_string(&paths.r_last_glimpse_txt)?;
    Ok(format!(
        "R objects file: {}\nLast R command: {}\nLast R status: {}\nLast R output file: {}\nLast R output:\n{}\nLast R glimpse file: {}\nLast R glimpse:\n{}\nObjects snapshot:\n{}",
        paths.r_objects_json.display(),
        if last_command.trim().is_empty() { "(none)" } else { last_command.trim() },
        if last_status.trim().is_empty() { "(none)" } else { last_status.trim() },
        paths.r_last_output_txt.display(),
        if last_output.trim().is_empty() { "(none)" } else { last_output.trim() },
        paths.r_last_glimpse_txt.display(),
        if last_glimpse.trim().is_empty() { "(none)" } else { last_glimpse.trim() },
        if objects.trim().is_empty() { "(empty)" } else { objects.trim() }
    ))
}

fn touch(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::write(path, [])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{
        build_codex_preamble, clear_r_snapshot, render_context, write_r_glimpse, write_r_snapshot,
        SessionContextPaths,
    };
    use crate::r::session::ExecutionResult;

    #[test]
    fn writes_and_renders_snapshot() {
        let root = test_dir("context");
        let paths = SessionContextPaths::initialize(&root).expect("init");
        let result = ExecutionResult {
            output: "[1] 2".to_string(),
            error: None,
            plot_path: None,
        };
        write_r_snapshot(
            &paths,
            &["x".to_string()],
            Some("x <- 1 + 1"),
            Some("ok"),
            Some(&result),
        )
        .expect("write snapshot");

        let rendered = render_context(&paths).expect("render");
        assert!(rendered.contains("x <- 1 + 1"));
        assert!(rendered.contains("r-objects.json"));
        assert!(build_codex_preamble(&paths).contains("last R output"));
        write_r_glimpse(&paths, "fit", "summary output").expect("glimpse");
        let rendered = render_context(&paths).expect("render with glimpse");
        assert!(rendered.contains("summary output"));
        cleanup(&root);
    }

    #[test]
    fn clears_snapshot() {
        let root = test_dir("context-clear");
        let paths = SessionContextPaths::initialize(&root).expect("init");
        clear_r_snapshot(&paths).expect("clear");
        let objects = fs::read_to_string(paths.r_objects_json).expect("objects");
        assert!(objects.contains("\"objects\": []"));
        cleanup(&root);
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
