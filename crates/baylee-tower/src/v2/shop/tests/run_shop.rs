use super::*;
use crate::v2::generation::RUN;

fn fixture() -> (Catalog, Collection, Economy) {
    let (catalog, collection) = crate::cards::run_tests::fixture();
    (catalog, collection, Economy::for_run())
}

#[test]
fn freeze_persists_and_only_bought_slots_refill_across_visits() {
    let (catalog, cards, _) = fixture();
    let economy = Economy::for_run();
    let stock = Stock {
        catalog: &catalog,
        cards: &cards,
        economy: &economy,
    };
    let original = Shop::open_for(RUN, &[41; 32], 1, stock, None).unwrap();
    assert!(
        original
            .offers
            .iter()
            .all(|offer| !matches!(offer.goods, Goods::Card(_)))
    );
    let frozen = original.frozen(true, &catalog).unwrap();
    assert_eq!(frozen.reroll_price(), None);
    assert!(frozen.refreshed(&[41; 32], stock).is_err());
    assert!(frozen.pinned(frozen.offers[0].id, true, &catalog).is_err());
    let id = frozen.offers[0].id;
    let purchase = frozen.buy(id, &Delivery::Inventory, stock).unwrap();
    assert!(purchase.shop.frozen);
    assert_eq!(purchase.shop.reroll_price(), Some(2));
    let stock = Stock {
        economy: &purchase.economy,
        cards: &purchase.cards,
        ..stock
    };
    let next = Shop::open_for(RUN, &[41; 32], 2, stock, Some(&purchase.shop)).unwrap();
    assert!(next.frozen);
    assert!(!next.offers[0].sold);
    assert_ne!(next.offers[0].id, id);
    assert_eq!(next.offers[1..], original.offers[1..]);
    let encoded = serde_json::to_vec(&next).unwrap();
    let restored: Shop = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(restored, next);
    assert_eq!(restored.reroll_price(), None);
    let thawed = restored.frozen(false, &catalog).unwrap();
    let third = Shop::open_for(RUN, &[41; 32], 3, stock, Some(&thawed)).unwrap();
    assert!(
        third
            .offers
            .iter()
            .zip(&next.offers)
            .all(|(new, old)| new.id != old.id)
    );
}

#[test]
fn paid_refresh_refills_only_holes_and_stale_offer_cannot_be_rebought() {
    let (catalog, cards, _) = fixture();
    let economy = Economy::for_run();
    let stock = Stock {
        catalog: &catalog,
        cards: &cards,
        economy: &economy,
    };
    let shop = Shop::open_for(RUN, &[7; 32], 1, stock, None)
        .unwrap()
        .frozen(true, &catalog)
        .unwrap();
    let purchase = shop
        .buy(shop.offers[1].id, &Delivery::Inventory, stock)
        .unwrap();
    let stock = Stock {
        economy: &purchase.economy,
        cards: &purchase.cards,
        ..stock
    };
    assert!(
        purchase
            .shop
            .buy(shop.offers[1].id, &Delivery::Inventory, stock)
            .is_err()
    );
    let (rerolled, paid) = purchase.shop.refreshed(&[7; 32], stock).unwrap();
    assert_eq!(paid.gold, purchase.economy.gold - 2);
    for (index, offer) in rerolled.offers.iter().enumerate() {
        if index == 1 {
            assert_ne!(offer.id, shop.offers[index].id);
        } else {
            assert_eq!(*offer, shop.offers[index]);
        }
    }
    assert!(rerolled.frozen);
    assert_eq!(rerolled.reroll_price(), None);
}

#[test]
fn all_bought_frozen_shelf_refills_and_preserves_freeze_switch() {
    let (catalog, cards, _) = fixture();
    let mut economy = Economy {
        gold: 200,
        ..Economy::for_run()
    };
    let mut shop = Shop::open_for(
        RUN,
        &[17; 32],
        1,
        Stock {
            catalog: &catalog,
            cards: &cards,
            economy: &economy,
        },
        None,
    )
    .unwrap()
    .frozen(true, &catalog)
    .unwrap();
    let ids: Vec<_> = shop.offers.iter().map(|offer| offer.id).collect();
    for id in &ids {
        let purchase = shop
            .buy(
                *id,
                &Delivery::Inventory,
                Stock {
                    catalog: &catalog,
                    cards: &cards,
                    economy: &economy,
                },
            )
            .unwrap();
        shop = purchase.shop;
        economy = purchase.economy;
    }
    let next = Shop::open_for(
        RUN,
        &[17; 32],
        2,
        Stock {
            catalog: &catalog,
            cards: &cards,
            economy: &economy,
        },
        Some(&shop),
    )
    .unwrap();
    assert!(next.frozen);
    assert!(
        next.offers
            .iter()
            .all(|offer| !offer.sold && !ids.contains(&offer.id))
    );
}

#[test]
fn automatic_purchase_cannot_be_bypassed_by_explicit_slot_delivery() {
    let (catalog, cards, _) = fixture();
    let economy = Economy::for_run();
    let stock = Stock {
        catalog: &catalog,
        cards: &cards,
        economy: &economy,
    };
    let shop = Shop::open_for(RUN, &[2; 32], 1, stock, None).unwrap();
    let offer = &shop.offers[0];
    let Goods::Relic(definition) = offer.goods else {
        panic!("equipment slot")
    };
    let economy = economy
        .bought(definition, Slot::Active(5))
        .unwrap()
        .bought(definition, Slot::Inventory)
        .unwrap();
    let stock = Stock {
        economy: &economy,
        ..stock
    };
    assert!(
        shop.buy(offer.id, &Delivery::Slot(Slot::Inventory), stock)
            .is_err()
    );
    let purchase = shop.buy(offer.id, &Delivery::Inventory, stock).unwrap();
    assert_eq!(purchase.economy.inventory.relics.len(), 1);
    assert_eq!(
        purchase.economy.inventory.relics[0].definition.rarity,
        Rarity::Uncommon
    );
    assert_eq!(purchase.economy.inventory.relics[0].slot, Slot::Active(5));
    let mut legacy = economy.clone();
    legacy.inventory = crate::v2::inventory::Inventory::default();
    assert!(
        shop.buy(
            offer.id,
            &Delivery::Inventory,
            Stock {
                economy: &legacy,
                ..stock
            }
        )
        .is_err()
    );
}

#[test]
fn run_reroll_price_is_capped_and_wallet_failure_is_atomic() {
    let (catalog, cards, _) = fixture();
    let mut economy = Economy {
        gold: 200,
        ..Economy::for_run()
    };
    let mut shop = Shop::open_for(
        RUN,
        &[22; 32],
        1,
        Stock {
            catalog: &catalog,
            cards: &cards,
            economy: &economy,
        },
        None,
    )
    .unwrap();
    for price in [2, 3, 4, 5, 6, 6, 6] {
        assert_eq!(shop.reroll_price(), Some(price));
        let before = economy.gold;
        (shop, economy) = shop
            .refreshed(
                &[22; 32],
                Stock {
                    catalog: &catalog,
                    cards: &cards,
                    economy: &economy,
                },
            )
            .unwrap();
        assert_eq!(economy.gold, before - price);
    }
    economy.gold = 5;
    let before = shop.clone();
    assert!(
        shop.refreshed(
            &[22; 32],
            Stock {
                catalog: &catalog,
                cards: &cards,
                economy: &economy
            }
        )
        .is_err()
    );
    assert_eq!(shop, before);
    assert_eq!(economy.gold, 5);
}
