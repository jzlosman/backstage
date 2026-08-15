use std::fs;
use std::path::Path;

#[test]
fn pure_registry_annotation_and_wayfinder_core_has_no_network_process_or_pi_capability() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(crate_root.join("Cargo.toml")).expect("core manifest");
    for forbidden_dependency in ["reqwest", "ureq", "tokio", "tauri", "rusqlite"] {
        assert!(
            !manifest.lines().any(|line| {
                line.trim_start()
                    .starts_with(&format!("{forbidden_dependency}.workspace"))
                    || line
                        .trim_start()
                        .starts_with(&format!("{forbidden_dependency} ="))
            }),
            "pure core unexpectedly depends on {forbidden_dependency}"
        );
    }

    for entry in fs::read_dir(crate_root.join("src")).expect("core sources") {
        let path = entry.expect("source entry").path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("Rust source");
        for forbidden_capability in [
            "std::process::Command",
            "std::net::",
            "TcpStream",
            "PiJobRunner",
            "SystemCommandRunner",
        ] {
            assert!(
                !source.contains(forbidden_capability),
                "{} contains forbidden capability {forbidden_capability}",
                path.display()
            );
        }
    }
}
