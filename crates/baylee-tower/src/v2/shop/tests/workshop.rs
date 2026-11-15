#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn expanded_stock_explores_every_family_and_owned_affinity_ignores_slot_position() {
    for generation in [2, 3] {
        check_workshop(generation);
    }
}

fn check_workshop(generation: u16) {
    let (catalog, cards, economy) = fixture();
    let definition = Definition {
        family: if generation == 2 {
            Family::BloodAmber
        } else {
            Family::RootReservoir
        },
        rarity: Rarity::Common,
    };
    let active = economy.bought(definition, Slot::Active(0)).unwrap();
    let reserve = active.moved(1, Slot::Reserve(5)).unwrap();
    let stock = Stock {
        catalog: &catalog,
        cards: &cards,
        economy: &economy,
    };
    let mut empty_matches = 0;
    let mut owned_matches = 0;
    let mut seen = Vec::new();
    for seed in 0..128 {
        let empty = Shop::open_for(generation, &[seed; 32], 1, stock, None).unwrap();
        let owned = Shop::open_for(
            generation,
            &[seed; 32],
            1,
            Stock {
                economy: &active,
                ..stock
            },
            None,
        )
        .unwrap();
        let moved = Shop::open_for(
            generation,
            &[seed; 32],
            1,
            Stock {
                economy: &reserve,
                ..stock
            },
            None,
        )
        .unwrap();
        assert_eq!(owned, moved, "equipping cannot resample affinity");
        assert!(
            empty.offers[..2]
                .iter()
                .all(|offer| matches!(offer.goods, Goods::Card(_)))
        );
        for offer in &empty.offers[2..6] {
            let Goods::Relic(item) = offer.goods else {
                panic!("four relic slots");
            };
            empty_matches += usize::from(item.family == definition.family);
            if !seen.contains(&item.family) {
                seen.push(item.family);
            }
        }
        owned_matches += owned.offers[2..6]
            .iter()
            .filter(|offer| offer.goods == Goods::Relic(definition))
            .count();
        let mut changed = owned;
        changed.generator = 1;
        if changed
            .offers
            .iter()
            .any(|offer| matches!(offer.goods, Goods::Relic(item) if item.id() > 16))
        {
            assert!(
                changed.validate(&catalog).is_err(),
                "new items cannot enter a saved old shop"
            );
        }
    }
    assert_eq!(
        seen.len(),
        crate::v2::generation::families(generation).unwrap().len()
    );
    assert!(
        owned_matches > empty_matches * 2,
        "affinity must materially help finding copies"
    );
    assert!(
        owned_matches < 512 / 2,
        "most stock must still permit exploration"
    );
}
