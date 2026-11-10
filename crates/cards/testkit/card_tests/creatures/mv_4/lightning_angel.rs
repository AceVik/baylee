//! `cards/creatures/mv_4/lightning_angel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lightning Angel prints `{1}{U}{R}{W}` for a 3/4 Angel with flying,
/// vigilance and haste. The three keywords answer three different questions
/// about a creature that arrived this same turn: flying and vigilance are
/// characteristics the layers project, while haste is the one that makes the
/// attack declaration *legal* on the very turn summoning sickness would
/// otherwise forbid (CR 302.6). The cast is paid out of a real pool of four
/// lands, so the Angel is on the battlefield as the result of playing the
/// card and not of a placement, and the untapped attacker afterwards is
/// vigilance read off the board rather than off the card file.
#[test]
fn lightning_angel_arrives_with_flying_vigilance_and_haste_and_attacks_untapped() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), island(), mountain(), plains()])
        .hand(0, &[lightning_angel()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // `cast_from_hand` taps the board first: `{1}` off the second Plains and
    // the three colours off the Island and the Mountain, which is exactly the
    // printed cost.
    cast_from_hand(&mut engine, p0, lightning_angel());
    pass_until(&mut engine, stack_is_empty);
    let angel = on_battlefield(&engine, p0, lightning_angel()).expect("the Angel resolved");
    assert_eq!(pt(&engine, angel), (3, 4), "the printed 3/4 body");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{U}}{{R}}{{W}} spent every mana the four lands produced"
    );

    let kw = keywords(&engine, angel);
    assert!(kw.contains(KeywordSet::FLYING), "flying");
    assert!(kw.contains(KeywordSet::VIGILANCE), "vigilance");
    assert!(kw.contains(KeywordSet::HASTE), "haste");

    // Haste is the half that needs the turn to be read: the Angel entered
    // this turn, so CR 302.6 is what its presence in the offer has to beat.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&angel),
        "haste is what makes a creature that arrived this turn attackable: {attackers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(angel, Defender::Player(p1))],
            },
        )
        .expect("the Angel came out of the list that offered it");
    assert!(
        !is_tapped(&engine, angel),
        "vigilance: attacking does not tap the Angel"
    );
}
