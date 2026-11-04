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
fn gorilla_berserkers_requires_three_blockers_and_applies_rampage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let board = [grizzly_bears(), hill_giant(), gray_ogre()];
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gorilla_berserkers()])
        .battlefield(1, &board)
        .start();

    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ape = on_battlefield(&engine, p0, gorilla_berserkers()).unwrap();
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

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
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { bounds, .. } = engine.pending() else {
        unreachable!("pass_until stopped at the blockers question")
    };
    let bound = bounds
        .iter()
        .find(|bound| bound.attacker == ape)
        .expect("the question states Gorilla Berserkers' minimum");
    assert_eq!((bound.min_blockers, bound.max_blockers), (3, u32::MAX));

    for blockers in [vec![(bears, ape)], vec![(bears, ape), (giant, ape)]] {
        assert!(
            engine
                .apply(p1, PlayerAction::DeclareBlockers { blockers })
                .is_err(),
            "one or two blockers cannot satisfy the three-blocker minimum"
        );
    }
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, ape), (giant, ape), (ogre, ape)],
            },
        )
        .expect("three blockers satisfy the restriction");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, ape),
        (6, 7),
        "two blockers beyond the first give rampage 2 a +4/+4 bonus"
    );
}

/// The Berserkers attack player 1, who owns the creatures in `board` and
/// blocks with those in `blocking`. Returns once the block trigger resolved.
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

/// Rampage counts each blocker beyond the first, trample stays present, and
/// declining to block neither violates the minimum nor triggers rampage.
#[test]
fn gorilla_berserkers_rampage_counts_only_blockers_beyond_the_first() {
    let three = [grizzly_bears(), hill_giant(), gray_ogre()];
    let (engine, ape) = berserkers_blocked_by(&three, &three);
    assert_eq!(pt(&engine, ape), (6, 7), "two beyond the first, +2/+2 each");

    let four = [grizzly_bears(), hill_giant(), gray_ogre(), quiet_creature()];
    let (engine, ape) = berserkers_blocked_by(&four, &four);
    assert_eq!(pt(&engine, ape), (8, 9), "three beyond the first");
    assert!(keywords(&engine, ape).contains(KeywordSet::TRAMPLE));

    let (engine, ape) = berserkers_blocked_by(&three, &[]);
    assert_eq!(pt(&engine, ape), (2, 3), "unblocked is not blocked");
}

#[test]
fn gorilla_berserkers_cannot_be_blocked_when_fewer_than_three_could_block() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[gorilla_berserkers()])
        .battlefield(1, &[grizzly_bears(), hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ape = on_battlefield(&engine, p0, gorilla_berserkers()).unwrap();

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
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending() else {
        unreachable!("pass_until stopped at the blockers question")
    };
    assert!(
        blockers
            .iter()
            .all(|option| !option.attackers.contains(&ape)),
        "no offered pair may promise a block that two creatures cannot make"
    );

    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("zero blockers remains legal");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        18,
        "the 2/3 trampler is unblocked"
    );
}
