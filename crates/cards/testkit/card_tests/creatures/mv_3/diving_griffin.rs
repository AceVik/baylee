//! `cards/creatures/mv_3/diving_griffin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Diving Griffin is `{1}{W}{W}` for a 2/2 Griffin with flying and vigilance,
/// and the two printed keywords are proven the two different ways this rig
/// can: flying is a characteristic the layer system projects, so it is read
/// off `characteristics().keywords`, while vigilance is a *behaviour* —
/// CR 508.1f taps an attacker as it is declared and CR 702.20 is the
/// exception — so the Griffin is declared as an attacker and read afterwards,
/// still standing and still having dealt its two damage. The three Plains are
/// the whole payment and no other permanent on the board makes mana or has a
/// keyword, so every number here belongs to the card and nothing else.
#[test]
fn diving_griffin_attacks_and_stays_untapped() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(4711, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[diving_griffin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Three Plains pay exactly `{1}{W}{W}`, and `cast_from_hand` taps them
    // before the cast, so the spell comes off mana that is really floating.
    cast_from_hand(&mut engine, p0, diving_griffin());
    pass_until(&mut engine, stack_is_empty);
    let griffin = on_battlefield(&engine, p0, diving_griffin()).expect("the Griffin resolved");
    assert_eq!(pt(&engine, griffin), (2, 2), "the body the card prints");
    assert!(
        types(&engine, griffin).contains(TypeSet::CREATURE),
        "and it is the creature it prints"
    );
    let granted = keywords(&engine, griffin);
    assert!(
        granted.contains(KeywordSet::FLYING),
        "Flying, read through the layers: {granted:?}"
    );
    assert!(
        granted.contains(KeywordSet::VIGILANCE),
        "and vigilance, which the combat step below puts to work: {granted:?}"
    );
    assert!(
        !is_tapped(&engine, griffin),
        "it entered untapped, so there is a `{{T}}` for the combat step to spend"
    );

    // CR 302.6: a creature that arrived this turn cannot attack, so the turn
    // goes round once first. Both halves are needed — `walk_to_own_main` on
    // its own returns where it stands, because this already *is* p0's own
    // main phase.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the turn came back round, and the summoning sickness with it"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on the attack declaration")
    };
    assert!(
        attackers.contains(&griffin),
        "an untapped 2/2 with no summoning sickness may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(griffin, Defender::Player(p1))],
            },
        )
        .unwrap();

    // CR 508.1f taps an attacker as it is declared, and vigilance is the
    // printed exception — read here, the moment the declaration has landed,
    // because that is the only reading that tells it from a creature that
    // simply was never tapped.
    assert!(
        !is_tapped(&engine, griffin),
        "vigilance: the Griffin attacks and stays standing"
    );

    // Not `stack_is_empty`: the stack is empty the moment attackers are
    // declared, so that would stop *before* the combat damage step. The end
    // step is past damage (CR 510.2).
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        engine.state().players[1].life,
        18,
        "two combat damage off the Griffin's printed power"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nothing came back for it: the damage is one-way"
    );
    assert!(
        !is_tapped(&engine, griffin),
        "still untapped at the end step, which is what vigilance bought — a \
         creature without it would have been tapped by its own declaration"
    );
    assert!(
        on_battlefield(&engine, p0, diving_griffin()).is_some(),
        "and the Griffin is still on the battlefield it attacked from"
    );
}
