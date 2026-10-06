//! `cards/lands/towns/gohn_town_of_ruin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "13fbd525-c9ee-4bd6-b269-94fb264024b6"

/// Gohn, Town of Ruin prints two lines — "This land enters tapped" and
/// "{T}: Add {B} or {G}" — and each is only legible because the other has
/// already happened, so both are played in one game. The land is played rather
/// than seated: `starting_battlefield` places a permanent with `Cause::Setup`
/// and never runs an entry, so a card that arrives tapped is exactly the claim
/// a placement cannot make. The arrival turn is then read as an *offer* — a
/// tapped land has no {T} to pay with, and neither list carries it — and the
/// mana line is read on the turn after, once the untap step has stood it back
/// up and the choice the card prints comes back two colours wide.
#[test]
fn gohn_town_of_ruin_enters_tapped_and_taps_for_black_or_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[gohn_town_of_ruin()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, gohn_town_of_ruin());
    assert!(
        types(&engine, land).contains(TypeSet::LAND),
        "what was played is the land it prints"
    );
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — a real entry and not a placement"
    );

    // The arrival turn: a tapped land is no {T}, so the card's only line is
    // absent from both lists the offer keeps — and the pool is empty besides,
    // so nothing about mana could have supplied what the tap owes.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating to be mistaken for the price"
    );
    assert!(
        !legal.mana_abilities.contains(&land)
            && !legal.abilities.iter().any(|(src, _)| *src == land),
        "a tapped land has no {{T}} left to pay with, so the mana line is \
         offered nowhere: mana_abilities {:?}, abilities {:?}",
        legal.mana_abilities,
        legal.abilities
    );

    // Across the opponent's turn and back, which is the whole point of the
    // first printed line: the land is a mana source only from the turn the
    // untap step stood it back up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step stood it back up");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing has been tapped on this board yet"
    );

    activate(&mut engine, p0, gohn_town_of_ruin(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options,
        vec![ManaColor::Black, ManaColor::Green],
        "the two colours the card prints, and no third"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "`or` is exclusive: naming one half of the choice is not making both"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana, off one tap, with no other land on the board touched for it"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
}
