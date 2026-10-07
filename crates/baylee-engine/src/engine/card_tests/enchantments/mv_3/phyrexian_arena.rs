//! `cards/enchantments/mv_3/phyrexian_arena.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Phyrexian Arena — {1}{B}{B} enchantment: "At the beginning of your upkeep,
/// you draw a card and you lose 1 life." The word that carries the card is
/// *your*, so the board is read twice: once after the opponent has taken a
/// whole turn, where neither library nor life total may have moved, and once
/// after the Arena's controller has had an upkeep of their own, where the life
/// total is exactly one lower — one trigger, not one per upkeep of the table —
/// and the library is two cards shorter, counting the turn's own draw step
/// beside the card the enchantment gave.
#[test]
fn phyrexian_arena_draws_a_card_and_bites_its_controller_on_their_own_upkeep_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[phyrexian_arena()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // `{1}{B}{B}` off three Swamps, and nothing else stands on this board: no
    // other permanent here can touch a life total or a library.
    cast_from_hand(&mut engine, p0, phyrexian_arena());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, phyrexian_arena()).is_some(),
        "the Arena resolved onto the battlefield"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the three Swamps paid `{{1}}{{B}}{{B}}` and nothing is left floating"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let life_before = engine.state().players[0].life;
    let their_life_before = engine.state().players[1].life;

    // The opponent's whole turn goes by. "At the beginning of *your* upkeep"
    // is the Arena's controller's upkeep and nobody else's, so p1's upkeep has
    // to pass without a card drawn or a life lost.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        (
            library_size(&engine, p0),
            engine.state().players[0].life,
            engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        ),
        (library_before, life_before, hand_before),
        "\"your upkeep\" is not the opponent's: p1's turn moved neither p0's \
         library, nor p0's life, nor p0's hand"
    );
    assert_eq!(
        engine.state().players[1].life,
        their_life_before,
        "and the Arena drains nobody but its own controller: the other seat is \
         untouched too"
    );

    // Back round to the Arena's controller, whose upkeep is when the sentence
    // fires. One turn of the table is one trigger, so the life total drops by
    // exactly one and the library by two: the turn's own draw step and the
    // card the Arena gave on top of it.
    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(
        engine.state().players[0].life,
        life_before - 1,
        "\"you lose 1 life\" once per one of your own upkeeps, and not once per \
         upkeep of the table"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 2,
        "two cards off the top of the library: the draw step's own card and the \
         one the Arena draws at the beginning of the upkeep, which comes first"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2,
        "and both are in hand, so a library that merely emptied would not \
         satisfy the count above"
    );
    assert_eq!(
        engine.state().players[1].life,
        their_life_before,
        "the life is still the controller's: a trigger keyed on the wrong seat \
         would have taken a point from p1 instead, and the drawn card would not \
         have left p0's hand two larger"
    );
    assert!(
        on_battlefield(&engine, p0, phyrexian_arena()).is_some(),
        "the Arena pays its own price in life and never in permanents, so it is \
         still on the battlefield"
    );
}
