//! Hazduhr the Abbot — `{3}{W}{W}` 2/5 Legendary Creature — Human Cleric (Homelands)
//!
//! `{X}`, `{T}`: The next X damage that would be dealt this turn to target
//! white creature you control is dealt to Hazduhr instead.
//!
//! (`Coverage::Partial`: redirecting the prevented X damage to Hazduhr the
//! Abbot itself is not in `Effect`; `{X}`, `{T}` prevents the next X damage
//! that would be dealt this turn to target white creature you control.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn hazduhr_the_abbot() -> CardIndex {
    card_index("35844d8b-8f68-4248-b921-e766757a9e26")
}

#[test]
fn hazduhr_the_abbot_prevents_next_x_damage_to_target_white_creature_you_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[lightning_bolt()])
        .battlefield(0, &[hazduhr_the_abbot(), plains(), plains(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hazduhr = on_battlefield(&engine, p0, hazduhr_the_abbot()).unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    let plains_ids = all_on_battlefield(&engine, p0, plains());
    let mtn = on_battlefield(&engine, p0, mountain()).unwrap();

    // Tap 2 Plains for X=2 and activate Hazduhr targeting the white creature (Hazduhr itself).
    tap_mana_where(&mut engine, p0, |id| plains_ids.contains(&id));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: hazduhr,
                ability_index: 0,
            },
        )
        .unwrap();
    engine.apply(p0, PlayerAction::ChooseNumber(2)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![hazduhr],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Cast Lightning Bolt (3 damage) at Hazduhr: 2 damage is prevented, so it takes 1 damage.
    tap_mana_where(&mut engine, p0, |id| id == mtn);
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![hazduhr],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(hazduhr).unwrap().damage, 1);
}
