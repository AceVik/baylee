//! `cards/artifacts/equipment/mv_2/demonspine_whip.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn demonspine_whip() -> CardIndex {
    card_index("a588f047-02b4-4c6b-bce2-6e4d564296ec")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Demonspine Whip (a588f047-02b4-4c6b-bce2-6e4d564296ec) — {B}{R} Artifact — Equipment:
/// "{X}: Equipped creature gets +X/+0 until end of turn."
/// "Equip {1}"
///
/// Equips for {1}, activates {X} for X = 3 to give +3/+0 until end of turn
/// (leaving unequipped creatures untouched), wears off at end of turn, and
/// activating while unequipped buffs no creature.
#[test]
fn demonspine_whip_equips_and_pumps_equipped_creature_by_x_until_eot() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                demonspine_whip(),
                grizzly_bears(),
                hill_giant(),
                swamp(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let whip = on_battlefield(&engine, p0, demonspine_whip()).unwrap();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p0, hill_giant()).unwrap();

    // Activate {X} with X = 1 while unequipped: buffs neither creature.
    let sw = on_battlefield(&engine, p0, swamp()).unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: sw })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: whip,
                ability_index: 1,
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (2, 2));
    assert_eq!(pt(&engine, giant), (3, 3));

    // Equip {1} to Grizzly Bears (4 Mountains -> 3 left), then activate {X} with X = 3 -> 5/2!
    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: whip,
                ability_index: 0,
            },
        )
        .expect("equip {1}");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: whip,
                ability_index: 1,
            },
        )
        .expect("activate {X}");
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(pt(&engine, bears), (5, 2), "2/2 + 3/+0 = 5/2");
    assert_eq!(
        pt(&engine, giant),
        (3, 3),
        "unequipped Hill Giant untouched"
    );

    // Next turn: +3/+0 wore off at end of turn.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, bears), (2, 2), "+X/+0 wore off at end of turn");
}
