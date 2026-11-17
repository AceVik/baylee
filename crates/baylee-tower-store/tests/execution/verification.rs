use super::{Flow, Sandbox, operator, terminal};
use baylee_tower::v2::run::{Outcome, Phase, Settlement};
use baylee_tower_store::{Error, Principal, Recovery, Role, Store, VerificationJob};
use sea_orm::ConnectionTrait;
use uuid::Uuid;

#[tokio::test]
async fn independent_verifier_leases_reject_hosts_expiry_and_replaced_evidence() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let flow = Flow::prepare(&store).await;
        let verifier = operator(&store, Role::Verifier, 5).await;
        let replacement = operator(&store, Role::Verifier, 6).await;
        assert!(
            store
                .verification_candidates(&verifier, flow.initial.cohort, 32)
                .await
                .unwrap()
                .is_empty()
        );
        let claimed = terminal(&store, &flow).await;
        let reservation = claimed.lease.reservation;
        assert_eq!(
            store
                .verification_candidates(&verifier, flow.initial.cohort, 32)
                .await
                .unwrap(),
            vec![reservation]
        );
        assert!(
            store
                .verification_candidates(&flow.host, flow.initial.cohort, 32)
                .await
                .is_err()
        );
        assert!(
            store
                .claim_verification(&flow.host, reservation, Uuid::now_v7(), 120_000)
                .await
                .is_err()
        );
        let claim = Uuid::now_v7();
        let first = store
            .claim_verification(&verifier, reservation, claim, 120_000)
            .await
            .unwrap();
        let retry = store
            .claim_verification(&verifier, reservation, claim, 120_000)
            .await
            .unwrap();
        assert_eq!(first.lease, retry.lease);
        assert!(first.package == claimed.package);
        assert!(
            store
                .claim_verification(&replacement, reservation, Uuid::now_v7(), 120_000)
                .await
                .is_err()
        );
        assert!(
            store
                .verification_chunk(&flow.host, first.lease, 0)
                .await
                .is_err()
        );
        assert_eq!(
            store
                .verification_chunk(&verifier, first.lease, 0)
                .await
                .unwrap()
                .data,
            b"synthetic terminal fixture\n"
        );
        db.execute_unprepared(&format!(
            "UPDATE \"{schema}\".verification_jobs SET lease_until=1"
        ))
        .await
        .unwrap();
        expired_then_recovered(&store, &flow, &verifier, &replacement, &first).await;
    })
    .await;
    sandbox.close().await;
    observed.expect("independent verification lease scenario completed");
}

async fn expired_then_recovered(
    store: &Store,
    flow: &Flow,
    verifier: &Principal,
    replacement: &Principal,
    first: &VerificationJob,
) {
    let reservation = first.lease.reservation;
    let claim = first.lease.claim;
    assert!(
        store
            .renew_verification(verifier, first.lease, 120_000)
            .await
            .is_err()
    );
    assert!(
        store
            .claim_verification(verifier, reservation, claim, 120_000)
            .await
            .is_err()
    );
    let second = store
        .claim_verification(replacement, reservation, Uuid::now_v7(), 120_000)
        .await
        .unwrap();
    assert!(second.lease.generation > first.lease.generation);
    assert_eq!(second.lease.binding, first.lease.binding);
    assert!(
        store
            .verification_chunk(verifier, first.lease, 0)
            .await
            .is_err()
    );
    let state = store.get(flow.account, flow.initial.id).await.unwrap();
    let stopped = store
        .supervise(
            &flow.supervisor,
            flow.initial.id,
            state.state.revision,
            Recovery::Quarantine,
        )
        .await
        .unwrap();
    assert!(
        store
            .verification_chunk(replacement, second.lease, 0)
            .await
            .is_err()
    );
    assert!(
        store
            .verification_candidates(verifier, flow.initial.cohort, 32)
            .await
            .unwrap()
            .is_empty()
    );
    store
        .supervise(
            &flow.supervisor,
            flow.initial.id,
            stopped.state.revision,
            Recovery::Resume { host: None },
        )
        .await
        .unwrap();
    let third = store
        .claim_verification(verifier, reservation, Uuid::now_v7(), 120_000)
        .await
        .unwrap();
    assert!(third.lease.generation > second.lease.generation);
    assert!(third.lease.binding.fence > second.lease.binding.fence);
    assert_eq!(third.lease.binding.prefix, first.lease.binding.prefix);
}

#[tokio::test]
async fn evidence_whose_claims_are_spent_is_quarantined_instead_of_heading_the_queue() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let flow = Flow::prepare(&store).await;
        let verifier = operator(&store, Role::Verifier, 5).await;
        let reservation = terminal(&store, &flow).await.lease.reservation;
        let expire = format!("UPDATE \"{schema}\".verification_jobs SET lease_until=0");
        for _ in 0..baylee_tower_store::MAX_EXECUTION_CLAIMS {
            assert!(
                store
                    .quarantine_unclaimable(&flow.supervisor, reservation)
                    .await
                    .is_err(),
                "a job with a claim left is not isolated"
            );
            store
                .claim_verification(&verifier, reservation, Uuid::now_v7(), 120_000)
                .await
                .unwrap();
            db.execute_unprepared(&expire).await.unwrap();
        }
        assert_eq!(
            store
                .claim_verification(&verifier, reservation, Uuid::now_v7(), 120_000)
                .await
                .map(|_| ()),
            Err(Error::Refused(baylee_tower::Refusal::Capacity))
        );
        assert_eq!(
            store
                .verification_candidates(&verifier, flow.initial.cohort, 1)
                .await
                .unwrap(),
            vec![reservation]
        );
        assert!(
            store
                .quarantine_unclaimable(&verifier, reservation)
                .await
                .is_err()
        );
        let quarantined = store
            .quarantine_unclaimable(&flow.supervisor, reservation)
            .await
            .unwrap();
        assert_eq!(quarantined.state.phase, Phase::Quarantined);
        assert!(
            store
                .verification_candidates(&verifier, flow.initial.cohort, 1)
                .await
                .unwrap()
                .is_empty()
        );
        let row = db
            .query_one_raw(sea_orm::Statement::from_string(
                sea_orm::DbBackend::Postgres,
                format!("SELECT failure FROM \"{schema}\".verification_jobs"),
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<String>("", "failure").unwrap(), "budget");
    })
    .await;
    sandbox.close().await;
    observed.expect("exhausted verification scenario completed");
}

#[tokio::test]
async fn adjudication_income_eligibility_and_outbox_commit_once_or_not_at_all() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let flow = Flow::prepare(&store).await;
        let verifier = operator(&store, Role::Verifier, 5).await;
        let claimed = terminal(&store, &flow).await;
        let job = store.claim_verification(&verifier, claimed.lease.reservation, Uuid::now_v7(), 120_000).await.unwrap();
        let report = Settlement { binding: job.lease.binding, outcome: Outcome::Defeat, turns: 42, used: [0; 2] };
        assert!(store.settle_verified(&flow.host, job.lease, &report, [99; 32]).await.is_err());
        let mut altered = report.clone();
        altered.binding.prefix.digest[0] ^= 1;
        assert!(store.settle_verified(&verifier, job.lease, &altered, [99; 32]).await.is_err());
        altered = report.clone();
        altered.used = [1, 0];
        assert!(store.settle_verified(&verifier, job.lease, &altered, [99; 32]).await.is_err());
        let before = store.get(flow.account, flow.initial.id).await.unwrap();
        db.execute_unprepared(&format!("ALTER TABLE \"{schema}\".award_outbox ADD CONSTRAINT injected_failure CHECK (false)")).await.unwrap();
        assert_eq!(store.settle_verified(&verifier, job.lease, &report, [99; 32]).await, Err(Error::Unavailable));
        assert_eq!(store.get(flow.account, flow.initial.id).await.unwrap(), before);
        let row = db.query_one_raw(sea_orm::Statement::from_string(sea_orm::DbBackend::Postgres,
            format!("SELECT NOT EXISTS(SELECT 1 FROM \"{schema}\".settled_duels) AND NOT EXISTS(SELECT 1 FROM \"{schema}\".echo_snapshots) AND NOT EXISTS(SELECT 1 FROM \"{schema}\".eligible_echoes) AND NOT EXISTS(SELECT 1 FROM \"{schema}\".award_outbox) AS empty"))).await.unwrap().unwrap();
        assert!(row.try_get::<bool>("", "empty").unwrap());
        db.execute_unprepared(&format!("ALTER TABLE \"{schema}\".award_outbox DROP CONSTRAINT injected_failure")).await.unwrap();
        let mut attempts = Vec::new();
        for _ in 0..8 {
            let store = store.clone();
            let verifier = verifier.clone();
            let report = report.clone();
            let lease = job.lease;
            attempts.push(tokio::spawn(async move { store.settle_verified(&verifier, lease, &report, [99; 32]).await }));
        }
        let mut results = Vec::new();
        for attempt in attempts { results.push(attempt.await.unwrap().unwrap()); }
        assert!(results.iter().all(|result| result == &results[0]));
        let result = &results[0];
        assert_eq!(result.state.phase, Phase::Shop);
        assert_eq!(result.state.shop.stage, 2);
        assert_eq!(result.state.economy.gold, flow.initial.state.economy.gold + 12);
        assert_eq!(result.state.defeats(), 1);
        let row = db.query_one_raw(sea_orm::Statement::from_string(sea_orm::DbBackend::Postgres,
            format!("SELECT (SELECT count(*) FROM \"{schema}\".settled_duels)=1 AND (SELECT count(*) FROM \"{schema}\".eligible_echoes)=1 AND (SELECT count(*) FROM \"{schema}\".award_outbox)=1 AND (SELECT count(*) FROM \"{schema}\".echo_snapshots)=1 AS once,
            p.receipt_bytes=(SELECT sum(octet_length(response)) FROM \"{schema}\".requests)+(SELECT sum(octet_length(response)) FROM \"{schema}\".settled_duels) AS bytes_ok FROM \"{schema}\".players p"))).await.unwrap().unwrap();
        assert!(row.try_get::<bool>("", "once").unwrap());
        assert!(row.try_get::<bool>("", "bytes_ok").unwrap());
        let mut different = report.clone(); different.outcome = Outcome::Victory;
        assert_eq!(store.settle_verified(&verifier, job.lease, &different, [88; 32]).await, Err(Error::Conflict));
        store.command(flow.account, Uuid::now_v7(), result.id, result.state.revision, &baylee_tower::v2::run::Command::Abandon {}).await.unwrap();
        let restarted = store.start(flow.account, Uuid::now_v7(), super::Start { cohort: result.cohort, starter: 0, consent: super::CONSENT_REVISION }, [44; 32]).await.unwrap();
        let expected = baylee_tower::v2::run::Run::begin_for(&super::publication().catalog, super::publication().starters[0].clone(), &[99; 32], 3).unwrap();
        assert_eq!(restarted.state.shop, expected.shop, "first verified duel rotates future starts once");
        assert_eq!(store.settle_verified(&verifier, job.lease, &report, [88; 32]).await.unwrap(), *result);
    }).await;
    sandbox.close().await;
    observed.expect("atomic verified settlement scenario completed");
}
