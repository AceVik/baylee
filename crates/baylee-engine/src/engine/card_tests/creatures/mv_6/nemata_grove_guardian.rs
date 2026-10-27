//! `cards/creatures/mv_6/nemata_grove_guardian.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;
use baylee_core::generated::subtypes;

/// Nemata, Grove Guardian — {4}{G}{G} Legendary Creature — Treefolk 4/5
/// "{2}{G}: Create a 1/1 green Saproling creature token.
/// Sacrifice a Saproling: Saproling creatures get +1/+1 until end of turn."
#[test]
fn nemata_grove_guardian_creates_saprolings_and_sacrifices_one_to_pump_all_saprolings() {
    let nemata = card_index("5ce58279-7f1e-4ef7-a352-ecbaee31b6ff");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                nemata,
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let nemata_id = on_battlefield(&engine, p0, nemata).expect("Nemata on board");
    tap_all_mana(&mut engine, p0);

    // Activate ability 0 twice to create two 1/1 Saprolings.
    for _ in 0..2 {
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: nemata_id,
                    ability_index: 0,
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
    }

    let saprolings: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.characteristics()
                    .subtypes
                    .contains(subtypes::creature::SAPROLING)
            })
        })
        .collect();
    assert_eq!(saprolings.len(), 2);
    assert_eq!(power_toughness(&engine, saprolings[0]), (1, 1));
    assert_eq!(power_toughness(&engine, saprolings[1]), (1, 1));

    // Activate ability 1 sacrificing the first Saproling.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: nemata_id,
                ability_index: 1,
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![saprolings[0]],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(power_toughness(&engine, saprolings[1]), (2, 2));
    assert_eq!(power_toughness(&engine, nemata_id), (4, 5));
}
