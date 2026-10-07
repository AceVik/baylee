//! `cards/creatures/mv_3/spellseeker.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Spellseeker — {2}{U} 1/1 Human Wizard: "When this creature enters, you may
/// search your library for an instant or sorcery card with mana value 2 or
/// less, reveal it, put it into your hand, then shuffle."
///
/// A tutor's whole text is its filter, so the library is built to make one
/// printed word fail at a time: a Counterspell sits exactly on "mana value 2 or
/// less", a Sultai Charm ({B}{G}{U}, mana value 3) is one mana past it, a
/// Llanowar Elves is cheap and neither an instant nor a sorcery, and the rest
/// of the deck is Elves — so a filter that had widened to any of them would put
/// extra card indices on the menu rather than on the battlefield. The card the
/// search offers is then read as a *move*, into the hand and out of a library
/// one card shorter, because a menu that was merely published says nothing
/// about where the card went.
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn spellseeker_searches_out_a_cheap_instant_and_declines_the_rest() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[island(), island(), island()])
        .hand(
            0,
            &[spellseeker(), counterspell(), sultai_charm(), dark_ritual()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The three cards the search is about go on top of the library by the
    // harness: `SeatSpec` has a field for an opening hand and one for a
    // starting battlefield and none for a library, which is the same gap
    // `seed_graveyard` exists for.
    let planted: Vec<ObjectId> = [counterspell(), sultai_charm(), dark_ritual()]
        .iter()
        .map(|card| in_hand(&engine, p0, *card).expect("the planted card was dealt"))
        .collect();
    {
        let state = engine
            .dev_state_mut(p0)
            .expect("the harness may set boards up");
        for card in planted {
            state
                .move_object(
                    card,
                    ZoneLocation::Library(p0),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .expect("the harness moves a card");
        }
    }
    // The offer was computed at the priority `reach_main_phase` stopped on,
    // which was before this: the castable list would otherwise be a stale
    // reading of a hand that has just lost three cards.
    engine.refresh_offer();

    // {2}{U} off the three Islands and nothing else — the Elves the deck drew
    // are in hand, not on the table, so three blue is the whole pool.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Islands, three blue"
    );
    cast_with_floating(&mut engine, p0, spellseeker());
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            } | Pending::ChooseCards { .. }
        )
    });
    // The printed "you may" is a question before it is a search, and it is
    // answered yes here rather than by the walker, because the tutor is what
    // this test came for.
    if let Pending::YesNo { player, .. } = engine.pending().clone() {
        assert_eq!(player, p0, "the Spellseeker's controller is the one asked");
        engine
            .apply(p0, PlayerAction::YesNo(true))
            .expect("taking the tutor is always legal");
        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. })
        });
    }

    let seeker = on_battlefield(&engine, p0, spellseeker()).expect("the Spellseeker resolved");
    assert_eq!(pt(&engine, seeker), (1, 1), "the printed 1/1 body");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{2}}{{U}} came out of the pool"
    );

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the tutor asks for a card, got {:?}", engine.pending());
    };
    assert_eq!(player, p0, "the seat that cast the Spellseeker searches");
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "the tutor's own question, and not a scry or a discard"
    );
    assert!(min <= 1 && max >= 1, "a card may be taken: ({min}, {max})");

    let offered: Vec<CardIndex> = options
        .iter()
        .filter_map(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .map(|c| c.index)
        })
        .collect();
    assert!(
        offered.contains(&counterspell()),
        "\"mana value 2 or less\" reaches the boundary: {offered:?}"
    );
    assert!(
        offered.contains(&dark_ritual()),
        "and a one-mana instant is well inside it: {offered:?}"
    );
    assert!(
        !offered.contains(&sultai_charm()),
        "a three-mana instant is one past the printed bound: {offered:?}"
    );
    assert!(
        !offered.contains(&llanowar_elves()),
        "a creature is neither an instant nor a sorcery, whatever its mana value: {offered:?}"
    );
    assert_eq!(
        offered.len(),
        2,
        "the library holds exactly two cards that answer the filter: {offered:?}"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let slot = options
        .iter()
        .position(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == counterspell())
        })
        .expect("the Counterspell was one of the cards the search offered");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[slot]],
            },
        )
        .expect("a card the search offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, counterspell()).is_some(),
        "\"put it into your hand\": the very card the search offered, and not \
         some other copy of the same printing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "one card up, which a reveal that left the card where it was could not do"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"then shuffle\": the found card left the library, and shuffling \
         reorders what is left without changing how much of it there is"
    );
    assert!(
        in_graveyard(&engine, p0, counterspell()).is_none()
            && in_graveyard(&engine, p0, sultai_charm()).is_none(),
        "the tutor's destination is the hand: neither the card it found nor the \
         card it declined was put anywhere else"
    );
}
