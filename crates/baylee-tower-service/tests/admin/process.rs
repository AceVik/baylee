use serde_json::{Value, json};
use std::{
    process::{Output, Stdio},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt as _, AsyncWriteExt as _},
    process::Command,
};

pub struct Client {
    pub configuration: Value,
}

impl Client {
    pub fn new(schema: &str) -> Self {
        Self {
            configuration: json!({"database_url": std::env::var("DATABASE_URL").unwrap(),
            "schema": schema, "development": true}),
        }
    }

    pub async fn call(&self, mode: &str, operation: Value) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_baylee-tower-service"))
            .arg(mode)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let output = tokio::spawn(async move {
            let mut bytes = Vec::new();
            stdout
                .take(4097)
                .read_to_end(&mut bytes)
                .await
                .map(|_| bytes)
        });
        let error = tokio::spawn(async move {
            let mut bytes = Vec::new();
            stderr
                .take(4097)
                .read_to_end(&mut bytes)
                .await
                .map(|_| bytes)
        });
        let input = serde_json::to_vec(&json!({"configuration": self.configuration,
            "operation": operation}))
        .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let sent = tokio::time::timeout(Duration::from_secs(5), stdin.write_all(&input)).await;
        drop(stdin);
        let status = tokio::time::timeout(Duration::from_secs(10), child.wait()).await;
        if !matches!(status, Ok(Ok(_))) {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
        let stdout = output.await.unwrap().unwrap();
        let stderr = error.await.unwrap().unwrap();
        assert!(
            matches!(sent, Ok(Ok(()))),
            "private command input reached owned child"
        );
        assert!(stdout.len() <= 4096 && stderr.len() <= 4096);
        Output {
            status: status.expect("owned command deadline").unwrap(),
            stdout,
            stderr,
        }
    }

    pub async fn ok(&self, operation: Value) -> Value {
        let output = self.call("--admin", operation).await;
        succeeds(&output);
        serde_json::from_slice(&output.stdout).unwrap()
    }

    pub async fn refused(&self, operation: Value) {
        refused(&self.call("--admin", operation).await);
    }
}

pub fn succeeds(output: &Output) {
    assert!(output.status.success(), "offline command succeeded");
    assert!(
        output.stderr.is_empty(),
        "no successful diagnostic contains private input"
    );
}

pub fn refused(output: &Output) {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        output.stderr == b"Tower service configuration, storage or worker failed\n",
        "refusal diagnostics contain only the fixed public error"
    );
}
