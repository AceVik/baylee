use super::{Flow, Sandbox, operator, verified_fixture};
use baylee_tower::v2::run::Outcome;
use baylee_tower_store::{AwardDelivery, Principal, Role, Store};
use sea_orm::{ConnectionTrait, DbBackend, Statement};

async fn event(store: &Store) -> (Flow, Principal) {
    let flow = Flow::prepare(store).await;
    let verifier = operator(store, Role::Verifier, 5).await;
    verified_fixture(store, &flow, &verifier, Outcome::Defeat).await;
    (flow, verifier)
}

#[tokio::test]
async fn only_couriers_claim_verified_facts_and_expired_workers_cannot_acknowledge() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let courier = operator(&store, Role::Courier, 6).await;
        assert!(store.claim_award(&courier).await.unwrap().is_none());
        let (flow, verifier) = event(&store).await;
        let publisher = operator(&store, Role::Publisher, 7).await;
        for actor in [&flow.host, &flow.supervisor, &verifier, &publisher] {
            assert!(store.claim_award(actor).await.is_err());
        }
        let first = store.claim_award(&courier).await.unwrap().unwrap();
        assert_eq!(first.account, flow.account);
        assert_eq!(first.award.run, flow.initial.id);
        assert_eq!(first.award.cohort, flow.initial.cohort);
        assert_eq!((first.award.schema, first.award.stage), (2, 1));
        assert_eq!((first.award.wins, first.award.defeats), (0, 1));
        assert_eq!(first.award.outcome, Outcome::Defeat);
        assert_eq!(first.lease.event.get_version_num(), 7);
        assert_eq!(first.lease.claim.get_version_num(), 7);
        assert!(store.claim_award(&courier).await.unwrap().is_none());
        assert!(
            store
                .acknowledge_award(&flow.host, first.lease)
                .await
                .is_err()
        );
        assert!(store.defer_award(&verifier, first.lease).await.is_err());
        let other = operator(&store, Role::Courier, 8).await;
        assert!(store.acknowledge_award(&other, first.lease).await.is_err());
        db.execute_unprepared(&format!(
            "UPDATE \"{schema}\".award_outbox SET lease_until=1"
        ))
        .await
        .unwrap();
        assert!(
            store
                .acknowledge_award(&courier, first.lease)
                .await
                .is_err()
        );
        assert!(store.defer_award(&courier, first.lease).await.is_err());
        let replacement = store.claim_award(&other).await.unwrap().unwrap();
        assert_same_event(&first, &replacement);
        assert!(
            store
                .acknowledge_award(&courier, first.lease)
                .await
                .is_err()
        );
        store
            .acknowledge_award(&other, replacement.lease)
            .await
            .unwrap();
        store
            .acknowledge_award(&other, replacement.lease)
            .await
            .unwrap();
        assert!(store.defer_award(&other, replacement.lease).await.is_err());
        assert!(store.claim_award(&courier).await.unwrap().is_none());
        assert_eq!(
            store
                .get(flow.account, flow.initial.id)
                .await
                .unwrap()
                .state
                .economy
                .gold,
            30
        );
    })
    .await;
    sandbox.close().await;
    observed.expect("award ownership and fencing scenario completed");
}

fn assert_same_event(first: &AwardDelivery, next: &AwardDelivery) {
    assert_eq!(first.lease.event, next.lease.event);
    assert_ne!(first.lease.claim, next.lease.claim);
    assert_eq!(first.account, next.account);
    assert_eq!(first.award, next.award);
}

#[tokio::test]
async fn delivery_failures_back_off_without_losing_facts_and_revocation_is_live() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        event(&store).await;
        let courier = operator(&store, Role::Courier, 6).await;
        let first = store.claim_award(&courier).await.unwrap().unwrap();
        store.defer_award(&courier, first.lease).await.unwrap();
        assert!(store.claim_award(&courier).await.unwrap().is_none());
        assert!(store.acknowledge_award(&courier, first.lease).await.is_err());
        let row = db.query_one_raw(Statement::from_string(DbBackend::Postgres, format!(
            "SELECT backoff=1 AND courier IS NULL AND claim IS NULL AND lease_until=0
                AND retry_after>floor(extract(epoch FROM clock_timestamp())*1000)::bigint
                AND retry_after<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint+5000
                AND delivered_at IS NULL AS deferred FROM \"{schema}\".award_outbox"
        ))).await.unwrap().unwrap();
        assert!(row.try_get::<bool>("", "deferred").unwrap());
        db.execute_unprepared(&format!(
            "UPDATE \"{schema}\".award_outbox SET retry_after=0,backoff=10"
        )).await.unwrap();
        let next = store.claim_award(&courier).await.unwrap().unwrap();
        assert_same_event(&first, &next);
        store.defer_award(&courier, next.lease).await.unwrap();
        let row = db.query_one_raw(Statement::from_string(DbBackend::Postgres, format!(
            "SELECT backoff=10 AND retry_after-floor(extract(epoch FROM clock_timestamp())*1000)::bigint
                BETWEEN 3590000 AND 3600000 AS capped FROM \"{schema}\".award_outbox"
        ))).await.unwrap().unwrap();
        assert!(row.try_get::<bool>("", "capped").unwrap());
        db.execute_unprepared(&format!("UPDATE \"{schema}\".award_outbox SET retry_after=0"))
            .await.unwrap();
        let latest = store.claim_award(&courier).await.unwrap().unwrap();
        db.execute_unprepared(&format!(
            "UPDATE \"{schema}\".operators SET enabled=false WHERE role='courier'"
        )).await.unwrap();
        assert!(store.claim_award(&courier).await.is_err());
        assert!(store.acknowledge_award(&courier, latest.lease).await.is_err());
        assert!(store.defer_award(&courier, latest.lease).await.is_err());
    }).await;
    sandbox.close().await;
    observed.expect("award retry and revocation scenario completed");
}

#[tokio::test]
async fn concurrent_couriers_get_distinct_events_and_deletion_cannot_resurrect_them() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let (flow, verifier) = event(&store).await;
        let second = flow.another(&store).await;
        verified_fixture(&store, &second, &verifier, Outcome::Victory).await;
        let one = operator(&store, Role::Courier, 6).await;
        let two = operator(&store, Role::Courier, 7).await;
        let (a, b) = tokio::join!(store.claim_award(&one), store.claim_award(&two));
        let a = a.unwrap().unwrap();
        let b = b.unwrap().unwrap();
        assert_ne!(a.lease.event, b.lease.event);
        assert_ne!(a.account, b.account);
        assert!(store.claim_award(&one).await.unwrap().is_none());
        // Deletion first withdraws and drains the dense echo index. Its RESTRICT
        // foreign key intentionally prevents silently leaving holes in that index.
        store.withdraw_echoes(flow.account).await.unwrap();
        store.withdraw_echoes(second.account).await.unwrap();
        assert_eq!(
            store
                .prune_echoes(&flow.supervisor, flow.initial.cohort, 1, 64)
                .await
                .unwrap(),
            2
        );
        db.execute_unprepared(&format!("DELETE FROM \"{schema}\".players"))
            .await
            .unwrap();
        assert!(store.acknowledge_award(&one, a.lease).await.is_err());
        assert!(store.defer_award(&two, b.lease).await.is_err());
        assert!(store.claim_award(&one).await.unwrap().is_none());
    })
    .await;
    sandbox.close().await;
    observed.expect("award concurrent claim and deletion scenario completed");
}
