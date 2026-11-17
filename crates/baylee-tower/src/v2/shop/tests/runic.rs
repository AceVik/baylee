use super::*;
use crate::v2::generation::{EXPEDITIONS, RUNIC};
use std::collections::BTreeSet;

#[test]
fn expanded_shelf_reaches_every_family_and_preserves_frozen_generations() {
    let (catalog, cards) = crate::cards::run_tests::fixture();
    let economy = Economy::for_run();
    let stock = Stock {
        catalog: &catalog,
        cards: &cards,
        economy: &economy,
    };
    let mut seen = BTreeSet::new();
    for byte in 0..=255 {
        let shop = Shop::open_for(RUNIC, &[byte; 32], 1, stock, None).unwrap();
        assert_eq!(shop.offers.len(), 12);
        assert_eq!(
            shop,
            Shop::open_for(RUNIC, &[byte; 32], 1, stock, None).unwrap()
        );
        for offer in &shop.offers[..10] {
            let Goods::Relic(definition) = offer.goods else {
                panic!("relic shelf")
            };
            assert_eq!(definition.rarity, Rarity::Common);
            seen.insert(definition.id());
        }
        assert!(
            shop.offers[10..]
                .iter()
                .all(|offer| matches!(offer.goods, Goods::Potion(_)))
        );
    }
    assert_eq!(seen.len(), 178, "each distinct family is reachable");
    let old = Shop::open_for(EXPEDITIONS, &[3; 32], 1, stock, None).unwrap();
    assert_eq!(old.offers.len(), 8);
    assert!(old.offers.iter().all(|offer| !matches!(
        offer.goods,
        Goods::Relic(Definition {
            family: Family::Runic(_),
            ..
        })
    )));
}

#[test]
fn exact_tier_affinity_fuses_equipment_with_bag_and_freeze_keeps_other_slots() {
    let (catalog, cards) = crate::cards::run_tests::fixture();
    let definition = Definition {
        family: Family::ALL[177],
        rarity: Rarity::Uncommon,
    };
    let economy = Economy {
        gold: 200,
        ..Economy::for_run()
    }
    .bought(definition, Slot::Inventory)
    .unwrap()
    .bought(definition, Slot::Active(4))
    .unwrap();
    let stock = Stock {
        catalog: &catalog,
        cards: &cards,
        economy: &economy,
    };
    let first = Shop::open_for(RUNIC, &[4; 32], 1, stock, None).unwrap();
    let shop = Shop::open_for(RUNIC, &[4; 32], 2, stock, Some(&first)).unwrap();
    assert_eq!(shop.offers[0].goods, Goods::Relic(definition));
    let moved = economy.moved(2, Slot::Active(0)).unwrap();
    let reordered = Shop::open_for(
        RUNIC,
        &[4; 32],
        2,
        Stock {
            economy: &moved,
            ..stock
        },
        Some(&first),
    )
    .unwrap();
    assert_eq!(
        shop, reordered,
        "equipment placement does not reroll affinity"
    );
    let frozen = shop.frozen(true, &catalog).unwrap();
    let bought = frozen
        .buy(shop.offers[0].id, &Delivery::Inventory, stock)
        .unwrap();
    let fused = &bought.economy.inventory.relics[0];
    assert_eq!(bought.economy.inventory.relics.len(), 1);
    assert_eq!(fused.definition.rarity, Rarity::Rare);
    assert_eq!(fused.slot, Slot::Active(4));
    assert_eq!(fused.basis, 15);
    assert_eq!(bought.economy.gold, 185);
    let next = Shop::open_for(
        RUNIC,
        &[4; 32],
        3,
        Stock {
            economy: &bought.economy,
            ..stock
        },
        Some(&bought.shop),
    )
    .unwrap();
    assert_eq!(next.offers[1..], shop.offers[1..]);
    assert_ne!(next.offers[0].id, shop.offers[0].id);
    assert!(!next.offers[0].sold);
    let mut forged = bought.economy.clone();
    forged.inventory.relics[0].basis = 19;
    assert!(forged.validate().is_err());
}

#[test]
fn material_choice_uses_last_two_slots_and_refusals_do_not_spend() {
    let (catalog, cards) = crate::cards::run_tests::fixture();
    let economy = Economy::for_run();
    let stock = Stock {
        catalog: &catalog,
        cards: &cards,
        economy: &economy,
    };
    let mut shop = Shop::open_for(RUNIC, &[8; 32], 1, stock, None).unwrap();
    for stage in 2..=7 {
        shop = Shop::open_for(RUNIC, &[8; 32], stage, stock, Some(&shop)).unwrap();
    }
    for slot in [0, 6, 7, 9, 12, 255] {
        assert!(shop.chose_mythic(slot, true, &catalog).is_err());
    }
    for slot in [10, 11] {
        let chosen = shop.chose_mythic(slot, false, &catalog).unwrap();
        assert_eq!(chosen.offers[usize::from(slot)].goods, Goods::Mythic);
        assert!(!chosen.mythic_choice);
        // The chosen crafting offer is one a funded run can actually buy.
        let rich = Economy {
            gold: 99,
            ..Economy::for_run()
        };
        let bought = chosen
            .buy(
                chosen.offers[usize::from(slot)].id,
                &Delivery::Inventory,
                Stock {
                    economy: &rich,
                    ..stock
                },
            )
            .unwrap();
        assert!(bought.shop.offers[usize::from(slot)].sold);
    }
    let poor = Economy {
        gold: 0,
        ..Economy::for_run()
    };
    assert_eq!(
        shop.buy(
            shop.offers[0].id,
            &Delivery::Inventory,
            Stock {
                economy: &poor,
                ..stock
            }
        ),
        Err(Refusal::Gold)
    );
    assert!(!shop.offers[0].sold);
}
