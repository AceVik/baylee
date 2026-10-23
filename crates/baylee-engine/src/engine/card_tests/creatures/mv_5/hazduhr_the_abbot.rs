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

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// The shield is on a white creature of its own controller's, for the next X
/// damage only: the 3 damage of a Lightning Bolt leaves 1 on a 2/5 white
/// Veteran Bodyguard after X = 2. A creature of another colour, and a white
/// one the opponent controls, are not legal targets.
#[test]
fn hazduhr_the_abbot_prevents_next_x_damage_to_target_white_creature_you_control() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[lightning_bolt()])
        .battlefield(
            0,
            &[
                hazduhr_the_abbot(),
                veteran_bodyguard(),
                bears(),
                plains(),
                plains(),
                mountain(),
            ],
        )
        .battlefield(1, &[savannah_lions()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hazduhr = on_battlefield(&engine, p0, hazduhr_the_abbot()).unwrap();
    let bodyguard = on_battlefield(&engine, p0, veteran_bodyguard()).unwrap();
    let green = on_battlefield(&engine, p0, bears()).unwrap();
    let their_white = on_battlefield(&engine, PlayerId::new(1), savannah_lions()).unwrap();
    let bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    let plains_ids = all_on_battlefield(&engine, p0, plains());
    let mtn = on_battlefield(&engine, p0, mountain()).unwrap();

    // Tap 2 Plains for X=2 and activate Hazduhr.
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
    for illegal in [green, their_white] {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![illegal],
                        players: vec![],
                    },
                )
                .is_err(),
            "only a white creature you control can be the target"
        );
    }
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bodyguard],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Lightning Bolt (3 damage) at the Bodyguard: 2 of it is prevented.
    tap_mana_where(&mut engine, p0, |id| id == mtn);
    engine
        .apply(p0, PlayerAction::CastSpell { card: bolt })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bodyguard],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(bodyguard).unwrap().damage, 1);
}
