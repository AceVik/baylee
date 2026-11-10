//! `cards/lands/filter/sunscorched_divide.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sunscorched Divide is a land with no basic land type and one printed line:
/// "{1}, {T}: Add {R}{W}." That is a mana ability in the rules (CR 605.1a) but
/// not the CR 305.6 shortcut — it prints a cost and carries an index — so the
/// only way to read it is to press it. The board makes the trade unmistakable:
/// a Sol Ring is the entire pool, so before the activation there is nothing but
/// colorless mana on the table and the {1} is the reason the line is not even
/// offered; afterwards the pool holds one red and one white, which no other
/// permanent here could have produced. The empty stack is CR 605.3b: the mana
/// arrives as the answer is applied, because a mana ability uses no stack.
#[test]
fn sunscorched_divide_charges_a_generic_for_a_red_and_a_white_off_its_own_tap() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[quiet_artifact()])
        .hand(0, &[sunscorched_divide()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let divide = play_land(&mut engine, p0, sunscorched_divide());
    assert!(!is_tapped(&engine, divide), "a plain land arrives untapped");

    // With nothing in the pool the {1} cannot be paid, and `can_afford` reads
    // the pool rather than the untapped permanents — so the line is missing
    // from the offer for exactly as long as the mana is.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(divide, 0)),
        "{{1}} is part of the cost and an empty pool cannot pay it: {:?}",
        legal.abilities
    );

    // Mana before the claim. `tap_all_mana` taps the Sol Ring for {C}{C} and
    // leaves the Divide standing: its whole price is not its own tap, so the
    // helper's `{{T}}`-only rule does not reach it (#159).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "one Sol Ring is two colorless"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        0,
        "and this board has made no colored mana at all yet"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        0,
        "so anything red or white below came off the land"
    );
    assert!(!is_tapped(&engine, divide), "which is still untapped");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(divide, 0)),
        "with {{C}}{{C}} floating the one line the land prints is offered — as \
         an ordinary (source, index) entry and not the basic-type shortcut: {:?}",
        legal.abilities
    );

    // Ability 0 is the only ability the card prints.
    activate(&mut engine, p0, sunscorched_divide(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "the {{1}} came out of the pool"
    );
    assert_eq!(pool.available(ManaColor::Red), 1, "{{R}} off the land");
    assert_eq!(pool.available(ManaColor::White), 1, "and {{W}} beside it");
    assert_eq!(
        pool.total(),
        3,
        "two colorless in and three mana out is the trade the card prints"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack, so nothing is pending"
    );
    assert!(
        is_tapped(&engine, divide),
        "the other half of the cost is the tap symbol the land paid itself"
    );
}
