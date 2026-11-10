//! `cards/creatures/mv_1/blood_celebrant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blood Celebrant prints one line — "{B}, Pay 1 life: Add one mana of any
/// color" — and it is a mana ability that neither taps nor uses the stack
/// (CR 605.1, 605.3b), so the card is one decision: which colour, paid for out
/// of the pool with a life point on top of it.
///
/// The board is a single Swamp because the arithmetic is the proof. The Swamp
/// is tapped for the {B} the cost charges, so the pool is empty when the colour
/// question is asked and the mana that appears afterwards can only be the
/// Celebrant's own tap-free production — and the five colours on the menu are
/// the "any color" the card prints rather than the black its own cost is paid
/// in. The life total is the other half of the price and is read after the tap
/// and before the answer, so neither number can stand in for the other.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn blood_celebrant_trades_its_black_and_a_life_for_a_mana_of_the_colour_you_name() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[blood_celebrant(), swamp()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let celebrant = on_battlefield(&engine, p0, blood_celebrant()).expect("the Celebrant is out");
    let land = on_battlefield(&engine, p0, swamp()).expect("the Swamp is out");
    assert_eq!(pt(&engine, celebrant), (1, 1), "a printed 1/1");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats before the Swamp is tapped"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and nothing is paid yet"
    );

    // Mana into the pool first: `can_afford` reads the pool, not the untapped
    // land standing beside the creature.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "one Swamp, one black mana",
    );
    assert!(is_tapped(&engine, land), "which the tap turned over");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.iter().any(|(src, _)| *src == celebrant),
        "a printed mana ability is an ordinary `(source, index)` entry in \
         `abilities`, not the CR 305.6 shortcut: {:?}",
        legal.abilities
    );

    // Ability 0 is the only line the card prints.
    activate(&mut engine, p0, blood_celebrant(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("`any color` is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{B}} is spent before the question is asked: costs are the last \
         step of an activation (CR 601.2h)"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "and the other half of the price is a life point"
    );
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
        "the whole wheel, not the black the ability's own cost is paid in \
         (CR 105.4): {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named — one this board's lone Swamp could never \
         have produced"
    );
    assert_eq!(pool.total(), 1, "one mana, and nothing else went with it");
    assert_eq!(
        engine.state().players[0].life,
        19,
        "the life was paid once and not twice"
    );
    assert!(
        !is_tapped(&engine, celebrant),
        "the price is {{B}} and a life, not a tap: the Celebrant still stands"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(
        is_tapped(&engine, land),
        "and the Swamp is the one that paid the {{B}}, so the blue has no \
         other source on this board"
    );
}
