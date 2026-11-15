//! Isolated authority transactions. Synthetic records test persistence, not MTG.
use super::{CONSENT_REVISION, Flow, Sandbox, Start, operator, terminal};
use baylee_core::{
    color::{Color, ColorSet},
    ids::CardIndex,
};
use baylee_tower::{
    cards::{Card, Catalog, Collection, Formation, RunCard, Stack},
    v2::{
        generation,
        package::Cohort,
        run::{Command, Outcome, Phase, Settlement},
        shop::Delivery,
    },
};
use baylee_tower_store::{Publication, Role, Snapshot, Store};
use uuid::Uuid;

fn publication() -> Publication {
    let mut cards: Vec<_> = Color::ALL
        .into_iter()
        .enumerate()
        .map(|(index, color)| Card {
            card: CardIndex::new(u32::try_from(index).unwrap()),
            price: 0,
            basic_land: true,
            run: Some(RunCard {
                identity: ColorSet::of(color),
                commander: false,
            }),
        })
        .collect();
    cards.extend((5..12).map(|id| Card {
        card: CardIndex::new(id),
        price: 4,
        basic_land: false,
        run: Some(RunCard {
            identity: ColorSet::of(Color::Green),
            commander: id < 10,
        }),
    }));
    let catalog = Catalog { cards };
    let starters = (5..10)
        .map(|id| Collection {
            formation: Some(Formation {
                commander: CardIndex::new(id),
                sideboard: vec![],
            }),
            owned: [id, 10, 11]
                .map(|card| Stack {
                    card: CardIndex::new(card),
                    copies: 1,
                })
                .into(),
            deck: vec![
                Stack {
                    card: CardIndex::new(4),
                    copies: 37,
                },
                Stack {
                    card: CardIndex::new(10),
                    copies: 1,
                },
                Stack {
                    card: CardIndex::new(11),
                    copies: 1,
                },
            ],
        })
        .collect();
    let mut executor = super::publication().cohort.executor;
    executor.rules = "tower-5".into();
    executor.ai = "tower-house-22".into();
    Publication {
        cohort: Cohort::for_generation(executor, &catalog, generation::RUN).unwrap(),
        catalog,
        starters,
    }
}

#[tokio::test]
async fn five_starters_reward_retry_concurrent_claim_freeze_and_fresh_run_are_atomic() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let observed = tokio::spawn(async move {
        let published = publication();
        let (flow, other) = enrolled(&store, &published).await;
        let pending = settled(&store, &flow).await;
        let next = claim(&store, &flow, &pending).await;
        restarted(&store, &flow, other, &next, &published).await;
    })
    .await;
    sandbox.close().await;
    observed.expect("isolated run workshop transaction scenario completed");
}

async fn enrolled(store: &Store, published: &Publication) -> (Flow, Uuid) {
    let mut flow = Flow::prepare_with(store, published).await;
    let other = Uuid::now_v7();
    let fifth = store
        .start(
            other,
            Uuid::now_v7(),
            Start {
                cohort: flow.initial.cohort,
                starter: 4,
                consent: CONSENT_REVISION,
            },
            [3; 32],
        )
        .await
        .unwrap();
    assert_eq!(
        fifth.state.cards.formation.unwrap().commander,
        CardIndex::new(9)
    );
    assert!(
        store
            .start(
                Uuid::now_v7(),
                Uuid::now_v7(),
                Start {
                    cohort: flow.initial.cohort,
                    starter: 5,
                    consent: CONSENT_REVISION
                },
                [3; 32]
            )
            .await
            .is_err()
    );
    let run = &flow.initial;
    flow.initial = store
        .command(
            flow.account,
            Uuid::now_v7(),
            run.id,
            run.state.revision,
            &Command::Freeze { frozen: true },
        )
        .await
        .unwrap();
    let run = &flow.initial;
    flow.initial = store
        .command(
            flow.account,
            Uuid::now_v7(),
            run.id,
            run.state.revision,
            &Command::Buy {
                offer: run.state.shop.offers[0].id,
                delivery: Delivery::Inventory,
            },
        )
        .await
        .unwrap();
    (flow, other)
}

async fn settled(store: &Store, flow: &Flow) -> Snapshot {
    let verifier = operator(store, Role::Verifier, 5).await;
    let claimed = terminal(store, flow).await;
    let job = store
        .claim_verification(
            &verifier,
            claimed.lease.reservation,
            Uuid::now_v7(),
            120_000,
        )
        .await
        .unwrap();
    let report = Settlement {
        binding: job.lease.binding,
        outcome: Outcome::Victory,
        turns: 36,
        used: [0; 2],
    };
    let pending = store
        .settle_verified(&verifier, job.lease, &report, [88; 32])
        .await
        .unwrap();
    assert_eq!(pending.state.phase, Phase::Reward);
    assert_eq!(
        store
            .settle_verified(&verifier, job.lease, &report, [99; 32])
            .await
            .unwrap(),
        pending
    );
    assert_eq!(store.get(flow.account, pending.id).await.unwrap(), pending);
    let view = baylee_tower_api::Run::project(pending.id, pending.cohort, pending.state.clone());
    let json = serde_json::to_value(view).unwrap();
    assert!(json["reward"].get("package").is_none());
    assert!(json.get("execution").is_none());
    assert!(
        store
            .command(
                flow.account,
                Uuid::now_v7(),
                pending.id,
                pending.state.revision,
                &Command::Refresh {}
            )
            .await
            .is_err()
    );
    pending
}

async fn claim(store: &Store, flow: &Flow, pending: &Snapshot) -> Snapshot {
    let command = Command::Claim {
        cards: pending.state.reward.as_ref().unwrap().cards.clone(),
    };
    let request = Uuid::now_v7();
    let (first, retry) = tokio::join!(
        store.command(
            flow.account,
            request,
            pending.id,
            pending.state.revision,
            &command
        ),
        store.command(
            flow.account,
            request,
            pending.id,
            pending.state.revision,
            &command
        )
    );
    let next = first.unwrap();
    assert_eq!(retry.unwrap(), next);
    assert_eq!(next.state.revision, pending.state.revision + 1);
    assert_eq!(next.state.phase, Phase::Shop);
    assert_eq!(next.state.shop.stage, 2);
    assert!(next.state.shop.frozen);
    assert_ne!(
        next.state.shop.offers[0].id,
        pending.state.shop.offers[0].id
    );
    assert_eq!(next.state.shop.offers[1..], pending.state.shop.offers[1..]);
    assert_eq!(next.state.claims.len(), 1);
    assert_eq!(
        next.state.economy.gold,
        pending.state.economy.gold + u16::from(next.state.claims[0].duplicates)
    );
    assert!(
        store
            .command(
                flow.account,
                Uuid::now_v7(),
                next.id,
                next.state.revision,
                &command
            )
            .await
            .is_err()
    );
    next
}

async fn restarted(
    store: &Store,
    flow: &Flow,
    other: Uuid,
    next: &Snapshot,
    published: &Publication,
) {
    assert!(
        store
            .command(
                other,
                Uuid::now_v7(),
                next.id,
                next.state.revision,
                &Command::Abandon {}
            )
            .await
            .is_err()
    );
    store
        .command(
            flow.account,
            Uuid::now_v7(),
            next.id,
            next.state.revision,
            &Command::Abandon {},
        )
        .await
        .unwrap();
    let fresh = store
        .start(
            flow.account,
            Uuid::now_v7(),
            Start {
                cohort: next.cohort,
                starter: 0,
                consent: CONSENT_REVISION,
            },
            [2; 32],
        )
        .await
        .unwrap();
    assert!(fresh.state.economy.inventory.relics.is_empty());
    assert!(fresh.state.claims.is_empty());
    assert!(fresh.state.reward.is_none());
    assert_eq!(fresh.state.economy.gold, 18);
    assert_eq!(fresh.state.cards, published.starters[0]);
}
