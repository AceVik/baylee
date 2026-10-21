//! Retribution — `{2}{R}{R}` Sorcery (Homelands)
//!
//! Choose two target creatures controlled by the same opponent. That player
//! chooses and sacrifices one of those creatures. Put a -1/-1 counter on the
//! other.
//!
//! (`Coverage::Partial`: same-opponent two-target constraint and opponent
//! choosing one of the two targets to sacrifice while the other gets a -1/-1
//! counter are not in `Effect`; puts a -1/-1 counter on two target creatures
//! an opponent controls.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn retribution() -> CardIndex {
    card_index("2c7b0034-7eb5-4950-b6dc-8e1d07d26899")
}

fn bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

#[test]
fn retribution_puts_m1m1_counter_on_two_target_opponent_creatures() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[retribution()])
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[bears(), hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spell = in_hand(&engine, p0, retribution()).unwrap();
    let bear = on_battlefield(&engine, p1, bears()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bear, giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (1, 1));
    assert_eq!(pt(&engine, giant), (2, 2));
}
