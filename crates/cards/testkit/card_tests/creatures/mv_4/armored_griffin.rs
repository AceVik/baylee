//! `cards/creatures/mv_4/armored_griffin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Armored Griffin is `{3}{W}` for a 2/3 Griffin whose entire text is
/// "Flying, vigilance", and both words have to be read off the permanent the
/// spell became rather than off the card file: they are projected
/// characteristics, and only a real cast puts them on a body.
///
/// Vigilance means something in exactly one place — the attack declaration,
/// where every ordinary creature taps itself (CR 702.20b) — so the scenario
/// crosses a turn boundary first, because a creature that entered this turn
/// may not attack at all (CR 302.6).
///
/// The Llanowar Elves beside it is the control on both halves: a filter that
/// had widened to "creatures you control" would hand it flying and vigilance
/// too, and the 2 damage to the opponent is what says the still-untapped
/// Griffin actually attacked instead of having been held back.
#[test]
fn armored_griffin_lands_as_a_flying_vigilant_two_three_and_attacks_untapped() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), llanowar_elves()],
        )
        .hand(0, &[armored_griffin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Read before the cast: `cast_from_hand` taps every mana source on the
    // board, and the Elf is one of them — that it prints a `{T}: Add {G}`
    // does not make it a flier.
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are out");
    let bare = keywords(&engine, elves);
    assert!(
        !bare.contains(KeywordSet::FLYING) && !bare.contains(KeywordSet::VIGILANCE),
        "the bystander is the control: neither keyword comes off \
         \"creatures you control\""
    );

    // {3}{W} off the four Plains, with the Elf's own green paying into the
    // same pool.
    cast_from_hand(&mut engine, p0, armored_griffin());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, armored_griffin()).is_some()
    });
    let griffin = on_battlefield(&engine, p0, armored_griffin()).expect("the Griffin resolved");

    assert_eq!(pt(&engine, griffin), (2, 3), "the body the card prints");
    let granted = keywords(&engine, griffin);
    assert!(granted.contains(KeywordSet::FLYING), "\"Flying\"");
    assert!(granted.contains(KeywordSet::VIGILANCE), "and \"vigilance\"");
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::FLYING),
        "the pair lands on the Griffin and on no other creature on the board"
    );

    // Summoning sickness (CR 302.6): the attack the vigilance clause is
    // about is a turn away.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, griffin),
        "and it is standing untapped again for its second turn"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&griffin),
        "a 2/3 that has been on the battlefield since last turn is offered: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(griffin, Defender::Player(p1))],
            },
        )
        .expect("the Griffin was one of the attackers the offer named");
    assert!(
        !is_tapped(&engine, griffin),
        "\"vigilance\" (CR 702.20b): attacking does not tap it"
    );

    // Not `stack_is_empty`: the stack is already empty the moment attackers
    // are declared, so that predicate would stop the walk before the combat
    // damage step and leave both life totals at 20. The end step is past
    // damage (CR 510.2).
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        18,
        "an unblocked 2/3 dealt its printed 2, so the untapped body above is \
         an attacker and not one that stayed home"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the defending seat"
    );
}
