//! `cards/creatures/mv_4/flowstone_charger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flowstone Charger — a 2/5 Beast for {2}{R}{W} whose whole text is
/// "Whenever this creature attacks, it gets +3/-3 until end of turn."
///
/// The card is cast and then attacks on its controller's next turn, because a
/// creature that has just arrived may not attack (CR 302.6), and the body is
/// read three times on one board: the printed 2/5 while it stands there, the
/// 5/2 once the attack trigger has resolved, and the printed 2/5 again a turn
/// later. The pairing is the claim — a `+3/+3` would read 5/8 and a `-3/-3`
/// 2/2 — and the five points an unblocked 5/2 takes is what says the three
/// power was real at the damage step and not merely on a projection.
#[test]
fn flowstone_charger_pumps_itself_when_it_attacks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), plains(), plains()])
        .hand(0, &[flowstone_charger()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, flowstone_charger());
    pass_until(&mut engine, stack_is_empty);
    let charger = on_battlefield(&engine, p0, flowstone_charger()).expect("the Charger resolved");
    assert_eq!(
        pt(&engine, charger),
        (2, 5),
        "the body the card prints: the pump belongs to the attack and to nothing else"
    );

    // It arrived this turn, so it may not attack yet (CR 302.6). A turn of
    // standing there is the control for the reading below, since a static
    // bonus would have moved the body already.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, charger),
        (2, 5),
        "a turn later it is still the printed 2/5"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing else")
    };
    assert_eq!(player, p0, "the active seat is the one that declares");
    assert!(
        attackers.contains(&charger),
        "untapped and past summoning sickness, so the offer names it: {attackers:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(charger, Defender::Player(p1))],
            },
        )
        .expect("the attacker came out of the list that offered it");

    // The trigger resolves before blockers are declared (CR 603.3b), so the
    // stack is empty again by the time the body is read.
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, charger),
        (5, 2),
        "\"it gets +3/-3\": three power onto a 2/5 and three toughness off it"
    );

    // The pump was real where it counts: an unblocked 5/2 takes five points,
    // where the printed 2/5 would have taken two.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().players[1].life,
        15,
        "five combat damage from the creature the trigger pumped"
    );

    // "until end of turn": a turn later the base body has to be back on a
    // creature that is still standing, so what expired was the pump and not
    // the creature.
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, flowstone_charger()).is_some(),
        "a 5/2 that attacked unblocked survived, so this is read on the creature"
    );
    assert_eq!(
        pt(&engine, charger),
        (2, 5),
        "the pump lasted the turn it was made in and no longer"
    );
}
