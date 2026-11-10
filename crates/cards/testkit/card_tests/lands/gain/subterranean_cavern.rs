//! `cards/lands/gain/subterranean_cavern.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Subterranean Cavern enters tapped, gains its controller a life on the way
/// in, and then taps for black or green. The two halves cannot be played in
/// one turn, and that is the scenario: the land arrives tapped, so its `{T}`
/// is not even in the offer until an untap step has stood it up (CR 502.3) —
/// which makes the walk across the opponent's turn the thing that turns the
/// ability from a board state into a payment. Afterwards the colour question
/// is the "or": a card that read `{T}: Add {B}` would answer without asking.
#[test]
fn subterranean_cavern_enters_tapped_gains_a_life_and_taps_for_black_or_green() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .life(0, 20)
        .life(1, 20)
        .hand(0, &[subterranean_cavern()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, subterranean_cavern());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        entered_tapped(&engine, land),
        "the printed line: this land enters tapped"
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"when this land enters, you gain 1 life\""
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life belongs to the controller, not to the table"
    );

    // Tapped is not merely inconvenient: `{T}` is a price that cannot be
    // paid, so the ability is not in the offer at all in this turn.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "the first main phase hands priority back: {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == land),
        "a tapped land cannot pay {{T}}: {:?}",
        legal.abilities
    );

    // A whole turn cycle: their main phase, then back to this seat's own.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stands it up again (CR 502.3)"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(src, _)| *src == land)
        .expect("the printed {T}: Add {B} or {G} is offered once it is untapped");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("{{T}} is the whole price and the land is untapped");
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"or\" is a question and not a default, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the land's controller names the colour");
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Green),
        "both colours the card prints: {options:?}"
    );
    assert_eq!(options.len(), 2, "and no third: {options:?}");

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the two it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not the other half of the line"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing waits to resolve"
    );
}
