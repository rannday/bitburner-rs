use std::path::{Component, Path, PathBuf};

use crate::{BitburnerError, Result};

pub fn relative_remote_path(
    local_root: &Path,
    local_path: &Path,
    remote_dir: Option<&str>,
) -> Result<Option<String>> {
    let relative = match local_path.strip_prefix(local_root) {
        Ok(relative) => relative,
        Err(_) => return Ok(None),
    };
    Ok(Some(join_remote_paths(
        remote_dir.unwrap_or(""),
        &path_to_forward_slashes(relative)?,
    )?))
}

pub fn path_to_forward_slashes(path: &Path) -> Result<String> {
    let mut parts = Vec::new();

    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().to_string()),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(BitburnerError::invalid_path(
                    "remote paths must not contain '..'",
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(BitburnerError::invalid_path(
                    "remote paths must be relative",
                ));
            }
        }
    }

    normalize_remote_path(&parts.join("/"))
}

pub fn normalize_remote_path(path: &str) -> Result<String> {
    let replaced = path.replace('\\', "/");
    if replaced.starts_with('/') {
        return Err(BitburnerError::invalid_path(
            "remote paths must be relative",
        ));
    }
    if has_windows_drive_prefix(&replaced) {
        return Err(BitburnerError::invalid_path(
            "remote paths must be relative",
        ));
    }

    let mut parts = Vec::new();

    for part in replaced.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return Err(BitburnerError::invalid_path(
                "remote paths must not contain '..'",
            ));
        }
        parts.push(part);
    }

    Ok(parts.join("/"))
}

pub fn normalize_remote_file_path(path: &str) -> Result<String> {
    let normalized = normalize_remote_path(path)?;
    if normalized.is_empty() {
        return Err(BitburnerError::invalid_path(
            "remote file path must not be empty",
        ));
    }
    Ok(normalized)
}

fn has_windows_drive_prefix(path: &str) -> bool {
    let mut chars = path.chars();
    matches!(
        (chars.next(), chars.next(), chars.next()),
        (Some(letter), Some(':'), None | Some('/')) if letter.is_ascii_alphabetic()
    )
}

pub fn join_remote_paths(prefix: &str, path: &str) -> Result<String> {
    let prefix = normalize_remote_path(prefix)?;
    let path = normalize_remote_path(path)?;

    Ok(match (prefix.is_empty(), path.is_empty()) {
        (true, true) => String::new(),
        (true, false) => path,
        (false, true) => prefix,
        (false, false) => format!("{prefix}/{path}"),
    })
}

pub fn remote_path_to_local(relative: &str) -> Result<PathBuf> {
    let mut path = PathBuf::new();
    let normalized = normalize_remote_path(relative)?;
    for part in normalized.split('/') {
        if !part.is_empty() {
            path.push(part);
        }
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_remote_paths() {
        assert_eq!(
            normalize_remote_path(r".\scripts\\hacking\jit-hack.js").expect("path"),
            "scripts/hacking/jit-hack.js"
        );
        assert_eq!(
            join_remote_paths("scripts/", r".\hacking\jit-hack.js").expect("path"),
            "scripts/hacking/jit-hack.js"
        );
    }

    #[test]
    fn rejects_parent_segments() {
        assert!(normalize_remote_path("../hack.js").is_err());
        assert!(normalize_remote_path("scripts/../hack.js").is_err());
        assert!(join_remote_paths("scripts/..", "hack.js").is_err());
    }

    #[test]
    fn rejects_absolute_remote_paths() {
        assert!(normalize_remote_path("/scripts/hack.js").is_err());
        assert!(join_remote_paths("/scripts", "hack.js").is_err());
    }

    #[test]
    fn rejects_windows_drive_prefixes() {
        assert!(normalize_remote_path(r"C:\scripts\hack.js").is_err());
        assert!(normalize_remote_path("C:/scripts/hack.js").is_err());
    }

    #[test]
    fn rejects_empty_remote_file_paths() {
        assert!(normalize_remote_file_path("").is_err());
        assert!(normalize_remote_file_path(".").is_err());
    }

    #[test]
    fn maps_relative_path_with_remote_dir() {
        let root = Path::new("project");
        let local = Path::new("project")
            .join("src")
            .join("hacking")
            .join("jit-hack.js");

        assert_eq!(
            relative_remote_path(root, &local, Some("scripts")).expect("path"),
            Some("scripts/src/hacking/jit-hack.js".to_string())
        );
    }

    #[test]
    fn maps_relative_path_without_remote_dir() {
        let root = Path::new(".");
        let local = Path::new(".").join("src").join("main.js");

        assert_eq!(
            relative_remote_path(root, &local, None).expect("path"),
            Some("src/main.js".to_string())
        );
    }
}
