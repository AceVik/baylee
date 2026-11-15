use super::*;
use crate::{
    cards::Card,
    v2::{
        catalog::{Family, Potion, Rarity},
        generation,
        inventory::Slot,
        run::Command,
        shop::{Delivery, Goods},
    },
};
use baylee_core::preset::{Finish, LoopPolicy, SeatCapabilities, SeatController};

mod commander;
mod generations;

fn fixture() -> (Catalog, Run, Cohort, Opponent) {
    let catalog = Catalog {
        cards: vec![
            Card {
                card: CardIndex::new(0),
                price: 0,
                basic_land: true,
                run: None,
            },
            Card {
                card: CardIndex::new(1),
                price: 4,
                basic_land: false,
                run: None,
            },
        ],
    };
    let starter = Collection {
        formation: None,
        owned: vec![Stack {
            card: CardIndex::new(1),
            copies: 4,
        }],
        deck: vec![
            Stack {
                card: CardIndex::new(0),
                copies: 36,
            },
            Stack {
                card: CardIndex::new(1),
                copies: 4,
            },
        ],
    };
    let run = Run::begin_for(&catalog, starter, &[3; 32], 3).unwrap();
    let mut executor = Identity::native(Variant::Tower, "package-test", "registry-test", 21);
    executor.rules = "tower-4".into();
    let cohort = Cohort::for_generation(executor, &catalog, 3).unwrap();
    let mut echo = Build::freeze(&run, &catalog).unwrap();
    echo.active = vec![Definition {
        family: Family::Compass,
        rarity: Rarity::Common,
    }];
    echo.potions = [1, 0];
    let opponent = Opponent {
        stage: 1,
        cohort: cohort.clone(),
        origin: Origin::Echo {
            snapshot: uuid::Uuid::now_v7().into_bytes(),
            account: uuid::Uuid::now_v7().into_bytes(),
        },
        build: echo,
    };
    (catalog, run, cohort, opponent)
}

fn seeds() -> Seeds {
    Seeds {
        shuffle: 11,
        policy: 19,
    }
}

fn printing(card: CardIndex) -> Option<PrintInfo> {
    (card.get() <= 1).then(|| PrintInfo {
        scryfall_id: uuid::Uuid::nil(),
        lang: "en".into(),
        finish: Finish::Normal,
    })
}

fn equipped(mut run: Run, catalog: &Catalog) -> Run {
    let merchandise = [
        (
            Goods::Relic(Definition {
                family: Family::Heartwood,
                rarity: Rarity::Common,
            }),
            Delivery::Slot(Slot::Active(0)),
        ),
        (
            Goods::Relic(Definition {
                family: Family::Grove,
                rarity: Rarity::Common,
            }),
            Delivery::Slot(Slot::Reserve(0)),
        ),
        (Goods::Potion(Potion::Healing), Delivery::Inventory),
        (Goods::Potion(Potion::Mana), Delivery::Inventory),
    ];
    for (index, (goods, delivery)) in merchandise.into_iter().enumerate() {
        // Explicit saved offers isolate package admission from distribution tests.
        run.shop.offers[index].goods = goods;
        let offer = run.shop.offers[index].id;
        run = run
            .command(
                run.revision,
                &Command::Buy { offer, delivery },
                catalog,
                &[3; 32],
            )
            .unwrap();
    }
    run
}

#[test]
fn freeze_excludes_reserves_and_binds_the_native_asymmetric_preset() {
    let (catalog, run, cohort, opponent) = fixture();
    let run = equipped(run, &catalog);
    let reserved = Package::reserve(
        &run,
        run.revision,
        &catalog,
        &cohort,
        opponent,
        seeds(),
        4096,
    )
    .unwrap();
    assert_eq!(reserved.run.phase, Phase::Reserved);
    assert_eq!(reserved.run.economy, run.economy);
    assert_eq!(reserved.package.builds[0].active.len(), 1);
    let package = &reserved.package;
    assert_eq!(
        reserved.run.execution.as_ref().unwrap().package,
        package.digest(&catalog).unwrap()
    );
    let preset = package
        .preset(&catalog, &cohort.executor, printing)
        .unwrap();
    assert_eq!(preset.seed, 11);
    assert_eq!(preset.house_rules.challenge_policy_seed, Some(19));
    assert_eq!(preset.house_rules.loop_policy, LoopPolicy::Off);
    assert_eq!(preset.house_rules.rule_pack.as_ref().unwrap().revision, 4);
    assert_eq!(preset.seats[0].starting_life, Some(24));
    assert_eq!(preset.seats[1].starting_life, Some(20));
    assert_eq!(preset.house_rules.opening_for(0).free_mulligans, 1);
    assert_eq!(preset.house_rules.opening_for(1).free_mulligans, 2);
    assert_eq!(preset.house_rules.supplies[0].mana, 1);
    assert_eq!(preset.house_rules.supplies[1].mana, 0);
    assert_eq!(preset.seats[0].controller, SeatController::Open);
    assert_eq!(
        preset.seats[1].controller,
        SeatController::Ai(generation::contract(3).unwrap().expert())
    );
    assert!(
        preset
            .seats
            .iter()
            .all(|seat| seat.capabilities == SeatCapabilities::default()
                && seat.starting_hand.is_none()
                && seat.starting_battlefield.is_empty()
                && seat.emblems.is_empty()
                && seat.sideboard.is_empty()
                && seat.commanders.is_empty())
    );
    let encoded = package.encode(&catalog).unwrap();
    let decoded = Package::decode(&encoded, &catalog, &cohort.executor).unwrap();
    assert!(&decoded == package);
    assert_eq!(
        decoded
            .preset(&catalog, &cohort.executor, printing)
            .unwrap(),
        preset
    );
    assert!(Build::freeze(&reserved.run, &catalog).is_err());
}

#[test]
fn reservation_rejects_skew_and_every_execution_identity_field_matters() {
    let (catalog, run, cohort, opponent) = fixture();
    for changed in 0..3 {
        let mut bad = opponent.clone();
        match changed {
            0 => bad.stage += 1,
            1 => bad.cohort.executor.build.push('x'),
            _ => bad.cohort.generator += 1,
        }
        assert!(
            Package::reserve(&run, run.revision, &catalog, &cohort, bad, seeds(), 4096).is_err()
        );
    }
    assert!(
        Package::reserve(
            &run,
            run.revision + 1,
            &catalog,
            &cohort,
            opponent.clone(),
            seeds(),
            4096
        )
        .is_err()
    );
    let package = Package::reserve(
        &run,
        run.revision,
        &catalog,
        &cohort,
        opponent,
        seeds(),
        4096,
    )
    .unwrap()
    .package;
    for changed in 0..8 {
        let mut identity = cohort.executor.clone();
        match changed {
            0 => identity.schema += 1,
            1 => identity.protocol += 1,
            2 => identity.variant = Variant::Standard,
            3 => identity.build.push('x'),
            4 => identity.features = "0".repeat(64),
            5 => identity.rules.push('x'),
            6 => identity.cards.push('x'),
            _ => identity.ai.push('x'),
        }
        assert!(package.validate(&catalog, &identity).is_err());
    }
}

#[test]
fn package_decode_is_bounded_strict_and_cannot_inject_administrative_rules() {
    let (catalog, run, cohort, opponent) = fixture();
    let package = Package::reserve(
        &run,
        run.revision,
        &catalog,
        &cohort,
        opponent,
        seeds(),
        4096,
    )
    .unwrap()
    .package;
    let base = serde_json::to_value(&package).unwrap();
    for path in [
        "capabilities",
        "starting_hand",
        "price",
        "script",
        "engine_path",
    ] {
        let mut forged = base.clone();
        forged[path] = serde_json::json!("injected");
        assert!(
            Package::decode(
                &serde_json::to_vec(&forged).unwrap(),
                &catalog,
                &cohort.executor
            )
            .is_err()
        );
    }
    let mut bad = package.clone();
    bad.builds[1].deck.reverse();
    assert!(bad.validate(&catalog, &cohort.executor).is_err());
    bad = package.clone();
    bad.builds[1].potions = [2, 1];
    assert!(bad.validate(&catalog, &cohort.executor).is_err());
    bad = package;
    bad.opponent = Origin::Echo {
        snapshot: [0; 16],
        account: [0; 16],
    };
    assert!(bad.validate(&catalog, &cohort.executor).is_err());
    assert!(
        Package::decode(
            &vec![b' '; MAX_PACKAGE_BYTES + 1],
            &catalog,
            &cohort.executor
        )
        .is_err()
    );
    let mut trailing = serde_json::to_vec(&base).unwrap();
    trailing.extend_from_slice(b"{}");
    assert!(Package::decode(&trailing, &catalog, &cohort.executor).is_err());
}

#[test]
fn commitments_bind_seeds_equipment_origin_and_prices_but_not_catalog_order() {
    let (catalog, run, cohort, opponent) = fixture();
    let package = Package::reserve(
        &run,
        run.revision,
        &catalog,
        &cohort,
        opponent,
        seeds(),
        4096,
    )
    .unwrap()
    .package;
    let digest = package.digest(&catalog).unwrap();
    let mut reordered = catalog.clone();
    reordered.cards.reverse();
    assert_eq!(package.digest(&reordered).unwrap(), digest);
    for mutation in 0..4 {
        let mut altered = package.clone();
        match mutation {
            0 => altered.shuffle_seed += 1,
            1 => altered.policy_seed += 1,
            2 => altered.builds[1].active.clear(),
            _ => altered.opponent = Origin::House { archetype: 1 },
        }
        assert_ne!(altered.digest(&catalog).unwrap(), digest);
    }
    let mut changed = catalog;
    changed.cards[1].price += 1;
    assert!(package.digest(&changed).is_err());
    assert_ne!(catalog_digest_for(&changed, 3).unwrap(), cohort.catalog);
}
