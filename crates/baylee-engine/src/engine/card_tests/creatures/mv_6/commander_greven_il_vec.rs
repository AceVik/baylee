//! `cards/creatures/mv_6/commander_greven_il_vec.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn commander_greven_il_vec() -> CardIndex {
    card_index("08fa1bee-cedf-4a29-9b02-cbdcc0926f8f")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn bounty_hunter() -> CardIndex {
    card_index("2fd1cfc5-fbcd-4d31-ba8b-759d8c41b435")
}

/// p0 casts Greven with six Swamps beside a Llanowar Elves; p1 has Grizzly
/// Bears. The game stops on Greven's "sacrifice a creature" question; the
/// options it offered are returned with Greven's, the Elves' and the Bears'
/// objects.
fn greven_enters() -> (Engine<RegistryLookup>, Vec<ObjectId>, ObjectId, ObjectId) {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[commander_greven_il_vec()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let elves = on_battlefield(&engine, P0, quiet_creature()).unwrap();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();

    cast_from_hand(&mut engine, P0, commander_greven_il_vec());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, P0);
    assert!(options.contains(&elves));
    assert!(
        !options.contains(&bears),
        "\"sacrifice a creature\" is a creature you control: {options:?}"
    );
    (engine, options, elves, bears)
}

/// Commander Greven il-Vec (08fa1bee-cedf-4a29-9b02-cbdcc0926f8f) — {3}{B}{B}{B} Legendary Creature — Phyrexian Human Warrior:
/// "Fear. When Commander Greven il-Vec enters, sacrifice a creature."
#[test]
fn commander_greven_il_vec_sacrifices_a_creature_you_control_when_entering() {
    let (mut engine, _, elves, bears) = greven_enters();

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, commander_greven_il_vec()).is_some());
    assert!(in_graveyard(&engine, P0, quiet_creature()).is_some());
    assert!(
        engine
            .state()
            .object(bears)
            .is_some_and(|o| o.zone == Zone::Battlefield),
        "the opponent's creature is untouched"
    );
}

#[test]
fn commander_greven_il_vec_may_sacrifice_itself() {
    let (mut engine, options, _, _) = greven_enters();
    let greven = on_battlefield(&engine, P0, commander_greven_il_vec()).unwrap();
    assert!(
        options.contains(&greven),
        "Greven is a creature you control too: {options:?}"
    );

    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![greven],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, commander_greven_il_vec()).is_some());
    assert!(on_battlefield(&engine, P0, quiet_creature()).is_some());
}

#[test]
fn commander_greven_il_vec_has_fear_only_artifact_and_black_creatures_may_block_it() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[commander_greven_il_vec()])
        .battlefield(1, &[grizzly_bears(), bounty_hunter()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let greven = on_battlefield(&engine, P0, commander_greven_il_vec()).unwrap();
    let bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let hunter = on_battlefield(&engine, P1, bounty_hunter()).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(greven, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(bears, greven)],
                },
            )
            .is_err(),
        "a green creature cannot block a creature with fear"
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(hunter, greven)],
            },
        )
        .expect("a black creature can");
}
