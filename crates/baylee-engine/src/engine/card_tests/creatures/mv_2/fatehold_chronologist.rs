//! `cards/creatures/mv_2/fatehold_chronologist.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fatehold Chronologist "enters prepared", and the printed reminder says
/// what that buys: "While it's prepared, you may cast a copy of its spell."
/// Its spell is Peer Review on the back face, so the assertion is that the
/// creature carries an offer its face alone does not explain — a 1/2 flier
/// with no printed activated ability, standing there with one.
///
/// Nothing below casts the copy, so what is asserted is the marker and the
/// offer rather than the spell's own text; the next test casts Peer Review.
#[test]
fn fatehold_chronologist_enters_prepared_and_carries_the_offer_that_buys() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[fatehold_chronologist()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A card with a castable second face is offered both, so the front one is
    // named rather than assumed: mode 1 is Peer Review at {2}{W/U} and would
    // put a sorcery on the stack where this test wants a creature.
    cast_from_hand(&mut engine, p0, fatehold_chronologist());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!(
            "a card with two castable faces asks which, got {:?}",
            engine.pending()
        )
    };
    let normal = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Normal))
        .expect("the creature itself is one of the offers");
    engine
        .apply(p0, PlayerAction::ChooseMode(normal))
        .expect("the mode the question enumerated");
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

/// Peer Review, Fatehold Chronologist's back face: "Create a 2/2 colorless
/// Wizard Soldier creature token named Cadet. Surveil 1."
///
/// Cast from hand as the sorcery, so the Cadet is the spell's own and not
/// the prepared copy's. The token is read for every printed word — the
/// name, both creature types, the 2/2 and no colour — because a registry
/// entry that differed in one of them would still be *a* token.
#[test]
fn peer_review_makes_a_cadet_and_then_surveils_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[fatehold_chronologist()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "no tokens before the spell"
    );

    cast_from_hand(&mut engine, p0, fatehold_chronologist());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!(
            "a card with two castable faces asks which, got {:?}",
            engine.pending()
        )
    };
    let review = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Face(1)))
        .expect("Peer Review, the back face, is one of the offers");
    engine
        .apply(p0, PlayerAction::ChooseMode(review))
        .expect("the mode the question enumerated");
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
    assert_eq!(tokens.len(), 1, "one token, before the surveil is answered");
    let cadet = engine
        .state()
        .object(tokens[0])
        .expect("the token")
        .characteristics();
    assert_eq!(engine.state().names.get(cadet.name), "Cadet");
    assert!(cadet.types.contains(TypeSet::CREATURE));
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
    assert!(cadet.colors.is_empty(), "colorless");
    assert_eq!(pt(&engine, tokens[0]), (2, 2));

    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the surveil is answered and the game comes to rest"
    );
    assert!(
        in_graveyard(&engine, p0, fatehold_chronologist()).is_some(),
        "the card went to the graveyard as a resolved sorcery"
    );
}
