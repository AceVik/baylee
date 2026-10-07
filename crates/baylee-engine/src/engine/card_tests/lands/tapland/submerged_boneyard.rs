//! `cards/lands/tapland/submerged_boneyard.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn submerged_boneyard_enters_tapped_then_taps_for_blue_or_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[submerged_boneyard()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Played and not placed: `PlayLand` is a real land drop, so the printed
    // replacement effect runs and the land arrives tapped.
    let land = play_land(&mut engine, p0, submerged_boneyard());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — a seeded battlefield would have skipped \
         the very modifier this is about"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "arriving tapped makes no mana on the way in"
    );

    // A tapped permanent has no {T} to pay with, so the mana line is not on
    // offer at all in the turn the land arrives.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the land drop hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.contains(&(land, 0)),
        "a land that entered tapped cannot pay {{T}}: {:?}",
        legal.abilities
    );

    // One turn cycle: the untap step stands it back up, and the mana line is
    // read where the engine reads it.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the Boneyard's controller takes another turn"
    );
    assert!(
        !is_tapped(&engine, land),
        "the untap step ran and stood the land back up, without which the \
         offer below would be satisfied by a game that never advanced"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "an untapped land with nothing else to pay is the whole price of its \
         one line, so it is offered: {:?}",
        legal.abilities
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the price is the tap symbol and no mana at all"
    );

    activate(&mut engine, p0, submerged_boneyard(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{U}} or {{B}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert!(
        options.contains(&ManaColor::Blue) && options.contains(&ManaColor::Black),
        "\"{{U}} or {{B}}\" includes both: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and exactly the two colours the card prints — {{C}} is no colour at \
         all (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "the other half of the `or` was not added alongside it"
    );
    assert_eq!(
        pool.total(),
        1,
        "one mana off one tap, and nothing else on a board holding nothing but \
         this land"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
