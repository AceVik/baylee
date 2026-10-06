//! `cards/sorceries/mv_1/green_sun_s_zenith.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Green Sun's Zenith — {X}{G} sorcery: "Search your library for a green
/// creature card with mana value X or less, put it onto the battlefield, then
/// shuffle. Shuffle Green Sun's Zenith into its owner's library." The file is
/// `Coverage::Partial`: the search is written and the shuffle-back is not, so
/// the halves a board can hold the card to are the announced X, the creature
/// that arrives, and where the spell itself ends up.
///
/// The library is sixty Llanowar Elves — one green creature card, mana value
/// 1 — and X is announced as **2**. Reading "mana value X or less" against a
/// card strictly cheaper than X is the only reading that separates the printed
/// bound from a search for mana value *X*, and a library of a single printing
/// makes the menu the searching player's own library rather than a hand-picked
/// pair. CR 608.2m then puts the Zenith in the graveyard, which is exactly the
/// sentence the printing replaces.
#[test]
fn green_suns_zenith_finds_a_green_creature_of_mana_value_x_or_less() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, llanowar_elves())
        .battlefield(0, &[forest(), forest(), forest()])
        .hand(0, &[green_suns_zenith()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = library_size(&engine, p0);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "nothing but the three Forests stands before the Zenith looks"
    );

    // {X}{G} with X announced as 2 is three mana, and the three Forests are
    // the whole of it: `cast_from_hand` taps them first, so the pool the
    // engine prices X against is the pool the card is paid out of.
    cast_from_hand(&mut engine, p0, green_suns_zenith());
    let Pending::ChooseNumber {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!(
            "a spell with an X in its cost announces it, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster announces their own X");
    assert!(
        min <= 2 && 2 <= max,
        "three green in the pool leaves X somewhere in {min}..={max}"
    );
    engine
        .apply(p0, PlayerAction::ChooseNumber(2))
        .expect("two is inside the range the engine published");

    // The spell is on the stack now, and the search is asked as it resolves.
    // Not `pass_until`: that walker panics on a `SearchLibrary` question, and
    // this is the question under test.
    let mut steps = 0;
    while !matches!(engine.pending(), Pending::ChooseCards { .. }) {
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!(
                "expected priority while the Zenith resolves, got {:?}",
                engine.pending()
            )
        };
        engine.apply(player, PlayerAction::PassPriority).unwrap();
        steps += 1;
        assert!(steps <= 20, "the Zenith never reached its search");
    }

    let Pending::ChooseCards {
        player,
        options,
        min,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the loop only leaves on a card choice")
    };
    assert_eq!(player, p0, "the caster searches their own library");
    assert_eq!(prompt, crate::choice::ChoicePrompt::SearchLibrary);
    assert_eq!(min, 1, "the search is not optional and asks for one card");
    assert!(
        !options.is_empty(),
        "every card in this library is a green creature of mana value 1, so \
         a bound of 2 cannot leave the menu empty"
    );
    let cards_in_library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert!(
        options.iter().all(|id| cards_in_library.contains(id)),
        "and every option comes out of the searching player's own library: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("one of the cards the question offered");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "\"put it onto the battlefield\": a mana value 1 creature arrived off \
         an announced X of 2, which a search for mana value *X* would never \
         have offered"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "the found card left the library, and the shuffle returned no other"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{X}}{{G}} took the whole three the Forests made"
    );
    assert!(
        in_graveyard(&engine, p0, green_suns_zenith()).is_some(),
        "CR 608.2m: the resolving spell lies in the graveyard, because \
         \"shuffle Green Sun's Zenith into its owner's library\" is the clause \
         the card's file leaves off"
    );
}
