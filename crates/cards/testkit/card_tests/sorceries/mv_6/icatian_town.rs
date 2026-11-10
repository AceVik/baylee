//! `cards/sorceries/mv_6/icatian_town.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Icatian Town — {5}{W} sorcery: "Create four 1/1 white Citizen creature
/// tokens." The card is one sentence and every word of it is a different
/// reading, so the board is six Plains and nothing else: they pay the cost to
/// the last mana, and the four permanents that arrive afterwards are counted,
/// weighed, coloured and named off the tokens themselves rather than off the
/// card file. An effect that made three, or one 4/4, or green Soldiers would
/// satisfy any single one of those checks.
#[test]
fn icatian_town_creates_four_one_one_white_citizen_tokens() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(); 6])
        .hand(0, &[icatian_town()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing has been made yet, so every token below belongs to this cast"
    );

    // {5}{W} is six mana and the six Plains are the whole board, so the offer
    // is read with the mana already in the pool: `can_afford` reads the pool
    // and not the untapped lands, and this sorcery has no target that could
    // hold it back for a second reason.
    let card = in_hand(&engine, p0, icatian_town()).expect("the sorcery is in hand");
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Plains tapped, six white — and no creature on the board to add a seventh"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "six mana pays {{5}}{{W}} in a main phase with an empty stack: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, icatian_town());
    assert!(
        !stack_is_empty(&engine),
        "a sorcery does not resolve the moment it is cast"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and nothing is made while it is still on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 4, "one cast, four Citizens");
    for token in tokens {
        let printed = engine
            .state()
            .object(token)
            .expect("the Citizen is on the battlefield")
            .token
            .expect("it knows which token it is");
        assert_eq!(printed.name, "Citizen", "the creature the spell names");
        assert_eq!(
            (printed.power, printed.toughness),
            (Some(1), Some(1)),
            "a 1/1 and not a single 4/4"
        );
        assert!(
            printed.colors.contains(baylee_core::color::Color::White),
            "white, which is the colour the card prints"
        );
        assert_eq!(
            pt(&engine, token),
            (1, 1),
            "and the body the board projects is the body the spell makes"
        );
        assert!(
            types(&engine, token).contains(TypeSet::CREATURE),
            "a creature token, and not four artifacts that happen to be 1/1"
        );
    }
    assert!(
        tokens_of(&engine, p1).is_empty(),
        "the tokens belong to the caster: the opponent's board never moved"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the six mana went into the spell"
    );
}
