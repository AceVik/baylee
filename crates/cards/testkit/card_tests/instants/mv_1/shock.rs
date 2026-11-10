//! `cards/instants/mv_1/shock.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shock prints one line — "Shock deals 2 damage to any target" — and the test
/// plays both halves of "any" off one card. The first cast is aimed at the
/// opposing player from a `ChooseTargets` whose two lists are both populated
/// (CR 115.4), and 20 life becoming 18 is what pins the number: a one-damage
/// Shock leaves 19 and a three-damage one leaves 17. The second is aimed at a
/// printed 1/2, which dies to exactly the same two and would have survived one,
/// so the object half of the spec is read with the amount already fixed.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn shock_deals_two_to_a_player_and_to_a_printed_one_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[shock(), shock()])
        .battlefield(1, &[a_one_two_bird()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bird = on_battlefield(&engine, p1, a_one_two_bird()).expect("the Bird is out");
    assert_eq!(
        pt(&engine, bird),
        (1, 2),
        "a printed 1/2: two damage is exactly lethal where one would not be"
    );

    // Mana before the claim: the offer is read off the pool, and the two
    // Mountains are exactly the {R}{R} two Shocks cost in this one main phase
    // (CR 500.5, which ends at the step and not before).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "two Mountains, two red"
    );

    cast_with_floating(&mut engine, p0, shock());
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "`any target` is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster aims it");
    assert_eq!((min, max), (1, 1), "one target, no more and no fewer");
    assert!(
        options.contains(&bird),
        "the creature across the table is one of the object options: {options:?}"
    );
    assert!(
        player_options.contains(&p0) && player_options.contains(&p1),
        "CR 115.4: \"any target\" counts players in the same choice, both of \
         them: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("a player the prompt enumerated is a legal answer");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "\"deals 2 damage\": 19 would be one and 17 would be three"
    );
    assert!(
        on_battlefield(&engine, p1, a_one_two_bird()).is_some(),
        "the damage went to the player who was named and not to their board"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one of the two red paid the first Shock"
    );

    // The object half of the same spec, off the red still floating.
    cast_with_floating(&mut engine, p0, shock());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "the second Shock asks for a target too, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&bird),
        "the same 1/2 is offered again: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bird],
                players: vec![],
            },
        )
        .expect("the creature the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, a_one_two_bird()).is_some(),
        "two damage to a printed 1/2 is lethal (CR 704.5g)"
    );
    assert_eq!(
        engine.state().players[1].life,
        18,
        "and the second Shock went to the creature: the life total never moved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{R}} it charged came out of the pool"
    );
}
