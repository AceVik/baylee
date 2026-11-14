//! Metadata process bounds and exact installed-artifact routing.
#![allow(clippy::missing_docs_in_private_items)]
mod support;

#[tokio::test]
async fn probes_are_bounded_reaped_and_never_silently_dropped_from_inventory() {
    let temp = tempfile::tempdir().unwrap();
    let executable = std::fs::canonicalize(support::executable(temp.path())).unwrap();
    let paths = [executable.clone()];
    let valid = baylee_agent::inventory::Inventory::discover(&paths)
        .await
        .unwrap();
    assert_eq!(
        valid.executable(&support::identity()),
        Some(executable.as_path())
    );
    for mode in ["huge", "failure", "timeout"] {
        std::fs::write(temp.path().join("mode"), mode).unwrap();
        assert!(
            baylee_agent::inventory::Inventory::discover(&paths)
                .await
                .is_err(),
            "{mode}"
        );
        #[cfg(unix)]
        {
            let pid = std::fs::read_to_string(temp.path().join("probe.pid")).unwrap();
            let status = std::process::Command::new("/bin/kill")
                .args(["-0", &pid])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .unwrap();
            assert!(!status.success(), "probe {mode} was not reaped");
        }
    }
    std::fs::write(temp.path().join("mode"), "").unwrap();
    std::fs::write(temp.path().join("metadata"), b"{}\n{}").unwrap();
    assert!(
        baylee_agent::inventory::Inventory::discover(&paths)
            .await
            .is_err()
    );
}
