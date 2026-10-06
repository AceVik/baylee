//! `cards/lands/fetch/flood_plain.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Flood Plain prints two sentences, and neither can be read off a card file.
/// "This land enters tapped" only happens on a real `PlayLand` — the harness'
/// `starting_battlefield` places a permanent without an entry, so a land
/// seeded that way arrives untapped whatever it prints — and `{T}, Sacrifice
/// this land: Search your library for a Plains or Island card, put it onto
/// the battlefield, then shuffle` spends the permanent *itself* as a cost.
/// Sixty Plains make up the library, so the single Plains that can ever stand
/// on the battlefield is the one the fetch put there: the hand stays the
/// length it was and the library is one card shorter, which is what
/// `Find::BATTLEFIELD` decides and what a fetch into hand would not do.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn flood_plain_enters_tapped_and_trades_itself_for_a_plains_onto_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains()).hand(0, &[flood_plain()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, flood_plain());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — read off an entry, not off a seeded board"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing was tapped to pay for it, so the tapped status is the modifier's doing"
    );

    // The tap symbol in the cost is what gives the entry its teeth: a tapped
    // land cannot pay it, and this ability asks for no mana at all, so the
    // only thing that can be keeping it out of the offer is the tap.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == land),
        "a tapped Flood Plain cannot pay its own {{T}}: {:?}",
        legal.abilities
    );

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood it back up, so the offer below is an answer \
         about the ability rather than about a tap that never happened"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.iter().any(|(src, _)| *src == land),
        "untapped and with an empty mana pool, the fetch is on offer: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert!(
        all_on_battlefield(&engine, p0, plains()).is_empty(),
        "no Plains is on this board yet, so the one the search finds is the \
         one that arrives"
    );

    activate(&mut engine, p0, flood_plain(), 0);

    // CR 601.2h: the costs are the last thing paid, and with nothing to
    // choose between the two parts they are paid as the ability is announced.
    assert!(
        on_battlefield(&engine, p0, flood_plain()).is_none(),
        "the sacrifice took the land itself off the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, flood_plain()).is_some(),
        "and into its owner's graveyard, before the search ever resolved"
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
        unreachable!("pass_until only stops on a card choice")
    };
    assert_eq!(player, p0, "the land's controller is the one searching");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "a library search and not a mill or a discard"
    );
    assert!(
        !options.is_empty(),
        "the filter has something to find: sixty Plains is the whole library"
    );
    assert!(
        options.iter().all(|id| engine
            .state()
            .object(*id)
            .is_some_and(|o| o.card.is_some_and(|c| c.index == plains()))),
        "\"a Plains or Island card\" is read: every option this library can \
         offer is one of them: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("the card the search put on the menu is a legal find");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        all_on_battlefield(&engine, p0, plains()).len(),
        1,
        "\"put it onto the battlefield\" — the found card is in play"
    );
    assert!(
        in_hand(&engine, p0, plains()).is_some(),
        "control: the opening draws are Plains too, so the Plains above is not \
         merely the only one in the game"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and it came out of the library, which is what a search does to one"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "it went to the battlefield rather than to hand, which is the half \
         `Find::BATTLEFIELD` decides"
    );
}
