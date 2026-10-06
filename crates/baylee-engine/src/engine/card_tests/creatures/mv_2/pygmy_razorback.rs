//! `cards/creatures/mv_2/pygmy_razorback.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Pygmy Razorback prints `{1}{G}` for a 2/1 creature of type Boar with
/// Trample and nothing else. Trample is the only line that does something on
/// the battlefield, and the only kind of proof that a keyword cannot read
/// out of the card text is combat: the Razorback attacks, is blocked by a
/// 1/1, and the excess of 1 goes to the defender, while the blocker's life
/// points die. A test that only read `CHARACTERISTICS` would be green for a
/// card without the ability.
#[test]
fn pygmy_razorback_tramples_one_damage_over_the_blocker_into_the_defender() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[pygmy_razorback()])
        .battlefield(1, &[llanowar_elves()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{G} from two Forests; the Elves are not on the battlefield, so
    // the Razorback is the only creature under p0 and can attack alone.
    cast_from_hand(&mut engine, p0, pygmy_razorback());
    pass_until(&mut engine, stack_is_empty);
    let boar = on_battlefield(&engine, p0, pygmy_razorback()).expect("the Boar resolved");
    assert_eq!(pt(&engine, boar), (2, 1), "ein 2/1, wie gedruckt");
    assert!(
        keywords(&engine, boar).contains(KeywordSet::TRAMPLE),
        "the card prints Trample"
    );

    // Its own next turn: the Boar arrived this turn and may not yet attack
    // (CR 302.6) — the first attack window is its own and empty, and the
    // question at issue here is only asked in the next one.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseAttackers { attackers, .. } if attackers.contains(&boar)
        )
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(boar, Defender::Player(p1))],
            },
        )
        .unwrap();

    // The Elf on the table blocks, and that is the only way to make Trample
    // visible: without the ability all damage would be stuck on the
    // blocker and p1 would lose nothing.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their blocker is out");
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(their_elf, boar)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "1 Schaden an einem 1/1 ist tödlich (CR 704.5g)"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "the attacker takes no damage"
    );
    assert_eq!(
        engine.state().players[1].life,
        19,
        "Trample: der Überschuss von 1 geht auf den Verteidiger (CR 702.19b)"
    );
}
