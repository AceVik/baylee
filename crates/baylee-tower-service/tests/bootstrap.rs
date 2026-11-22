//! Owned standalone process, synthetic PostgreSQL and graceful Unix service shutdown.
#![forbid(unsafe_code)]
#![cfg(unix)]
#[path = "support/command.rs"]
mod command;
#[path = "../../baylee-tower-store/tests/support/mod.rs"]
mod support;

use bytes::Bytes;
use http_body_util::{BodyExt as _, Empty, Limited};
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use serde_json::json;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader},
    process::{Child, Command},
};

use command::command;

async fn configured(mut command: Command, config: &serde_json::Value) -> Child {
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(config).unwrap())
        .await
        .unwrap();
    child
}

#[tokio::test]
async fn standalone_configuration_migration_start_and_shutdown_are_isolated() {
    let sandbox = support::Sandbox::open().await;
    let config = json!({ "database_url": std::env::var("DATABASE_URL").unwrap(),
        "schema": sandbox.schema, "development": true, "service": {
            "listen": "127.0.0.1:0", "origin": "http://127.0.0.1:29871", "central": "http://127.0.0.1:29872",
            "record_bytes": 1_048_576, "workers": [] } });
    let preliminary = tokio::spawn({
        let config = config.clone();
        let store = sandbox.store.clone();
        async move {
            let publication = support::publication();
            let id = store.publish(&publication).await.unwrap();
            configuration_checks(config).await;
            assert_eq!(
                store.published(id).await.unwrap(),
                publication,
                "explicit migration preserves existing publication"
            );
        }
    })
    .await;
    if preliminary.is_err() {
        sandbox.close().await;
        preliminary.unwrap();
        return;
    }
    let mut child = configured(command(&config), &config).await;
    let stdout = child.stdout.take().unwrap();
    let address = Arc::new(Mutex::new(None));
    let observed = tokio::spawn({
        let address = address.clone();
        async move {
            let mut reader = BufReader::new(stdout.take(256));
            let mut line = String::new();
            tokio::time::timeout(Duration::from_secs(10), reader.read_line(&mut line))
                .await
                .unwrap()
                .unwrap();
            let listen: SocketAddr = line
                .trim()
                .strip_prefix("Tower ready on ")
                .unwrap()
                .parse()
                .unwrap();
            *address.lock().unwrap() = Some(listen);
            assert!(listen.ip().is_loopback());
            let client: Client<_, Empty<Bytes>> =
                Client::builder(TokioExecutor::new()).build_http();
            let response = client
                .get(format!("http://{listen}/health").parse().unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), 200);
            assert_eq!(
                Limited::new(response.into_body(), 64)
                    .collect()
                    .await
                    .unwrap()
                    .to_bytes(),
                "ready"
            );
            let request = hyper::Request::post(format!("http://{listen}/v2/player"))
                .body(Empty::new())
                .unwrap();
            assert_ne!(
                client.request(request).await.unwrap().status(),
                200,
                "development never bypasses authentication"
            );
        }
    })
    .await;
    let closed = stop(&mut child).await;
    let port = *address.lock().unwrap();
    let port_closed = match port {
        Some(port) => matches!(
            tokio::time::timeout(Duration::from_secs(2), tokio::net::TcpStream::connect(port))
                .await,
            Ok(Err(_))
        ),
        None => false,
    };
    sandbox.close().await;
    assert!(
        closed && port_closed,
        "owned standalone process exited and port closed"
    );
    observed.expect("standalone HTTP scenario passed");
}

async fn configuration_checks(mut config: serde_json::Value) {
    let mut check = command(&config);
    check.arg("--check");
    let output = configured(check, &config)
        .await
        .wait_with_output()
        .await
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    let mut migrate = command(&config);
    migrate.arg("--migrate");
    let output = configured(migrate, &config)
        .await
        .wait_with_output()
        .await
        .unwrap();
    assert!(
        output.status.success(),
        "explicit synthetic migration is idempotent"
    );
    config["database_url"] = json!("PRIVATE_CONFIGURATION_CANARY");
    let mut check = command(&config);
    check.arg("--check");
    let output = configured(check, &config)
        .await
        .wait_with_output()
        .await
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("PRIVATE_CONFIGURATION_CANARY"));
}

async fn stop(child: &mut Child) -> bool {
    if matches!(child.try_wait(), Ok(Some(_))) {
        return false;
    }
    let Some(pid) = child.id() else {
        return false;
    };
    let signalled = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status()
        .await
        .is_ok_and(|status| status.success());
    if !signalled {
        let _ = child.start_kill();
    }
    if let Ok(Ok(status)) = tokio::time::timeout(Duration::from_secs(15), child.wait()).await {
        signalled && status.success()
    } else {
        let _ = child.start_kill();
        let _ = child.wait().await;
        false
    }
}
