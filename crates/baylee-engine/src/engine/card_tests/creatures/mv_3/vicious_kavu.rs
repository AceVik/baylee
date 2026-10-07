//! `cards/creatures/mv_3/vicious_kavu.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vicious Kavu — {1}{B}{R}, a printed 2/2 — "Whenever this creature
/// attacks, it gets +2/+0 until end of turn."
///
/// The body is read at four moments because no single one of them tells the
/// card from its neighbours: a 2/2 the turn it lands rules out anything that
/// fires on arrival or stands permanently, a 2/2 *while its attack trigger
/// waits on the stack* rules out a static reading "while attacking", a 4/2
/// once that trigger resolves is the printed +2/+0 and not a toughness pump,
/// and a 2/2 on the following turn is the "until end of turn". The opponent's
/// life total keeps the projection honest, since a body nothing ever swings
/// with would satisfy every other reading here.
#[test]
fn vicious_kavu_pumps_itself_for_the_turn_it_attacks_and_no_longer() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), mountain(), mountain()])
        .hand(0, &[vicious_kavu()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own main phase"
    );

    // {1}{B}{R} out of a Swamp and two Mountains, so the Kavu arrives the way
    // the card arrives rather than being seated by the harness.
    cast_from_hand(&mut engine, p0, vicious_kavu());
    pass_until(&mut engine, stack_is_empty);
    let kavu = on_battlefield(&engine, p0, vicious_kavu()).expect("the Kavu resolved");
    assert_eq!(
        pt(&engine, kavu),
        (2, 2),
        "the printed body: nothing has happened for the trigger to have done"
    );

    // Summoning sickness (CR 302.6) puts the attack a turn away, so the walk
    // goes across p1's turn and back to p0's own main phase.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, kavu),
        "the untap step gave it back, so it may be declared below"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on the attack declaration");
    };
    assert!(
        attackers.contains(&kavu),
        "an untapped 2/2 past summoning sickness may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(kavu, Defender::Player(p1))],
            },
        )
        .unwrap();

    // CR 603.2: attacking *triggers*, so the pump goes on the stack. A static
    // reading "as long as this creature is attacking" would already be a 4/2
    // in this very read.
    assert!(
        !stack_is_empty(&engine),
        "the attack trigger is waiting on the stack"
    );
    assert_eq!(
        pt(&engine, kavu),
        (2, 2),
        "and it changes nothing until it resolves"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, kavu),
        (4, 2),
        "+2/+0 — the bonus the card prints, and no toughness with it"
    );

    // Past the combat damage step (CR 510.2), where the extra power stops
    // being a projection: an unblocked 4/2 takes four, not two.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending) && e.state().turn.active == p0
    });
    assert_eq!(
        engine.state().players[1].life,
        16,
        "the pumped Kavu dealt four combat damage to the defending player"
    );

    // CR 514.2: the cleanup step is where "until end of turn" ends, so the
    // next turn's reading is the printed body again.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, kavu),
        (2, 2),
        "\"until end of turn\" expired with the turn it attacked in"
    );
    assert_eq!(
        engine.state().players[1].life,
        16,
        "and the damage it dealt is not undone with the effect"
    );
}
