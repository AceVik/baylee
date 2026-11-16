use super::*;
use crate::echoes::domain::v2::{
    catalog::{Definition, Family, Rarity},
    inventory::Slot,
};

#[test]
fn confirmed_purchase_fusion_and_move_are_once_only_across_bag_and_equipment() {
    let id = Uuid::from_u128(1);
    let definition = Definition {
        family: Family::Grove,
        rarity: Rarity::Common,
    };
    let mut economy = Economy {
        gold: 100,
        ..Economy::for_run()
    };
    economy = economy.bought(definition, Slot::Active(2)).unwrap();
    economy = economy.bought(definition, Slot::Inventory).unwrap();
    let mut offers = vec![Offer {
        id: 1,
        goods: Goods::Relic(definition),
        sold: false,
        pinned: false,
    }];
    let mut trace = Trace::default();
    assert!(trace.observe(id, 1, &economy, &offers).is_none());
    economy = economy.bought_automatic(definition).unwrap();
    offers[0].sold = true;
    let change = trace.observe(id, 2, &economy, &offers).unwrap();
    assert_eq!(change.gold, -4);
    assert_eq!(change.purchase, Some(1));
    assert_eq!(change.flights.len(), 3);
    assert!(change.flights.iter().all(|flight| flight.fusion));
    let fused = economy.inventory.relics[0].id;
    assert_eq!(
        change.fused,
        Some((
            fused,
            Goods::Relic(Definition {
                rarity: Rarity::Uncommon,
                ..definition
            })
        ))
    );
    assert!(trace.observe(id, 2, &economy, &offers).is_none());
    assert!(trace.observe(id, 1, &economy, &offers).is_none());
    economy = economy.moved(fused, Slot::Inventory).unwrap();
    let change = trace.observe(id, 3, &economy, &offers).unwrap();
    assert_eq!(change.gold, 0);
    assert_eq!(
        change.flights,
        vec![Flight {
            goods: Goods::Relic(economy.inventory.relics[0].definition),
            from: Anchor::Item(fused),
            to: Anchor::Item(fused),
            fusion: false
        }]
    );
}

#[test]
fn reconnect_and_run_switch_never_replay_spending_or_income() {
    let mut trace = Trace::default();
    let mut economy = Economy::for_run();
    let id = Uuid::from_u128(1);
    assert!(trace.observe(id, 1, &economy, &[]).is_none());
    economy.gold += 9;
    assert_eq!(trace.observe(id, 2, &economy, &[]).unwrap().gold, 9);
    economy.gold -= 2;
    assert!(trace.observe(id, 4, &economy, &[]).is_none());
    assert!(
        trace
            .observe(Uuid::from_u128(2), 5, &economy, &[])
            .is_none()
    );
}
