//! `cards/lands/pain/skyshroud_forest.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Skyshroud Forest prints three lines and this scenario reads all three off
/// one board: it enters tapped, it taps for `{C}`, and it taps for `{G}` or
/// `{U}` while dealing 1 damage to you. Playing it and then asking the offer
/// is what puts the entry modifier into the rules rather than into a status
/// flag — a permanent that is already tapped cannot pay a `{T}`, so on the
/// turn it arrives the land is offered nothing at all — and the colour
/// question on the following turn is where the printed "or" and the printed
/// price are read: two colours and no third, one blue in the pool, one life
/// off its controller and none off the opponent.
#[test]
fn skyshroud_forest_enters_tapped_and_charges_one_life_for_the_color_it_makes() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // A filler deck of basic Forests, so the only blue this board could ever
    // produce is the blue this land names.
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[skyshroud_forest()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, skyshroud_forest());
    assert!(
        entered_tapped(&engine, land),
        "\"this land enters tapped\" — and it arrives through a real land \
         drop, not through a placement no replacement effect would look at"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("a land drop hands priority back: {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land),
        "a tapped permanent cannot pay a {{T}}, so neither Add line is in \
         the offer this turn: {:?}",
        legal.abilities
    );

    // A whole turn cycle: only the untap step stands the land back up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the untap step untapped it");

    // Ability 0 is `{T}: Add {C}`; ability 1 is the coloured line.
    activate(&mut engine, p0, skyshroud_forest(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{G}} or {{U}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        2,
        "the two colours the card prints and no third: {options:?}"
    );
    assert!(
        options.contains(&ManaColor::Green) && options.contains(&ManaColor::Blue),
        "{{G}} or {{U}}: {options:?}"
    );
    assert!(
        !options.contains(&ManaColor::Colorless),
        "the {{C}} line is a second ability, and colourless is no colour at \
         all (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, off the land that named it"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"this land deals 1 damage to you\" — the price is paid by its \
         controller"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and never by the opponent"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
