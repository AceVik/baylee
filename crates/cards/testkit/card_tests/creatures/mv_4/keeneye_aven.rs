//! `cards/creatures/mv_4/keeneye_aven.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Keeneye Aven prints two lines and one board plays both, which is why it is
/// two copies rather than one: "{3}{U}" for a 2/3 Bird Soldier with flying, and
/// cycling — "{2}, Discard this card: Draw a card" — an ability a card offers
/// from *hand*, so the second copy is played while the first is cycled away.
/// Six tapped Islands are exactly the cycling's two and the cast's four, so a
/// pool of six, then four, then nothing is both printed prices read off one
/// pool; and the cycling is read in two zones at once, the card in its owner's
/// graveyard and a library one shorter, because a discard that never happened
/// would leave the same hand size behind as a discard and a draw.
#[test]
fn keeneye_aven_flies_as_a_two_three_and_cycles_itself_away_for_a_card() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 6])
        .hand(0, &[keeneye_aven(), keeneye_aven()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Mana before the claim: `can_afford` reads the pool and not the untapped
    // Islands, so the cycling is offered only once its {2} is really floating.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Islands, six blue, and the Avens make no mana of their own"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let cyclable = in_hand(&engine, p0, keeneye_aven()).expect("an Aven is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(cyclable, 0)),
        "cycling is an ability the card offers from hand, so it is an ordinary \
         `(source, index)` entry in `abilities`: {:?}",
        legal.abilities
    );

    // Ability 0 is the printed cycling. `DiscardSelf` is a cost, so the card is
    // in its owner's graveyard the moment the activation is applied — while the
    // draw it pays for is still waiting on the stack.
    activate(&mut engine, p0, keeneye_aven(), 0);
    assert!(
        in_graveyard(&engine, p0, keeneye_aven()).is_some(),
        "\"Discard this card\" is a cost, paid on announcement (CR 601.2h)"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "and the {{2}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "drawing a card is no mana ability, so the cycling is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\": one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card discarded and one drawn, so the hand is the size it was"
    );

    // The other copy, cast off what the cycling left: {3}{U} is exactly four.
    cast_with_floating(&mut engine, p0, keeneye_aven());
    pass_until(&mut engine, stack_is_empty);

    let aven = on_battlefield(&engine, p0, keeneye_aven()).expect("the second Aven resolved");
    assert!(
        types(&engine, aven).contains(TypeSet::CREATURE),
        "a creature, and the cast is how it got to the battlefield"
    );
    assert!(
        keywords(&engine, aven).contains(KeywordSet::FLYING),
        "\"Flying\" reaches the permanent through the layers"
    );
    assert_eq!(pt(&engine, aven), (2, 3), "the printed 2/3 body");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the cast's {{3}}{{U}} is the last four of the six the Islands made"
    );
}
