use backstage_core::{ApprovedRoot, ArtifactPath, DomainError};

#[test]
fn approved_root_rejects_relative_paths() {
    let error = ApprovedRoot::new("projects", true).expect_err("relative root must fail");

    assert_eq!(error, DomainError::RootMustBeAbsolute);
    assert_eq!(
        serde_json::to_value(error).expect("error serializes"),
        serde_json::json!({ "code": "root_must_be_absolute" })
    );
}

#[test]
fn approved_root_rejects_non_directories() {
    let error = ApprovedRoot::new("/Users/dev/file.md", false).expect_err("files must fail");

    assert_eq!(error, DomainError::RootMustBeDirectory);
}

#[test]
fn approved_root_normalizes_lexically_and_has_a_stable_id() {
    let first = ApprovedRoot::new("/Users/dev/work/../projects/./", true).expect("valid root");
    let second = ApprovedRoot::new("/Users/dev/projects", true).expect("valid root");

    assert_eq!(first.path(), "/Users/dev/projects");
    assert_eq!(first.id(), second.id());
    assert!(first.id().starts_with("root_"));
}

#[test]
fn artifact_path_must_be_an_absolute_file_beneath_its_root() {
    let root = ApprovedRoot::new("/Users/dev/projects", true).expect("valid root");

    assert_eq!(
        ArtifactPath::new(&root, "proposal.md", true).expect_err("relative artifact must fail"),
        DomainError::ArtifactPathMustBeAbsolute
    );
    assert_eq!(
        ArtifactPath::new(&root, "/Users/dev/other/proposal.md", true)
            .expect_err("outside artifact must fail"),
        DomainError::OutsideApprovedRoot
    );
    assert_eq!(
        ArtifactPath::new(&root, "/Users/dev/projects/proposal.md", false)
            .expect_err("directory artifact must fail"),
        DomainError::ArtifactMustBeFile
    );
}

#[test]
fn artifact_path_exposes_normalized_relative_path_and_stable_id() {
    let root = ApprovedRoot::new("/Users/dev/projects", true).expect("valid root");
    let artifact = ArtifactPath::new(
        &root,
        "/Users/dev/projects/openspec/changes/search/../search/tasks.md",
        true,
    )
    .expect("contained artifact");

    assert_eq!(
        artifact.absolute(),
        "/Users/dev/projects/openspec/changes/search/tasks.md"
    );
    assert_eq!(artifact.relative(), "openspec/changes/search/tasks.md");
    assert!(artifact.id().starts_with("artifact_"));
}
