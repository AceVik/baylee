//! `cards/instants/mv_2/eladamri_s_call.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Eladamri's Call — {G}{W} instant: "Search your library for a creature
/// card, reveal that card, put it into your hand, then shuffle."
///
/// The board makes the filter legible rather than assumed: the 60-card deck is
/// Forests, so the only creature card in the game is one Llanowar Elves moved
/// out of the hand into the library, and an offer holding exactly that card is
/// "creature card" being read while a library of lands is passed over — a
/// search that had dropped the filter would have offered all of it. The move
/// afterwards is read off the zones, because the reveal is not a state: one
/// card leaves the library (the shuffle reorders what is left without changing
/// how much of it there is) and lands in the hand, while the {G}{W} is paid out
/// of a pool the Forest and the Plains filled and nothing else could have.
#[test]
fn eladamri_s_call_finds_the_one_creature_card_in_a_library_of_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), plains()])
        .hand(0, &[eladamri_s_call(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The one creature card anywhere, put where the Call has to go looking for
    // it: the harness' own dev capability, used exactly as `seed_graveyard`
    // uses it, since `SeatSpec` has no field for a library.
    let elf = in_hand(&engine, p0, llanowar_elves()).expect("the Elf is in hand");
    engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up")
        .move_object(
            elf,
            ZoneLocation::Library(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("the harness moves a card");
    // The offer was computed when priority was granted, which was before the
    // Elf left the hand, so it has to be recomputed before it is read again.
    engine.refresh_offer();

    let library_before = library_size(&engine, p0);

    cast_from_hand(&mut engine, p0, eladamri_s_call());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{G}}{{W}} out of the two basic lands, and nothing left floating"
    );

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
    assert_eq!(player, p0, "the caster searches their own library");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry or a discard"
    );
    assert_eq!(
        options,
        vec![elf],
        "one creature card in a library of lands: \"creature card\" is read, \
         not skipped"
    );

    // Captured with the instant already off the hand and on the stack, so the
    // only thing that can move the count is the card the search is about to
    // hand over.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("a card the search offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        in_hand(&engine, p0, llanowar_elves()),
        Some(elf),
        "\"put that card into your hand\": the very card the search offered, \
         and not another copy of the same printing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "the hand is one card up — a reveal that left the card where it was \
         could not do that"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what remains without changing how much of it there is"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .iter()
            .all(|id| {
                engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == forest()))
            }),
        "the library is the filler deck again: the one creature card in the \
         game is the one that moved"
    );
    assert!(
        in_graveyard(&engine, p0, eladamri_s_call()).is_some(),
        "the instant resolved and went to its owner's graveyard"
    );
}
