//! `cards/creatures/mv_3/standing_troops.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Standing Troops is a {2}{W} 1/4 Human Soldier whose whole printed text is
/// "Vigilance", so the one thing a board can hold it to is what the keyword
/// does: attacking does not tap it (CR 702.20b).
///
/// A lone attacker proves nothing — "the Troops is still untapped" is
/// satisfied just as well by a combat step that taps nobody — so the seat
/// attacks with a Llanowar Elves beside it, a creature with no keyword to keep
/// it standing. One declaration, two answers: the Elves is tapped by attacking
/// (CR 508.1f) and the Troops is not, and both are read a second time after the
/// damage step has been and gone so that neither reading is taken mid-step.
#[test]
fn standing_troops_attacks_without_tapping_because_of_vigilance() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[standing_troops(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );

    let troops = on_battlefield(&engine, p0, standing_troops()).expect("the Troops are seated");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are seated");
    assert_eq!(pt(&engine, troops), (1, 4), "the body the card prints");
    assert!(
        keywords(&engine, troops).contains(KeywordSet::VIGILANCE),
        "the printed keyword reaches the permanent through the layers"
    );
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::VIGILANCE),
        "and it is the Troops' own word: nothing on this board grants it"
    );
    assert!(
        !is_tapped(&engine, troops) && !is_tapped(&engine, elves),
        "both stand untapped before anything is declared"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player,
        attackers,
        defenders,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert_eq!(player, p0, "the active seat declares its own attackers");
    assert!(
        attackers.contains(&troops) && attackers.contains(&elves),
        "two untapped, unsick creatures with no demand on them are offered: {attackers:?}"
    );
    assert!(
        defenders.contains(&Defender::Player(p1)),
        "and the opponent is what may be attacked: {defenders:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![
                    (troops, Defender::Player(p1)),
                    (elves, Defender::Player(p1)),
                ],
            },
        )
        .unwrap();

    assert!(
        !is_tapped(&engine, troops),
        "CR 702.20b: the declaration tapped everything it attacked with except \
         the creature that has vigilance"
    );
    assert!(
        is_tapped(&engine, elves),
        "and CR 508.1f is what tapped that one: the very same declaration"
    );

    // Past the damage step, so the attack really happened rather than being
    // rolled back, and the standing Troops is still standing.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        18,
        "two unblocked attackers dealt 1 + 1 combat damage (CR 510.2)"
    );
    assert!(
        !is_tapped(&engine, troops) && is_tapped(&engine, elves),
        "and the tap states outlive the combat they were set in"
    );
}
