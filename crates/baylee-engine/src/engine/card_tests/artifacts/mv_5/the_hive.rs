//! `cards/artifacts/mv_5/the_hive.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The Hive — {5} artifact: "{5}, {T}: Create a 1/1 colorless Insect artifact
/// creature token with flying named Wasp."
///
/// Both halves of that price are the engine's answer rather than the card's, so
/// both are played on one board: ten Forests are exactly the {5} the artifact
/// costs plus the {5} the ability charges, and the pool reads five before the
/// activation and nothing after it. The Wasp is read only once the stack has
/// emptied, because making a token is no mana ability — its printed 1/1 body,
/// its artifact-creature type line, its flying, its colourlessness and the name
/// the card gives it are five claims a token that merely "arrived" would not
/// tell apart. The same board one turn later is the control for the price: the
/// Hive has stood back up and the pool is empty, and `can_afford` reads the
/// pool rather than ten untapped Forests, so the line is not offered at all.
#[test]
#[allow(clippy::too_many_lines)]
fn the_hive_taps_and_five_mana_for_a_colorless_flying_wasp() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 10])
        .hand(0, &[the_hive()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the Forests are tapped"
    );

    // Ten Forests into the pool: {5} for the artifact and the {5} its ability
    // then charges are one payment inside one main phase (CR 500.5).
    cast_from_hand(&mut engine, p0, the_hive());
    pass_until(&mut engine, stack_is_empty);
    let hive = on_battlefield(&engine, p0, the_hive()).expect("The Hive resolved");
    assert!(!is_tapped(&engine, hive), "an artifact enters untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "ten Forests paid the {{5}} the cast costs and exactly the {{5}} the \
         ability charges is left floating"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(hive, 0)),
        "with {{5}} in the pool the one line the card prints is offered: {:?}",
        legal.abilities
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing has been activated yet, so nothing has been made"
    );

    activate(&mut engine, p0, the_hive(), 0);
    assert!(
        is_tapped(&engine, hive),
        "{{T}} is half the price and is paid as the ability is activated"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the other half was the five mana that was floating"
    );
    assert!(
        !stack_is_empty(&engine),
        "making a token is no mana ability, so the ability is on the stack"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and the Wasp arrives on resolution, not on announcement"
    );

    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Wasp");
    let wasp = engine
        .state()
        .object(tokens[0])
        .expect("the Wasp is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(
        wasp.name, "Wasp",
        "\"a … token with flying named Wasp\": the name is the whole of what \
         tells this token from the pool's other 1/1 fliers"
    );
    assert_eq!(
        (wasp.power, wasp.toughness),
        (Some(1), Some(1)),
        "the body the card prints"
    );
    assert!(
        wasp.keywords.contains(KeywordSet::FLYING),
        "the printed flying reaches the token"
    );
    for color in [
        baylee_core::color::Color::White,
        baylee_core::color::Color::Blue,
        baylee_core::color::Color::Black,
        baylee_core::color::Color::Red,
        baylee_core::color::Color::Green,
    ] {
        assert!(
            !wasp.colors.contains(color),
            "\"colorless\" is the first word of the token's type line"
        );
    }
    let kinds = types(&engine, tokens[0]);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::CREATURE),
        "\"artifact creature token\": {kinds:?}"
    );

    // The price is a real one, and the reading that says so is the same board
    // one turn later: the Hive has untapped and the pool emptied with the step
    // that ended (CR 500.5), while `can_afford` reads the pool rather than the
    // ten untapped Forests. An empty pool is therefore the only difference, and
    // the line is not offered at all.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, hive),
        "the untap step stood the Hive back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool emptied with the step that ended"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(hive, 0)),
        "{{5}} is not five: with nothing floating the cost is unpayable, and an \
         unaffordable ability is absent from the offer rather than refused: {:?}",
        legal.abilities
    );

    // With the mana really floating the same {T} is a price again, so the card
    // is a repeatable engine and not a one-shot.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        10,
        "ten Forests untapped in the same main phase they came back in"
    );
    activate(&mut engine, p0, the_hive(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        tokens_of(&engine, p0).len(),
        2,
        "the untap step gave the Hive its {{T}} back, so the same artifact makes \
         another Wasp"
    );
    assert!(
        on_battlefield(&engine, p0, the_hive()).is_some(),
        "the price was the tap and the mana, so the artifact is still standing"
    );
}
