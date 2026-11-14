//! Real PostgreSQL transactions in a unique owned schema, including fault injection.
#![forbid(unsafe_code)]
mod support;

use baylee_tower::{
    Refusal,
    v2::{run::Command, shop::Delivery},
};
use baylee_tower_store::{CONSENT_REVISION, Error, Snapshot, Start, Store};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use support::{Sandbox, publication};
use uuid::Uuid;

#[tokio::test]
async fn owner_commands_are_durable_idempotent_and_atomic_under_contention() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let observed = tokio::spawn(async move { concurrent_story(store).await }).await;
    sandbox.close().await;
    observed.expect("transaction scenario completed without assertion failure");
}

async fn enrolled(store: &Store) -> (Uuid, Start, Snapshot) {
    let published = publication();
    let cohort = store.publish(&published).await.unwrap();
    assert_eq!(cohort.get_version_num(), 7);
    let mut reordered = published.clone();
    reordered.catalog.cards.reverse();
    assert_eq!(store.publish(&reordered).await.unwrap(), cohort);
    reordered.starters[0].deck[0].copies += 1;
    assert_eq!(store.publish(&reordered).await, Err(Error::Conflict));
    let account = Uuid::now_v7();
    let request = Uuid::now_v7();
    let choice = Start {
        cohort,
        starter: 0,
        consent: CONSENT_REVISION,
    };
    let first = store
        .start(account, request, choice, [7; 32])
        .await
        .unwrap();
    assert_eq!(first.id.get_version_num(), 7);
    assert_eq!(
        store
            .start(account, request, choice, [8; 32])
            .await
            .unwrap(),
        first
    );
    assert_eq!(
        store.start(account, Uuid::now_v7(), choice, [8; 32]).await,
        Err(Error::Conflict)
    );
    assert_eq!(
        store
            .start(
                account,
                request,
                Start {
                    starter: 1,
                    ..choice
                },
                [7; 32]
            )
            .await,
        Err(Error::Conflict)
    );
    assert_eq!(
        store.get(Uuid::now_v7(), first.id).await,
        Err(Error::NotFound)
    );
    (account, choice, first)
}

async fn concurrent_story(store: Store) {
    let (account, choice, first) = enrolled(&store).await;
    let command = Command::Buy {
        offer: first.state.shop.offers[0].id,
        delivery: Delivery::Inventory,
    };
    let request = Uuid::now_v7();
    let mut tasks = Vec::new();
    for _ in 0..8 {
        let store = store.clone();
        let command = command.clone();
        let id = first.id;
        tasks.push(tokio::spawn(async move {
            store.command(account, request, id, 0, &command).await
        }));
    }
    let mut replies = Vec::new();
    for task in tasks {
        replies.push(task.await.unwrap().unwrap());
    }
    assert!(replies.iter().all(|reply| reply == &replies[0]));
    let bought = store.get(account, first.id).await.unwrap();
    assert_eq!(bought.state.revision, 1);
    assert_eq!(bought.state.economy.gold, 16);
    assert_eq!(
        store
            .command(account, request, first.id, 1, &Command::Refresh {})
            .await,
        Err(Error::Conflict)
    );

    let mut tasks = Vec::new();
    for _ in 0..8 {
        let store = store.clone();
        let id = first.id;
        tasks.push(tokio::spawn(async move {
            store
                .command(account, Uuid::now_v7(), id, 1, &Command::Refresh {})
                .await
        }));
    }
    let mut successes = 0;
    for task in tasks {
        match task.await.unwrap() {
            Ok(_) => successes += 1,
            Err(error) => assert_eq!(error, Error::Refused(Refusal::Stale)),
        }
    }
    assert_eq!(successes, 1);
    let current = store.get(account, first.id).await.unwrap();
    assert_eq!(current.state.revision, 2);
    assert_eq!(current.state.economy.gold, 14);
    store
        .command(account, Uuid::now_v7(), first.id, 2, &Command::Abandon {})
        .await
        .unwrap();
    let second = store
        .start(account, Uuid::now_v7(), choice, [99; 32])
        .await
        .unwrap();
    assert_ne!(second.id, first.id);
    assert_eq!(
        second.state.shop, first.state.shop,
        "early abandonment never rerolls the starting sequence"
    );
    assert_eq!(
        store
            .command(account, request, first.id, 0, &command)
            .await
            .unwrap(),
        bought,
        "an old retry returns its original committed response, not later state"
    );
}

#[tokio::test]
async fn receipt_failure_rolls_back_wallet_inventory_revision_and_availability() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        let cohort = store.publish(&publication()).await.unwrap();
        let account = Uuid::now_v7();
        let first = store.start(account, Uuid::now_v7(), Start { cohort, starter: 0, consent: CONSENT_REVISION }, [4; 32]).await.unwrap();
        let request = Uuid::now_v7();
        db.execute_unprepared(&format!("ALTER TABLE \"{schema}\".requests ADD CONSTRAINT injected_failure CHECK (request <> '{request}')"))
            .await.unwrap();
        let command = Command::Buy { offer: first.state.shop.offers[0].id, delivery: Delivery::Inventory };
        assert_eq!(store.command(account, request, first.id, 0, &command).await, Err(Error::Unavailable));
        assert_eq!(store.get(account, first.id).await.unwrap(), first);
        db.execute_unprepared(&format!("ALTER TABLE \"{schema}\".requests DROP CONSTRAINT injected_failure"))
            .await.unwrap();
        let committed = store.command(account, request, first.id, 0, &command).await.unwrap();
        assert_eq!(committed.state.economy.gold, 16);
        assert_eq!(committed.state.revision, 1);
        let other = Uuid::now_v7();
        assert_eq!(store.command(other, Uuid::now_v7(), first.id, 1, &Command::Abandon {}).await, Err(Error::NotFound));
        assert_eq!(store.get(account, first.id).await.unwrap(), committed);
    }).await;
    sandbox.close().await;
    observed.expect("fault-injection scenario completed without assertion failure");
}

#[tokio::test]
async fn migration_is_repeatable_but_unknown_history_and_implicit_consent_are_refused() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let db = sandbox.db.clone();
    let schema = sandbox.schema.clone();
    let observed = tokio::spawn(async move {
        store.migrate().await.unwrap();
        store.ready().await.unwrap();
        assert!(Store::new(db.clone(), "public; DROP SCHEMA public").is_err());
        let cohort = store.publish(&publication()).await.unwrap();
        assert!(
            store
                .start(
                    Uuid::now_v7(),
                    Uuid::now_v7(),
                    Start {
                        cohort,
                        starter: 0,
                        consent: 0
                    },
                    [5; 32]
                )
                .await
                .is_err()
        );
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!("UPDATE \"{schema}\".schema_version SET digest=$1"),
            [vec![0_u8; 32].into()],
        ))
        .await
        .unwrap();
        assert_eq!(store.ready().await, Err(Error::Schema));
        assert_eq!(store.migrate().await, Err(Error::Schema));
    })
    .await;
    sandbox.close().await;
    observed.expect("migration scenario completed without assertion failure");
}
