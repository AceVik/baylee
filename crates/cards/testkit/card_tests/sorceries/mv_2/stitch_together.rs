//! `cards/sorceries/mv_2/stitch_together.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn stitch_together_returns_creature_to_hand_below_threshold_and_to_battlefield_at_threshold() {
    let stitch = card_index("bc3d5911-3580-4132-9daf-2826495b5739");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
                gray_ogre(),
            ],
        )
        .hand(0, &[stitch, stitch])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let ogre = on_battlefield(&engine, p0, gray_ogre()).unwrap();
    bury(&mut engine, &[elves, ogre]);
    seed_graveyard(&mut engine, p0, 4); // 2 creatures + 4 lands = 6 cards in graveyard

    let gy_elves = in_graveyard(&engine, p0, llanowar_elves()).unwrap();
    let gy_ogre = in_graveyard(&engine, p0, gray_ogre()).unwrap();

    // 1. With 6 cards in graveyard (below threshold), Stitch Together returns Llanowar Elves to hand.
    let swamps = all_on_battlefield(&engine, p0, swamp());
    for &s in &swamps[..2] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: s })
            .unwrap();
    }
    let first_stitch = in_hand(&engine, p0, stitch).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: first_stitch })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![gy_elves],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_none());

    // 2. Seed 1 more card so graveyard has 7 cards (threshold): second Stitch Together
    // returns Gray Ogre directly to the battlefield.
    seed_graveyard(&mut engine, p0, 1);
    for &s in &swamps[2..4] {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: s })
            .unwrap();
    }
    let second_stitch = in_hand(&engine, p0, stitch).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::CastSpell {
                card: second_stitch,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![gy_ogre],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, gray_ogre()).is_some());
}
