#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

pub type RepositoryManifest = BTreeMap<PathBuf, ManifestEntry>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ManifestEntry {
    Directory,
    File(Vec<u8>),
    Symlink(PathBuf),
}

pub struct FixtureRepo {
    temp: TempDir,
}

impl FixtureRepo {
    pub fn open_spec() -> Self {
        let temp = TempDir::new().expect("fixture tempdir");
        let root = temp.path();
        fs::create_dir_all(root.join("openspec/changes/ship-search/specs/search"))
            .expect("fixture directories");
        fs::write(root.join("README.md"), "# Fixture project\n").expect("fixture README");
        fs::write(
            root.join("openspec/changes/ship-search/proposal.md"),
            "# Ship search\n\nAdd local bundle search.\n",
        )
        .expect("fixture proposal");
        fs::write(
            root.join("openspec/changes/ship-search/design.md"),
            "# Design\n\nUse deterministic matching.\n",
        )
        .expect("fixture design");
        fs::write(
            root.join("openspec/changes/ship-search/tasks.md"),
            "# Tasks\n\n- [x] Parse query\n- [ ] Filter bundles\n",
        )
        .expect("fixture tasks");
        fs::write(
            root.join("openspec/changes/ship-search/specs/search/spec.md"),
            "# Search\n\n## ADDED Requirements\n",
        )
        .expect("fixture spec");
        fs::write(root.join("PLAN.md"), "# Candidate plan\n").expect("fixture candidate");

        run_git(root, &["init", "-q"]);
        run_git(root, &["config", "user.name", "Backstage Fixture"]);
        run_git(root, &["config", "user.email", "fixture@backstage.invalid"]);
        run_git(root, &["add", "."]);
        run_git(root, &["commit", "-qm", "fixture"]);

        Self { temp }
    }

    pub fn path(&self) -> &Path {
        self.temp.path()
    }

    pub fn manifest(&self) -> RepositoryManifest {
        repository_manifest(self.path())
    }

    pub fn assert_unchanged(&self, before: &RepositoryManifest) {
        assert_eq!(
            &self.manifest(),
            before,
            "a read-only integration flow changed repository structure or bytes"
        );
    }
}

pub fn repository_manifest(root: &Path) -> RepositoryManifest {
    fn visit(root: &Path, path: &Path, entries: &mut RepositoryManifest) {
        let mut children = fs::read_dir(path)
            .expect("fixture directory should be readable")
            .map(|entry| entry.expect("fixture entry should be readable"))
            .collect::<Vec<_>>();
        children.sort_by_key(|entry| entry.path());

        for child in children {
            let path = child.path();
            let relative = path
                .strip_prefix(root)
                .expect("entry stays in fixture")
                .to_path_buf();
            let kind = child
                .file_type()
                .expect("fixture metadata should be readable");
            if kind.is_symlink() {
                entries.insert(
                    relative,
                    ManifestEntry::Symlink(fs::read_link(path).expect("symlink target")),
                );
            } else if kind.is_dir() {
                entries.insert(relative, ManifestEntry::Directory);
                visit(root, &path, entries);
            } else if kind.is_file() {
                entries.insert(
                    relative,
                    ManifestEntry::File(fs::read(path).expect("fixture file should be readable")),
                );
            }
        }
    }

    let mut entries = BTreeMap::new();
    visit(root, root, &mut entries);
    entries
}

fn run_git(root: &Path, arguments: &[&str]) {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(root)
        .output()
        .expect("git should be installed");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        arguments,
        String::from_utf8_lossy(&output.stderr)
    );
}
