use super::*;

fn offer(relic: Relic, rank: u8) -> Merchandise {
    Merchandise::Relic { relic, rank }
}

#[test]
fn refused_purchases_never_spend_or_change_inventory() {
    let start = Economy::default();
    assert_eq!(start.bought(offer(Relic::Satchel, 1)), Err(Refusal::Gold));
    assert_eq!(
        start.bought(offer(Relic::Heartwood, 2)),
        Err(Refusal::Stale)
    );
    assert_eq!(
        start.bought(offer(Relic::Heartwood, 0)),
        Err(Refusal::Invalid)
    );
    assert_eq!(start, Economy::default());
    let once = start.bought(offer(Relic::Heartwood, 1)).unwrap();
    assert_eq!(once.gold, 6);
    assert_eq!(once.bought(offer(Relic::Heartwood, 1)), Err(Refusal::Stale));
    assert_eq!(
        once.equipment,
        [Equipment {
            relic: Relic::Heartwood,
            rank: 1
        }]
    );
}

#[test]
fn equipment_slots_ranks_and_opening_effects_are_independently_bounded() {
    let mut wallet = Economy {
        gold: MAX_GOLD,
        ..Economy::default()
    };
    for (kind, ranks) in [
        (Relic::Heartwood, 3),
        (Relic::Compass, 2),
        (Relic::Satchel, 1),
    ] {
        for rank in 1..=ranks {
            wallet = wallet.bought(offer(kind, rank)).unwrap();
        }
    }
    assert_eq!(
        items::attributes(&wallet.equipment).unwrap(),
        items::Attributes {
            life: 32,
            opening_cards: 8,
            maximum_hand: 8,
            free_mulligans: 3,
            seedlings: 0,
        }
    );
    assert_eq!(
        wallet.bought(offer(Relic::Seedling, 1)),
        Err(Refusal::Capacity)
    );
    assert_eq!(
        wallet.bought(offer(Relic::Satchel, 2)),
        Err(Refusal::Invalid)
    );
    wallet = wallet
        .sold(Relic::Satchel)
        .unwrap()
        .bought(offer(Relic::Seedling, 1))
        .unwrap();
    let attributes = items::attributes(&wallet.equipment).unwrap();
    assert_eq!(
        (
            attributes.opening_cards,
            attributes.maximum_hand,
            attributes.seedlings
        ),
        (7, 7, 1)
    );
}

#[test]
fn every_buy_sell_cycle_loses_gold_instead_of_farming_it() {
    for relic in Relic::ALL {
        let mut wallet = Economy {
            gold: MAX_GOLD,
            ..Economy::default()
        };
        for rank in 1..=relic.max_rank() {
            wallet = wallet.bought(offer(relic, rank)).unwrap();
            let sold = wallet.sold(relic).unwrap();
            assert!(sold.gold < MAX_GOLD);
            assert!(sold.equipment.is_empty());
        }
    }
}

#[test]
fn potion_slots_and_verified_consumption_cannot_underflow_or_partially_apply() {
    let wallet = Economy::default()
        .bought(Merchandise::Potion {
            potion: Potion::Healing,
        })
        .unwrap()
        .bought(Merchandise::Potion {
            potion: Potion::Mana,
        })
        .unwrap();
    assert_eq!(
        wallet.bought(Merchandise::Potion {
            potion: Potion::Healing
        }),
        Err(Refusal::Capacity)
    );
    assert_eq!(wallet.consumed([1, 2]), Err(Refusal::Invalid));
    assert_eq!(wallet.potions, [1, 1]);
    let used = wallet.consumed([0, 1]).unwrap();
    assert_eq!(used.potions, [1, 0]);
    assert_eq!(used.gold, wallet.gold);
    assert_eq!(used.consumed([0, 1]), Err(Refusal::Invalid));
}

#[test]
fn hostile_saved_state_is_rejected_before_economy_arithmetic() {
    let mut invalid = Economy {
        potions: [255, 255],
        ..Economy::default()
    };
    assert_eq!(invalid.consumed([0, 0]), Err(Refusal::Invalid));
    invalid.potions = [0, 0];
    invalid.gold = u16::MAX;
    assert_eq!(invalid.validate(), Err(Refusal::Invalid));
    invalid.gold = 0;
    invalid.equipment = vec![Equipment {
        relic: Relic::Heartwood,
        rank: 255,
    }];
    assert_eq!(invalid.validate(), Err(Refusal::Invalid));
    invalid.equipment[0].rank = 1;
    invalid.equipment.push(invalid.equipment[0]);
    assert_eq!(invalid.validate(), Err(Refusal::Invalid));
    assert!(
        serde_json::from_str::<Economy>(
            r#"{"gold":12,"equipment":[],"potions":[0,0],"admin":true}"#
        )
        .is_err()
    );
}

#[test]
fn a_relic_sells_for_half_of_everything_paid_for_it_rounded_down_once() {
    for (relic, rank, value) in [
        (Relic::Heartwood, 1, 3),
        (Relic::Heartwood, 2, 7),
        (Relic::Heartwood, 3, 13),
        (Relic::Compass, 1, 6),
        (Relic::Compass, 2, 14),
        (Relic::Satchel, 1, 7),
        (Relic::Seedling, 1, 4),
    ] {
        assert_eq!(relic.sale(rank), Ok(value), "{relic:?} at rank {rank}");
    }
    for relic in [
        Relic::Heartwood,
        Relic::Compass,
        Relic::Satchel,
        Relic::Seedling,
    ] {
        for rank in [0, relic.max_rank() + 1, u8::MAX] {
            assert_eq!(
                relic.sale(rank),
                Err(Refusal::Invalid),
                "{relic:?} at {rank}"
            );
        }
    }
}

#[test]
fn a_sale_pays_into_the_wallet_takes_the_relic_and_stops_at_the_wallets_bound() {
    let held = Economy {
        gold: 20,
        ..Economy::default()
    }
    .bought(offer(Relic::Heartwood, 1))
    .unwrap()
    .bought(offer(Relic::Compass, 1))
    .unwrap();
    assert_eq!(held.gold, 2);
    let sold = held.sold(Relic::Heartwood).unwrap();
    assert_eq!(sold.gold, 5);
    assert_eq!(
        sold.equipment,
        [Equipment {
            relic: Relic::Compass,
            rank: 1
        }]
    );
    assert_eq!(sold.sold(Relic::Heartwood), Err(Refusal::Unavailable));

    let full = |gold| Economy {
        gold,
        ..held.clone()
    };
    assert_eq!(
        full(MAX_GOLD - 3).sold(Relic::Heartwood).unwrap().gold,
        MAX_GOLD
    );
    assert_eq!(
        full(MAX_GOLD - 2).sold(Relic::Heartwood),
        Err(Refusal::Capacity)
    );
}
