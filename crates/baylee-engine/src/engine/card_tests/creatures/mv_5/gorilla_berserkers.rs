//! `cards/creatures/mv_5/gorilla_berserkers.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn gorilla_berserkers() -> CardIndex {
    card_index("6373c66d-edfc-44cb-af2a-2273e0ba53e0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

#[test]
fn gorilla_berserkers_rampage_two_pumps_per_blocker_beyond_first() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gorilla_berserkers()])
        .battlefield(1, &[grizzly_bears(), hill_giant()])
        .start();

    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ape = on_battlefield(&engine, p0, gorilla_berserkers()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ape, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, ape), (giant, ape)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, ape),
        (4, 5),
        "Gorilla Berserkers (2/3) with rampage 2 blocked by two creatures should become 4/5"
    );
}

/// The Berserkers (placed on the battlefield) attack player 1, who owns the creatures in `board`
/// and blocks with those in `blocking`. Returns the Berserkers once the block's triggers resolved.
fn berserkers_blocked_by(
    board: &[CardIndex],
    blocking: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gorilla_berserkers()])
        .battlefield(1, board)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ape = on_battlefield(&engine, p0, gorilla_berserkers()).unwrap();
    let blocking: Vec<_> = blocking
        .iter()
        .map(|card| (on_battlefield(&engine, p1, *card).unwrap(), ape))
        .collect();
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(ape, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: blocking })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    (engine, ape)
}

/// "Rampage 2: +2/+2 for each creature blocking it beyond the first": one blocker is none, three
/// are two. It is a trampler, and unblocked it gets nothing.
#[test]
fn gorilla_berserkers_rampage_counts_only_blockers_beyond_the_first() {
    let (engine, ape) = berserkers_blocked_by(&[grizzly_bears()], &[grizzly_bears()]);
    assert_eq!(pt(&engine, ape), (2, 3), "one blocker is no bonus");
    assert!(keywords(&engine, ape).contains(KeywordSet::TRAMPLE));

    let three = [grizzly_bears(), hill_giant(), gray_ogre()];
    let (engine, ape) = berserkers_blocked_by(&three, &three);
    assert_eq!(pt(&engine, ape), (6, 7), "two beyond the first, +2/+2 each");

    let (engine, ape) = berserkers_blocked_by(&[grizzly_bears()], &[]);
    assert_eq!(pt(&engine, ape), (2, 3), "unblocked is not blocked");
}
