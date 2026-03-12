use std::{fs, path::PathBuf};

use anyhow::Result;

use crate::codex::runner::CodexRunSummary;
use crate::config::Config;
use crate::context::SessionContextPaths;
use crate::r::session::RSession;
use crate::transcript::{Transcript, TranscriptEntry};

#[derive(Debug)]
pub struct AppState {
    pub cwd: PathBuf,
    pub project_root: PathBuf,
    pub config: Config,
    pub workspace: WorkspacePaths,
    pub context: SessionContextPaths,
    pub r_session: Option<RSession>,
    pub last_r_command: Option<String>,
    pub last_r_status: Option<String>,
    pub last_codex_run: Option<CodexRunSummary>,
    pub transcript: Transcript,
}

impl AppState {
    pub fn new(
        cwd: PathBuf,
        project_root: PathBuf,
        config: Config,
        workspace: WorkspacePaths,
        context: SessionContextPaths,
    ) -> Self {
        Self {
            cwd,
            project_root,
            config,
            workspace,
            context,
            r_session: None,
            last_r_command: None,
            last_r_status: None,
            last_codex_run: None,
            transcript: Transcript::default(),
        }
    }

    pub fn push_entry(&mut self, entry: TranscriptEntry) {
        self.transcript.push(entry);
    }
}

#[derive(Debug, Clone)]
pub struct WorkspacePaths {
    pub root: PathBuf,
    pub artifacts_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub session_file: PathBuf,
    pub app_log: PathBuf,
    pub r_log: PathBuf,
    pub codex_log: PathBuf,
    pub config_file: PathBuf,
}

impl WorkspacePaths {
    pub fn initialize(project_root: PathBuf) -> Result<Self> {
        let root = project_root.join(".rconsole");
        let artifacts_dir = root.join("artifacts");
        let logs_dir = root.join("logs");
        let session_file = root.join("session.json");
        let app_log = logs_dir.join("app.log");
        let r_log = logs_dir.join("r.log");
        let codex_log = logs_dir.join("codex.log");
        let config_file = root.join("config.toml");

        fs::create_dir_all(&artifacts_dir)?;
        fs::create_dir_all(&logs_dir)?;
        touch(&session_file)?;
        touch(&app_log)?;
        touch(&r_log)?;
        touch(&codex_log)?;

        Ok(Self {
            root,
            artifacts_dir,
            logs_dir,
            session_file,
            app_log,
            r_log,
            codex_log,
            config_file,
        })
    }
}

fn touch(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        fs::write(path, [])?;
    }
    Ok(())
}
