use std::path::{Path, PathBuf};

use crate::{Result, join_remote_paths, path_to_forward_slashes};

pub const DEFAULT_IGNORED_DIR_NAMES: &[&str] = &[
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadableFileKind {
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadableExtension {
    JavaScript,
    TypeScript,
    Text,
    Script,
    Netscript,
}

impl UploadableExtension {
    pub const ALL: [Self; 5] = [
        Self::JavaScript,
        Self::TypeScript,
        Self::Text,
        Self::Script,
        Self::Netscript,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::JavaScript => "js",
            Self::TypeScript => "ts",
            Self::Text => "txt",
            Self::Script => "script",
            Self::Netscript => "ns",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFileEntry {
    pub relative_path: PathBuf,
    pub content_kind: UploadableFileKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncOptions {
    pub remote_dir: Option<String>,
    pub allowed_extensions: Vec<UploadableExtension>,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            remote_dir: None,
            allowed_extensions: UploadableExtension::ALL.to_vec(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncItem {
    pub relative_path: PathBuf,
    pub remote_path: String,
}

pub fn build_sync_plan_from_entries(
    entries: impl IntoIterator<Item = LocalFileEntry>,
    options: &SyncOptions,
) -> Result<Vec<SyncItem>> {
    let mut items = Vec::new();

    for entry in entries {
        if !is_uploadable_path_with_extensions(&entry.relative_path, &options.allowed_extensions) {
            continue;
        }
        let remote_path = join_remote_paths(
            options.remote_dir.as_deref().unwrap_or(""),
            &path_to_forward_slashes(&entry.relative_path)?,
        )?;
        items.push(SyncItem {
            relative_path: entry.relative_path,
            remote_path,
        });
    }

    items.sort_by(|left, right| left.remote_path.cmp(&right.remote_path));
    Ok(items)
}

pub fn is_uploadable_path(path: &Path) -> bool {
    is_uploadable_path_with_extensions(path, &UploadableExtension::ALL)
}

pub fn is_uploadable_path_with_extensions(
    path: &Path,
    allowed_extensions: &[UploadableExtension],
) -> bool {
    let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
        return false;
    };

    allowed_extensions
        .iter()
        .any(|allowed| extension.eq_ignore_ascii_case(allowed.as_str()))
}

pub fn is_default_ignored_dir_name(name: &str) -> bool {
    DEFAULT_IGNORED_DIR_NAMES.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str) -> LocalFileEntry {
        LocalFileEntry {
            relative_path: PathBuf::from(path),
            content_kind: UploadableFileKind::Text,
        }
    }

    #[test]
    fn filters_uploadable_extensions() {
        assert!(is_uploadable_path(Path::new("hack.js")));
        assert!(is_uploadable_path(Path::new("types.ts")));
        assert!(is_uploadable_path(Path::new("note.txt")));
        assert!(is_uploadable_path(Path::new("old.script")));
        assert!(is_uploadable_path(Path::new("daemon.ns")));
        assert!(is_uploadable_path(Path::new("hack.JS")));
        assert!(!is_uploadable_path(Path::new("data.json")));
        assert!(!is_uploadable_path(Path::new("image.png")));
        assert!(!is_uploadable_path(Path::new("README")));
    }

    #[test]
    fn identifies_default_ignored_dirs() {
        for name in [
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
        ] {
            assert!(is_default_ignored_dir_name(name), "{name}");
        }
        assert!(!is_default_ignored_dir_name("Target"));
        assert!(!is_default_ignored_dir_name("src"));
    }

    #[test]
    fn builds_sync_plan_from_abstract_entries() {
        let plan = build_sync_plan_from_entries(
            [
                entry("src/main.js"),
                entry("src/types.ts"),
                entry("src/data.json"),
            ],
            &SyncOptions {
                remote_dir: Some("scripts".to_string()),
                ..SyncOptions::default()
            },
        )
        .expect("plan");

        assert_eq!(
            plan.iter()
                .map(|item| item.remote_path.as_str())
                .collect::<Vec<_>>(),
            vec!["scripts/src/main.js", "scripts/src/types.ts"]
        );
    }

    #[test]
    fn supports_exact_allowed_upload_extensions() {
        let plan = build_sync_plan_from_entries(
            [
                entry("a.js"),
                entry("b.ts"),
                entry("c.txt"),
                entry("d.script"),
                entry("e.ns"),
                entry("f.json"),
            ],
            &SyncOptions::default(),
        )
        .expect("plan");

        assert_eq!(
            plan.iter()
                .map(|item| item.remote_path.as_str())
                .collect::<Vec<_>>(),
            vec!["a.js", "b.ts", "c.txt", "d.script", "e.ns"]
        );
    }

    #[test]
    fn sync_order_is_deterministic_by_remote_path() {
        let plan = build_sync_plan_from_entries(
            [entry("src/z.js"), entry("src/a.js")],
            &SyncOptions {
                remote_dir: Some("scripts".to_string()),
                ..SyncOptions::default()
            },
        )
        .expect("plan");

        assert_eq!(
            plan.iter()
                .map(|item| item.remote_path.as_str())
                .collect::<Vec<_>>(),
            vec!["scripts/src/a.js", "scripts/src/z.js"]
        );
    }

    #[test]
    fn remote_dir_edge_cases_are_normalized() {
        for (remote_dir, expected) in [
            (Some(""), "main.js"),
            (Some("."), "main.js"),
            (Some("scripts/"), "scripts/main.js"),
            (Some(r"scripts\batch"), "scripts/batch/main.js"),
            (None, "main.js"),
        ] {
            let plan = build_sync_plan_from_entries(
                [entry("main.js")],
                &SyncOptions {
                    remote_dir: remote_dir.map(str::to_string),
                    ..SyncOptions::default()
                },
            )
            .expect("plan");

            assert_eq!(plan[0].remote_path, expected);
        }
    }

    #[test]
    fn remote_dir_parent_segments_fail() {
        let err = build_sync_plan_from_entries(
            [entry("main.js")],
            &SyncOptions {
                remote_dir: Some("../scripts".to_string()),
                ..SyncOptions::default()
            },
        )
        .expect_err("error");

        assert!(err.to_string().contains(".."));
    }
}
