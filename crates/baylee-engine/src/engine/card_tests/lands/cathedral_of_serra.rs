//! `cards/lands/cathedral_of_serra.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Jasmine Boreal: a green-white legendary 4/5 with no text.
fn jasmine_boreal() -> CardIndex {
    card_index("942407bf-862a-4a0b-b19d-b9f8ee833026")
}

/// Tobias Andrion: a white-blue legendary 4/4 with no text.
fn tobias_andrion() -> CardIndex {
    card_index("55dfe722-7487-4d25-b90c-e6ad2c82a671")
}

/// Barktooth Warbeard: a black-red legendary 6/5 with no text.
fn barktooth_warbeard() -> CardIndex {
    card_index("4a221518-c850-4e8a-a697-765c0a0e76e7")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// A duel on `p0`'s first main phase with `mine` out for p0 and a Hill Giant
/// for p1.
fn start(mine: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, mine)
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    engine
}

/// Walks to the attack declaration and attacks p1 with `attackers`.
#[track_caller]
fn attack(engine: &mut Engine<RegistryLookup>, attackers: &[ObjectId]) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: attackers
                    .iter()
                    .map(|a| (*a, Defender::Player(p1)))
                    .collect(),
            },
        )
        .expect("the attack is legal");
}

/// Cathedral of Serra (4b9922b9-c5b5-467e-8a5b-1e45e862194d) — "White legendary creatures you control have "bands with
/// other legendary creatures.""
///
/// The grant is by colour and by controller, and its reach is "other
/// legendary creatures": every pairing of the three legends is read in both
/// directions, a plain Grizzly Bears is never a mate, and the same legends
/// on the other side of the table are not given anything.
#[test]
fn cathedral_of_serra_gives_white_legends_you_control_bands_with_other_legends() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let legends = [jasmine_boreal(), tobias_andrion(), barktooth_warbeard()];
    // Which of the three (Jasmine GW, Tobias WU, Barktooth BR) are white.
    let granted: [usize; 2] = [0, 1];
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                card_index("4b9922b9-c5b5-467e-8a5b-1e45e862194d"),
                legends[0],
                legends[1],
                legends[2],
                grizzly_bears(),
            ],
        )
        .battlefield(1, &legends)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let mine: Vec<ObjectId> = legends
        .iter()
        .map(|c| on_battlefield(&engine, p0, *c).expect("my legend is out"))
        .collect();
    let theirs: Vec<ObjectId> = legends
        .iter()
        .map(|c| on_battlefield(&engine, p1, *c).expect("their legend is out"))
        .collect();
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears are out");

    for leader in 0..3 {
        for mate in 0..3 {
            if leader == mate {
                continue;
            }
            assert_eq!(
                crate::combat::can_band_with_other(engine.state(), mine[leader], mine[mate]),
                granted.contains(&leader),
                "legend {leader} bands with legend {mate} exactly when it is white"
            );
            assert!(
                !crate::combat::can_band_with_other(engine.state(), theirs[leader], theirs[mate]),
                "the opponent's legends {leader} and {mate} get nothing from my land"
            );
        }
        assert!(
            !crate::combat::can_band_with_other(engine.state(), mine[leader], bears),
            "a nonlegendary creature is never a mate (legend {leader})"
        );
        assert!(
            !crate::combat::can_band_with_other(engine.state(), bears, mine[leader]),
            "and the Bears have no ability to band with anyone"
        );
    }
}

/// The grant is a real banding ability in combat: the attack asks the
/// white legend which attackers band with it, the other legend joins,
/// and a block on one member of the band blocks the other too.
#[test]
fn cathedral_of_serra_lets_its_legend_attack_in_a_band_that_is_blocked_as_a_group() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (leader_card, mate_card) = (tobias_andrion(), barktooth_warbeard());
    let mut engine = start(&[
        card_index("4b9922b9-c5b5-467e-8a5b-1e45e862194d"),
        leader_card,
        mate_card,
    ]);
    let leader = on_battlefield(&engine, p0, leader_card).expect("the leader is out");
    let mate = on_battlefield(&engine, p0, mate_card).expect("the mate is out");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("the Giant is out");

    attack(&mut engine, &[leader, mate]);
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the band question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the attacking player announces bands");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::Band { with: leader },
        "asked of the legend that has the ability"
    );
    assert_eq!(options, vec![mate], "the other legend may join the band");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mate],
            },
        )
        .expect("one legend bands with the other");
    let combat = &engine.state().combat;
    assert!(combat.band_of(leader).is_some(), "the leader is in a band");
    assert_eq!(
        combat.band_of(leader),
        combat.band_of(mate),
        "with the other legend"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, mate)],
            },
        )
        .expect("the Giant blocks the legend that has no ability");
    assert!(
        engine.state().combat.is_blocked(leader),
        "bands are blocked as a group: blocking one member blocks the leader"
    );
}

/// The two boards the grant is absent on: without the land the same two
/// legends attack with no band question and a block on one leaves the other
/// unblocked; with the land, a nonlegendary creature is not offered as a
/// mate, so no band question stands either.
#[test]
fn cathedral_of_serra_asks_for_no_band_without_the_land_or_for_a_nonlegendary_mate() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (leader_card, mate_card) = (tobias_andrion(), barktooth_warbeard());

    let mut engine = start(&[leader_card, mate_card]);
    let leader = on_battlefield(&engine, p0, leader_card).expect("the leader is out");
    let mate = on_battlefield(&engine, p0, mate_card).expect("the mate is out");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("the Giant is out");
    attack(&mut engine, &[leader, mate]);
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "no land, no band question: {:?}",
        engine.pending()
    );
    assert_eq!(engine.state().combat.band_of(leader), None);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, mate)],
            },
        )
        .expect("the Giant blocks one legend");
    assert!(
        !engine.state().combat.is_blocked(leader),
        "unbanded, the other legend stays unblocked"
    );

    let mut engine = start(&[
        card_index("4b9922b9-c5b5-467e-8a5b-1e45e862194d"),
        leader_card,
        grizzly_bears(),
    ]);
    let leader = on_battlefield(&engine, p0, leader_card).expect("the leader is out");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("the Bears are out");
    attack(&mut engine, &[leader, bears]);
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "a nonlegendary creature is not a mate, so nothing is asked: {:?}",
        engine.pending()
    );
    assert_eq!(engine.state().combat.band_of(leader), None);
}
