//! `cards/creatures/mv_5/sliver_queen.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sliver Queen — {W}{U}{B}{R}{G}, a legendary 7/7 Sliver whose whole
/// printed text is "{2}: Create a 1/1 colorless Sliver creature token."
///
/// The scenario casts her for real off the five basic lands that produce
/// exactly her five colours, and then presses that one ability on the next
/// turn, where the {2} comes out of a pool the same lands filled. Both
/// halves of the price are read where the engine reads them: the ability is
/// *not* offered while five untapped lands stand and nothing floats
/// (`can_afford` reads the pool, not the board), and it is offered the
/// moment the mana is there. The token is then read as the printed body,
/// name and colourlessness, and the Queen is still standing to make another.
#[test]
#[allow(clippy::too_many_lines)]
fn sliver_queen_builds_a_one_one_sliver_token_for_two_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), island(), swamp(), mountain(), forest()])
        .hand(0, &[sliver_queen()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {W}{U}{B}{R}{G} off the five basic lands that make exactly those five
    // colours, so the card arrives the way the card arrives.
    cast_from_hand(&mut engine, p0, sliver_queen());
    pass_until(&mut engine, stack_is_empty);
    let queen = on_battlefield(&engine, p0, sliver_queen()).expect("the Queen resolved");
    assert_eq!(pt(&engine, queen), (7, 7), "the body the card prints");
    assert!(
        types(&engine, queen).contains(TypeSet::CREATURE),
        "and what arrived is the creature it prints"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "five lands paid the five-colour cost to the last mana"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing has been made yet: the only ability is an activated one"
    );

    // A turn round the table, because the five lands that paid for the Queen
    // are still tapped and the {2} has to come off a pool that really exists.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pool emptied with the step that ended (CR 500.5)"
    );

    // Five untapped lands and an empty pool: `can_afford` reads the pool, so
    // the ability is not offered even though the mana is standing right there.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(queen, 0)),
        "{{2}} is not two, so the cost is unpayable and nothing is offered: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five basic lands, five mana of five colours"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(queen, 0)),
        "with mana floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, sliver_queen(), 0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "the {{2}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "making a token is no mana ability, so the ability is on the stack"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "and the Sliver arrives on resolution, not on announcement"
    );

    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation, one Sliver");
    let sliver = tokens[0];
    let kinds = types(&engine, sliver);
    assert!(
        kinds.contains(TypeSet::CREATURE) && !kinds.contains(TypeSet::ARTIFACT),
        "\"a 1/1 colorless Sliver creature token\": {kinds:?}"
    );
    assert_eq!(pt(&engine, sliver), (1, 1), "the printed 1/1 body");
    let printed = engine
        .state()
        .object(sliver)
        .expect("the Sliver token is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Sliver");
    assert_eq!((printed.power, printed.toughness), (Some(1), Some(1)));
    for color in [
        baylee_core::color::Color::White,
        baylee_core::color::Color::Blue,
        baylee_core::color::Color::Black,
        baylee_core::color::Color::Red,
        baylee_core::color::Color::Green,
    ] {
        assert!(
            !printed.colors.contains(color),
            "\"colorless\" is no colour at all, and one of the five is"
        );
    }
    assert!(
        on_battlefield(&engine, p0, sliver_queen()).is_some(),
        "the {{2}} and no sacrifice is the whole price, so the Queen stays to \
         make another one"
    );
}
