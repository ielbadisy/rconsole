use std::path::PathBuf;

pub fn detect_codex_binary() -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    detect_binary_in_os_path(binary_name(), &path_var)
}

pub fn detect_binary_in_path(binary: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    detect_binary_in_os_path(binary, &path_var)
}

fn detect_binary_in_os_path(binary: &str, path_var: &std::ffi::OsStr) -> Option<PathBuf> {
    for entry in std::env::split_paths(path_var) {
        let candidate = entry.join(binary);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn binary_name() -> &'static str {
    "codex"
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::detect_binary_in_os_path;

    #[test]
    fn finds_binary_in_path() {
        let temp = test_dir("codex-detect");
        let bin = temp.join("codex");
        fs::write(&bin, "#!/bin/sh\n").expect("write binary");

        let path_value = std::env::join_paths([temp.as_path()]).expect("join paths");
        let detected = detect_binary_in_os_path("codex", &path_value);

        assert_eq!(detected, Some(bin));
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn returns_none_when_missing() {
        let temp = test_dir("codex-missing");
        let path_value = std::env::join_paths([temp.as_path()]).expect("join paths");
        let detected = detect_binary_in_os_path("codex", &path_value);

        assert_eq!(detected, None);
        let _ = fs::remove_dir_all(temp);
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
}
