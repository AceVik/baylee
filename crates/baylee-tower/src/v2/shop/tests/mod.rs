use super::*;
use crate::{
    cards::{Card, Stack},
    v2::{
        catalog::{Definition, Family, MYTHIC_PRICE, Rarity},
        inventory::Slot,
    },
};
use baylee_core::ids::CardIndex;

mod generations;
mod run_shop;
mod workshop;

fn fixture() -> (Catalog, Collection, Economy) {
    let mut cards = vec![Card {
        card: CardIndex::new(0),
        price: 0,
        basic_land: true,
        run: None,
    }];
    cards.extend((1..=8).map(|index| Card {
        card: CardIndex::new(index),
        price: 2,
        basic_land: false,
        run: None,
    }));
    (
        Catalog { cards },
        Collection {
            formation: None,
            owned: Vec::new(),
            deck: vec![Stack {
                card: CardIndex::new(0),
                copies: 40,
            }],
        },
        Economy::default(),
    )
}

#[test]
fn all_visits_are_reproducible_bounded_and_respect_rarity_unlocks() {
    let (catalog, cards, economy) = fixture();
    let mut reversed = catalog.clone();
    reversed.cards.reverse();
    for seed in 0..16 {
        let mut previous = None;
        for stage in 1..=MAX_STAGES {
            let stock = Stock {
                catalog: &catalog,
                cards: &cards,
                economy: &economy,
            };
            let shop = Shop::open(&[seed; 32], stage, stock, previous.as_ref()).unwrap();
            assert_eq!(
                shop,
                Shop::open(
                    &[seed; 32],
                    stage,
                    Stock {
                        catalog: &reversed,
                        ..stock
                    },
                    previous.as_ref()
                )
                .unwrap()
            );
            assert_eq!(shop.offers.len(), OFFERS);
            assert!(shop.offers.iter().all(|offer| offer.goods.admitted(stage)));
            assert!(
                shop.offers
                    .iter()
                    .any(|offer| !offer.pinned && offer.goods.usable(stock))
            );
            let restored: Shop =
                serde_json::from_slice(&serde_json::to_vec(&shop).unwrap()).unwrap();
            assert_eq!(restored, shop);
            restored.validate(&catalog).unwrap();
            previous = Some(shop);
        }
    }
}

#[test]
fn pins_consume_slots_refreshes_spend_exact_prices_and_old_offers_stay_stale() {
    let (catalog, cards, mut economy) = fixture();
    let mut shop = Shop::open(
        &[19; 32],
        1,
        Stock {
            catalog: &catalog,
            cards: &cards,
            economy: &economy,
        },
        None,
    )
    .unwrap();
    let ids = [shop.offers[0].id, shop.offers[1].id];
    shop = shop
        .pinned(ids[0], true, &catalog)
        .unwrap()
        .pinned(ids[1], true, &catalog)
        .unwrap();
    assert!(shop.pinned(shop.offers[2].id, true, &catalog).is_err());
    let retained = shop.offers[..2].to_vec();
    let replaced = shop.offers[2].id;
    for price in [2, 3, 4] {
        assert_eq!(shop.reroll_price(), Some(price));
        let before = economy.gold;
        (shop, economy) = shop
            .refreshed(
                &[19; 32],
                Stock {
                    catalog: &catalog,
                    cards: &cards,
                    economy: &economy,
                },
            )
            .unwrap();
        assert_eq!(economy.gold, before - price);
        assert_eq!(&shop.offers[..2], retained.as_slice());
        assert_eq!(shop.offers.len(), 8);
    }
    let final_stock = Stock {
        catalog: &catalog,
        cards: &cards,
        economy: &economy,
    };
    assert!(shop.refreshed(&[19; 32], final_stock).is_err());
    assert!(
        shop.buy(replaced, &Delivery::Inventory, final_stock)
            .is_err()
    );
    let bought = shop
        .buy(
            ids[0],
            &Delivery::Inventory,
            Stock {
                catalog: &catalog,
                cards: &cards,
                economy: &economy,
            },
        )
        .unwrap();
    assert!(bought.shop.offers[0].sold);
    assert!(!bought.shop.offers[0].pinned);
    assert!(
        bought
            .shop
            .buy(
                ids[0],
                &Delivery::Inventory,
                Stock {
                    catalog: &catalog,
                    cards: &bought.cards,
                    economy: &bought.economy
                }
            )
            .is_err()
    );
    let next = Shop::open(
        &[19; 32],
        2,
        Stock {
            catalog: &catalog,
            cards: &bought.cards,
            economy: &bought.economy,
        },
        Some(&bought.shop),
    )
    .unwrap();
    assert_eq!(next.offers[1], retained[1]);
    assert_ne!(next.offers[0].id, ids[0]);
}

#[test]
fn stage_seven_material_choice_requires_explicit_pin_replacement_and_is_once_only() {
    let (catalog, cards, mut economy) = fixture();
    economy.gold = 200;
    let stock = Stock {
        catalog: &catalog,
        cards: &cards,
        economy: &economy,
    };
    let mut shop = Shop::open(&[2; 32], 1, stock, None).unwrap();
    for stage in 2..=6 {
        shop = Shop::open(&[2; 32], stage, stock, Some(&shop)).unwrap();
    }
    shop = shop.pinned(shop.offers[6].id, true, &catalog).unwrap();
    shop = Shop::open(&[2; 32], 7, stock, Some(&shop)).unwrap();
    assert!(shop.mythic_choice);
    assert!(shop.chose_mythic(6, false, &catalog).is_err());
    assert!(shop.chose_mythic(0, true, &catalog).is_err());
    let chosen = shop.chose_mythic(6, true, &catalog).unwrap();
    assert!(chosen.offers[6].pinned);
    assert_ne!(chosen.offers[6].id, shop.offers[6].id);
    assert_eq!(
        chosen.offers[6].goods.price(&catalog).unwrap(),
        MYTHIC_PRICE
    );
    assert!(chosen.chose_mythic(7, true, &catalog).is_err());
    let bought = chosen
        .buy(chosen.offers[6].id, &Delivery::Inventory, stock)
        .unwrap();
    assert_eq!(bought.economy.gold, 176);
    assert_eq!(bought.economy.inventory.materials.len(), 1);
    assert!(!bought.shop.mythic_choice);
}

#[test]
fn affordability_includes_full_bag_fusion_and_zero_gold_terminates_without_invention() {
    let (catalog, mut cards, mut economy) = fixture();
    cards.owned = (1..=8)
        .map(|index| Stack {
            card: CardIndex::new(index),
            copies: 4,
        })
        .collect();
    economy.gold = 200;
    let definition = Definition {
        family: Family::Heartwood,
        rarity: Rarity::Common,
    };
    for slot in (0..3).map(Slot::Active).chain((0..6).map(Slot::Reserve)) {
        economy = economy.bought(definition, slot).unwrap();
    }
    economy.potions = [1, 1];
    economy.gold = 4;
    for seed in 0..32 {
        let stock = Stock {
            catalog: &catalog,
            cards: &cards,
            economy: &economy,
        };
        let shop = Shop::open(&[seed; 32], 1, stock, None).unwrap();
        let offer = shop
            .offers
            .iter()
            .find(|offer| offer.goods == Goods::Relic(definition))
            .unwrap();
        let bought = shop
            .buy(
                offer.id,
                &Delivery::Fuse(crate::v2::economy::Recipe {
                    relics: vec![1, 2],
                    mythic: None,
                    destination: Slot::Active(0),
                }),
                stock,
            )
            .unwrap();
        assert_eq!(bought.economy.gold, 0);
        assert_eq!(bought.economy.inventory.relics.len(), 8);
    }
    economy.gold = 0;
    let stock = Stock {
        catalog: &catalog,
        cards: &cards,
        economy: &economy,
    };
    let shop = Shop::open(&[0; 32], 1, stock, None).unwrap();
    assert!(!shop.offers.iter().any(|offer| offer.goods.usable(stock)));
    assert_eq!(shop.offers.len(), 8);
}

#[test]
fn an_affordable_fusion_may_need_reserve_because_equipping_it_would_exceed_caps() {
    let (catalog, mut cards, mut economy) = fixture();
    cards.owned = (1..=8)
        .map(|index| Stack {
            card: CardIndex::new(index),
            copies: 4,
        })
        .collect();
    let compass = Definition {
        family: Family::Compass,
        rarity: Rarity::Uncommon,
    };
    let definitions = [
        compass,
        Definition {
            family: Family::Compass,
            rarity: Rarity::MythicRare,
        },
        Definition {
            family: Family::Heartwood,
            rarity: Rarity::Common,
        },
        compass,
        Definition {
            family: Family::Grove,
            rarity: Rarity::Rare,
        },
        Definition {
            family: Family::Grove,
            rarity: Rarity::Rare,
        },
        Definition {
            family: Family::Grove,
            rarity: Rarity::Rare,
        },
        Definition {
            family: Family::Grove,
            rarity: Rarity::Rare,
        },
        Definition {
            family: Family::Grove,
            rarity: Rarity::Rare,
        },
    ];
    economy.inventory.relics = definitions
        .into_iter()
        .zip((0..3).map(Slot::Active).chain((0..6).map(Slot::Reserve)))
        .enumerate()
        .map(|(index, (definition, slot))| crate::v2::inventory::Relic {
            id: u32::try_from(index + 1).unwrap(),
            definition,
            slot,
            basis: 0,
        })
        .collect();
    economy.inventory.next_id = 10;
    economy.gold = 10;
    economy.potions = [1, 1];
    economy.validate().unwrap();
    let recipe = crate::v2::economy::Recipe {
        relics: vec![1, 4],
        mythic: None,
        destination: Slot::Active(0),
    };
    assert!(economy.bought_and_fused(compass, &recipe).is_err());
    assert!(
        economy
            .bought_and_fused(
                compass,
                &crate::v2::economy::Recipe {
                    destination: Slot::Reserve(0),
                    ..recipe
                }
            )
            .is_ok()
    );
    let stock = Stock {
        catalog: &catalog,
        cards: &cards,
        economy: &economy,
    };
    for seed in 0..16 {
        let first = Shop::open(&[seed; 32], 1, stock, None).unwrap();
        let next = Shop::open(&[seed; 32], 2, stock, Some(&first)).unwrap();
        assert!(
            next.offers
                .iter()
                .any(|offer| offer.goods == Goods::Relic(compass))
        );
    }
}
