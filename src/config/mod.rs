pub mod root;

use std::{fs, path::Path};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_r_binary")]
    pub r_binary: String,
    #[serde(default = "default_codex_binary")]
    pub codex_binary: String,
    #[serde(default)]
    pub project_root_markers: Vec<String>,
    #[serde(default = "default_artifacts_dir")]
    pub artifacts_dir: String,
    #[serde(default = "default_chat_backend")]
    pub chat_backend: String,
    #[serde(default = "default_chat_model")]
    pub chat_model: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            r_binary: default_r_binary(),
            codex_binary: default_codex_binary(),
            project_root_markers: Vec::new(),
            artifacts_dir: default_artifacts_dir(),
            chat_backend: default_chat_backend(),
            chat_model: default_chat_model(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = fs::read_to_string(path)?;
        Ok(toml::from_str(&raw)?)
    }
}

fn default_r_binary() -> String {
    "R".to_string()
}

fn default_codex_binary() -> String {
    "codex".to_string()
}

fn default_artifacts_dir() -> String {
    "artifacts".to_string()
}

fn default_chat_backend() -> String {
    "placeholder".to_string()
}

fn default_chat_model() -> String {
    "local-placeholder".to_string()
}
