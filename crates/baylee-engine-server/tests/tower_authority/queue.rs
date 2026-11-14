use super::*;
use baylee_tower_service::{Queue, QueueResult};
use baylee_tower_store::{Principal, Recovery, Role, VerificationFailure};
use sea_orm::ConnectionTrait as _;
use tokio::sync::oneshot;
use uuid::Uuid;

async fn queue(sandbox: &Sandbox, flow: &Flow) -> (Queue, Principal) {
    let store = &sandbox.store;
    store
        .provision_operator("synthetic queue supervisor", Role::Supervisor, [4; 32])
        .await
        .unwrap();
    let supervisor = store.authenticate_operator([4; 32]).await.unwrap();
    // Expire only the synthetic initial claimant. The queue must take a new
    // independent generation; no game input or committed record is changed.
    sandbox
        .db
        .execute_unprepared(&format!(
            "UPDATE \"{}\".verification_jobs SET lease_until=0",
            sandbox.schema
        ))
        .await
        .unwrap();
    let artifact = Artifact::probe(Path::new(env!("CARGO_BIN_EXE_baylee-engine-server")))
        .await
        .unwrap();
    (
        Queue::new(
            store.clone(),
            flow.verifier.clone(),
            supervisor.clone(),
            artifact,
            Resources::default(),
        )
        .unwrap(),
        supervisor,
    )
}

#[tokio::test]
async fn queue_automatically_settles_and_stops_without_repeated_rewards() {
    let sandbox = Sandbox::open().await;
    let observed = checked(async {
        let store = &sandbox.store;
        let flow = Flow::prepare(store, false).await;
        let before = store.get(flow.account, flow.run).await.unwrap();
        let (queue, _) = queue(&sandbox, &flow).await;
        let (stop, stopped) = oneshot::channel();
        // Scope completion even on a panic below: sender drop also stops serve.
        let task = tokio::spawn(async move {
            queue
                .serve(before.cohort, async {
                    let _ = stopped.await;
                })
                .await
        });
        let arrived = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let saved = store.get(flow.account, flow.run).await.unwrap();
                if saved.state.phase == Phase::Shop {
                    return saved;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        let _ = stop.send(());
        let summary = task.await.unwrap().unwrap();
        let saved = arrived.expect("automatic queue completed the actual native job");
        assert_eq!(summary.settled, 1);
        assert_eq!(summary.quarantined, 0);
        assert_eq!(saved.state.economy.gold, before.state.economy.gold + 12);
        assert!(
            store
                .verification_candidates(&flow.verifier, before.cohort, 1)
                .await
                .unwrap()
                .is_empty()
        );
    })
    .await;
    sandbox.close().await;
    observed.expect("automatic queue scenario completed");
}

#[tokio::test]
async fn queue_quarantines_failed_evidence_without_awarding_or_harming_new_generations() {
    let sandbox = Sandbox::open().await;
    let observed = checked(async {
        let store = &sandbox.store;
        let flow = Flow::prepare(store, true).await;
        let before = store.get(flow.account, flow.run).await.unwrap();
        let (queue, supervisor) = queue(&sandbox, &flow).await;
        assert!(store.quarantine_verification(&flow.verifier, flow.job.lease, VerificationFailure::InvalidRecord).await.is_err());
        let outcome = queue.next(before.cohort, pending()).await.unwrap();
        let QueueResult::Quarantined { snapshot, reason } = outcome else { panic!("invalid evidence must be retained for recovery"); };
        assert_eq!(reason, VerificationFailure::InvalidRecord);
        assert_eq!(snapshot.state.phase, Phase::Quarantined);
        assert_eq!(snapshot.state.economy, before.state.economy);
        assert_eq!(snapshot.state.wins(), 0);
        assert_eq!(snapshot.state.defeats(), 0);
        assert_eq!(queue.next(before.cohort, pending()).await.unwrap(), QueueResult::Idle);
        let row = sandbox.db.query_one_raw(sea_orm::Statement::from_string(sea_orm::DbBackend::Postgres,
            format!("SELECT failure,(SELECT count(*) FROM \"{}\".award_outbox) AS awards FROM \"{}\".verification_jobs", sandbox.schema, sandbox.schema))).await.unwrap().unwrap();
        assert_eq!(row.try_get::<String>("", "failure").unwrap(), "invalid_record");
        assert_eq!(row.try_get::<i64>("", "awards").unwrap(), 0);
        let resumed = store.supervise(&supervisor, flow.run, snapshot.state.revision, Recovery::Resume { host: None }).await.unwrap();
        let current = store.claim_verification(&flow.verifier, flow.job.lease.reservation, Uuid::now_v7(), 120_000).await.unwrap();
        assert_ne!(current.lease.binding, flow.job.lease.binding);
        assert!(store.quarantine_verification(&supervisor, flow.job.lease, reason).await.is_err());
        assert_eq!(store.get(flow.account, flow.run).await.unwrap(), resumed);
    }).await;
    sandbox.close().await;
    observed.expect("failed verification was fenced without a fabricated outcome");
}

fn checked(
    body: impl std::future::Future<Output = ()>,
) -> impl std::future::Future<Output = std::thread::Result<()>> {
    use futures_util::FutureExt as _;
    Box::pin(std::panic::AssertUnwindSafe(body).catch_unwind())
}
