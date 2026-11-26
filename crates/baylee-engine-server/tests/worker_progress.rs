//! Private progress from the real attached executable, not a driver mock.
use baylee_cards_pool as _;
use baylee_protocol::v1::{self, Envelope};
use futures_util::{FutureExt as _, SinkExt as _, StreamExt as _};
use prost::Message as _;
use std::{panic::AssertUnwindSafe, process::Stdio, time::Duration};
use tokio::io::AsyncReadExt as _;

#[tokio::test]
async fn the_attached_engine_reports_work_privately_and_keeps_logs_off_the_pipe() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_baylee-engine-server"))
        .args([
            "--attach",
            &format!("ws://{address}/engine/ws"),
            "--game",
            "worker-test",
        ])
        .env("BAYLEE_ENGINE_TOKEN", "synthetic-worker-test-token")
        .env_remove("BAYLEE_AGENT_TOKEN")
        .env("BAYLEE_FORESIGHT", "off")
        .env(baylee_worker::REPORT_ENV, "1")
        .env("RUST_LOG", "info")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let result = AssertUnwindSafe(tokio::time::timeout(Duration::from_secs(10), async {
        let owner_pipe = child.stdin.take().unwrap();
        let mut reports = child.stdout.take().unwrap();
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
        let hello = socket.next().await.unwrap().unwrap();
        let Some(v1::envelope::Msg::EngineHello(hello)) =
            Envelope::decode(hello.into_data()).unwrap().msg
        else {
            panic!("the engine authenticates before receiving a game")
        };
        assert_eq!(
            hello.execution,
            baylee_engine_server::variant::identity().to_metadata()
        );
        assert_eq!(reports.read_u8().await.unwrap(), baylee_worker::IDLE);
        let deck = baylee_cards::decks::house_deck("[deck:Worker]\n60 Forest\n").unwrap();
        let mut preset = baylee_cards::decks::preset_for_all(11, &[&deck, &deck]);
        preset.house_rules.loop_policy = baylee_engine_server::variant::default_loop_policy();
        for seat in &mut preset.seats {
            seat.controller = baylee_core::preset::SeatController::Open;
        }
        let setup = Envelope {
            msg: Some(v1::envelope::Msg::GameSetup(v1::GameSetup {
                execution: hello.execution,
                game_id: "worker-test".into(),
                preset_json: serde_json::to_vec(&preset).unwrap(),
                seat_names: vec!["One".into(), "Two".into()],
                seat_accounts: vec![false, false],
                seat_guests: vec![],
                ..Default::default()
            })),
        };
        socket
            .send(tokio_tungstenite::tungstenite::Message::Binary(
                setup.encode_to_vec().into(),
            ))
            .await
            .unwrap();
        assert_eq!(reports.read_u8().await.unwrap(), baylee_worker::BUSY);
        assert_eq!(reports.read_u8().await.unwrap(), baylee_worker::IDLE);
        // The owner vanishes while the gateway socket is still open.
        // Its independent liveness thread must stop the actual engine.
        drop(owner_pipe);
        assert!(!child.wait().await.unwrap().success());
        let _ = socket.close(None).await;
    }))
    .catch_unwind()
    .await;
    // Every failure above comes through here before it is asserted.
    let _ = child.start_kill();
    let exited = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
    drop(listener);
    assert!(tokio::net::TcpStream::connect(address).await.is_err());
    exited.unwrap().unwrap();
    result.unwrap().unwrap();
}
