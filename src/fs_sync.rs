use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::path::{is_uploadable_path, relative_remote_path};

const DEFAULT_IGNORED_DIR_NAMES: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    ".zed",
    ".vscode",
    ".idea",
    "coverage",
    "tmp",
    "temp",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncItem {
    pub local_path: PathBuf,
    pub remote_path: String,
}

pub fn build_sync_plan(local_root: &Path, remote_dir: Option<&str>) -> AppResult<Vec<SyncItem>> {
    let metadata = fs::metadata(local_root)?;
    if !metadata.is_dir() {
        return Err(AppError::Usage(format!(
            "local-dir '{}' is not a directory",
            local_root.display()
        )));
    }

    let mut items = Vec::new();
    visit(local_root, local_root, remote_dir, &mut items)?;
    items.sort_by(|left, right| left.remote_path.cmp(&right.remote_path));
    Ok(items)
}

fn visit(
    local_root: &Path,
    current: &Path,
    remote_dir: Option<&str>,
    items: &mut Vec<SyncItem>,
) -> AppResult<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            if is_default_ignored_dir_name(&entry.file_name()) {
                continue;
            }
            visit(local_root, &path, remote_dir, items)?;
        } else if file_type.is_file() && is_uploadable_path(&path) {
            let Some(remote_path) = relative_remote_path(local_root, &path, remote_dir) else {
                continue;
            };
            items.push(SyncItem {
                local_path: path,
                remote_path,
            });
        }
    }

    Ok(())
}

fn is_default_ignored_dir_name(name: &std::ffi::OsStr) -> bool {
    name.to_str()
        .map_or(false, |name| DEFAULT_IGNORED_DIR_NAMES.contains(&name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir_all, remove_dir_all, write};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn builds_sync_path_mapping() {
        let root = temp_root("bbrs-sync-map");
        let scripts = root.join("scripts").join("hacking");
        create_dir_all(&scripts).expect("mkdir");
        write(
            scripts.join("jit-hack.js"),
            "export async function main() {}",
        )
        .expect("write");
        write(scripts.join("skip.md"), "skip").expect("write");

        let plan = build_sync_plan(&root, Some("scripts")).expect("plan");

        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].remote_path, "scripts/scripts/hacking/jit-hack.js");

        remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn maps_when_root_is_scripts_dir() {
        let root = temp_root("bbrs-sync-root-src");
        let src = root.join("src").join("hacking");
        create_dir_all(&src).expect("mkdir");
        write(src.join("jit-hack.js"), "export async function main() {}").expect("write");

        let plan = build_sync_plan(&root.join("src"), Some("scripts")).expect("plan");

        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].remote_path, "scripts/hacking/jit-hack.js");

        remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn maps_without_remote_dir() {
        let root = temp_root("bbrs-sync-no-remote");
        create_dir_all(root.join("src")).expect("mkdir");
        write(
            root.join("src").join("main.ts"),
            "export async function main() {}",
        )
        .expect("write");

        let plan = build_sync_plan(&root, None).expect("plan");

        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].remote_path, "src/main.ts");

        remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn build_sync_plan_ignores_target_json() {
        let root = temp_root("bbrs-sync-ignore-target");
        create_dir_all(root.join("target")).expect("mkdir target");
        write(root.join("target").join("metadata.json"), "{}").expect("write");

        let plan = build_sync_plan(&root, None).expect("plan");

        assert!(plan.is_empty());

        remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn build_sync_plan_ignores_git_json() {
        let root = temp_root("bbrs-sync-ignore-git");
        create_dir_all(root.join(".git")).expect("mkdir git");
        write(root.join(".git").join("metadata.json"), "{}").expect("write");

        let plan = build_sync_plan(&root, None).expect("plan");

        assert!(plan.is_empty());

        remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn build_sync_plan_includes_normal_src_js() {
        let root = temp_root("bbrs-sync-include-src");
        create_dir_all(root.join("src")).expect("mkdir src");
        write(
            root.join("src").join("foo.js"),
            "export async function main() {}",
        )
        .expect("write");

        let plan = build_sync_plan(&root, None).expect("plan");

        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].remote_path, "src/foo.js");

        remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn build_sync_plan_skips_nested_ignored_dirs() {
        let root = temp_root("bbrs-sync-ignore-nested");
        create_dir_all(root.join("src").join("target")).expect("mkdir nested target");
        write(root.join("src").join("target").join("metadata.json"), "{}").expect("write");
        write(
            root.join("src").join("foo.js"),
            "export async function main() {}",
        )
        .expect("write");

        let plan = build_sync_plan(&root.join("src"), Some("scripts")).expect("plan");

        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].remote_path, "scripts/foo.js");

        remove_dir_all(root).expect("cleanup");
    }

    fn temp_root(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{stamp}"))
    }
}
