//! Kill one owned native child after a real durable view, then recover the same prefix.
use super::{Fixture, Runtime, Served, common, native};
use baylee_protocol::v1::{self, Envelope};
use baylee_tower::v2::run::{Phase, Run};
use sea_orm::ConnectionTrait;
use std::time::Duration;
use uuid::Uuid;

pub(super) fn crash(
    runtime: &Runtime,
    served: &Served,
    run: Uuid,
    grant: &serde_json::Value,
    agent: &mut common::Socket,
) -> Vec<u8> {
    let fixture = served.tower.as_ref().unwrap();
    runtime.block_on(async {
        let v1::envelope::Msg::StartEngine(start) = native::next(agent).await else { panic!("first native execution"); };
        assert_eq!(start.game_id, grant["game_id"].as_str().unwrap());
        let engine = native::Engine::start(&start, fixture);
        let (mut socket, pending) = observed_choice(&served.gateway, grant).await;
        let before = saved(fixture, run).await;
        let retained = before.execution.as_ref().unwrap();
        assert!(retained.prefix.bytes > 0 && retained.prefix.chunks > 0, "observed choices have a durable prefix");
        drop(engine); // RAII kills and reaps only this child.
        let v1::envelope::Msg::StopEngine(stopped) = native::next(agent).await else { panic!("gateway closes interrupted execution before its successor"); };
        assert_eq!(stopped.game_id, start.game_id);
        let _ = socket.close(None).await;
        let schema = &fixture.sandbox.schema;
        // Expire this owned test lease without a minute-long wall-clock wait.
        // Production has no player/host route to perform this mutation.
        fixture.sandbox.db.execute_unprepared(&format!("UPDATE \"{schema}\".runs SET state=convert_to(jsonb_set(convert_from(state,'UTF8')::jsonb,'{{execution,lease_until}}','1'::jsonb)::text,'UTF8') WHERE id='{run}';
            UPDATE \"{schema}\".execution_watch SET lease_until=1 WHERE run='{run}';")).await.unwrap();
        let recovered = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let next = saved(fixture, run).await;
                if next.phase == Phase::Reserved { return next; }
                assert_eq!(next.phase, Phase::Playing, "a valid retained prefix needs no technical quarantine");
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }).await.expect("automatic expiry supervision returns the same reservation");
        let next = recovered.execution.as_ref().unwrap();
        assert_eq!(next.prefix, retained.prefix);
        assert_eq!(next.package, retained.package);
        assert!(next.fence > retained.fence);
        assert_eq!(recovered.economy, before.economy);
        assert_eq!(recovered.history, before.history);
        pending
    })
}

async fn saved(fixture: &Fixture, run: Uuid) -> Run {
    let row = fixture
        .sandbox
        .db
        .query_one_raw(sea_orm::Statement::from_string(
            sea_orm::DbBackend::Postgres,
            format!(
                "SELECT state FROM \"{}\".runs WHERE id='{run}'",
                fixture.sandbox.schema
            ),
        ))
        .await
        .unwrap()
        .unwrap();
    serde_json::from_slice(&row.try_get::<Vec<u8>>("", "state").unwrap()).unwrap()
}

async fn observed_choice(
    gateway: &common::Gateway,
    grant: &serde_json::Value,
) -> (common::Socket, Vec<u8>) {
    let mut socket = common::dial_seat(
        gateway.port,
        grant["game_id"].as_str().unwrap(),
        grant["seat_token"].as_str().unwrap(),
    )
    .await;
    loop {
        match native::next(&mut socket).await {
            v1::envelope::Msg::GameStatic(_) => {
                common::send(
                    &mut socket,
                    &Envelope {
                        msg: Some(v1::envelope::Msg::SeatReady(v1::SeatReady {})),
                    },
                )
                .await;
            }
            v1::envelope::Msg::ChoiceRequest(question) => {
                return (socket, question.pending);
            }
            v1::envelope::Msg::Error(_) => panic!("native startup refused"),
            v1::envelope::Msg::TowerFrame(_) | v1::envelope::Msg::GameRecordChunk(_) => {
                panic!("private recovery input leaked")
            }
            _ => {}
        }
    }
}

pub(super) async fn stale_status(agent: &mut common::Socket, previous: &str) {
    common::send(
        agent,
        &Envelope {
            msg: Some(v1::envelope::Msg::EngineStatus(v1::EngineStatus {
                game_id: previous.into(),
                kind: v1::engine_status::Kind::Failed as i32,
                detail: String::new(),
            })),
        },
    )
    .await;
}
