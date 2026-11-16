//! Real PostgreSQL concurrency, admission and anti-resampling for synthetic decks.
#![forbid(unsafe_code)]
#[path = "support/mod.rs"]
mod support;
use baylee_cards_pool as _;
use baylee_core::{
    color::{Color, ColorSet},
    ids::CardIndex,
};
use baylee_tower::{
    cards::{Card, Catalog, Collection, Formation, RunCard, Stack},
    v2::{generation, package::Cohort, run::Command},
};
use baylee_tower_store::{CONSENT_REVISION, Publication, Start};
use sea_orm::{ConnectionTrait, Statement};
use uuid::Uuid;

#[tokio::test]
async fn real_fifty_deck_publication_roundtrips_with_bounded_player_responses() {
    let content = baylee_tower_content::expeditions().unwrap();
    let mut fixture = publication();
    fixture.cohort = Cohort::for_generation(
        fixture.cohort.executor,
        &content.catalog,
        generation::EXPEDITIONS,
    )
    .unwrap();
    fixture.catalog = content.catalog;
    fixture.starters = content.starters;
    let bytes = serde_json::to_vec(&fixture).unwrap();
    assert!(bytes.len() <= baylee_tower_store::MAX_PUBLICATION_BYTES);
    let db = support::Sandbox::open().await;
    let id = db.store.publish(&fixture).await.unwrap();
    assert_eq!(db.store.published(id).await.unwrap(), fixture);
    let offered = db
        .store
        .starter_choices(Uuid::now_v7(), id, [9; 32])
        .await
        .unwrap();
    let public = baylee_tower_api::Cohort {
        id,
        contract: fixture.cohort,
        catalog: fixture.catalog,
        starters: fixture.starters,
        offered,
    };
    public.validate().unwrap();
    let response = baylee_tower_api::Response::Cohorts {
        consent: CONSENT_REVISION,
        cohorts: vec![public; baylee_tower_api::COHORT_PAGE_SIZE],
    };
    assert!(serde_json::to_vec(&response).unwrap().len() < baylee_tower_api::MAX_RESPONSE_BYTES);
    db.close().await;
}

fn publication() -> Publication {
    let identity = |color: Color| ColorSet::of(color);
    let mut cards: Vec<_> = [
        Color::White,
        Color::Blue,
        Color::Black,
        Color::Red,
        Color::Green,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, color)| Card {
        card: CardIndex::new(u32::try_from(index).unwrap()),
        price: 0,
        basic_land: true,
        run: Some(RunCard {
            identity: identity(color),
            commander: false,
        }),
    })
    .collect();
    let starters = (5..55)
        .map(|index| {
            let commander = CardIndex::new(index);
            cards.push(Card {
                card: commander,
                price: 4,
                basic_land: false,
                run: Some(RunCard {
                    identity: identity(Color::Green),
                    commander: true,
                }),
            });
            Collection {
                formation: Some(Formation {
                    commander,
                    sideboard: Vec::new(),
                }),
                owned: vec![Stack {
                    card: commander,
                    copies: 1,
                }],
                deck: vec![Stack {
                    card: CardIndex::new(4),
                    copies: 39,
                }],
            }
        })
        .collect();
    let catalog = Catalog { cards };
    let mut executor = baylee_protocol::execution::Identity::native(
        baylee_protocol::execution::Variant::Tower,
        "draft-test",
        "synthetic",
        22,
    );
    executor.rules = "tower-5".into();
    Publication {
        cohort: Cohort::for_generation(executor, &catalog, generation::EXPEDITIONS).unwrap(),
        catalog,
        starters,
    }
}

#[tokio::test]
async fn concurrent_drafts_refuse_unoffered_starts_and_survive_unplayed_abandons() {
    let db = support::Sandbox::open().await;
    let cohort = db.store.publish(&publication()).await.unwrap();
    let account = Uuid::now_v7();
    let (one, two) = tokio::join!(
        db.store.starter_choices(account, cohort, [11; 32]),
        db.store.starter_choices(account, cohort, [22; 32]),
    );
    let offered = one.unwrap();
    assert_eq!(offered, two.unwrap());
    assert_eq!(offered.len(), 3);
    let refused = (0..50).find(|index| !offered.contains(index)).unwrap();
    let choice = |starter| Start {
        cohort,
        starter,
        consent: CONSENT_REVISION,
    };
    assert!(
        db.store
            .start(account, Uuid::now_v7(), choice(refused), [33; 32])
            .await
            .is_err()
    );
    let row = db
        .db
        .query_one_raw(Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            format!(
                "SELECT echo_consent,run_count FROM \"{}\".players WHERE account=$1",
                db.schema
            ),
            [account.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert!(
        !row.try_get::<bool>("", "echo_consent").unwrap(),
        "reading drafts never consents"
    );
    assert_eq!(
        row.try_get::<i32>("", "run_count").unwrap(),
        0,
        "refusal rolls allocation back"
    );
    let request = Uuid::now_v7();
    let run = db
        .store
        .start(account, request, choice(offered[0]), [44; 32])
        .await
        .unwrap();
    assert_eq!(
        run,
        db.store
            .start(account, request, choice(offered[0]), [55; 32])
            .await
            .unwrap()
    );
    db.store
        .command(
            account,
            Uuid::now_v7(),
            run.id,
            run.state.revision,
            &Command::Abandon {},
        )
        .await
        .unwrap();
    assert_eq!(
        offered,
        db.store
            .starter_choices(account, cohort, [66; 32])
            .await
            .unwrap()
    );
    let next = db
        .store
        .start(account, Uuid::now_v7(), choice(offered[1]), [77; 32])
        .await
        .unwrap();
    assert_ne!(run.id, next.id);
    assert_eq!(
        next.state.cards.formation.as_ref().unwrap().commander.get(),
        u32::from(offered[1]) + 5
    );
    verified_seed_notifies(&db, account, cohort).await;
    db.close().await;
}

async fn verified_seed_notifies(db: &support::Sandbox, account: Uuid, cohort: Uuid) {
    // Stand in for the verified settlement's owner-locked seed replacement.
    // The database must notify even when a different authority process writes it.
    let mut listener = db.store.changes().await.unwrap();
    db.db
        .execute_raw(Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            format!(
                "UPDATE \"{}\".enrollment SET start_seed=$2 WHERE account=$1",
                db.schema
            ),
            [account.into(), vec![88_u8; 32].into()],
        ))
        .await
        .unwrap();
    let notice = tokio::time::timeout(std::time::Duration::from_secs(2), listener.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(notice.payload(), format!("draft:{account}"));
    assert_eq!(
        db.store
            .starter_choices(account, cohort, [99; 32])
            .await
            .unwrap(),
        baylee_tower::v2::draft::choices(&[88; 32], 50).unwrap()
    );
    drop(listener);
}
