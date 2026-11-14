//! Metadata process bounds and exact installed-artifact routing.
#![allow(clippy::missing_docs_in_private_items)]
mod support;

#[cfg(unix)]
#[tokio::test]
async fn shutdown_during_inventory_reaps_the_actual_agents_probe() {
    use futures_util::FutureExt as _;
    use std::{panic::AssertUnwindSafe, process::Stdio, time::Duration};
    let temp = tempfile::tempdir().unwrap();
    let executable = support::executable(temp.path());
    std::fs::write(temp.path().join("mode"), "timeout").unwrap();
    let mut agent = tokio::process::Command::new(env!("CARGO_BIN_EXE_baylee-agent"))
        .env("BAYLEE_AGENT_TOKEN", "synthetic-shutdown-test")
        .env("BAYLEE_ENGINE_BIN", executable)
        .env("BAYLEE_ENGINE_BINS", "")
        .env("BAYLEE_GATEWAY", "http://127.0.0.1:1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let result = AssertUnwindSafe(tokio::time::timeout(Duration::from_secs(5), async {
        let probe = loop {
            if let Ok(pid) = std::fs::read_to_string(temp.path().join("probe.pid")) {
                break pid;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        assert!(
            tokio::process::Command::new("/bin/kill")
                .args(["-TERM", &agent.id().unwrap().to_string()])
                .status()
                .await
                .unwrap()
                .success()
        );
        assert!(agent.wait().await.unwrap().success());
        let probe_exists = std::process::Command::new("/bin/kill")
            .args(["-0", &probe])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(!probe_exists.success(), "metadata probe survived its owner");
    }))
    .catch_unwind()
    .await;
    let _ = agent.start_kill();
    agent.wait().await.unwrap();
    result.unwrap().unwrap();
}

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
