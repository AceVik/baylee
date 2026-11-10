//! `cards/lands/gain/scoured_barrens.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Scoured Barrens prints three sentences and one scenario reads all three:
/// it enters tapped, its controller gains 1 life as it enters, and it taps
/// for {W} or {B}. The land has to be *played*, because
/// `starting_battlefield` places a permanent with `Cause::Setup` and no
/// replacement effect looks at it — a Scoured Barrens built that way arrives
/// untapped whatever the card says. The colour line is read a turn later,
/// since a land that arrives tapped has no `{T}` to spend in the turn it
/// lands, and the empty pool asserted first is what makes "exactly one
/// black" an exact claim rather than a coincidence.
#[test]
fn scoured_barrens_enters_tapped_gains_one_life_and_taps_for_white_or_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .hand(0, &[scoured_barrens()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let land = play_land(&mut engine, p0, scoured_barrens());
    assert!(
        entered_tapped(&engine, land),
        "\"This land enters tapped\" — read off a real land drop, not off a \
         seeded battlefield"
    );

    pass_until(&mut engine, |e| {
        stack_is_empty(e) && e.state().players[0].life == 21
    });
    assert_eq!(
        engine.state().players[0].life,
        21,
        "\"When this land enters, you gain 1 life\""
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the life goes to the land's controller, not across the table"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "a land that arrives tapped has nothing to spend in the turn it lands"
    );

    // The untap step is what makes the mana line readable at all.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step stood it back up, so its {{T}} is payable now"
    );

    // Ability 0 is the enters trigger; ability 1 is the printed mana ability.
    activate(&mut engine, p0, scoured_barrens(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"{{T}}: Add {{W}} or {{B}}\" is a question with two answers, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the land names the colour");
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Black),
        "white and black are the two the card prints: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "one line, two colours, and no third: {options:?}"
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
        pool.total(),
        1,
        "one mana off one tap, with nothing floating beside it"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
