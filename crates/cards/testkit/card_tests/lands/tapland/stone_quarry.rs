//! `cards/lands/tapland/stone_quarry.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Stone Quarry prints two lines, and both are the engine's answer rather than
/// a fact in the card file: "This land enters tapped" and "{T}: Add {R} or
/// {W}". The scenario separates them by one turn — on the turn it is played the
/// land is read off the battlefield *and* off the offer (tapped, and its {T} in
/// neither list), and on the controller's next main phase both readings flip.
/// The colour that ends up in the pool is the colour that was named, so the
/// choice is read and not defaulted, and the lone permanent on the board makes
/// "one mana" an exact claim about this land.
#[test]
fn stone_quarry_enters_tapped_and_taps_for_the_colour_its_controller_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[stone_quarry()]).start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // A real land drop, not a `starting_battlefield` placement: a placement
    // enters without an entry, so no enter modifier would ever look at it.
    let quarry = play_land(&mut engine, p0, stone_quarry());
    assert!(
        entered_tapped(&engine, quarry),
        "\"This land enters tapped\" — the printed enter modifier ran"
    );

    // Its price is the tap symbol and nothing else, so an offer that is missing
    // here is missing because the land arrived tapped and not for want of mana.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("playing a land uses no stack, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == quarry)
            && !legal.mana_abilities.contains(&quarry),
        "a tapped land has no {{T}} to spend, so it is offered in neither list \
         on the turn it arrives: {:?} / {:?}",
        legal.abilities,
        legal.mana_abilities
    );

    // Across the opponent's turn and back, which is where the untap step can be
    // read: the land that was down last turn is the one standing now.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, quarry),
        "the untap step ran, so the {{T}} pressed below is a real price and not \
         a permanent that merely happened to be standing"
    );

    // The Quarry is the only permanent on this board, so the pool read after
    // the answer is the whole of what the land produced.
    activate(&mut engine, p0, stone_quarry(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{W}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player, p0,
        "the seat that taps the land is the one that names the colour"
    );
    assert_eq!(
        options.len(),
        2,
        "the two colours the card prints and no third: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Red),
        "{{R}} is offered: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::White),
        "and {{W}}: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::White))
        .expect("white was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the colour that was named, and not a default the answer never reached"
    );
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "and nothing of the other half of the printed line"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, quarry), "the land paid its own {{T}}");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds priority again, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == quarry)
            && !legal.mana_abilities.contains(&quarry),
        "and a tapped land has no {{T}} left to pay with: {:?} / {:?}",
        legal.abilities,
        legal.mana_abilities
    );
}
