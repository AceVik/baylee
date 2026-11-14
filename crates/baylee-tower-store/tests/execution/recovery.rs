use super::{Flow, Sandbox, operator};
use baylee_tower::v2::{execution::Prefix, run::Phase};
use baylee_tower_store::{Claimed, Recovery, Role, Snapshot, Store};
use sea_orm::ConnectionTrait;
use uuid::Uuid;

#[tokio::test]
async fn recovery_fences_old_workers_and_retains_exact_bytes_package_and_seal() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let observed = tokio::spawn(async move {
        let flow = Flow::prepare(&store).await;
        flow.reserve(&store).await;
        let claimed = store
            .claim(&flow.host, flow.initial.id, Uuid::now_v7(), 120_000)
            .await
            .unwrap();
        let head = store
            .append(
                &flow.host,
                claimed.lease,
                Prefix::default(),
                b"before recovery\n",
            )
            .await
            .unwrap()
            .prefix;
        let stopped = quarantined(&store, &flow, &claimed, head).await;
        let recovered = resumed(&store, &flow, &claimed, &stopped, head).await;
        sealed(&store, &flow, &recovered, head).await;
    })
    .await;
    sandbox.close().await;
    observed.expect("fenced recovery scenario completed");
}

async fn quarantined(store: &Store, flow: &Flow, claimed: &Claimed, head: Prefix) -> Snapshot {
    let state = store.get(flow.account, flow.initial.id).await.unwrap();
    let verifier = operator(store, Role::Verifier, 5).await;
    assert!(
        store
            .claim(&verifier, flow.initial.id, Uuid::now_v7(), 120_000)
            .await
            .is_err()
    );
    for wrong in [&flow.host, &verifier] {
        assert!(
            store
                .supervise(
                    wrong,
                    flow.initial.id,
                    state.state.revision,
                    Recovery::Quarantine
                )
                .await
                .is_err()
        );
    }
    let stopped = store
        .supervise(
            &flow.supervisor,
            flow.initial.id,
            state.state.revision,
            Recovery::Quarantine,
        )
        .await
        .unwrap();
    assert_eq!(stopped.state.phase, Phase::Quarantined);
    assert!(
        store
            .append(&flow.host, claimed.lease, head, b"late\n")
            .await
            .is_err()
    );
    assert!(
        store
            .append(
                &flow.host,
                claimed.lease,
                Prefix::default(),
                b"before recovery\n"
            )
            .await
            .is_err()
    );
    assert!(
        store
            .renew(&flow.host, claimed.lease, 120_000)
            .await
            .is_err()
    );
    assert!(
        store
            .read_chunk(&flow.host, claimed.lease, 0)
            .await
            .is_err()
    );
    stopped
}

async fn resumed(
    store: &Store,
    flow: &Flow,
    claimed: &Claimed,
    stopped: &Snapshot,
    head: Prefix,
) -> Claimed {
    let resumed = store
        .supervise(
            &flow.supervisor,
            flow.initial.id,
            stopped.state.revision,
            Recovery::Resume {
                host: Some(flow.next_host.id()),
            },
        )
        .await
        .unwrap();
    assert_eq!(resumed.state.phase, Phase::Reserved);
    assert!(
        store
            .claim(&flow.host, flow.initial.id, Uuid::now_v7(), 120_000)
            .await
            .is_err()
    );
    let recovered = store
        .claim(&flow.next_host, flow.initial.id, Uuid::now_v7(), 120_000)
        .await
        .unwrap();
    assert!(recovered.package == claimed.package);
    assert_eq!(recovered.prefix, head);
    assert!(recovered.lease.fence > claimed.lease.fence);
    assert_eq!(
        store
            .read_chunk(&flow.next_host, recovered.lease, 0)
            .await
            .unwrap()
            .data,
        b"before recovery\n"
    );
    recovered
}

async fn sealed(store: &Store, flow: &Flow, recovered: &Claimed, head: Prefix) {
    let end = store
        .append(
            &flow.next_host,
            recovered.lease,
            head,
            b"terminal candidate\n",
        )
        .await
        .unwrap();
    assert_eq!(
        store
            .seal(&flow.next_host, recovered.lease, end.prefix)
            .await
            .unwrap(),
        end
    );
    assert_eq!(
        store
            .seal(&flow.next_host, recovered.lease, end.prefix)
            .await
            .unwrap(),
        end
    );
    let sealed = store.get(flow.account, flow.initial.id).await.unwrap();
    assert_eq!(sealed.state.phase, Phase::Verifying);
    assert_eq!(sealed.state.economy, flow.initial.state.economy);
    assert!(
        sealed.state.history.is_empty(),
        "a host seal is not an adjudicated outcome"
    );
    let stopped = store
        .supervise(
            &flow.supervisor,
            flow.initial.id,
            sealed.state.revision,
            Recovery::Quarantine,
        )
        .await
        .unwrap();
    assert!(
        store
            .supervise(
                &flow.supervisor,
                flow.initial.id,
                stopped.state.revision,
                Recovery::Resume {
                    host: Some(flow.host.id())
                }
            )
            .await
            .is_err()
    );
    let pending = store
        .supervise(
            &flow.supervisor,
            flow.initial.id,
            stopped.state.revision,
            Recovery::Resume { host: None },
        )
        .await
        .unwrap();
    assert_eq!(pending.state.phase, Phase::Verifying);
    assert_eq!(pending.state.execution.unwrap().prefix, end.prefix);
    assert!(
        store
            .seal(&flow.next_host, recovered.lease, end.prefix)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn an_expired_lease_needs_supervision_and_interruption_grants_no_result() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let flow = Flow::prepare(&store).await;
        flow.reserve(&store).await;
        let claimed = store.claim(&flow.host, flow.initial.id, Uuid::now_v7(), 120_000).await.unwrap();
        // Expire the authority metadata without sleeping or changing the game clock.
        db.execute_unprepared(&format!("UPDATE \"{schema}\".runs SET state=convert_to(jsonb_set(convert_from(state,'UTF8')::jsonb,'{{execution,lease_until}}','1'::jsonb)::text,'UTF8')")).await.unwrap();
        assert!(store.claim(&flow.host, flow.initial.id, claimed.lease.claim, 120_000).await.is_err());
        assert!(store.claim(&flow.host, flow.initial.id, Uuid::now_v7(), 120_000).await.is_err());
        assert!(store.renew(&flow.host, claimed.lease, 120_000).await.is_err());
        assert!(store.append(&flow.host, claimed.lease, Prefix::default(), b"too late\n").await.is_err());
        let state = store.get(flow.account, flow.initial.id).await.unwrap();
        let stopped = store.supervise(&flow.supervisor, flow.initial.id, state.state.revision, Recovery::Quarantine).await.unwrap();
        let resumed = store.supervise(&flow.supervisor, flow.initial.id, stopped.state.revision, Recovery::Resume { host: Some(flow.host.id()) }).await.unwrap();
        assert!(store.claim(&flow.host, flow.initial.id, claimed.lease.claim, 120_000).await.is_err());
        let stopped = store.supervise(&flow.supervisor, flow.initial.id, resumed.state.revision, Recovery::Quarantine).await.unwrap();
        let retired = store.supervise(&flow.supervisor, flow.initial.id, stopped.state.revision, Recovery::Interrupt).await.unwrap();
        assert_eq!(retired.state.phase, Phase::Interrupted);
        assert_eq!(retired.state.economy, flow.initial.state.economy);
        assert!(retired.state.history.is_empty());
    }).await;
    sandbox.close().await;
    observed.expect("expiry/technical interruption scenario completed");
}
