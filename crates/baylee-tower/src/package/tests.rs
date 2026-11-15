use super::*;
use crate::{cards::Card, items::Relic};
use baylee_core::ids::CardIndex;

fn printing(card: CardIndex) -> Option<PrintInfo> {
    (card.get() <= 1).then(|| PrintInfo {
        scryfall_id: uuid::Uuid::nil(),
        lang: "en".into(),
        finish: baylee_core::preset::Finish::Normal,
    })
}

fn fixture() -> (Catalog, Package) {
    let catalog = Catalog {
        cards: vec![
            Card {
                card: CardIndex::new(0),
                basic_land: true,
                run: None,
                price: 0,
            },
            Card {
                card: CardIndex::new(1),
                basic_land: false,
                run: None,
                price: 4,
            },
        ],
    };
    let build = Build {
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
        equipment: Vec::new(),
        potions: [0; 2],
    };
    let package = Package {
        schema: 1,
        cohort: Cohort {
            rules: RULES_VERSION,
            executor: "test-executor".into(),
            catalog: catalog_digest(&catalog).unwrap(),
        },
        stage: 2,
        builds: [build.clone(), build],
        shuffle_seed: 18,
        policy_seed: 29,
    };
    (catalog, package)
}

#[test]
fn frozen_tower_package_applies_asymmetric_inventory_without_privileged_capabilities() {
    let (catalog, mut package) = fixture();
    package.builds[0].equipment = vec![
        Equipment {
            relic: Relic::Heartwood,
            rank: 2,
        },
        Equipment {
            relic: Relic::Compass,
            rank: 1,
        },
        Equipment {
            relic: Relic::Satchel,
            rank: 1,
        },
    ];
    package.builds[0].potions = [1, 1];
    package.builds[1].equipment = vec![Equipment {
        relic: Relic::Seedling,
        rank: 1,
    }];
    let preset = package.preset(&catalog, "test-executor", printing).unwrap();
    assert_eq!(preset.seed, 18);
    assert_eq!(preset.house_rules.challenge_policy_seed, Some(29));
    assert_eq!(preset.house_rules.opening_for(0).cards, 8);
    assert_eq!(preset.house_rules.opening_for(0).free_mulligans, 2);
    assert_eq!(preset.house_rules.opening_for(1).cards, 7);
    assert_eq!(preset.seats[0].starting_life, Some(28));
    assert_eq!(preset.seats[1].starting_life, Some(20));
    assert_eq!(preset.house_rules.supplies[0].healing, 1);
    assert_eq!(preset.house_rules.supplies[0].mana, 1);
    assert!(preset.house_rules.supplies[1].seedling);
    assert!(matches!(preset.seats[0].controller, SeatController::Open));
    assert_eq!(
        preset.seats[1].controller,
        SeatController::Ai(AIProfile::TOWER_EXPERT)
    );
    assert!(preset.seats.iter().all(|seat| {
        seat.capabilities == baylee_core::preset::SeatCapabilities::default()
            && seat.starting_hand.is_none()
            && seat.team.is_none()
            && seat.sideboard.is_empty()
            && seat.deck.len() == 40
    }));
    let bytes = package.encode(&catalog).unwrap();
    let decoded = Package::decode(&bytes, &catalog, "test-executor").unwrap();
    assert!(decoded == package);
    assert_eq!(
        decoded.preset(&catalog, "test-executor", printing).unwrap(),
        preset
    );
}

#[test]
fn tower_package_refuses_incompatible_material_and_unbounded_or_forged_builds() {
    let (catalog, package) = fixture();
    assert!(package.preset(&catalog, "another-build", printing).is_err());
    let mut changed = catalog.clone();
    changed.cards[1].price += 1;
    assert!(package.preset(&changed, "test-executor", printing).is_err());
    for mutate in [
        |p: &mut Package| p.schema = 2,
        |p: &mut Package| p.stage = 0,
        |p: &mut Package| p.stage = MAX_STAGES + 1,
        |p: &mut Package| p.cohort.rules += 1,
        |p: &mut Package| p.builds[1].potions = [2, 1],
        |p: &mut Package| p.builds[1].deck[1].copies = 5,
        |p: &mut Package| p.builds[1].deck[1].card = CardIndex::new(99),
    ] {
        let mut invalid = package.clone();
        mutate(&mut invalid);
        assert!(invalid.preset(&catalog, "test-executor", printing).is_err());
    }
    assert!(
        Package::decode(
            &vec![b' '; MAX_PACKAGE_BYTES + 1],
            &catalog,
            "test-executor"
        )
        .is_err()
    );
    let mut unknown: serde_json::Value = serde_json::to_value(&package).unwrap();
    unknown["capabilities"] = serde_json::json!({ "see_hidden": true });
    assert!(
        Package::decode(
            &serde_json::to_vec(&unknown).unwrap(),
            &catalog,
            "test-executor"
        )
        .is_err()
    );
}

#[test]
fn catalog_fingerprint_ignores_order_but_binds_prices_and_card_identity() {
    let (catalog, _) = fixture();
    let mut reordered = catalog.clone();
    reordered.cards.reverse();
    assert_eq!(catalog_digest(&catalog), catalog_digest(&reordered));
    reordered.cards[0].card = CardIndex::new(7);
    assert_ne!(catalog_digest(&catalog), catalog_digest(&reordered));
}

#[test]
fn frozen_build_requires_owned_cards_and_a_shop_before_reservation() {
    let (catalog, package) = fixture();
    let starter = Collection {
        formation: None,
        owned: vec![Stack {
            card: CardIndex::new(1),
            copies: 4,
        }],
        deck: package.builds[0].deck.clone(),
    };
    let mut run = Run::begin(&catalog, starter, &[5; 32]).unwrap();
    assert!(Build::freeze(&run, &catalog).unwrap() == package.builds[0]);
    run.phase = Phase::Queued;
    assert!(Build::freeze(&run, &catalog).is_err());
    run.phase = Phase::Shop;
    run.cards.owned[0].copies = 3;
    assert!(Build::freeze(&run, &catalog).is_err());
}
