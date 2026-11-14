use super::*;
use crate::v2::catalog::{self, Family, Rarity};

fn funded() -> Economy {
    Economy {
        gold: 200,
        ..Economy::default()
    }
}

fn definition(family: Family, rarity: Rarity) -> Definition {
    Definition { family, rarity }
}

fn at(economy: &Economy, slot: Slot) -> u32 {
    economy
        .inventory
        .relics
        .iter()
        .find(|item| item.slot == slot)
        .unwrap()
        .id
}

#[test]
fn nine_common_path_preserves_basis_and_uses_exactly_one_rare_plus_potion() {
    for family in Family::ALL {
        let mut economy = funded();
        for output in 0..3 {
            for index in 3..6 {
                economy = economy
                    .bought(definition(family, Rarity::Common), Slot::Reserve(index))
                    .unwrap();
            }
            let recipe = Recipe {
                relics: (3..6)
                    .map(|index| at(&economy, Slot::Reserve(index)))
                    .collect(),
                mythic: None,
                destination: Slot::Reserve(output),
            };
            economy = economy.fused(&recipe).unwrap();
        }
        let rare = Recipe {
            relics: (0..3)
                .map(|index| at(&economy, Slot::Reserve(index)))
                .collect(),
            mythic: None,
            destination: Slot::Active(0),
        };
        economy = economy.fused(&rare).unwrap().bought_material().unwrap();
        assert_eq!(economy.gold, 140);
        let recipe = Recipe {
            relics: vec![at(&economy, Slot::Active(0))],
            mythic: Some(economy.inventory.materials[0].id),
            destination: Slot::Active(0),
        };
        let mythic = economy.fused(&recipe).unwrap();
        let item = &mythic.inventory.relics[0];
        assert_eq!(item.definition, definition(family, Rarity::MythicRare));
        assert_eq!(item.basis, 60);
        assert!(mythic.inventory.materials.is_empty());
        assert!(
            mythic.fused(&recipe).is_err(),
            "consumed IDs cannot be spent twice"
        );
        assert_eq!(mythic.sold(item.id).unwrap().gold, 165);
    }
}

#[test]
fn direct_rare_recipe_and_atomic_buy_fuse_conserve_actual_cost() {
    let rare = definition(Family::Heartwood, Rarity::Rare);
    let economy = funded().bought_material().unwrap();
    let recipe = Recipe {
        relics: vec![],
        mythic: Some(economy.inventory.materials[0].id),
        destination: Slot::Active(0),
    };
    let next = economy.bought_and_fused(rare, &recipe).unwrap();
    assert_eq!(next.gold, 150);
    assert_eq!(next.inventory.relics[0].basis, 50);
    assert_eq!(
        next.inventory.relics[0].definition.rarity,
        Rarity::MythicRare
    );
    assert!(economy.inventory.relics.is_empty());
    assert_eq!(economy.gold, 176);
    assert_eq!(next.sold(next.inventory.relics[0].id).unwrap().gold, 175);
    assert!(
        next.bought(
            definition(Family::Grove, Rarity::MythicRare),
            Slot::Reserve(0)
        )
        .is_err()
    );
}

#[test]
fn a_full_bag_can_buy_and_fuse_but_cannot_overwrite_an_unconsumed_item() {
    let common = definition(Family::Heartwood, Rarity::Common);
    let mut economy = funded();
    for slot in (0..3).map(Slot::Active).chain((0..6).map(Slot::Reserve)) {
        economy = economy.bought(common, slot).unwrap();
    }
    let recipe = Recipe {
        relics: vec![
            at(&economy, Slot::Reserve(0)),
            at(&economy, Slot::Reserve(1)),
        ],
        mythic: None,
        destination: Slot::Reserve(0),
    };
    assert!(economy.bought(common, Slot::Reserve(0)).is_err());
    let fused = economy.bought_and_fused(common, &recipe).unwrap();
    assert_eq!(fused.inventory.relics.len(), 8);
    assert_eq!(fused.gold, economy.gold - 4);
    assert!(
        fused
            .inventory
            .relics
            .iter()
            .all(|item| !recipe.relics.contains(&item.id))
    );
    assert!(
        economy
            .bought_and_fused(
                common,
                &Recipe {
                    destination: Slot::Reserve(2),
                    ..recipe
                }
            )
            .is_err()
    );
    assert_eq!(economy.inventory.relics.len(), 9);
}

#[test]
fn wrong_recipes_and_identity_aliases_are_rejected() {
    for rarity in Rarity::ALL {
        for material in [false, true] {
            for count in 1..=3 {
                let mut economy = funded();
                for index in 0..count {
                    let id = economy.inventory.allocate().unwrap();
                    economy.inventory.relics.push(Relic {
                        id,
                        definition: definition(Family::Grove, rarity),
                        slot: Slot::Reserve(index),
                        basis: 0,
                    });
                }
                economy = economy.bought_material().unwrap();
                let recipe = Recipe {
                    relics: economy
                        .inventory
                        .relics
                        .iter()
                        .map(|item| item.id)
                        .collect(),
                    mythic: material.then_some(economy.inventory.materials[0].id),
                    destination: Slot::Reserve(0),
                };
                let legal = matches!(
                    (rarity, count, material),
                    (Rarity::Common | Rarity::Uncommon, 3, false) | (Rarity::Rare, 1, true)
                );
                assert_eq!(
                    economy.fused(&recipe).is_ok(),
                    legal,
                    "{recipe:?}, {rarity:?}"
                );
                let alias = Recipe {
                    relics: vec![1, 1, 1],
                    mythic: None,
                    ..recipe
                };
                assert!(economy.fused(&alias).is_err());
            }
        }
    }
    let mut economy = funded();
    for (index, family) in [Family::Grove, Family::Grove, Family::Heartwood]
        .into_iter()
        .enumerate()
    {
        economy = economy
            .bought(
                definition(family, Rarity::Common),
                Slot::Reserve(u8::try_from(index).unwrap()),
            )
            .unwrap();
    }
    assert!(
        economy
            .fused(&Recipe {
                relics: vec![1, 2, 3],
                mythic: None,
                destination: Slot::Reserve(0),
            })
            .is_err()
    );
}

#[test]
fn active_limits_are_checked_after_moves_and_fusion_without_clamping() {
    let satchel = definition(Family::Satchel, Rarity::Common);
    let economy = funded()
        .bought(satchel, Slot::Active(0))
        .unwrap()
        .bought(satchel, Slot::Active(1))
        .unwrap()
        .bought(satchel, Slot::Reserve(0))
        .unwrap();
    let reserve = at(&economy, Slot::Reserve(0));
    assert!(economy.moved(reserve, Slot::Active(2)).is_err());
    let swapped = economy.moved(reserve, Slot::Active(0)).unwrap();
    assert_eq!(at(&swapped, Slot::Active(0)), reserve);
    assert_eq!(
        catalog::attributes(&swapped.inventory.active())
            .unwrap()
            .maximum_hand,
        9
    );
    let fused = economy
        .fused(&Recipe {
            relics: economy
                .inventory
                .relics
                .iter()
                .map(|item| item.id)
                .collect(),
            mythic: None,
            destination: Slot::Active(0),
        })
        .unwrap();
    let stats = catalog::attributes(&fused.inventory.active()).unwrap();
    assert_eq!((stats.cards, stats.maximum_hand), (8, 8));
    assert!(economy.moved(reserve, Slot::Reserve(6)).is_err());
}

#[test]
fn forged_basis_overflow_and_free_grant_resale_cannot_create_gold() {
    let common = definition(Family::Heartwood, Rarity::Common);
    let economy = funded().bought(common, Slot::Active(0)).unwrap();
    let mut free = economy.clone();
    free.inventory.relics[0].basis = 0;
    assert_eq!(free.sold(1).unwrap().gold, free.gold);
    let mut forged = economy.clone();
    forged.inventory.relics[0].basis = 5;
    assert!(forged.sold(1).is_err());
    let mut exhausted = economy.clone();
    exhausted.inventory.next_id = u32::MAX;
    assert!(exhausted.bought(common, Slot::Reserve(0)).is_err());
    let mut full = economy.clone();
    full.gold = 200;
    assert!(full.sold(1).is_err());
    let materials = economy
        .bought_material()
        .unwrap()
        .bought_material()
        .unwrap();
    assert!(materials.bought_material().is_err());
    let mut alias = materials;
    alias.inventory.materials[0].id = 1;
    assert!(alias.validate().is_err());
    assert_eq!(economy.gold, 196);
    assert_eq!(economy.inventory.next_id, 2);
}

#[test]
fn all_native_definition_ids_round_trip_and_unknown_ids_are_refused() {
    let mut ids = Vec::new();
    for family in Family::ALL {
        for rarity in Rarity::ALL {
            let definition = definition(family, rarity);
            assert_eq!(Definition::from_id(definition.id()), Some(definition));
            ids.push(definition.id());
        }
    }
    assert_eq!(ids, (1..=16).collect::<Vec<_>>());
    for id in [0, 17, 256, u16::MAX] {
        assert!(Definition::from_id(id).is_none());
    }
}

#[test]
fn table_charges_are_bought_and_settled_separately_from_crafting_materials() {
    let economy = Economy::default()
        .bought_potion(Potion::Healing)
        .unwrap()
        .bought_potion(Potion::Mana)
        .unwrap();
    assert_eq!(economy.gold, 10);
    assert_eq!(economy.potions, [1, 1]);
    assert!(economy.bought_potion(Potion::Healing).is_err());
    assert!(economy.settled([2, 0]).is_err());
    let next = economy.settled([1, 0]).unwrap();
    assert_eq!(next.potions, [0, 1]);
    assert_eq!(next.gold, 22);
    assert!(next.inventory.materials.is_empty());
    assert_eq!(economy.potions, [1, 1]);
}
