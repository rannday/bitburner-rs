use std::path::{Component, Path, PathBuf};

pub fn is_uploadable_path(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };

    matches!(
        extension.to_ascii_lowercase().as_str(),
        "js" | "ts" | "txt" | "script" | "json"
    )
}

pub fn relative_remote_path(
    local_root: &Path,
    local_path: &Path,
    remote_dir: Option<&str>,
) -> Option<String> {
    let relative = local_path.strip_prefix(local_root).ok()?;
    Some(join_remote_paths(
        remote_dir.unwrap_or(""),
        &path_to_forward_slashes(relative),
    ))
}

pub fn path_to_forward_slashes(path: &Path) -> String {
    let mut parts = Vec::new();

    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().to_string()),
            Component::CurDir => {}
            _ => parts.push(component.as_os_str().to_string_lossy().to_string()),
        }
    }

    normalize_remote_path(&parts.join("/"))
}

pub fn normalize_remote_path(path: &str) -> String {
    let replaced = path.replace('\\', "/");
    let mut parts = Vec::new();

    for part in replaced.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            parts.pop();
        } else {
            parts.push(part);
        }
    }

    parts.join("/")
}

pub fn join_remote_paths(prefix: &str, path: &str) -> String {
    let prefix = normalize_remote_path(prefix);
    let path = normalize_remote_path(path);

    match (prefix.is_empty(), path.is_empty()) {
        (true, true) => String::new(),
        (true, false) => path,
        (false, true) => prefix,
        (false, false) => format!("{prefix}/{path}"),
    }
}

#[allow(dead_code)]
pub fn remote_path_to_local(relative: &str) -> PathBuf {
    let mut path = PathBuf::new();
    for part in normalize_remote_path(relative).split('/') {
        if !part.is_empty() {
            path.push(part);
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_uploadable_extensions() {
        assert!(is_uploadable_path(Path::new("hack.js")));
        assert!(is_uploadable_path(Path::new("types.TS")));
        assert!(is_uploadable_path(Path::new("note.txt")));
        assert!(is_uploadable_path(Path::new("old.script")));
        assert!(is_uploadable_path(Path::new("data.json")));
        assert!(!is_uploadable_path(Path::new("image.png")));
        assert!(!is_uploadable_path(Path::new("README")));
    }

    #[test]
    fn normalizes_remote_paths() {
        assert_eq!(
            normalize_remote_path(r".\scripts\\hacking\jit-hack.js"),
            "scripts/hacking/jit-hack.js"
        );
        assert_eq!(
            join_remote_paths("/scripts/", r".\hacking\jit-hack.js"),
            "scripts/hacking/jit-hack.js"
        );
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
            "scripts/src/hacking/jit-hack.js"
        );
    }

    #[test]
    fn maps_relative_path_without_remote_dir() {
        let root = Path::new(".");
        let local = Path::new(".").join("src").join("main.js");

        assert_eq!(
            relative_remote_path(root, &local, None).expect("path"),
            "src/main.js"
        );
    }
}
