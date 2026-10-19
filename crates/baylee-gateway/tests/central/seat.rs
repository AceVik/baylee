//! An old seat socket must never forward a post-revocation action to the real engine.
use super::{Runtime, common};
use baylee_protocol::v1::{self, Envelope};
use futures_util::StreamExt as _;
use std::sync::atomic::Ordering;

struct Agent(tokio::task::JoinHandle<()>);
impl Drop for Agent {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub(super) struct LiveSeat {
    pub(super) game: String,
    pub(super) token: String,
    socket: common::Socket,
    probe: common::EngineProbe,
    received: usize,
    _agent: Agent,
}

impl LiveSeat {
    pub(super) fn open(runtime: &Runtime, gateway: &common::Gateway, player: &str) -> Self {
        let (agent, probe) = runtime.block_on(common::attach_agent_probed(gateway));
        let agent = Agent(agent);
        let (status, deck) = common::http(
            gateway.port,
            "POST",
            "/decks",
            Some(player),
            r#"{"name":"revocation fixture","cards":["40 Forest","20 Swamp"]}"#,
        );
        assert_eq!(status, 200, "synthetic deck: {deck}");
        let body = serde_json::json!({"deck_id":common::json_field(&deck, "deck_id"),"mode":"ai"})
            .to_string();
        let (status, game) =
            common::http(gateway.port, "POST", "/lobby/games", Some(player), &body);
        assert_eq!(status, 200, "synthetic game: {game}");
        let game_id = common::json_field(&game, "game_id").to_owned();
        let token = common::json_field(&game, "seat_token").to_owned();
        let mut socket = runtime.block_on(common::dial_seat(gateway.port, &game_id, &token));
        runtime.block_on(async {
            common::send(&mut socket, &Envelope { msg: Some(v1::envelope::Msg::ClockProbe(v1::ClockProbe {
                client_time_ms:4242, server_time_ms:0,
            })) }).await;
            tokio::time::timeout(common::WAIT_BUDGET, async {
                while let Some(message) = common::next_msg(&mut socket).await {
                    if matches!(message, v1::envelope::Msg::ClockProbe(back) if back.client_time_ms==4242) { return; }
                }
                panic!("live authorized socket must reach its engine");
            }).await.unwrap();
        });
        let received = probe.seat_frames.load(Ordering::SeqCst);
        assert!(
            received >= 2,
            "the probe sees ready and the roundtrip frame"
        );
        Self {
            game: game_id,
            token,
            socket,
            probe,
            received,
            _agent: agent,
        }
    }

    pub(super) fn assert_revoked(&mut self, runtime: &Runtime) {
        runtime.block_on(async {
            common::send(
                &mut self.socket,
                &Envelope {
                    msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
                        game_id: String::new(),
                        seat_token: String::new(),
                        action_json: serde_json::to_vec(
                            &baylee_engine::choice::PlayerAction::Concede,
                        )
                        .unwrap(),
                    })),
                },
            )
            .await;
            tokio::time::timeout(std::time::Duration::from_secs(8), async {
                while let Some(frame) = self.socket.next().await {
                    if matches!(
                        frame,
                        Ok(tokio_tungstenite::tungstenite::Message::Close(_)) | Err(_)
                    ) {
                        return;
                    }
                }
            })
            .await
            .expect("revoked game socket must close");
        });
        assert_eq!(
            self.probe.seat_frames.load(Ordering::SeqCst),
            self.received,
            "the old device's Concede frame must never reach its engine"
        );
    }
}
