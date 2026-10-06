//! `cards/creatures/mv_5/plated_rootwalla.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Plated Rootwalla — {4}{G} for a printed 3/3 Lizard whose whole text is one
/// activated ability: "{2}{G}: This creature gets +3/+3 until end of turn.
/// Activate only once each turn."
///
/// Both clauses need a reading of their own, so one board gives both: twelve
/// Forests pay the {4}{G} and leave seven green, which covers the {2}{G} twice
/// over — so a withdrawal after the first activation is the printed
/// once-per-turn word and not `can_afford` reading an empty pool. The pump is
/// read off the layer projection, where (6, 6) on a printed (3, 3) is the only
/// body that applies the +3/+3, and the same creature is read again a turn
/// later at (3, 3), which is what tells an until-end-of-turn pump from a
/// +1/+1 counter or a static.
#[test]
fn plated_rootwalla_pumps_itself_once_a_turn_and_the_pump_lasts_the_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 12])
        .hand(0, &[plated_rootwalla()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Twelve Forests: {4}{G} brings the Lizard to the table and leaves seven
    // green behind it, which is more than the ability charges — so nothing the
    // offer says below is ever withheld for want of mana.
    cast_from_hand(&mut engine, p0, plated_rootwalla());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let lizard = on_battlefield(&engine, p0, plated_rootwalla()).expect("the Rootwalla resolved");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "twelve Forests less the {{4}}{{G}} the creature costs"
    );
    assert_eq!(pt(&engine, lizard), (3, 3), "the body the card prints");

    // The offer is read where the engine reads its prices: the pool.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(lizard, 0)),
        "with seven green floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, plated_rootwalla(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the {{2}}{{G}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so the ability is waiting on the stack"
    );
    assert_eq!(
        pt(&engine, lizard),
        (3, 3),
        "and nothing has been pumped while it is still on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, lizard),
        (6, 6),
        "+3/+3 until end of turn on the creature the ability names"
    );

    // Four green are still in the pool, so an absent offer here has exactly
    // one explanation left.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(lizard, 0)),
        "\"Activate only once each turn\": the mana to pay for it is floating \
         and the ability is withheld anyway: {:?}",
        legal.abilities
    );

    // Across the opponent's turn and back: the pump lasted the turn it was
    // made in (CR 514.2), and the limit is per turn rather than per game.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, lizard),
        (3, 3),
        "the pump left with the turn that made it"
    );
    assert!(
        on_battlefield(&engine, p0, plated_rootwalla()).is_some(),
        "and the creature is still standing, so the body changed rather than the board"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        12,
        "the untap step stood the twelve Forests back up"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(lizard, 0)),
        "\"once each turn\" is a turn and not a game: the new turn handed the \
         line back: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, plated_rootwalla(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, lizard),
        (6, 6),
        "and it may be taken again on this turn, off the same printed price"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        9,
        "the second {{2}}{{G}} came out of the twelve the Forests filled"
    );
}
