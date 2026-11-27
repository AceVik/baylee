//! Erasing one account's Tower data on an individual request
//! (`docs/privacy.md` §"Tower authority personal bests").
use super::{Flow, Sandbox, operator, settle_fixture, terminal, verified_fixture};
use baylee_tower::v2::{
    package::Origin,
    run::{Outcome, Phase},
};
use baylee_tower_store::{Erasure, Role};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use uuid::Uuid;

/// Rows of `table` that still name `account` in `column`.
async fn naming(
    db: &DatabaseConnection,
    schema: &str,
    table: &str,
    column: &str,
    account: Uuid,
) -> i64 {
    db.query_one_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        format!("SELECT count(*) AS n FROM \"{schema}\".{table} WHERE {column}=$1"),
        [account.into()],
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get::<i64>("", "n")
    .unwrap()
}

#[tokio::test]
async fn an_erased_echo_owner_leaves_the_duel_against_its_echo_to_settle() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let first = Flow::prepare(&store).await;
        let verifier = operator(&store, Role::Verifier, 5).await;
        verified_fixture(&store, &first, &verifier, Outcome::Defeat).await;
        let second = first.another(&store).await;
        // The second player's duel meets the first one's echo and is sealed...
        let echo = terminal(&store, &second).await;
        let Origin::Echo { account, .. } = echo.package.opponent else {
            panic!("a verified duel supplies an echo");
        };
        assert_eq!(Uuid::from_bytes(account), first.account);
        // ...when the first account is erased, and erased again.
        assert_eq!(
            store.erase_account(first.account).await.unwrap(),
            Erasure::Removed
        );
        assert_eq!(
            store.erase_account(first.account).await.unwrap(),
            Erasure::Absent
        );
        for (table, column) in [
            ("players", "account"),
            ("enrollment", "account"),
            ("runs", "account"),
            ("requests", "account"),
            ("echo_snapshots", "account"),
            ("eligible_echoes", "account"),
            ("personal_bests", "account"),
            ("award_outbox", "account"),
        ] {
            assert_eq!(
                naming(&db, &schema, table, column, first.account).await,
                0,
                "{table} still names the erased account"
            );
        }
        // The pool stays dense: its population counts the slots that remain.
        let pool = db
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                format!(
                    "SELECT p.population, (SELECT count(*) FROM \"{schema}\".eligible_echoes e
                     WHERE e.cohort=p.cohort AND e.stage=p.stage) AS slots
                     FROM \"{schema}\".echo_partitions p WHERE p.stage=1"
                ),
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            pool.try_get::<i64>("", "population").unwrap(),
            pool.try_get::<i64>("", "slots").unwrap()
        );
        // The other player's duel keeps its committed package and its echo
        // lane (a missing opponent would read as a house duel), and settles.
        assert_eq!(
            naming(
                &db,
                &schema,
                "reservations",
                "opponent_account",
                first.account
            )
            .await,
            1
        );
        let settled = settle_fixture(&store, &echo, &verifier, Outcome::Victory).await;
        assert_eq!(settled.state.phase, Phase::Reward);
    })
    .await;
    sandbox.close().await;
    observed.expect("erasure beside another player's duel completed");
}

#[tokio::test]
async fn an_account_whose_duel_is_under_way_is_not_erased() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let flow = Flow::prepare(&store).await;
        let verifier = operator(&store, Role::Verifier, 5).await;
        flow.reserve(&store).await;
        assert_eq!(
            store.erase_account(flow.account).await.unwrap(),
            Erasure::Busy
        );
        let claimed = store
            .claim(&flow.host, flow.initial.id, Uuid::now_v7(), 120_000)
            .await
            .unwrap();
        assert_eq!(
            store.erase_account(flow.account).await.unwrap(),
            Erasure::Busy
        );
        let ack = store
            .append(
                &flow.host,
                claimed.lease,
                claimed.prefix,
                b"synthetic record\n",
            )
            .await
            .unwrap();
        store
            .seal(&flow.host, claimed.lease, ack.prefix)
            .await
            .unwrap();
        assert_eq!(
            store.erase_account(flow.account).await.unwrap(),
            Erasure::Busy
        );
        assert_eq!(
            naming(&db, &schema, "players", "account", flow.account).await,
            1
        );
        settle_fixture(&store, &claimed, &verifier, Outcome::Defeat).await;
        assert_eq!(
            store.erase_account(flow.account).await.unwrap(),
            Erasure::Removed
        );
        assert_eq!(
            store.erase_account(Uuid::now_v7()).await.unwrap(),
            Erasure::Absent
        );
    })
    .await;
    sandbox.close().await;
    observed.expect("erasure around a duel under way completed");
}
