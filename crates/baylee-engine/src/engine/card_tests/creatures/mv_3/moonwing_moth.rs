//! `cards/creatures/mv_3/moonwing_moth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Moonwing Moth — {1}{W}{W} — Creature — Insect 2/1 with flying and
/// "{W}: This creature gets +0/+1 until end of turn."
///
/// The pump is the whole card, and three things about it are the engine's
/// answer rather than the printing's. `legal.abilities` is filtered through
/// `can_afford`, which reads the *pool* and not the untapped lands, so the
/// line is absent on an empty pool and present the moment one white floats.
/// The activation leaves the Moth standing, because the price is the mana it
/// prints and not a tap symbol it does not. And `UntilEndOfTurn` is worth
/// exactly one turn: a 2/1 becomes a 2/2 now and is a printed 2/1 again on
/// the next one.
#[test]
fn moonwing_moth_turns_a_white_into_one_toughness_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[moonwing_moth(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Three Plains pay {1}{W}{W} to the last drop, which is what the first
    // reading of the offer below rests on.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Plains, three white"
    );
    cast_with_floating(&mut engine, p0, moonwing_moth());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let moth = on_battlefield(&engine, p0, moonwing_moth()).expect("the Moth resolved");
    assert_eq!(pt(&engine, moth), (2, 1), "the body the card prints");
    assert!(
        keywords(&engine, moth).contains(KeywordSet::FLYING),
        "and the keyword it prints"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the cast took the whole pool, so nothing is floating for the pump"
    );

    // No white, no offer: `can_afford` reads the pool and not the untapped
    // lands, so the pump is not among the activations offered right now.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == moth),
        "{{W}} is not one white, so the pump is not offered: {:?}",
        legal.abilities
    );

    // The fourth Plains, played as this turn's land drop, is that white.
    play_land(&mut engine, p0, plains());
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the untapped Plains and nothing else: the Moth prints no mana ability"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(moth, 0)),
        "with the white floating, the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, moonwing_moth(), 0);
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so it uses the stack"
    );
    assert!(
        !is_tapped(&engine, moth),
        "the price is the {{W}} the card prints and not a tap symbol it does not"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, moth),
        (2, 2),
        "+0/+1 is one toughness: a 2/1 becomes a 2/2"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the white that paid for it is gone"
    );

    // Across the opponent's turn and back into the Moth's controller's next
    // one, where the duration has run out.
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        pt(&engine, moth),
        (2, 1),
        "\"until end of turn\": the pump is gone once the turn that took it is over"
    );
}
