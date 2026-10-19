//! New Central login resumes a real game without previous device data or authority.
use super::{Runtime, common};
use baylee_engine::choice::PlayerAction;
use baylee_protocol::v1::{self, Envelope};
use baylee_view::PlayerView;
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
    pub(super) opening: PlayerView,
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
        let opening = runtime.block_on(asked(&mut socket));
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
            opening,
            _agent: agent,
        }
    }

    pub(super) fn assert_revoked(&mut self, runtime: &Runtime) {
        let received = self.probe.seat_frames.load(Ordering::SeqCst);
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
            received,
            "the old device's Concede frame must never reach its engine"
        );
    }
}

/// No old account token, seat token, game ID or local resume file is supplied.
pub(super) struct ResumedSeat {
    pub(super) game: String,
    pub(super) token: String,
    socket: common::Socket,
    view: PlayerView,
}

impl ResumedSeat {
    pub(super) fn claim(runtime: &Runtime, gateway: &common::Gateway, fresh: &str) -> Self {
        let (status, body) = common::http(gateway.port, "GET", "/lobby/seats", Some(fresh), "");
        assert_eq!(status, 200, "fresh installation discovers its seat: {body}");
        let found: serde_json::Value = serde_json::from_str(&body).unwrap();
        let seats = found["seats"].as_array().unwrap();
        assert_eq!(seats.len(), 1);
        assert_eq!(seats[0]["playing"], true);
        let game = seats[0]["game_id"].as_str().unwrap().to_owned();
        let (status, body) = common::http(
            gateway.port,
            "POST",
            &format!("/lobby/games/{game}/seat"),
            Some(fresh),
            "{}",
        );
        assert_eq!(
            status, 200,
            "current account alone reclaims its chair: {body}"
        );
        let token = common::json_field(&body, "seat_token").to_owned();
        let mut socket = runtime.block_on(common::dial_seat(gateway.port, &game, &token));
        let view = runtime.block_on(asked(&mut socket));
        Self {
            game,
            token,
            socket,
            view,
        }
    }

    /// Real rules action after the old connection has closed; a late detach
    /// would leave this seat absent from the engine's routed responses.
    pub(super) fn assert_continues(&mut self, runtime: &Runtime, opening: &PlayerView) {
        assert_eq!(self.view.seat, opening.seat);
        assert!(self.view.seq >= opening.seq);
        assert_eq!(
            self.view.hand, opening.hand,
            "the existing hidden hand survives device replacement"
        );
        runtime.block_on(common::send(
            &mut self.socket,
            &Envelope {
                msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
                    game_id: String::new(),
                    seat_token: String::new(),
                    action_json: serde_json::to_vec(&PlayerAction::MulliganKeep).unwrap(),
                })),
            },
        ));
        let after = runtime.block_on(asked(&mut self.socket));
        assert!(
            after.seq > self.view.seq,
            "the new device can advance the same game"
        );
        assert!(
            !after.lost.iter().any(|lost| lost.seat == after.seat),
            "old socket cleanup cannot mark the current player offline"
        );
        assert!(
            opening
                .hand
                .iter()
                .all(|kept| after.hand.iter().any(|card| card.id == kept.id))
        );
        self.view = after;
    }
}

/// Read the current question and its view without resyncing or reattaching.
async fn asked(socket: &mut common::Socket) -> PlayerView {
    tokio::time::timeout(common::WAIT_BUDGET, async {
        let mut view = None;
        loop {
            match common::next_msg(socket).await {
                Some(v1::envelope::Msg::StateDelta(delta)) => {
                    view = Some(serde_json::from_slice(&delta.view_json).unwrap());
                }
                Some(v1::envelope::Msg::ChoiceRequest(_)) => {
                    if let Some(view) = view.take() {
                        return view;
                    }
                }
                Some(_) => {}
                None => panic!("the running seat closed"),
            }
        }
    })
    .await
    .expect("the real engine asks the resumed player for an action")
}
