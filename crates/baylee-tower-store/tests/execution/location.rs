use super::{Flow, Sandbox};
use baylee_tower::v2::run::Phase;
use baylee_tower_store::{Recovery, Store};
use sea_orm::ConnectionTrait;
use uuid::Uuid;

#[tokio::test]
async fn owned_routing_uses_only_approved_current_host_and_execution() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let flow = Flow::prepare(&store).await;
        let run = flow.initial.id;
        let shop = store.locate(flow.account, run).await.unwrap();
        assert_eq!(shop.phase, Phase::Shop);
        assert!(shop.host.is_none() && shop.game.is_none());
        flow.reserve(&store).await;
        assert!(
            store
                .locate(flow.account, run)
                .await
                .unwrap()
                .host
                .is_none(),
            "unapproved origins never have a fallback"
        );
        approvals(&store, &flow).await;
        let reserved = store.locate(flow.account, run).await.unwrap();
        assert_eq!(
            reserved.host.unwrap().origin,
            "https://first.example.invalid"
        );
        assert!(reserved.game.is_none());
        assert!(store.locate(Uuid::now_v7(), run).await.is_err());
        let foreign = flow.another(&store).await;
        assert!(store.locate(foreign.account, run).await.is_err());
        let first = store
            .claim(&flow.host, run, Uuid::now_v7(), 120_000)
            .await
            .unwrap();
        let playing = store.locate(flow.account, run).await.unwrap();
        assert_eq!(playing.phase, Phase::Playing);
        assert_eq!(playing.game, Some(first.lease.claim));
        let stopped = store
            .supervise(
                &flow.supervisor,
                run,
                playing.revision,
                Recovery::Quarantine,
            )
            .await
            .unwrap();
        let hidden = store.locate(flow.account, run).await.unwrap();
        assert!(hidden.host.is_none() && hidden.game.is_none());
        store
            .supervise(
                &flow.supervisor,
                run,
                stopped.state.revision,
                Recovery::Resume {
                    host: Some(flow.next_host.id()),
                },
            )
            .await
            .unwrap();
        let reassigned = store.locate(flow.account, run).await.unwrap();
        assert_eq!(
            reassigned.host.unwrap().origin,
            "https://second.example.invalid"
        );
        assert!(reassigned.game.is_none());
        let second = store
            .claim(&flow.next_host, run, Uuid::now_v7(), 120_000)
            .await
            .unwrap();
        assert_ne!(first.lease.claim, second.lease.claim);
        assert_eq!(
            store.locate(flow.account, run).await.unwrap().game,
            Some(second.lease.claim)
        );
        db.execute_unprepared(&format!(
            "UPDATE \"{schema}\".operators SET enabled=false WHERE id='{}'",
            flow.next_host.id()
        ))
        .await
        .unwrap();
        let disabled = store.locate(flow.account, run).await.unwrap();
        assert!(disabled.host.is_none() && disabled.game.is_none());
    })
    .await;
    sandbox.close().await;
    observed.expect("approved owner routing completed");
}

async fn approvals(store: &Store, flow: &Flow) {
    let origin = "https://first.example.invalid";
    store
        .approve_host(flow.host.id(), origin, false)
        .await
        .unwrap();
    store
        .approve_host(flow.host.id(), origin, false)
        .await
        .unwrap();
    assert!(
        store
            .approve_host(flow.host.id(), "https://changed.example.invalid", false)
            .await
            .is_err()
    );
    assert!(
        store
            .approve_host(
                flow.supervisor.id(),
                "https://not-a-host.example.invalid",
                false
            )
            .await
            .is_err()
    );
    for bad in [
        "http://127.0.0.1:9999",
        "https://first.example.invalid/path",
        "https://user:secret@first.example.invalid",
        "https://first.example.invalid?token=bad",
    ] {
        assert!(
            store
                .approve_host(flow.next_host.id(), bad, false)
                .await
                .is_err()
        );
    }
    assert!(
        store
            .approve_host(flow.next_host.id(), origin, true)
            .await
            .is_err()
    );
    assert!(
        store
            .approve_host(flow.next_host.id(), "http://localhost:9999", true)
            .await
            .is_err()
    );
    store
        .approve_host(flow.next_host.id(), "https://second.example.invalid", false)
        .await
        .unwrap();
}
