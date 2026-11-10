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

/// Casting grants protection, and that protection expires at your next turn.
#[test]
fn the_one_ring_cast_protects_you_until_your_next_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let ring = card_index("3aa83ed2-f48b-4ce6-a614-2c54ddf50538");
    let mut engine = Duel::new(306, mountain())
        .battlefield(0, &[mountain(); 6])
        .hand(0, &[ring, lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, ring);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, ring).is_some(),
        "the Ring entered by being cast"
    );

    cast_from_hand(&mut engine, p0, lightning_bolt());
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p0],
                },
            )
            .is_err(),
        "protection from everything refuses even your own target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent remains a legal target");
    pass_until(&mut engine, stack_is_empty);

    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > turn
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    cast_from_hand(&mut engine, p0, lightning_bolt());
    let life = engine.state().players[0].life;
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .expect("at your next turn you can be targeted again");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, life - 3);
}

/// The draw counts the counter just added, and upkeep counts all burdens.
#[test]
fn the_one_ring_adds_a_burden_before_drawing_and_loses_life_for_each_at_upkeep() {
    let p0 = PlayerId::new(0);
    let ring = card_index("3aa83ed2-f48b-4ce6-a614-2c54ddf50538");
    let mut engine = Duel::new(307, plains()).battlefield(0, &[ring]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ring = on_battlefield(&engine, p0, ring).expect("the Ring is out");
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(
        &mut engine,
        p0,
        card_index("3aa83ed2-f48b-4ce6-a614-2c54ddf50538"),
        2,
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, ring, baylee_cards_dsl::counters::BURDEN),
        1
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand + 1,
        "the first activation draws one, including its new burden"
    );

    let life = engine.state().players[0].life;
    let opponent_life = engine.state().players[1].life;
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > turn
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        engine.state().players[0].life,
        life - 1,
        "your upkeep loses one life for the one burden"
    );
    assert_eq!(
        engine.state().players[1].life,
        opponent_life,
        "the opponent's upkeep trigger does not belong to your Ring"
    );

    activate(
        &mut engine,
        p0,
        card_index("3aa83ed2-f48b-4ce6-a614-2c54ddf50538"),
        2,
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        counters_on(&engine, ring, baylee_cards_dsl::counters::BURDEN),
        2
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand + 4,
        "one ordinary draw plus one and then two burden draws"
    );
}
