use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};

use crate::error::AppResult;
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
    let metadata = fs::metadata(local_root)
        .with_context(|| format!("read metadata for '{}'", local_root.display()))?;
    if !metadata.is_dir() {
        bail!("local-dir '{}' is not a directory", local_root.display());
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
    for entry in
        fs::read_dir(current).with_context(|| format!("read directory '{}'", current.display()))?
    {
        let entry =
            entry.with_context(|| format!("read directory entry in '{}'", current.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("read file type for '{}'", path.display()))?;

        if file_type.is_dir() {
            if is_default_ignored_dir_name(&entry.file_name()) {
                continue;
            }
            visit(local_root, &path, remote_dir, items)?;
        } else if file_type.is_file() && is_uploadable_path(&path) {
            let Some(remote_path) = relative_remote_path(local_root, &path, remote_dir)? else {
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
        .is_some_and(|name| DEFAULT_IGNORED_DIR_NAMES.contains(&name))
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
            root.join("src").join("main.js"),
            "export async function main() {}",
        )
        .expect("write");

        let plan = build_sync_plan(&root, None).expect("plan");

        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].remote_path, "src/main.js");

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
    fn build_sync_plan_excludes_non_js_extensions() {
        let root = temp_root("bbrs-sync-js-only");
        create_dir_all(root.join("src")).expect("mkdir src");
        write(root.join("src").join("foo.js"), "js").expect("write js");
        write(root.join("src").join("foo.ts"), "ts").expect("write ts");
        write(root.join("src").join("foo.json"), "{}").expect("write json");
        write(root.join("src").join("foo.txt"), "txt").expect("write txt");
        write(root.join("src").join("foo.script"), "script").expect("write script");

        let plan = build_sync_plan(&root, None).expect("plan");

        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].remote_path, "src/foo.js");

        remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn build_sync_plan_is_deterministic() {
        let root = temp_root("bbrs-sync-order");
        create_dir_all(root.join("src")).expect("mkdir src");
        write(root.join("src").join("z.js"), "z").expect("write z");
        write(root.join("src").join("a.js"), "a").expect("write a");

        let plan = build_sync_plan(&root, Some("scripts")).expect("plan");
        let remote_paths: Vec<_> = plan.iter().map(|item| item.remote_path.as_str()).collect();

        assert_eq!(remote_paths, vec!["scripts/src/a.js", "scripts/src/z.js"]);

        remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn build_sync_plan_rejects_bad_remote_dir() {
        let root = temp_root("bbrs-sync-bad-remote-dir");
        create_dir_all(&root).expect("mkdir root");
        write(root.join("main.js"), "js").expect("write");

        let err = build_sync_plan(&root, Some("../scripts")).expect_err("error");

        assert!(err.to_string().contains(".."));

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
