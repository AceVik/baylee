//! Run power survives resumption, never enrollment into a new run.
#![forbid(unsafe_code)]
mod support;

use baylee_core::ids::CardIndex;
use baylee_tower::{
    cards::Stack,
    v2::{
        STARTING_GOLD,
        catalog::{Family, Potion, Rarity, attributes},
        economy::Recipe,
        inventory::Slot,
        run::{Command, Phase, Run},
        shop::{Delivery, Goods},
    },
};
use baylee_tower_store::{CONSENT_REVISION, Error, Publication, Snapshot, Start, Store};
use support::{Sandbox, retained_publication};
use uuid::Uuid;

#[tokio::test]
async fn new_run_resets_power_without_erasing_old_run_or_replaying_its_purchases() {
    let sandbox = Sandbox::open().await;
    let store = sandbox.store.clone();
    let observed = tokio::spawn(async move { run_scope_story(store).await }).await;
    sandbox.close().await;
    observed.expect("run isolation scenario completed without assertion failure");
}

fn fusion_seed(published: &Publication) -> [u8; 32] {
    (0..=u8::MAX)
        .find_map(|byte| {
            let seed = [byte; 32];
            let run = Run::begin_for(
                &published.catalog,
                published.starters[0].clone(),
                &seed,
                published.cohort.generator,
            )
            .unwrap();
            let offers = &run.shop.offers;
            let matching = matches!(offers[3].goods, Goods::Relic(definition)
                if definition.rarity == Rarity::Common && definition.family != Family::Grove)
                && offers[3..6]
                    .iter()
                    .all(|offer| offer.goods == offers[3].goods);
            let healing = offers[6..]
                .iter()
                .any(|offer| offer.goods == Goods::Potion(Potion::Healing));
            (matching && healing).then_some(seed)
        })
        .expect("bounded original-generator fixture has a legal opening fusion and healing offer")
}

async fn apply(store: &Store, account: Uuid, run: &mut Snapshot, command: Command) -> Uuid {
    let request = Uuid::now_v7();
    *run = store
        .command(account, request, run.id, run.state.revision, &command)
        .await
        .unwrap();
    request
}

async fn improve(store: &Store, account: Uuid, run: &mut Snapshot) {
    for (index, slot) in [(4, Slot::Reserve(0)), (5, Slot::Reserve(1))] {
        let offer = run.state.shop.offers[index].id;
        apply(
            store,
            account,
            run,
            Command::Buy {
                offer,
                delivery: Delivery::Slot(slot),
            },
        )
        .await;
    }
    let offer = run
        .state
        .shop
        .offers
        .iter()
        .find(|offer| offer.goods == Goods::Potion(Potion::Healing))
        .unwrap()
        .id;
    apply(
        store,
        account,
        run,
        Command::Buy {
            offer,
            delivery: Delivery::Inventory,
        },
    )
    .await;
    let offer = run.state.shop.offers[0].id;
    apply(
        store,
        account,
        run,
        Command::Buy {
            offer,
            delivery: Delivery::Inventory,
        },
    )
    .await;
    let deck = vec![
        Stack {
            card: CardIndex::new(0),
            copies: 39,
        },
        Stack {
            card: run.state.cards.owned[0].card,
            copies: 1,
        },
    ];
    apply(store, account, run, Command::Edit { deck }).await;
    let recipe = Recipe {
        relics: run
            .state
            .economy
            .inventory
            .relics
            .iter()
            .map(|relic| relic.id)
            .collect(),
        mythic: None,
        destination: Slot::Active(0),
    };
    apply(store, account, run, Command::Fuse { recipe }).await;
    let offer = run.state.shop.offers[1].id;
    apply(
        store,
        account,
        run,
        Command::Pin {
            offer,
            pinned: true,
        },
    )
    .await;
    assert_eq!(run.state.economy.gold, 1);
    assert_eq!(run.state.economy.potions, [1, 0]);
    assert_eq!(run.state.economy.inventory.relics.len(), 1);
    assert_eq!(
        run.state.economy.inventory.relics[0].definition.rarity,
        Rarity::Uncommon
    );
    assert_ne!(
        attributes(&run.state.economy.inventory.active()).unwrap(),
        attributes(&[]).unwrap()
    );
    assert_eq!(store.get(account, run.id).await.unwrap(), *run);
}

async fn run_scope_story(store: Store) {
    let published = retained_publication();
    let seed = fusion_seed(&published);
    let choice = Start {
        cohort: store.publish(&published).await.unwrap(),
        starter: 0,
        consent: CONSENT_REVISION,
    };
    let account = Uuid::now_v7();
    let first = store
        .start(account, Uuid::now_v7(), choice, seed)
        .await
        .unwrap();
    let purchase = Command::Buy {
        offer: first.state.shop.offers[3].id,
        delivery: Delivery::Slot(Slot::Active(0)),
    };
    let mut old = first.clone();
    let old_request = apply(&store, account, &mut old, purchase.clone()).await;
    let old_receipt = old.clone();
    improve(&store, account, &mut old).await;
    apply(&store, account, &mut old, Command::Abandon {}).await;
    assert_eq!(old.state.phase, Phase::Abandoned);

    let request = Uuid::now_v7();
    let fresh = store
        .start(account, request, choice, [99; 32])
        .await
        .unwrap();
    assert_ne!(fresh.id, old.id);
    assert_eq!(fresh.state.economy.gold, STARTING_GOLD);
    // Full state equality covers equipment, charges, purchased cards, pins, ledger,
    // revision and history. Abandoning before a duel does not reroll the shop seed.
    assert_eq!(fresh.state, first.state);
    assert_eq!(
        store
            .start(account, request, choice, [100; 32])
            .await
            .unwrap(),
        fresh
    );
    assert_eq!(
        store
            .command(
                account,
                old_request,
                old.id,
                first.state.revision,
                &purchase
            )
            .await
            .unwrap(),
        old_receipt,
    );
    assert_eq!(
        store
            .command(
                account,
                old_request,
                fresh.id,
                fresh.state.revision,
                &purchase
            )
            .await,
        Err(Error::Conflict),
    );
    reject_old_assets(&store, account, &old, &fresh).await;
    assert_eq!(store.get(account, fresh.id).await.unwrap(), fresh);
    assert_eq!(store.get(account, old.id).await.unwrap(), old);
}

async fn reject_old_assets(store: &Store, account: Uuid, old: &Snapshot, fresh: &Snapshot) {
    let item = old.state.economy.inventory.relics[0].id;
    for command in [
        Command::Move {
            item,
            slot: Slot::Active(0),
        },
        Command::Sell { item },
        Command::Edit {
            deck: old.state.cards.deck.clone(),
        },
    ] {
        let result = store
            .command(
                account,
                Uuid::now_v7(),
                fresh.id,
                fresh.state.revision,
                &command,
            )
            .await;
        assert!(
            matches!(result, Err(Error::Refused(_))),
            "old-run power cannot be imported"
        );
        assert_eq!(store.get(account, fresh.id).await.unwrap(), *fresh);
    }
}
