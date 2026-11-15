//! Real PostgreSQL -> owned native executable -> atomic reward, synthetic accounts.
#![forbid(unsafe_code)]
#![cfg(feature = "tower")]
use baylee_cards_pool as _;
mod fixture;
mod queue;
mod scenarios;
#[path = "../../../baylee-tower-store/tests/support/mod.rs"]
mod support;

use baylee_tower::v2::{run::Phase, verification::Resources};
use baylee_tower_service::{Artifact, Error, Verifier};
use fixture::Flow;
use std::{future::pending, path::Path, time::Duration};
use support::Sandbox;

async fn verifier() -> Verifier {
    let artifact = Artifact::probe(Path::new(env!("CARGO_BIN_EXE_baylee-engine-server")))
        .await
        .unwrap();
    assert_eq!(
        artifact.identity(),
        &baylee_engine_server::variant::identity()
    );
    Verifier::new(artifact, 1, Resources::default()).unwrap()
}

#[tokio::test]
async fn retained_game_is_independently_played_and_rewarded_exactly_once() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let observed = tokio::spawn(async move {
        let service = verifier().await;
        let flow = Flow::prepare(&store, false).await;
        let before = store.get(flow.account, flow.run).await.unwrap();
        assert_eq!(before.state.phase, Phase::Verifying);
        assert_eq!(
            service
                .adjudicate(&store, &flow.host, flow.job.clone(), pending())
                .await,
            Err(Error::Contract)
        );
        assert_eq!(
            service
                .adjudicate(&store, &flow.verifier, flow.job.clone(), async {})
                .await,
            Err(Error::Cancelled)
        );
        assert_eq!(service.running(), 0);
        let after = service
            .adjudicate(&store, &flow.verifier, flow.job.clone(), pending())
            .await
            .unwrap();
        assert_eq!(after.state.phase, Phase::Reward);
        assert_eq!(after.state.reward.as_ref().unwrap().take, 1);
        assert_eq!(after.state.defeats(), 1);
        assert_eq!(after.state.wins(), 0);
        assert_eq!(after.state.economy.gold, before.state.economy.gold + 12);
        let repeated = service
            .adjudicate(&store, &flow.verifier, flow.job.clone(), pending())
            .await
            .unwrap();
        assert_eq!(repeated, after);
        assert_eq!(service.running(), 0);
        let mut forged = flow.job.clone();
        forged.lease.generation += 1;
        assert!(
            service
                .adjudicate(&store, &flow.verifier, forged, pending())
                .await
                .is_err()
        );
        assert_eq!(store.get(flow.account, flow.run).await.unwrap(), after);
    })
    .await;
    sandbox.close().await;
    observed.expect("independent native adjudication completed");
}

#[tokio::test]
async fn altered_retained_play_and_technical_budgets_grant_no_result() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let observed = tokio::spawn(async move {
        let flow = Flow::prepare(&store, true).await;
        let before = store.get(flow.account, flow.run).await.unwrap();
        let service = verifier().await;
        assert_eq!(
            service
                .adjudicate(&store, &flow.verifier, flow.job.clone(), pending())
                .await,
            Err(Error::Record)
        );
        let artifact = Artifact::probe(Path::new(env!("CARGO_BIN_EXE_baylee-engine-server")))
            .await
            .unwrap();
        let bounded = Verifier::new(
            artifact,
            1,
            Resources {
                bytes: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            bounded
                .adjudicate(&store, &flow.verifier, flow.job.clone(), pending())
                .await,
            Err(Error::Budget)
        );
        assert_eq!(bounded.running(), 0);
        assert_eq!(store.get(flow.account, flow.run).await.unwrap(), before);
    })
    .await;
    sandbox.close().await;
    observed.expect("refused evidence is never a rules result");
}

async fn wait_running(service: &Verifier, expected: usize) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while service.running() != expected {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("owned worker capacity reached expected state");
}
