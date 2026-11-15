use super::{Fixture, common};
use baylee_protocol::v1::{self, Envelope};
use std::{
    process::{Child, Command, Stdio},
    time::Duration,
};

pub(super) async fn agent(gateway: &common::Gateway, fixture: &Fixture) -> common::Socket {
    let mut socket = common::dial(&format!("ws://127.0.0.1:{}/agent/ws", gateway.port))
        .await
        .unwrap();
    common::send(
        &mut socket,
        &Envelope {
            msg: Some(v1::envelope::Msg::AgentHello(v1::AgentHello {
                engines: vec![fixture.identity.to_metadata()],
                token: gateway.agent_token.clone(),
                name: "native-tower-fixture".into(),
                capacity: 1,
                protocol_version: baylee_protocol::PROTOCOL_VERSION,
                executor: String::new(),
            })),
        },
    )
    .await;
    assert!(matches!(
        next(&mut socket).await,
        v1::envelope::Msg::AgentWelcome(_)
    ));
    socket
}

pub(super) async fn next(socket: &mut common::Socket) -> v1::envelope::Msg {
    tokio::time::timeout(Duration::from_secs(20), common::next_msg(socket))
        .await
        .expect("native fixture response bound")
        .expect("native fixture message")
}

/// The control peer is synthetic; its child is the separately compiled real Tower binary.
pub(super) struct Engine(Child);
impl Engine {
    pub(super) fn start(start: &v1::StartEngine, fixture: &Fixture) -> Self {
        assert_eq!(start.execution, fixture.identity.to_metadata());
        Self(
            Command::new(&fixture.binary)
                .env_clear()
                .args(["--attach", &start.gateway_url, "--game", &start.game_id])
                .env("BAYLEE_ENGINE_TOKEN", &start.engine_token)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        )
    }
    pub(super) async fn finished(&mut self) {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if let Some(status) = self.0.try_wait().unwrap() {
                    assert!(status.success(), "native Tower child succeeded");
                    return;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .expect("native Tower child exits");
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
        }
        self.0.wait().expect("owned native Tower child reaped");
    }
}

pub(super) async fn concede(gateway: &common::Gateway, grant: &serde_json::Value, expected: &[u8]) {
    let mut socket = common::dial_seat(
        gateway.port,
        grant["game_id"].as_str().unwrap(),
        grant["seat_token"].as_str().unwrap(),
    )
    .await;
    let mut conceded = false;
    let mut ended = false;
    tokio::time::timeout(Duration::from_secs(30), async {
        while let Some(message) = common::next_msg(&mut socket).await {
            let response = match message {
                v1::envelope::Msg::GameStatic(_) => {
                    Some(v1::envelope::Msg::SeatReady(v1::SeatReady {}))
                }
                v1::envelope::Msg::ChoiceRequest(question) => {
                    let pending: baylee_engine::choice::Pending =
                        baylee_core::wire::decode(&question.pending).unwrap();
                    if matches!(pending, baylee_engine::choice::Pending::GameOver(_)) {
                        ended = true;
                        break;
                    }
                    if conceded {
                        continue;
                    }
                    assert_eq!(
                        question.pending.as_slice(),
                        expected,
                        "reconstruction retains the exact observed decision"
                    );
                    conceded = true;
                    Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
                        game_id: String::new(),
                        seat_token: String::new(),
                        action: baylee_core::wire::encode(
                            &baylee_engine::choice::PlayerAction::Concede,
                        ),
                    }))
                }
                v1::envelope::Msg::Error(error) => {
                    panic!("native human input refused: {}", error.message)
                }
                v1::envelope::Msg::TowerFrame(_) | v1::envelope::Msg::GameRecordChunk(_) => {
                    panic!("private Tower evidence leaked to the human seat")
                }
                _ => None,
            };
            if let Some(message) = response {
                common::send(&mut socket, &Envelope { msg: Some(message) }).await;
            }
        }
    })
    .await
    .expect("real native game finishes");
    socket.close(None).await.unwrap();
    assert!(
        conceded && ended,
        "actual human concession and durable terminal view"
    );
}
