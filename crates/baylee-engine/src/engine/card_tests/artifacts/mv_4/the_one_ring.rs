//! `cards/artifacts/mv_4/the_one_ring.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Indestructible remains effective with the complete activated and triggered abilities.
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

/// A Ring with no burdens offers its draw ability and loses no life at upkeep.
#[test]
fn the_one_ring_offers_its_draw_ability_and_zero_burdens_cost_no_life() {
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
        legal.abilities.iter().any(|(src, _)| *src == ring),
        "the complete Ring offers its activated draw ability"
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
        "upkeep loses zero life without burden counters"
    );
    assert_eq!(
        counters_on(&engine, ring, baylee_cards_dsl::counters::BURDEN),
        0,
        "and nothing put one there"
    );
}
