//! `cards/lands/caves/urza_s_cave.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Urza's Cave is a land with two printed lines: `{T}: Add {C}` and the
/// activated `{3}, {T}, Sacrifice this land: Search your library for a
/// land card, put it onto the battlefield tapped, then shuffle`. Three
/// of the four Forests are tapped for the `{3}`, the fourth deliberately
/// stays untapped — that way "enters tapped" is a statement about the
/// searched land and not about a board on which everything is lying
/// anyway. That the ability sacrifices its own source is read by the test
/// at both ends: the Cave is gone from the battlefield and lies in the
/// graveyard, the pool is empty, and the library has become exactly one
/// card smaller.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn urza_s_cave_fetches_a_land_tapped_and_sacrifices_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[urza_s_cave(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cave = on_battlefield(&engine, p0, urza_s_cave()).expect("Urza's Cave deployed");
    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(forests.len(), 4, "four Forests were dealt");
    let spared = forests[3];

    // Three of the four Forests pay the {3}; the fourth deliberately stays
    // untapped and is the control further down. The Cave is held back,
    // because its own {T} ability is paid right away.
    tap_mana_where(&mut engine, p0, |id| id != cave && id != spared);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Forests made three green, enough for the {{3}}"
    );
    assert!(is_tapped(&engine, forests[0]));
    assert!(is_tapped(&engine, forests[1]));
    assert!(is_tapped(&engine, forests[2]));
    assert!(
        !is_tapped(&engine, spared),
        "the spare Forest was left standing"
    );
    assert!(!is_tapped(&engine, cave), "and so was the Cave itself");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a main phase hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.abilities.contains(&(cave, 1)),
        "an untapped Cave and three floating mana make \
         \"{{3}}, {{T}}, sacrifice\" affordable, so the one activated \
         line the card prints is on the offer: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    activate(&mut engine, p0, urza_s_cave(), 1);

    // The costs are the last step of activation (CR 601.2h), so both are
    // already fixed while the search is still on the stack.
    assert!(
        on_battlefield(&engine, p0, urza_s_cave()).is_none(),
        "the Cave sacrificed itself to pay for its own ability"
    );
    assert!(
        in_graveyard(&engine, p0, urza_s_cave()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{3}} came out of the pool"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stops on nothing but the search");
    };
    assert_eq!(player, p0, "the searching seat answers");
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "\"Search your library for a land card\" is a filtered browse"
    );
    assert_eq!((min, max), (1, 1), "\"a land card\" is exactly one");
    assert!(
        !options.is_empty(),
        "the backing deck is Forests, so the search has a land to offer"
    );
    let chosen = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![chosen],
            },
        )
        .expect("the card the library put on the menu is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    let fetched: Vec<ObjectId> = all_on_battlefield(&engine, p0, forest())
        .into_iter()
        .filter(|id| !forests.contains(id))
        .collect();
    assert_eq!(fetched.len(), 1, "one land card came out of the library");
    let landed = fetched[0];
    assert!(
        is_tapped(&engine, landed),
        "\"put it onto the battlefield tapped\" — and the spared Forest \
         beside it is the control that this tapping is the entry modifier \
         and not a board on which everything happens to be down"
    );
    assert!(
        !is_tapped(&engine, spared),
        "the Forest nothing touched never moved"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "the fetched card left the library rather than being drawn"
    );
}
