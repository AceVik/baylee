//! Actual compiled Tower worker, bounded private pipes, and owned-child cleanup.
#![forbid(unsafe_code)]
#![cfg(feature = "tower")]
use baylee_cards_pool as _;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_tower::v2::{
    catalog::{Definition, Family, Rarity},
    execution::{Execution, MAX_CHUNK_BYTES, MAX_RECORD_BYTES},
    package::{Build, Cohort, Origin, Package},
    run::{Binding, Outcome},
    verification::{MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES, Rejected, Request, Resources, Response},
};
use futures_util::FutureExt as _;
use std::{panic::AssertUnwindSafe, process::Stdio, time::Duration};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

fn fixture() -> (Request, Vec<u8>) {
    let content = baylee_tower_content::standard().unwrap();
    let catalog = content.catalog;
    let starter = &content.starters[0];
    let build = Build {
        formation: starter.formation.clone(),
        deck: starter.deck.clone(),
        active: vec![Definition {
            family: Family::Grove,
            rarity: Rarity::Common,
        }],
        potions: [0; 2],
    };
    let identity = baylee_engine_server::variant::identity();
    let package = Package {
        schema: 2,
        cohort: Cohort::new(identity.clone(), &catalog).unwrap(),
        stage: 1,
        opponent: Origin::House { archetype: 1 },
        builds: [build.clone(), build],
        shuffle_seed: 19,
        policy_seed: 17,
    };
    let preset = package
        .preset(&catalog, &identity, |card| {
            Some(baylee_cards::decks::reference_print(card))
        })
        .unwrap();
    let mut live =
        baylee_engine_server::variant::HostedSession::recorded_for_execution(&preset, &identity)
            .unwrap();
    let me = PlayerId::new(0);
    for _ in 0..32 {
        if live.needs_pump() {
            live.pump_slice(1);
        } else if matches!(live.pending(), Pending::Priority { player, .. } if *player == me) {
            break;
        } else {
            live.act_slice(me, live.house_action(me).unwrap(), 0)
                .unwrap();
        }
    }
    live.act_slice(me, PlayerAction::Concede, 0).unwrap();
    let bytes = live.take_record();
    let mut execution = Execution::new(package.digest(&catalog).unwrap(), MAX_RECORD_BYTES)
        .unwrap()
        .claimed(0, 1)
        .unwrap();
    let mut stream = Vec::new();
    for chunk in bytes.chunks(1024) {
        baylee_worker::frame::write(&mut stream, chunk, MAX_CHUNK_BYTES).unwrap();
        execution = execution.appended(1, 0, execution.prefix, chunk).unwrap();
    }
    (
        Request {
            schema: 2,
            package,
            catalog,
            binding: Binding {
                package: execution.package,
                fence: 3,
                prefix: execution.prefix,
            },
            resources: Resources::default(),
        },
        stream,
    )
}

fn spawn() -> tokio::process::Child {
    tokio::process::Command::new(env!("CARGO_BIN_EXE_baylee-engine-server"))
        .arg("--tower-verify")
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap()
}

async fn run(request: &Request, stream: &[u8]) -> Response {
    let mut input = Vec::new();
    baylee_worker::frame::write(&mut input, &request.encode().unwrap(), MAX_REQUEST_BYTES).unwrap();
    input.extend_from_slice(stream);
    let mut child = spawn();
    let observed = AssertUnwindSafe(tokio::time::timeout(Duration::from_secs(10), async {
        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let mut reply = Vec::new();
        let mut progress = Vec::new();
        let mut stdout = stdout.take((MAX_RESPONSE_BYTES + 5) as u64);
        let mut stderr = stderr.take(256);
        let ((), _, _, status) = tokio::try_join!(
            async {
                stdin.write_all(&input).await?;
                stdin.shutdown().await?;
                drop(stdin);
                Ok::<(), std::io::Error>(())
            },
            stdout.read_to_end(&mut reply),
            stderr.read_to_end(&mut progress),
            child.wait(),
        )
        .unwrap();
        assert!(status.success());
        assert!(progress.starts_with(&[baylee_worker::IDLE, baylee_worker::BUSY]));
        assert_eq!(
            progress
                .iter()
                .rev()
                .find(|marker| **marker != baylee_worker::ALIVE),
            Some(&baylee_worker::IDLE)
        );
        let mut budget =
            baylee_worker::Budget::new(baylee_worker::Limits::default(), Duration::ZERO);
        for marker in progress {
            budget.observe(marker, Duration::ZERO).unwrap();
        }
        let mut reply = reply.as_slice();
        let response = serde_json::from_slice(
            &baylee_worker::frame::read(&mut reply, MAX_RESPONSE_BYTES).unwrap(),
        )
        .unwrap();
        assert!(reply.is_empty());
        response
    }))
    .catch_unwind()
    .await;
    let _ = child.start_kill();
    tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .unwrap()
        .unwrap();
    observed.unwrap().unwrap()
}

#[tokio::test]
async fn a_real_tower_worker_verifies_the_saved_record_and_refuses_a_changed_stream() {
    let (request, mut stream) = fixture();
    let response = run(&request, &stream).await;
    let Response::Verified { settlement } = response else {
        panic!("native recorded game must verify");
    };
    assert_eq!(settlement.binding, request.binding);
    assert_eq!(settlement.outcome, Outcome::Defeat);
    assert_eq!(settlement.used, [0; 2]);
    *stream.last_mut().unwrap() ^= 1;
    assert_eq!(
        run(&request, &stream).await,
        Response::Rejected {
            reason: Rejected::Invalid
        }
    );
}

#[tokio::test]
async fn losing_the_progress_owner_stops_a_worker_waiting_on_private_input() {
    let mut child = spawn();
    let observed = AssertUnwindSafe(tokio::time::timeout(Duration::from_secs(5), async {
        let _still_open = child.stdin.take().unwrap();
        let mut progress = child.stderr.take().unwrap();
        assert_eq!(progress.read_u8().await.unwrap(), baylee_worker::IDLE);
        assert_eq!(progress.read_u8().await.unwrap(), baylee_worker::BUSY);
        // stdin deliberately remains open: only the independent progress-owner
        // watcher can notice the lost supervisor during this blocked operation.
        drop(progress);
        assert!(!child.wait().await.unwrap().success());
    }))
    .catch_unwind()
    .await;
    let _ = child.start_kill();
    tokio::time::timeout(Duration::from_secs(5), child.wait())
        .await
        .unwrap()
        .unwrap();
    observed.unwrap().unwrap();
}
