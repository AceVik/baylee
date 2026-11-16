use super::*;
use crate::v2::{catalog::Family, inventory::Inventory};

fn funded() -> Economy {
    Economy {
        gold: 200,
        ..Economy::for_run()
    }
}

fn definition(rarity: Rarity) -> Definition {
    Definition {
        family: Family::Heartwood,
        rarity,
    }
}

#[test]
fn legacy_encoding_is_unchanged_and_cannot_gain_run_slots() {
    let json = r#"{"next_id":1,"relics":[],"materials":[]}"#;
    let inventory: Inventory = serde_json::from_str(json).unwrap();
    assert_eq!(inventory, Inventory::default());
    assert_eq!(serde_json::to_string(&inventory).unwrap(), json);
    let economy = Economy::default();
    for slot in [Slot::Active(3), Slot::Inventory] {
        assert!(economy.bought(definition(Rarity::Common), slot).is_err());
    }
    assert!(
        economy
            .bought_automatic(definition(Rarity::Common))
            .is_err()
    );
    let json = serde_json::to_string(&funded()).unwrap();
    assert_eq!(serde_json::from_str::<Economy>(&json).unwrap(), funded());
}

#[test]
fn fusion_prefers_first_equipped_slot_and_preserves_unrelated_items() {
    let common = definition(Rarity::Common);
    let other = Definition {
        family: Family::Grove,
        ..common
    };
    let economy = funded()
        .bought(common, Slot::Inventory)
        .unwrap()
        .bought(common, Slot::Active(5))
        .unwrap()
        .bought(common, Slot::Active(1))
        .unwrap()
        .bought(other, Slot::Active(0))
        .unwrap();
    let next = economy.bought_automatic(common).unwrap();
    assert_eq!(next.gold, 180);
    assert_eq!(next.inventory.relics.len(), 3);
    assert_eq!(next.inventory.relics[0], economy.inventory.relics[0]);
    assert_eq!(next.inventory.relics[1], economy.inventory.relics[3]);
    let fused = &next.inventory.relics[2];
    assert_eq!(fused.definition, definition(Rarity::Uncommon));
    assert_eq!(fused.slot, Slot::Active(1));
    assert_eq!(fused.basis, 12);
}

#[test]
fn ninth_common_cascades_to_rare_but_never_spends_a_mythic_potion() {
    let common = definition(Rarity::Common);
    let mut economy = funded();
    for _ in 0..9 {
        economy = economy.bought_automatic(common).unwrap();
    }
    assert_eq!(economy.gold, 164);
    assert_eq!(economy.inventory.relics.len(), 1);
    let rare = economy.inventory.relics[0].clone();
    assert_eq!(rare.definition, definition(Rarity::Rare));
    assert_eq!(rare.basis, 36);
    assert_eq!(rare.slot, Slot::Inventory);
    economy = economy.bought_material().unwrap();
    economy = economy.bought_automatic(rare.definition).unwrap();
    economy = economy.bought_automatic(rare.definition).unwrap();
    assert_eq!(economy.inventory.relics.len(), 3);
    assert!(
        economy
            .inventory
            .relics
            .iter()
            .all(|item| item.definition == rare.definition)
    );
    assert_eq!(economy.inventory.materials.len(), 1);
    assert_eq!(economy.inventory.relics[0], rare);
}

#[test]
fn run_storage_has_no_slot_ceiling_and_equipment_swaps_are_exact() {
    let common = definition(Rarity::Common);
    let mut economy = funded();
    for _ in 0..30 {
        economy = economy.bought(common, Slot::Inventory).unwrap();
    }
    for id in 1..=6 {
        economy = economy.toggled(id).unwrap();
        assert_eq!(
            economy.inventory.relics[id as usize - 1].slot,
            Slot::Active((id - 1) as u8)
        );
    }
    assert_eq!(economy.toggled(7), Err(Refusal::Capacity));
    assert!(economy.moved(7, Slot::Active(6)).is_err());
    assert!(economy.moved(7, Slot::Reserve(0)).is_err());
    economy = economy.moved(7, Slot::Active(2)).unwrap();
    assert_eq!(economy.inventory.relics[2].slot, Slot::Inventory);
    assert_eq!(economy.inventory.relics[6].slot, Slot::Active(2));
    economy = economy.toggled(1).unwrap();
    assert_eq!(economy.inventory.relics[0].slot, Slot::Inventory);
    assert_eq!(economy.inventory.relics[2].slot, Slot::Inventory);
    economy = economy.toggled(30).unwrap();
    assert_eq!(economy.inventory.relics[29].slot, Slot::Active(0));
    assert_eq!(economy.inventory.relics.len(), 30);
    economy.validate().unwrap();
}

#[test]
fn failed_auto_purchase_rolls_back_gold_identity_and_ingredients() {
    let common = definition(Rarity::Common);
    let economy = funded()
        .bought_automatic(common)
        .unwrap()
        .bought_automatic(common)
        .unwrap();
    let mut exhausted = economy.clone();
    // Purchase can allocate, but the fusion cannot. Both remain uncommitted.
    exhausted.inventory.next_id = u32::MAX - 1;
    assert_eq!(exhausted.bought_automatic(common), Err(Refusal::Capacity));
    assert_eq!(exhausted.gold, economy.gold);
    assert_eq!(exhausted.inventory.relics, economy.inventory.relics);
    let mut poor = economy.clone();
    poor.gold = 3;
    assert_eq!(poor.bought_automatic(common), Err(Refusal::Gold));
    assert_eq!(poor.inventory, economy.inventory);
}

#[test]
fn active_overlap_and_cross_kind_identity_aliases_are_rejected() {
    let common = definition(Rarity::Common);
    let economy = funded()
        .bought(common, Slot::Active(5))
        .unwrap()
        .bought(common, Slot::Inventory)
        .unwrap()
        .bought_material()
        .unwrap();
    let mut overlap = economy.clone();
    overlap.inventory.relics[1].slot = Slot::Active(5);
    assert!(overlap.validate().is_err());
    let mut alias = economy;
    alias.inventory.materials[0].id = alias.inventory.relics[0].id;
    assert!(alias.validate().is_err());
}

#[test]
fn purchase_and_equip_fuses_across_storage_and_keeps_the_first_equipped_slot() {
    let common = definition(Rarity::Common);
    let first = funded().bought_automatic_into(common, Some(4)).unwrap();
    assert_eq!(first.inventory.relics[0].slot, Slot::Active(4));
    let second = first.bought_automatic(common).unwrap();
    let fused = second.bought_automatic_into(common, Some(0)).unwrap();
    assert_eq!(fused.inventory.relics.len(), 1);
    assert_eq!(
        fused.inventory.relics[0].definition.rarity,
        Rarity::Uncommon
    );
    assert_eq!(fused.inventory.relics[0].slot, Slot::Active(4));
    assert_eq!(fused.gold, 188);
    let other = Definition {
        family: Family::Grove,
        ..common
    };
    assert_eq!(
        fused.bought_automatic_into(other, Some(4)),
        Err(Refusal::Capacity)
    );
    assert_eq!(
        fused.bought_automatic_into(other, Some(6)),
        Err(Refusal::Invalid)
    );
    assert_eq!(fused.gold, 188, "refusal never spends the confirmed wallet");
    let bag = funded()
        .bought_automatic(common)
        .unwrap()
        .bought_automatic(common)
        .unwrap();
    let fused = bag.bought_automatic_into(common, Some(2)).unwrap();
    assert_eq!(fused.inventory.relics.len(), 1);
    assert_eq!(fused.inventory.relics[0].slot, Slot::Active(2));
}
