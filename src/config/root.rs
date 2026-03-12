use std::path::{Path, PathBuf};

pub fn detect_project_root(cwd: &Path, markers: &[String]) -> PathBuf {
    for candidate in cwd.ancestors() {
        if candidate.join(".git").exists() {
            return candidate.to_path_buf();
        }
    }

    if !markers.is_empty() {
        for candidate in cwd.ancestors() {
            if markers.iter().any(|marker| candidate.join(marker).exists()) {
                return candidate.to_path_buf();
            }
        }
    }

    cwd.to_path_buf()
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::detect_project_root;

    #[test]
    fn finds_git_repo_root_in_parent() {
        let temp = test_dir("repo_root");
        let root = temp.join("repo");
        let nested = root.join("src/bin");
        fs::create_dir_all(root.join(".git")).expect("git dir");
        fs::create_dir_all(&nested).expect("nested dir");

        let detected = detect_project_root(&nested, &[]);
        assert_eq!(detected, root);
        cleanup(&temp);
    }

    #[test]
    fn falls_back_to_cwd_without_markers() {
        let temp = test_dir("cwd_fallback");
        let cwd = temp.join("plain");
        fs::create_dir_all(&cwd).expect("cwd");

        let detected = detect_project_root(&cwd, &[]);
        assert_eq!(detected, cwd);
        cleanup(&temp);
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
