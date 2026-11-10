//! `cards/creatures/mv_5/elvish_bard.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn elvish_bard() -> CardIndex {
    card_index("094b778f-95a0-436f-a9d0-20a46a674486")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Elvish Bard is cast and survives a turn cycle; then it attacks player 1, who owns a Grizzly
/// Bears and a Hill Giant (the Giant tapped first when `tap_giant`). Stops at the blocker question.
fn bard_attacks(tap_giant: bool) -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 5])
        .battlefield(1, &[grizzly_bears(), hill_giant()])
        .hand(0, &[elvish_bard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, elvish_bard());
    pass_until(&mut engine, stack_is_empty);
    let bard = on_battlefield(&engine, p0, elvish_bard()).expect("Elvish Bard enters");
    assert_eq!(pt(&engine, bard), (2, 4));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    if tap_giant {
        engine.state.set_tapped(giant, true);
    }
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bard, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    (engine, bard, bears, giant)
}

/// `Elvish Bard` ({3}{G}{G}, 2/4 Creature — Elf Shaman Bard) requires all
/// creatures able to block it to do so.
#[test]
fn elvish_bard_forces_all_able_creatures_to_block_it() {
    let p1 = PlayerId::new(1);
    let (mut engine, bard, bears, giant) = bard_attacks(false);

    // Declaring zero or only one blocker is illegal because both able creatures must block Elvish Bard.
    assert!(
        engine
            .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
            .is_err()
    );
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(bears, bard)],
                },
            )
            .is_err()
    );
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(giant, bard)],
                },
            )
            .is_err()
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, bard), (giant, bard)],
            },
        )
        .unwrap();
}

/// "All creatures *able* to block": a tapped creature cannot block, so it is neither forced nor
/// allowed to, and the untapped one alone satisfies the requirement.
#[test]
fn elvish_bard_does_not_force_a_creature_that_cannot_block() {
    let p1 = PlayerId::new(1);
    let (mut engine, bard, bears, giant) = bard_attacks(true);

    assert!(
        engine
            .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
            .is_err(),
        "the untapped Bears is still forced to block"
    );
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(giant, bard)],
                },
            )
            .is_err(),
        "a tapped creature cannot block"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, bard)],
            },
        )
        .unwrap();
}
