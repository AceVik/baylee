use super::*;
use domain::v2::{catalog::Family, economy::Economy};

#[test]
fn shelf_affinity_counts_equipment_and_predicts_the_actual_cascade() {
    let definition = Definition {
        family: Family::Grove,
        rarity: Rarity::Common,
    };
    let mut economy = Economy {
        gold: 150,
        ..Economy::for_run()
    };
    for (rarity, slot) in [
        (Rarity::Common, Slot::Active(2)),
        (Rarity::Common, Slot::Inventory),
        (Rarity::Uncommon, Slot::Inventory),
        (Rarity::Uncommon, Slot::Active(0)),
    ] {
        economy = economy
            .bought(
                Definition {
                    rarity,
                    ..definition
                },
                slot,
            )
            .unwrap();
    }
    assert_eq!(
        affinity(&economy.inventory, definition),
        (2, Some(Rarity::Rare))
    );
    let fused = economy.bought_automatic(definition).unwrap();
    assert_eq!(fused.inventory.relics.len(), 1);
    assert_eq!(fused.inventory.relics[0].definition.rarity, Rarity::Rare);
    assert_eq!(fused.inventory.relics[0].slot, Slot::Active(0));
    assert_eq!(inventory(&economy.inventory, Order::Family).len(), 2);
    assert_eq!(page(37, 99), (2, 3));
    assert_eq!(page(0, 99), (0, 1));
    assert!(matches!(
        quick_move(&economy.inventory, 2),
        Some(Command::Move {
            slot: Slot::Active(1),
            ..
        })
    ));
    assert!(matches!(
        quick_move(&economy.inventory, 1),
        Some(Command::Move {
            slot: Slot::Inventory,
            ..
        })
    ));
}

#[test]
fn quick_equip_refuses_a_full_loadout_but_explicit_drop_swaps_only_two_items() {
    let mut economy = Economy {
        gold: 100,
        ..Economy::for_run()
    };
    for (slot, family) in Family::ALL.into_iter().take(7).enumerate() {
        economy = economy
            .bought(
                Definition {
                    family,
                    rarity: Rarity::Common,
                },
                if slot < 6 {
                    Slot::Active(u8::try_from(slot).unwrap())
                } else {
                    Slot::Inventory
                },
            )
            .unwrap();
    }
    assert!(quick_move(&economy.inventory, 7).is_none());
    assert!(economy.toggled(7).is_err());
    let swapped = economy.moved(7, Slot::Active(2)).unwrap();
    assert_eq!(swapped.inventory.relics[2].slot, Slot::Inventory);
    assert_eq!(swapped.inventory.relics[6].slot, Slot::Active(2));
    assert_eq!(swapped.gold, economy.gold);
    assert_eq!(economy.inventory.relics[2].slot, Slot::Active(2));
}
