//! Retribution — `{2}{R}{R}` Sorcery (Homelands)
//!
//! Choose two target creatures controlled by the same opponent. That player
//! chooses and sacrifices one of those creatures. Put a -1/-1 counter on the
//! other.
//!
//! (`Coverage::Partial`: the same-opponent constraint and the sacrifice the
//! opponent chooses are not said; both targets get the -1/-1 counter, so
//! nothing is sacrificed.)

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
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), bears()],
        )
        .battlefield(1, &[bears(), hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spell = in_hand(&engine, p0, retribution()).unwrap();
    let bear = on_battlefield(&engine, p1, bears()).unwrap();
    let my_bear = on_battlefield(&engine, p0, bears()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    // "Two target creatures" an opponent controls: your own is no target,
    // and one target is not two.
    for illegal in [vec![bear, my_bear], vec![bear]] {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: illegal,
                        players: vec![],
                    },
                )
                .is_err(),
            "exactly two creatures the opponent controls"
        );
    }
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
    assert_eq!(
        pt(&engine, my_bear),
        (2, 2),
        "your own creature is untouched"
    );
}
