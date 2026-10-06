//! `cards/artifacts/mv_4/the_one_ring.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `The One Ring` (`Coverage::Partial`):
/// "Indestructible. When `The One Ring` enters, if you cast it, you gain protection from
/// everything until your next turn. At the beginning of your upkeep, you lose 1 life for
/// each burden counter on `The One Ring`. `{{T}}`: Put a burden counter on `The One Ring`,
/// then draw a card for each burden counter on `The One Ring`."
///
/// Under `Coverage::Partial`, burden counters, the ETB protection clause, and the tap-draw
/// ability are omitted, implementing `KeywordSet::INDESTRUCTIBLE` on a legendary artifact.
/// The test verifies that `The One Ring` possesses `KeywordSet::INDESTRUCTIBLE` on the
/// battlefield and survives a destroy effect from an opponent's `vindicate()`.
#[test]
fn the_one_ring_has_indestructible_and_survives_destroy_effects() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(304, plains())
        .battlefield(0, &[the_one_ring()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p0, the_one_ring()).expect("The One Ring on battlefield");
    assert!(
        keywords(&engine, ring).contains(KeywordSet::INDESTRUCTIBLE),
        "The One Ring has indestructible"
    );

    // Advance to p1's main phase to cast Vindicate targeting The One Ring.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for Vindicate, got {:?}",
            engine.pending()
        );
    };
    assert!(
        options.contains(&ring),
        "The One Ring is a legal target for Vindicate"
    );

    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    // Indestructible prevents destruction (CR 702.12b).
    assert!(
        on_battlefield(&engine, p0, the_one_ring()).is_some(),
        "The One Ring survives on the battlefield due to indestructible"
    );
    assert!(
        in_graveyard(&engine, p0, the_one_ring()).is_none(),
        "The One Ring was not put into the graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, vindicate()).is_some(),
        "Vindicate resolved and was put into p1's graveyard"
    );
}

/// What The One Ring does **not** do, which is three of its four sentences.
///
/// The card is `Coverage::Partial` with indestructible and nothing else: the
/// cast-triggered protection, the upkeep drain per burden counter and the
/// `{T}` draw are each refused by name at the foot of the card file. The
/// test above plays the sentence that works. This one pins the three that do
/// not, because a keyword is the one characteristic that keeps reading
/// correctly while everything around it is missing — and an artifact whose
/// whole reputation is drawing cards would look fine in a board state that
/// never asked it to.
#[test]
fn the_one_ring_offers_no_ability_and_costs_no_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(305, plains())
        .battlefield(0, &[the_one_ring(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p0, the_one_ring()).expect("The One Ring is out");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == ring),
        "`{{T}}: Put a burden counter on this, then draw a card for each` is \
         not written, so the Ring offers nothing to activate"
    );

    // Two of its own upkeeps, which is where the drain would show. It has no
    // burden counters either, so the two halves agree: nothing counts and
    // nothing is lost.
    let life = engine.state().players[0].life;
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3 && e.state().turn.active == p0
    });
    assert_eq!(
        engine.state().players[0].life,
        life,
        "no upkeep trigger is written, so no life is lost for a burden \
         counter that is never placed"
    );
    assert_eq!(
        counters_on(&engine, ring, CounterKind::Charge),
        0,
        "and nothing put one there"
    );
}
