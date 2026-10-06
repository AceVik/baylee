//! `cards/lands/guildmages_forum.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Guildmages' Forum prints two mana abilities and this plays both: `{T}: Add
/// {C}`, and `{1}, {T}: Add one mana of any color` — the counter the second is
/// supposed to leave behind on a multicolored creature spell is the
/// `Coverage::Partial` gap, and nothing here presses it. Two copies stand on the
/// board because the price of either line is the Forum's own `{T}`, so one land
/// taps once, and the `{C}` the first line makes is the `{1}` the second spends
/// — which is why the pool holds exactly one mana at the end rather than two.
/// The question the second line asks is asserted as the five colors of the game
/// with no colorless on it (CR 105.4), and the two untapped Forests beside the
/// Forums are the control: the black mana that arrives has no green source on
/// this board that could have produced it.
#[test]
fn guildmages_forum_taps_for_colorless_and_trades_that_mana_for_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(41, forest())
        .battlefield(0, &[guildmages_forum(), guildmages_forum(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forums = all_on_battlefield(&engine, p0, guildmages_forum());
    assert_eq!(forums.len(), 2, "two copies, one for each printed line");
    let lands = all_on_battlefield(&engine, p0, forest());
    assert_eq!(lands.len(), 1, "and a Forest that never moves");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before anything is tapped"
    );

    // Ability 0, the `{T}: Add {C}` half. A mana ability uses no stack
    // (CR 605.3b), so the mana is in the pool the moment it is applied.
    activate(&mut engine, p0, guildmages_forum(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "{{C}} off the Forum's own tap"
    );
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "a mana ability never uses the stack"
    );
    assert_eq!(
        forums.iter().filter(|id| is_tapped(&engine, **id)).count(),
        1,
        "exactly one Forum paid for it"
    );

    // Ability 1, `{1}, {T}: Add one mana of any color`. Its {1} is the {C}
    // that is already floating — which is why it is offered at all: the offer
    // is read off the pool and not off what could still be tapped.
    activate(&mut engine, p0, guildmages_forum(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }
    assert_eq!(
        options.len(),
        5,
        "the five colors of the game, and colorless is no color at all \
         (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colors it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the color that was named, and not a default"
    );
    assert_eq!(
        pool.total(),
        1,
        "and the {{C}} is gone: the second line's {{1}} was paid out of it"
    );
    assert_eq!(
        pool.available(ManaColor::Colorless),
        0,
        "so nothing colorless is left over"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b again, for the second line"
    );
    assert!(
        forums.iter().all(|id| is_tapped(&engine, *id)),
        "both Forums have now been tapped, one apiece"
    );
    assert!(
        lands.iter().all(|id| !is_tapped(&engine, *id)),
        "and the Forest beside them never moved, so the black mana has no \
         green source on this board"
    );
}
