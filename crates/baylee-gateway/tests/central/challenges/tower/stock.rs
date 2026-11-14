//! Exercise the scheduled executable's private ready bundle and HTTP recovery.
use super::*;
use std::{fs, io::Write};

struct OwnedDirectory(PathBuf);

impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) fn resume(
    runtime: &Runtime,
    endpoint: &Endpoint,
    credential: &Token,
    prepared: &Prepared,
    publish: bool,
) {
    let binary = std::env::var_os("BAYLEE_CHALLENGE_BINARY")
        .expect("Tower integration requires the matching challenge executable");
    let directory = std::env::temp_dir().join(format!("baylee-tower-stock-{}", Uuid::now_v7()));
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        builder.mode(0o700);
    }
    builder.create(&directory).unwrap();
    let directory = OwnedDirectory(directory);
    let ready = directory.0.join(format!(
        "tower-{}.json",
        prepared.manifest.theme.period.starts_at
    ));
    let mut options = fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    options
        .open(&ready)
        .unwrap()
        .write_all(&serde_json::to_vec(prepared).unwrap())
        .unwrap();
    let output = runtime.block_on(async {
        let mut command = tokio::process::Command::new(binary);
        command
            .args([
                "stock",
                "--central-dev",
                endpoint.origin(),
                "--activities",
                "tower",
                "--days",
                "1",
                "--candidates",
                "1",
                "--source-dir",
            ])
            .arg(baylee_train::working::repo_root())
            .arg("--state-dir")
            .arg(&directory.0)
            .env("BAYLEE_CHALLENGE_BRIDGE_TOKEN", &*credential.hex())
            .kill_on_drop(true);
        tokio::time::timeout(Duration::from_secs(90), command.output())
            .await
            .expect("retained stock must not regenerate the graph")
            .expect("stock process")
    });
    assert!(
        output.status.success(),
        "retained tower stock failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains(if publish {
            ", 1 published"
        } else {
            ", 0 published"
        })
    );
    let path = directory.0.clone();
    drop(directory);
    assert!(!path.exists(), "owned stock evidence removed");
}
