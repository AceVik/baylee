//! `cards/creatures/mv_3/charging_paladin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Charging Paladin — {2}{W}, a 2/2 Human Knight with "Whenever this
/// creature attacks, it gets +0/+3 until end of turn."
///
/// The pump is only visible on the attack and must hit the Paladin and no
/// one else: the (2, 2) before the attack and the Llanowar Elves, which
/// remains (1, 1) under the same control, are the two sides of the same
/// claim — the +0/+3 come from the attack trigger and not from a static
/// ability on the whole battlefield. The final step reads "until end of
/// turn" a second time, where an expired pump showed 2/2 again.
#[test]
fn charging_paladin_pumps_itself_when_it_attacks_and_no_other_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), quiet_creature()])
        .hand(0, &[charging_paladin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {2}{W} off the three Plains, with the Elf named as the printing kept
    // back: it is the control the pump is read against, and a source tapped
    // for mana could not attack or block afterwards.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Plains, and the Elf kept standing"
    );
    cast_with_floating(&mut engine, p0, charging_paladin());
    pass_until(&mut engine, stack_is_empty);

    let paladin = on_battlefield(&engine, p0, charging_paladin()).expect("the Paladin resolved");
    let elf = on_battlefield(&engine, p0, quiet_creature()).expect("the Elf is still out");
    assert_eq!(
        pt(&engine, paladin),
        (2, 2),
        "a printed 2/2 before it attacks"
    );
    assert_eq!(pt(&engine, elf), (1, 1), "and nothing beside it is pumped");

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
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&paladin),
        "an untapped 2/2 may attack: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(paladin, Defender::Player(p1))],
            },
        )
        .unwrap();

    // The attack trigger is on the stack the moment attackers are declared,
    // and the pump is what is waiting there.
    pass_until(&mut engine, |e| !stack_is_empty(e));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, paladin),
        (2, 5),
        "+0/+3 on the creature that attacked, and never a power pump"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "the +0/+3 reaches the creature that attacked and no other"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::Ending)
    });
    assert_eq!(
        pt(&engine, paladin),
        (2, 5),
        "the pump is still in force in the end step: until end of turn"
    );
}
