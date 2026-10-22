//! Krovikan Horror — `{3}{B}` 2/2 Creature — Horror Spirit (Alliances)
//!
//! (`Coverage::Partial`: at the beginning of the end step, if it is in your
//! graveyard with at least one creature card above it, you may return it to
//! your hand, and `{1}`, sacrifice a creature deals 1 damage to any target;
//! requiring a creature card to be directly above it rather than anywhere above
//! it in your graveyard is not supported.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn krovikan_horror() -> CardIndex {
    card_index("a4c15223-784f-43a7-8626-30f96ccfdcaf")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

#[test]
fn krovikan_horror_sacrifices_creature_for_1_damage_and_returns_from_graveyard_at_end_step() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                krovikan_horror(),
                grizzly_bears(),
                hill_giant(),
                swamp(),
                swamp(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let horror = on_battlefield(&engine, p0, krovikan_horror()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();

    // 1. Activate {1}, Sacrifice a creature (sacrificing Krovikan Horror itself): deal 1 damage to p1.
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: horror,
                ability_index: 1,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![horror],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19);
    assert!(in_graveyard(&engine, p0, krovikan_horror()).is_some());

    // Put a creature card above Krovikan Horror in p0's graveyard by moving Grizzly Bears there.
    let _ = giant;
    engine
        .dev_state_mut(p0)
        .unwrap()
        .move_object(
            bears,
            ZoneLocation::Graveyard(p0),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();

    // At the beginning of the end step, accept the may-return-to-hand prompt.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, krovikan_horror()).is_some());
    assert!(in_graveyard(&engine, p0, krovikan_horror()).is_none());
}
