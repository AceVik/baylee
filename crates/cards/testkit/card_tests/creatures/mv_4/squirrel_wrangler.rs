//! Squirrel Wrangler — {2}{G}{G} — Creature — Human Druid
//! Oracle: {1}{G}, Sacrifice a land: Create two 1/1 green Squirrel creature tokens.
//! Oracle: {1}{G}, Sacrifice a land: Squirrel creatures get +1/+1 until end of turn.

use super::*;
use baylee_core::generated::subtypes;

#[test]
fn squirrel_wrangler_sacrifices_lands_to_create_squirrels_and_pump_them() {
    let squirrel_wrangler = card_index("4021b6b4-9b87-4dd5-a5df-79540bedbe87");
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[squirrel_wrangler])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, squirrel_wrangler);
    pass_until(&mut engine, stack_is_empty);
    let wrangler_id =
        on_battlefield(&engine, p0, squirrel_wrangler).expect("Squirrel Wrangler on board");

    // Tap remaining 4 Forests for {G}{G}{G}{G} and activate ability 0 (create two Squirrels).
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wrangler_id,
                ability_index: 0,
            },
        )
        .expect("activate Squirrel Wrangler ability 0");
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .expect("sacrifice a Forest");
    }
    pass_until(&mut engine, stack_is_empty);

    let squirrels: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == p0
                    && o.characteristics()
                        .subtypes
                        .contains(subtypes::creature::SQUIRREL)
            })
        })
        .collect();
    assert_eq!(squirrels.len(), 2);
    for &sq in &squirrels {
        let ch = engine.state().object(sq).unwrap().characteristics();
        assert_eq!((ch.power, ch.toughness), (Some(1), Some(1)));
    }

    // Activate ability 1 (Squirrel creatures get +1/+1 until end of turn).
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: wrangler_id,
                ability_index: 1,
            },
        )
        .expect("activate Squirrel Wrangler ability 1");
    if let Pending::ChooseCards { options, .. } = engine.pending().clone() {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .expect("sacrifice another Forest");
    }
    pass_until(&mut engine, stack_is_empty);

    for &sq in &squirrels {
        let ch = engine.state().object(sq).unwrap().characteristics();
        assert_eq!((ch.power, ch.toughness), (Some(2), Some(2)));
    }
    let w_ch = engine
        .state()
        .object(wrangler_id)
        .unwrap()
        .characteristics();
    assert_eq!((w_ch.power, w_ch.toughness), (Some(2), Some(2)));
}
