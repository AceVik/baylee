//! `cards/creatures/mv_4/steelshaper_apprentice.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Steelshaper Apprentice is {2}{W}{W} for a 1/3 whose whole text is "{W},
/// {T}, Return this creature to its owner's hand: Search your library for an
/// Equipment card, reveal that card, put it into your hand, then shuffle."
///
/// Every part of that price is read where it lands — the {W} out of a pool
/// only the Plains filled, and the creature in its owner's hand *while the
/// search is still unanswered*, which is what tells a return spelled as a
/// cost from the same words spelled as an effect (CR 601.2h). One Equipment
/// card is slid into a library of Forests first, so the search's filter has
/// something to be right about: the menu that comes back is that card, and
/// not the forest standing beside it.
#[test]
#[allow(clippy::too_many_lines)]
fn steelshaper_apprentice_returns_itself_for_an_equipment_card_out_of_the_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        // On the battlefield from the start: the ability's {T} can be paid
        // only by a creature its controller has held since the turn began
        // (CR 302.6), so one cast this turn could not be activated at all.
        .battlefield(0, &[steelshaper_apprentice(), plains()])
        .hand(0, &[lightning_greaves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The one Equipment card goes into a library of Forests, which is the
    // harness' own capability doing what it is for — the same door
    // `seed_graveyard` uses, and for the same reason: no game can put a card
    // there before the spell that searches for one has been cast.
    let greaves = in_hand(&engine, p0, lightning_greaves()).expect("the Greaves are in hand");
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    state
        .move_object(
            greaves,
            crate::zone::ZoneLocation::Library(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("the harness moves a card");
    engine.refresh_offer();

    // The one Plains into the pool: it is exactly the {W} the ability
    // charges. `legal.abilities` is filtered through `can_afford`, which
    // reads the pool rather than the untapped lands, so the offer below is
    // read with the mana already there.
    tap_all_mana(&mut engine, p0);
    let apprentice =
        on_battlefield(&engine, p0, steelshaper_apprentice()).expect("the Apprentice is out");
    assert_eq!(pt(&engine, apprentice), (1, 3), "the body the card prints");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "one Plains, and the one {{W}} the ability charges"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(apprentice, 0)),
        "the one line the card prints, now that its {{W}} is in the pool: {:?}",
        legal.abilities
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, steelshaper_apprentice(), 0);

    // The ability asks for no targets, so CR 601.2c passes and CR 601.2h is
    // the whole activation: the white leaves the pool and the creature leaves
    // the battlefield while the search is still to come.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{W}} it charges came out of the pool"
    );
    assert!(
        on_battlefield(&engine, p0, steelshaper_apprentice()).is_none(),
        "\"Return this creature to its owner's hand\" is a cost, so the 1/3 is \
         off the battlefield the moment the ability is announced"
    );
    assert!(
        in_hand(&engine, p0, steelshaper_apprentice()).is_some(),
        "and in its owner's hand, which is where the word \"owner\" puts it"
    );

    // The search is the resolution, and the question is the effect's own.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(
        player, p0,
        "the seat that activated searches its own library"
    );
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry or a discard"
    );
    assert_eq!(
        options,
        vec![greaves],
        "\"an Equipment card\" is the one such card in a library of Forests: {options:?}"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .contains(&greaves),
        "\"put it into your hand\" is the resolution, so the card is still in \
         the library while the question stands"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![greaves],
            },
        )
        .expect("the card the search offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, lightning_greaves()).is_some(),
        "\"put that card into your hand\": the very object the search offered"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2,
        "the Apprentice came back to the hand it left and the Equipment joined \
         it, so the hand is two cards up"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .contains(&greaves),
        "and the library it was found in no longer holds it"
    );
}
