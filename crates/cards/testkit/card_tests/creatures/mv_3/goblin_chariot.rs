//! `cards/creatures/mv_3/goblin_chariot.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goblin Chariot is `{2}{R}` for a 2/2 Goblin Warrior with haste, and haste
/// is the whole card: a creature that came under its controller's control this
/// turn may not attack (CR 302.6), so the Chariot appearing in the attack
/// declaration of the very turn it was cast is the only reading that says the
/// keyword reached the battlefield rather than the card file. Three Mountains
/// are exactly the printed `{2}{R}`, and the two damage p1 loses are dealt in
/// the same turn the creature arrived — which no printed 2/2 without the
/// keyword could do.
#[test]
fn goblin_chariot_attacks_the_turn_it_arrives_because_it_has_haste() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain()])
        .hand(0, &[goblin_chariot()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The three Mountains are the whole cost, so nothing on this board is a
    // creature that could have attacked on an earlier turn.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the Chariot is paid for"
    );
    cast_from_hand(&mut engine, p0, goblin_chariot());
    pass_until(&mut engine, stack_is_empty);

    let chariot = on_battlefield(&engine, p0, goblin_chariot()).expect("the Chariot resolved");
    assert_eq!(pt(&engine, chariot), (2, 2), "the body the card prints");
    assert!(
        keywords(&engine, chariot).contains(KeywordSet::HASTE),
        "haste is the keyword the card prints"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration");
    };
    assert!(
        attackers.contains(&chariot),
        "\"This creature can attack and {{T}} as soon as it comes under your \
         control\": a 2/2 that entered this turn is offered anyway: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(chariot, Defender::Player(p1))],
            },
        )
        .unwrap();

    // Not `stack_is_empty`: the stack is empty the moment attackers are
    // declared, so that predicate stops the walk *before* the combat damage
    // step (CR 510.2) and both life totals still read 20.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });

    assert_eq!(
        engine.state().players[1].life,
        18,
        "the 2/2 dealt its two combat damage in the turn it was cast"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nothing attacked back"
    );
    assert_eq!(
        on_battlefield(&engine, p0, goblin_chariot()),
        Some(chariot),
        "the Chariot is still on the battlefield after it attacked"
    );
}
