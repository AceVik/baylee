//! Actual SIGTERM must reap the engines started by the agent executable.
#![cfg(target_os = "linux")]
#![allow(clippy::missing_docs_in_private_items)]

use baylee_protocol::v1::{self, Envelope};
use futures_util::{SinkExt as _, StreamExt as _};
use prost::Message as _;
use std::os::unix::fs::PermissionsExt as _;
use std::time::Duration;

/// A private process group contains this test's agent and its fake engines.
/// Failure cleanup never signals a live service or a process it did not start.
struct OwnedGroup(Option<u32>);
impl Drop for OwnedGroup {
    fn drop(&mut self) {
        let Some(pid) = self.0.take() else { return };
        let _ = std::process::Command::new("/bin/kill")
            .args(["-KILL", "--", &format!("-{pid}")])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
}

#[tokio::test]
async fn sigterm_reaps_the_actual_agents_owned_engine() {
    let temp = tempfile::tempdir().unwrap();
    let fake = temp.path().join("fake-engine");
    let pidfile = temp.path().join("engine.pid");
    std::fs::write(
        &fake,
        "#!/bin/sh\nprintf '%s' \"$$\" > \"$BAYLEE_TEST_CHILD_PID\"\nexec /bin/sleep 60\n",
    )
    .unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o700)).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut agent = tokio::process::Command::new(env!("CARGO_BIN_EXE_baylee-agent"))
        .env("BAYLEE_AGENT_TOKEN", "test-placeholder")
        .env("BAYLEE_GATEWAY", format!("http://{address}"))
        .env("BAYLEE_ENGINE_BIN", &fake)
        .env("BAYLEE_TEST_CHILD_PID", &pidfile)
        .process_group(0)
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let pid = agent.id().unwrap();
    let mut group = OwnedGroup(Some(pid));
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        let (tcp, _) = listener.accept().await?;
        let mut ws = tokio_tungstenite::accept_async(tcp).await?;
        let hello = ws.next().await.ok_or("missing hello")??;
        let hello = Envelope::decode(hello.into_data())?;
        if !matches!(hello.msg, Some(v1::envelope::Msg::AgentHello(_))) {
            return Err("wrong hello".into());
        }
        ws.send(tokio_tungstenite::tungstenite::Message::Binary(
            Envelope {
                msg: Some(v1::envelope::Msg::StartEngine(v1::StartEngine {
                    game_id: "shutdown-test".into(),
                    engine_token: "test-placeholder".into(),
                    gateway_url: format!("ws://{address}/engine/ws"),
                })),
            }
            .encode_to_vec()
            .into(),
        ))
        .await?;
        while !pidfile.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let child: u32 = std::fs::read_to_string(&pidfile)?.parse()?;
        if !std::path::Path::new(&format!("/proc/{child}")).exists() {
            return Err("fake engine did not start".into());
        }
        let signal = tokio::process::Command::new("/bin/kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .await?;
        if !signal.success() {
            return Err("SIGTERM failed".into());
        }
        let exit = agent.wait().await?;
        if !exit.success() {
            return Err("agent did not shut down successfully".into());
        }
        if std::path::Path::new(&format!("/proc/{child}")).exists() {
            return Err("engine was not reaped before agent exit".into());
        }
        // Agent success plus disappearance of its only engine proves cleanup.
        // Disarm before the reaped leader PID can ever be reused.
        group.0 = None;
        drop(ws);
        Ok::<(), Box<dyn std::error::Error>>(())
    })
    .await;
    // Only on failure/timeout: stop this test's group, then reap the agent.
    drop(group);
    let _ = agent.wait().await;
    assert!(
        matches!(result, Ok(Ok(()))),
        "shutdown regression: {result:?}"
    );
}
