//! Opt-in, bounded local client demo using real authentication and native games.
use super::{Runtime, Served, common, fresh, native};
use baylee_protocol::v1::{self, Envelope};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    time::{Duration, Instant},
};
use tokio::sync::oneshot;

mod catalog;
pub(super) use catalog::{publication, publish_house};

pub(super) fn enabled() -> bool {
    std::env::var("BAYLEE_TOWER_V2_DEMO").as_deref() == Ok("1")
}

pub(super) fn run(runtime: &Runtime, served: &Served) {
    assert_eq!(
        std::env::var("BAYLEE_CENTRAL_DEMO").as_deref(),
        Ok("1"),
        "explicit synthetic identity mode"
    );
    let ready =
        PathBuf::from(std::env::var_os("BAYLEE_CENTRAL_UI_READY").expect("demo readiness path"));
    let fixture = served.tower.as_ref().unwrap();
    let socket = runtime.block_on(native::agent(&served.gateway, fixture));
    let (stop, stopped) = oneshot::channel();
    let mut running = runtime.spawn(agent(
        socket,
        fixture.binary.clone(),
        fixture.identity.clone(),
        stopped,
    ));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let account = fresh::register(
            runtime,
            &served.service,
            &served.endpoint,
            "echoes_demo",
            &super::super::demo::document(&served.origin),
        );
        let tower = account.tower().unwrap();
        let baylee_tower_api::Response::Run { run } = tower
            .exchange(&baylee_tower_api::Request::Start {
                request: uuid::Uuid::now_v7(),
                cohort: fixture.cohort,
                starter: 0,
                consent: baylee_tower_store::CONSENT_REVISION,
            })
            .unwrap()
        else {
            panic!("owned synthetic run");
        };
        std::fs::write(
            &ready,
            serde_json::to_vec(&serde_json::json!({
                "mode": "synthetic-only", "central": served.endpoint.origin(),
                "gateway": served.origin, "tower": fixture.origin, "account": "echoes_demo",
                "run": run.id, "cohort": fixture.cohort,
            }))
            .unwrap(),
        )
        .unwrap();
        let seconds = std::env::var("BAYLEE_CENTRAL_UI_HOLD_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(600)
            .clamp(1, 1200);
        let deadline = Instant::now() + Duration::from_secs(seconds);
        while Instant::now() < deadline && !ready.with_extension("stop").exists() {
            assert!(
                !running.is_finished(),
                "owned native agent must stay connected"
            );
            std::thread::sleep(Duration::from_millis(200));
        }
    }));
    let _ = std::fs::remove_file(&ready);
    let _ = stop.send(());
    let joined = runtime
        .block_on(async { tokio::time::timeout(Duration::from_secs(10), &mut running).await });
    if joined.is_err() {
        running.abort();
        let _ = runtime.block_on(running);
    }
    joined.expect("owned native demo agent shutdown").unwrap();
    if let Err(panic) = result {
        std::panic::resume_unwind(panic);
    }
}

async fn agent(
    mut socket: common::Socket,
    binary: PathBuf,
    identity: super::Identity,
    mut stopped: oneshot::Receiver<()>,
) {
    let mut children = BTreeMap::<String, native::Engine>::new();
    let mut heartbeat = tokio::time::interval(Duration::from_secs(5));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = &mut stopped => break,
            _ = heartbeat.tick() => {
                common::send(&mut socket, &Envelope { msg: Some(v1::envelope::Msg::Heartbeat(v1::Heartbeat {client_time_ms: 0})) }).await;
                let finished: Vec<_> = children.iter_mut().filter_map(|(id, child)| child.exited().then(||id.clone())).collect();
                for id in finished {
                    children.remove(&id);
                    common::send(&mut socket, &Envelope {msg: Some(v1::envelope::Msg::EngineStatus(v1::EngineStatus {
                        game_id: id, kind: v1::engine_status::Kind::Exited.into(), detail: String::new(),
                    }))}).await;
                }
            }
            message = common::next_msg(&mut socket) => match message {
                Some(v1::envelope::Msg::StartEngine(start)) => {
                    assert!(children.is_empty(), "one advertised native execution slot");
                    children.insert(start.game_id.clone(), native::Engine::launch(&start, &binary, &identity));
                }
                Some(v1::envelope::Msg::StopEngine(stop)) => { children.remove(&stop.game_id); }
                None => break,
                _ => {},
            },
        }
    }
    drop(children); // each child is killed/reaped before the control peer closes
    socket.close(None).await.unwrap();
}
