//! `cards/lands/fetch/grasslands.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Grasslands prints two sentences: it enters tapped, and "{T}, Sacrifice
/// this land: Search your library for a Forest or Plains card, put it onto
/// the battlefield, then shuffle."
///
/// The land is **played** and not seeded, because `starting_battlefield`
/// places a permanent with `Cause::Setup` and no replacement effect looks at
/// a placement — a seeded Grasslands arrives untapped and the first printed
/// sentence is never exercised. The turn cycle in between is the other half:
/// the untap step (CR 502.3) is the only thing that makes `{T}` payable, and
/// the search that follows moves exactly one card out of a library that holds
/// nothing but Forests and onto the battlefield under p0's control, while the
/// sacrificed fetch land itself is in the graveyard — the sacrifice is part
/// of the cost (CR 601.2h) and not part of the effect.
#[test]
fn grasslands_enters_tapped_and_trades_itself_for_a_land_onto_the_battlefield() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[grasslands()]).start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, grasslands());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — a real land drop, so the replacement \
         effect applies to it"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        1,
        "and it is the only land p0 controls, so the untap step below is \
         about this permanent and nothing else"
    );

    // A whole turn cycle, and back. `reach_their_main_phase` answers the
    // combat declarations `reach_main_phase` cannot cross.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, land),
        "the untap step ran, so the printed {{T}} is payable at last"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a main phase hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.abilities.iter().any(|(source, _)| *source == land),
        "an untapped Grasslands offers its one line, whose whole price is the \
         tap symbol and the land itself: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    // Nothing is floated first: the cost carries no mana at all, and an empty
    // pool is what makes "the price was the tap and the land" exact.
    activate(&mut engine, p0, grasslands(), 0);
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
        unreachable!("pass_until stops on nothing but a card choice")
    };
    assert_eq!(player, p0, "the seat that sacrificed the land searches");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "a browse of the library, which is a different question from a cost"
    );
    assert!(
        !options.is_empty(),
        "the filler deck is sixty Forests, so a Forest is there to find"
    );
    assert!(
        in_graveyard(&engine, p0, grasslands()).is_some(),
        "the sacrifice is paid to announce the ability (CR 601.2h), so the \
         land is already buried while the search is still being asked"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("a card the search itself put on the menu");
    pass_until(&mut engine, stack_is_empty);

    let fetched = on_battlefield(&engine, p0, forest()).expect("a Forest card off the deck");
    assert!(
        !is_tapped(&engine, fetched),
        "\"put it onto the battlefield\" — the card prints no tapped entry, \
         unlike the fetches that do"
    );
    assert!(
        on_battlefield(&engine, p0, grasslands()).is_none(),
        "the fetch land does not stay on the table, and does not fetch itself"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        library_before - 1,
        "exactly one card left the library for the battlefield, and then the \
         library was shuffled"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        1,
        "one land out, one land in: the searched card arrived and the \
         sacrificed land is in no land count"
    );
}
