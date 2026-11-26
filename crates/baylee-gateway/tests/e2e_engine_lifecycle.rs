//! Broken engine/agent transports must release their exact game assignments.
#![allow(clippy::missing_docs_in_private_items)]

use crate::common;

use baylee_protocol::v1::{self, Envelope};
use common::{Gateway, Socket, http, json_field, next_msg, send};

async fn next(ws: &mut Socket) -> v1::envelope::Msg {
    tokio::time::timeout(common::WAIT_BUDGET, next_msg(ws))
        .await
        .expect("control response deadline")
        .expect("control message")
}

async fn agent(gw: &Gateway) -> Socket {
    let mut ws = common::dial(&format!("ws://127.0.0.1:{}/agent/ws", gw.port))
        .await
        .unwrap();
    send(
        &mut ws,
        &Envelope {
            msg: Some(v1::envelope::Msg::AgentHello(v1::AgentHello {
                engines: vec![common::standard_execution().to_metadata()],
                token: gw.agent_token.clone(),
                name: "lifecycle-test".into(),
                capacity: 1,
                protocol_version: baylee_protocol::PROTOCOL_VERSION,
                executor: String::new(),
            })),
        },
    )
    .await;
    assert!(matches!(
        next(&mut ws).await,
        v1::envelope::Msg::AgentWelcome(_)
    ));
    ws
}

fn create(gw: &Gateway, name: &str) -> (String, String) {
    let token = common::login(gw.port, name, name);
    let (code, body) = http(
        gw.port,
        "POST",
        "/decks",
        Some(&token),
        r#"{"name":"d","cards":["40 Forest","20 Swamp"]}"#,
    );
    assert_eq!(code, 200, "{body}");
    let (code, body) = http(
        gw.port,
        "POST",
        "/lobby/games",
        Some(&token),
        &serde_json::json!({"deck_id":json_field(&body,"deck_id"),"mode":"ai"}).to_string(),
    );
    assert_eq!(code, 200, "{body}");
    (token, json_field(&body, "game_id").to_string())
}

async fn ordered(agent: &mut Socket, id: &str) -> v1::StartEngine {
    let v1::envelope::Msg::StartEngine(start) = next(agent).await else {
        panic!("start order")
    };
    assert_eq!(start.game_id, id);
    start
}

async fn engine(start: &v1::StartEngine) -> Socket {
    let mut ws = common::dial(&start.gateway_url).await.unwrap();
    send(
        &mut ws,
        &Envelope {
            msg: Some(v1::envelope::Msg::EngineHello(v1::EngineHello {
                execution: common::standard_execution().to_metadata(),
                game_id: start.game_id.clone(),
                token: start.engine_token.clone(),
                protocol_version: baylee_protocol::PROTOCOL_VERSION,
                pid: 0,
                build: baylee_build::short().into(),
            })),
        },
    )
    .await;
    assert!(matches!(
        next(&mut ws).await,
        v1::envelope::Msg::GameSetup(_)
    ));
    ws
}

async fn exited(agent: &mut Socket, id: &str) {
    send(
        agent,
        &Envelope {
            msg: Some(v1::envelope::Msg::EngineStatus(v1::EngineStatus {
                game_id: id.into(),
                kind: v1::engine_status::Kind::Exited.into(),
                detail: "exit status: 1".into(),
            })),
        },
    )
    .await;
}

fn listed(gw: &Gateway, token: &str, id: &str) -> bool {
    let (code, body) = http(gw.port, "GET", "/lobby/games", Some(token), "");
    assert_eq!(code, 200, "{body}");
    body.contains(id)
}

async fn gone(gw: &Gateway, token: &str, id: &str) {
    tokio::time::timeout(common::WAIT_BUDGET, async {
        while listed(gw, token, id) {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("game must release its lobby assignment");
}

#[tokio::test]
async fn an_exit_before_hello_releases_the_room_and_agent_capacity() {
    let gw = common::spawn_gateway("lifecycle-early-exit");
    let mut agent = agent(&gw).await;
    let (token, id) = create(&gw, "early");
    ordered(&mut agent, &id).await;
    exited(&mut agent, &id).await;
    assert!(
        matches!(next(&mut agent).await,v1::envelope::Msg::StopEngine(stop) if stop.game_id == id)
    );
    gone(&gw, &token, &id).await;
    let (_, second) = create(&gw, "next");
    ordered(&mut agent, &second).await;
    agent.close(None).await.unwrap();
}

#[tokio::test]
async fn the_requested_artifact_is_checked_after_auth_and_before_private_setup() {
    let gw = common::spawn_gateway("execution-contract");
    let mut owner = agent(&gw).await;
    let (token, id) = create(&gw, "contract");
    let start = ordered(&mut owner, &id).await;
    let expected = baylee_protocol::execution::Identity::from_metadata(&start.execution).unwrap();
    assert_eq!(expected, common::standard_execution());
    let mut wrong = expected.clone();
    wrong.features = "b".repeat(64);
    for authorized in [false, true] {
        let mut socket = common::dial(&start.gateway_url).await.unwrap();
        send(
            &mut socket,
            &Envelope {
                msg: Some(v1::envelope::Msg::EngineHello(v1::EngineHello {
                    game_id: id.clone(),
                    token: if authorized {
                        start.engine_token.clone()
                    } else {
                        "wrong-token".into()
                    },
                    protocol_version: baylee_protocol::PROTOCOL_VERSION,
                    pid: 0,
                    build: expected.build.clone(),
                    execution: wrong.to_metadata(),
                })),
            },
        )
        .await;
        let answer = tokio::time::timeout(common::WAIT_BUDGET, next_msg(&mut socket))
            .await
            .unwrap();
        if authorized {
            assert!(matches!(answer, Some(v1::envelope::Msg::Error(_))));
            assert!(matches!(
                next(&mut owner).await,
                v1::envelope::Msg::StopEngine(_)
            ));
            gone(&gw, &token, &id).await;
        } else {
            assert!(answer.is_none());
            assert!(
                listed(&gw, &token, &id),
                "unauthenticated claims cannot end the reservation"
            );
        }
        let _ = socket.close(None).await;
    }
    let (_, next_id) = create(&gw, "correct");
    let next_start = ordered(&mut owner, &next_id).await;
    let mut correct = engine(&next_start).await;
    correct.close(None).await.unwrap();
    owner.close(None).await.unwrap();
}

#[tokio::test]
async fn losing_an_agent_closes_only_its_pending_and_attached_games() {
    for attached in [false, true] {
        let gw = common::spawn_gateway(if attached {
            "lifecycle-attached"
        } else {
            "lifecycle-pending"
        });
        let mut owner = agent(&gw).await;
        let (token, id) = create(&gw, "owned");
        let start = ordered(&mut owner, &id).await;
        let mut engine = if attached {
            Some(engine(&start).await)
        } else {
            None
        };
        let mut other = agent(&gw).await;
        let (other_token, other_id) = create(&gw, "other");
        ordered(&mut other, &other_id).await;
        // An agent cannot end another agent's assignment through its status link.
        exited(&mut other, &id).await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(listed(&gw, &token, &id));
        owner.close(None).await.unwrap();
        gone(&gw, &token, &id).await;
        assert!(listed(&gw, &other_token, &other_id));
        if let Some(engine) = &mut engine {
            let _ = engine.close(None).await;
        }
        other.close(None).await.unwrap();
    }
}

#[tokio::test]
async fn exit_status_does_not_overtake_an_attached_engines_final_messages() {
    let gw = common::spawn_gateway("lifecycle-final-drain");
    let mut owner = agent(&gw).await;
    let (token, id) = create(&gw, "drain");
    let start = ordered(&mut owner, &id).await;
    let mut engine = engine(&start).await;
    exited(&mut owner, &id).await;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(listed(&gw, &token, &id));
    let mut written = Vec::new();
    for (seq, bytes) in [
        b"{\"type\":\"header\"}\n".as_slice(),
        b"{\"type\":\"end\"}\n".as_slice(),
    ]
    .into_iter()
    .enumerate()
    {
        use std::io::Write as _;
        let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gzip.write_all(bytes).unwrap();
        let data = gzip.finish().unwrap();
        written.extend_from_slice(bytes);
        send(
            &mut engine,
            &Envelope {
                msg: Some(v1::envelope::Msg::GameRecordChunk(v1::GameRecordChunk {
                    game_id: id.clone(),
                    seq: u32::try_from(seq).unwrap(),
                    data,
                    last: seq == 1,
                })),
            },
        )
        .await;
    }
    send(
        &mut engine,
        &Envelope {
            msg: Some(v1::envelope::Msg::GameEnded(v1::GameEnded {
                game_id: id.clone(),
                winners: vec![],
                reason: "test ending".into(),
                turn: 1,
                challenge: None,
            })),
        },
    )
    .await;
    assert!(
        matches!(next(&mut owner).await,v1::envelope::Msg::StopEngine(stop) if stop.game_id == id)
    );
    gone(&gw, &token, &id).await;
    // Record writes are asynchronous, but must retain every byte and ending
    // after the control status has overtaken the engine's final stream. A
    // finished record may be compacted into one piece, so the bytes are
    // compared unpacked.
    tokio::time::timeout(common::WAIT_BUDGET, async {
        while gw.scalar(&format!("SELECT count(*) FROM game_record WHERE game_id='{id}' AND complete AND end_reason='test ending'")) != 1 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    let stored = gw.text(&format!("SELECT encode(string_agg(data, ''::bytea ORDER BY seq),'hex') FROM game_record_chunk WHERE game_id='{id}'"));
    let packed = (0..stored.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&stored[at..at + 2], 16).unwrap())
        .collect::<Vec<u8>>();
    let mut unpacked = Vec::new();
    std::io::Read::read_to_end(
        &mut flate2::read::MultiGzDecoder::new(&packed[..]),
        &mut unpacked,
    )
    .unwrap();
    assert_eq!(unpacked, written);
    let _ = engine.close(None).await;
    owner.close(None).await.unwrap();
}

#[tokio::test]
async fn an_engine_that_never_connects_has_a_bounded_lifetime() {
    let gw = common::spawn_gateway("lifecycle-no-hello");
    let mut owner = agent(&gw).await;
    let (token, id) = create(&gw, "silent");
    ordered(&mut owner, &id).await;
    let stopped = tokio::time::timeout(std::time::Duration::from_secs(40), next_msg(&mut owner))
        .await
        .unwrap();
    assert!(matches!(stopped,Some(v1::envelope::Msg::StopEngine(stop)) if stop.game_id == id));
    gone(&gw, &token, &id).await;
    owner.close(None).await.unwrap();
}
