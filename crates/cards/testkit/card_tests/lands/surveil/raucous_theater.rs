//! `cards/lands/surveil/raucous_theater.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Raucous Theater prints `Land — Swamp Mountain`, "This land enters tapped",
/// "{T}: Add {B} or {R}" and "When this land enters, surveil 1" — and all
/// three are read here. The land is *played*, not seated: a
/// `starting_battlefield` permanent is a placement with no entry for a
/// replacement effect to look at, so a board built that way arrives untapped
/// whatever the card says. The following untap step is the other half — this
/// is a tapped entry, not a permanent held down (CR 502.3).
///
/// This is the *binning* direction of CR 701.25a: the one card the surveil
/// looked at is put into the graveyard, and both zones are counted, because
/// "look at the top card" and "put it in the graveyard" are different halves
/// and a surveil that quietly did neither would pass a test that only counted
/// the library. Thundering Falls below is the keeping direction.
#[test]
fn raucous_theater_enters_tapped_and_taps_for_black_or_red() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[raucous_theater()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = library_size(&engine, p0);
    let top_before = engine
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .copied()
        .expect("p0 has a library");

    let land = play_land(&mut engine, p0, raucous_theater());
    assert!(
        is_tapped(&engine, land),
        "\"This land enters tapped\", read off a real land drop"
    );

    // "When this land enters, surveil 1."
    let (cards, piles) = surveil_offer(&mut engine, p0);
    assert_eq!(
        cards,
        vec![top_before],
        "surveil 1 looks at exactly the top card of its own library"
    );
    assert_eq!(
        piles,
        surveil_piles(1),
        "any number of them, which here is 0 or 1"
    );
    engine
        .apply(p0, look_answer(&cards, &[top_before]))
        .expect("the card it just looked at");

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "the card it binned left the library"
    );
    assert_eq!(
        in_graveyard(&engine, p0, forest()),
        Some(top_before),
        "and it is in the graveyard, which is where a surveil differs from a \
         scry"
    );

    // One turn cycle: an entry that taps is not a permanent that never untaps.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, land),
        "the untap step gives the land back, so it entered tapped and was \
         not held down"
    );

    // Ability 0 is the printed "{T}: Add {B} or {R}".
    activate(&mut engine, p0, raucous_theater(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{R}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the land's controller names the colour");
    assert_eq!(
        options.len(),
        2,
        "two colours and nothing else: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Red),
        "{{B}} and {{R}} are the whole of the sentence: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, off the land's own {{T}}"
    );
    assert_eq!(pool.total(), 1, "one mana, and nothing beside it");
    assert!(is_tapped(&engine, land), "which tapped the land");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
}
