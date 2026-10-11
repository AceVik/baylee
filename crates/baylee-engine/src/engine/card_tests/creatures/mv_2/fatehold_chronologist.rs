//! `cards/creatures/mv_2/fatehold_chronologist.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The creature enters prepared; its inset spell cannot be cast from hand.
#[test]
fn fatehold_chronologist_enters_prepared_and_carries_the_offer_that_buys() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[fatehold_chronologist()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, fatehold_chronologist());
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the inset is not a cast mode for the physical card"
    );
    pass_until(&mut engine, |e| at_rest(e, p0));

    let bird = on_battlefield(&engine, p0, fatehold_chronologist()).expect("it resolved");
    assert_eq!(pt(&engine, bird), (1, 2), "a 1/2 Bird Wizard");
    assert!(
        keywords(&engine, bird).contains(KeywordSet::FLYING),
        "with flying"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.iter().any(|(id, _)| *id == bird),
        "\"While it's prepared, you may cast a copy of its spell\" — the \
         printed front face has no activated ability of its own, so this \
         offer is the prepared marker and nothing else: {:?}",
        legal.abilities
    );
}

/// The exiled prepare copy costs {2}{W/U}, creates its Cadet, surveils and
/// ceases to exist after resolving. The original creature stays in play.
#[test]
fn peer_review_is_cast_only_as_the_prepared_copy() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[fatehold_chronologist()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));
    cast_from_hand(&mut engine, p0, fatehold_chronologist());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let bird = on_battlefield(&engine, p0, fatehold_chronologist()).unwrap();
    let copy = engine.state().prepared_copy(bird).unwrap();
    let object = engine.state().object(copy).unwrap();
    assert_eq!(object.zone, crate::zone::Zone::Exile);
    assert_eq!(engine.state().names.get(object.base.name), "Peer Review");
    assert_eq!(object.base.types, baylee_core::types::TypeSet::SORCERY);
    assert_eq!(
        object.base.mana_cost,
        baylee_core::mana::ManaCost::parse("{2}{W/U}")
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: bird,
                ability_index: crate::choice::PREPARED_CAST,
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "two for the creature, three for its spell"
    );
    assert_eq!(engine.state().object(copy).unwrap().face_index, 1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::Arrange {
                prompt: ArrangePrompt::Surveil,
                ..
            }
        )
    });
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    let cadet = engine.state().object(tokens[0]).unwrap().characteristics();
    assert_eq!(engine.state().names.get(cadet.name), "Cadet");
    assert!(
        cadet
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::WIZARD)
    );
    assert!(
        cadet
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::SOLDIER)
    );
    assert!(cadet.colors.is_empty());
    assert_eq!(pt(&engine, tokens[0]), (2, 2));
    assert!(matches!(drive_to_rest(&mut engine, p0), Rest::Reached));
    assert!(engine.state().object(copy).is_none());
    assert!(on_battlefield(&engine, p0, fatehold_chronologist()).is_some());
    assert!(
        !engine
            .state()
            .object(bird)
            .unwrap()
            .riders
            .contains(&crate::object::Rider::Prepared)
    );
}
