//! `cards/lands/gain/kazandu_refuge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kazandu Refuge prints three sentences that are one turn apart. "This land
/// enters tapped" and "When this land enters, you gain 1 life" both happen in
/// the arrival turn, while `{T}: Add {R} or {G}` cannot be paid in it at all —
/// a tapped land has no `{T}` to spend, and only the untap step of its
/// controller's next turn (CR 502.3) stands it back up. Played as a real land
/// drop, the life total is what separates the entry trigger from nothing
/// happening, and the colour question asked a turn later is the only place the
/// printed "or" is visible: a land that made a single colour never asks.
#[test]
fn kazandu_refuge_enters_tapped_gains_a_life_then_taps_for_red_or_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[kazandu_refuge()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    let refuge = play_land(&mut engine, p0, kazandu_refuge());
    pass_until(&mut engine, |e| e.state().players[0].life > life_before);

    assert!(
        entered_tapped(&engine, refuge),
        "\"This land enters tapped\" — read off the entry the land actually \
         made, which is the half `starting_battlefield` would never show"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "\"When this land enters, you gain 1 life\" — exactly one, off \
         exactly one entry"
    );

    // A turn later the untap step has stood it up, which is the only thing
    // that makes its `{T}` price payable, and the pool is empty again because
    // mana empties with the step that made it (CR 500.5).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, refuge),
        "the untap step ran and nothing else tapped the land back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating, so whatever the tap makes has one source"
    );

    // Ability 1: the ETB trigger at index 0 takes no activation, so the
    // printed mana ability is the second entry in the card's list.
    activate(&mut engine, p0, kazandu_refuge(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the land's controller names the colour");
    assert_eq!(
        options.len(),
        2,
        "\"or\" is two colours and not the five of a free choice: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::Green),
        "red and green, as printed: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .expect("green was one of the two it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert_eq!(
        pool.available(ManaColor::Red),
        0,
        "and nothing came of the colour that was not named"
    );
    assert!(is_tapped(&engine, refuge), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability puts nothing on the stack"
    );
}
