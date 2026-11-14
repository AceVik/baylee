use super::{Flow, Sandbox, operator, settle_fixture, terminal, verified_fixture};
use baylee_tower::v2::{
    package::{Build, Origin},
    run::{Command, Outcome},
};
use baylee_tower_store::{Role, Store};
use sea_orm::ConnectionTrait;
use uuid::Uuid;

#[tokio::test]
async fn global_echoes_are_same_stage_nonself_and_independent_of_outcome() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let observed = tokio::spawn(async move {
        let first = Flow::prepare(&store).await;
        let verifier = operator(&store, Role::Verifier, 5).await;
        let defeated = verified_fixture(&store, &first, &verifier, Outcome::Defeat).await;
        let mut second = first.another(&store).await;
        second.host = first.next_host.clone(); // A different approved gateway sees the same pool.
        let echo = terminal(&store, &second).await;
        let Origin::Echo { account, snapshot } = echo.package.opponent else {
            panic!("verified defeat must supply an echo");
        };
        assert_eq!(Uuid::from_bytes(account), first.account);
        assert_eq!(echo.package.stage, 1);
        assert!(
            echo.package.builds[1]
                == Build::freeze(&first.initial.state, &super::publication().catalog).unwrap()
        );
        assert!(
            store
                .echo_visible(first.account, Uuid::from_bytes(snapshot))
                .await
                .unwrap()
        );
        publish_stage(&store, &first, 2).await;
        let next = first.continued(defeated);
        let stage_two = terminal(&store, &next).await;
        assert_eq!(stage_two.package.stage, 2);
        assert!(
            matches!(stage_two.package.opponent, Origin::House { .. }),
            "stage one snapshots cannot fill stage two"
        );
    })
    .await;
    sandbox.close().await;
    observed.expect("global same-stage fixture completed");
}

#[tokio::test]
async fn withdrawal_reenrollment_and_swap_removal_do_not_revive_or_duplicate_echoes() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let first = Flow::prepare(&store).await;
        let verifier = operator(&store, Role::Verifier, 5).await;
        let first_done = verified_fixture(&store, &first, &verifier, Outcome::Victory).await;
        let second = first.another(&store).await;
        verified_fixture(&store, &second, &verifier, Outcome::Draw).await;
        let old = db.query_one_raw(sea_orm::Statement::from_string(sea_orm::DbBackend::Postgres,
            format!("SELECT snapshot FROM \"{schema}\".eligible_echoes WHERE account='{}'", first.account))).await.unwrap().unwrap().try_get::<Uuid>("", "snapshot").unwrap();
        store.withdraw_echoes(first.account).await.unwrap();
        store.withdraw_echoes(first.account).await.unwrap();
        assert!(!store.echo_visible(first.account, old).await.unwrap());
        store.command(first.account, Uuid::now_v7(), first_done.id, first_done.state.revision, &Command::Abandon).await.unwrap();
        let restarted = store.start(first.account, Uuid::now_v7(), super::Start { cohort: first_done.cohort, starter: 0, consent: super::CONSENT_REVISION }, [91; 32]).await.unwrap();
        assert!(!store.echo_visible(first.account, old).await.unwrap(), "renewed consent is not retrospective");
        assert_eq!(store.prune_echoes(&first.supervisor, first_done.cohort, 1, 2).await.unwrap(), 1);
        let rows = db.query_all_raw(sea_orm::Statement::from_string(sea_orm::DbBackend::Postgres,
            format!("SELECT slot,account FROM \"{schema}\".eligible_echoes ORDER BY slot"))).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].try_get::<i64>("", "slot").unwrap(), 0);
        assert_eq!(rows[0].try_get::<Uuid>("", "account").unwrap(), second.account);
        let returned = first.continued(restarted);
        let new_done = verified_fixture(&store, &returned, &verifier, Outcome::Defeat).await;
        let row = db.query_one_raw(sea_orm::Statement::from_string(sea_orm::DbBackend::Postgres,
            format!("SELECT (SELECT count(*) FROM \"{schema}\".eligible_echoes)=2 AND population=2 AS dense FROM \"{schema}\".echo_partitions WHERE stage=1"))).await.unwrap().unwrap();
        assert!(row.try_get::<bool>("", "dense").unwrap());
        store.command(first.account, Uuid::now_v7(), new_done.id, new_done.state.revision, &Command::Abandon).await.unwrap();
        let again = store.start(first.account, Uuid::now_v7(), super::Start { cohort: first_done.cohort, starter: 0, consent: super::CONSENT_REVISION }, [92; 32]).await.unwrap();
        verified_fixture(&store, &first.continued(again), &verifier, Outcome::Victory).await;
        let row = db.query_one_raw(sea_orm::Statement::from_string(sea_orm::DbBackend::Postgres,
            format!("SELECT (SELECT count(*) FROM \"{schema}\".eligible_echoes)=2 AND population=2 AS one_per_account FROM \"{schema}\".echo_partitions WHERE stage=1"))).await.unwrap().unwrap();
        assert!(row.try_get::<bool>("", "one_per_account").unwrap());
        db.execute_unprepared(&format!("UPDATE \"{schema}\".echo_snapshots SET expires_at=pg_catalog.clock_timestamp()-interval '1 second'")).await.unwrap();
        let third = first.another(&store).await;
        let fallback = terminal(&store, &third).await;
        assert!(matches!(fallback.package.opponent, Origin::House { .. }));
    }).await;
    sandbox.close().await;
    observed.expect("consent and dense removal fixture completed");
}

async fn publish_stage(store: &Store, flow: &Flow, stage: u8) {
    let publisher = store.authenticate_operator([1; 32]).await.unwrap();
    let build = Build::freeze(&flow.initial.state, &super::publication().catalog).unwrap();
    for archetype in 1..=3 {
        store
            .publish_house(&publisher, flow.initial.cohort, stage, archetype, &build)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn indexed_matching_prefers_unseen_accounts_without_crossing_cohorts() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let observed = tokio::spawn(async move {
        let first = Flow::prepare(&store).await;
        let verifier = operator(&store, Role::Verifier, 5).await;
        let first_done = verified_fixture(&store, &first, &verifier, Outcome::Victory).await;
        let second = first.another(&store).await;
        let second_done = verified_fixture(&store, &second, &verifier, Outcome::Defeat).await;
        let third = first.another(&store).await;
        let first_opponent = terminal(&store, &third).await;
        let Origin::Echo {
            account: previous, ..
        } = first_opponent.package.opponent
        else {
            panic!("two eligible accounts exist");
        };
        let third_done = settle_fixture(&store, &first_opponent, &verifier, Outcome::Draw).await;
        publish_stage(&store, &first, 2).await;
        verified_fixture(
            &store,
            &first.continued(first_done),
            &verifier,
            Outcome::Defeat,
        )
        .await;
        let repeat = terminal(&store, &second.continued(second_done)).await;
        let Origin::Echo { account, .. } = repeat.package.opponent else {
            panic!("the sole nonself echo is reusable");
        };
        assert_eq!(Uuid::from_bytes(account), first.account);
        settle_fixture(&store, &repeat, &verifier, Outcome::Victory).await;
        let fresh = terminal(&store, &third.continued(third_done)).await;
        let Origin::Echo { account, .. } = fresh.package.opponent else {
            panic!("an unseen same-stage account exists");
        };
        assert_ne!(account, previous);
        assert!([first.account, second.account].contains(&Uuid::from_bytes(account)));
        isolated_cohort(&store, &first).await;
    })
    .await;
    sandbox.close().await;
    observed.expect("indexed exclusions and cohort isolation completed");
}

async fn isolated_cohort(store: &Store, template: &Flow) {
    let mut published = super::publication();
    published.cohort.executor.build.push_str("-separate-cohort");
    let cohort = store.publish(&published).await.unwrap();
    let mut separate = template.another(store).await;
    store
        .command(
            separate.account,
            Uuid::now_v7(),
            separate.initial.id,
            0,
            &Command::Abandon,
        )
        .await
        .unwrap();
    separate.initial = store
        .start(
            separate.account,
            Uuid::now_v7(),
            super::Start {
                cohort,
                starter: 0,
                consent: super::CONSENT_REVISION,
            },
            [29; 32],
        )
        .await
        .unwrap();
    store
        .advertise(
            &separate.host,
            &[
                super::publication().cohort.executor,
                published.cohort.executor,
            ],
        )
        .await
        .unwrap();
    publish_stage(store, &separate, 1).await;
    let isolated = terminal(store, &separate).await;
    assert!(matches!(isolated.package.opponent, Origin::House { .. }));
}
