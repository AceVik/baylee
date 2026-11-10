//! `cards/creatures/mv_1/elves_of_deep_shadow.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Elves of Deep Shadow is a 1/1 for `{G}` whose entire text is one line:
/// "`{T}`: Add `{B}`. This creature deals 1 damage to you."
///
/// Both halves are one ability, and either half alone is a lie about the
/// card, so this plays it on a board with nothing else that could make mana:
/// off an empty pool the black mana can only have come from the Elves, and
/// the life the controller is missing can only have come from the same
/// activation. "To you" is read against the opponent's untouched twenty, and
/// the tap is read as a cost — the ability is one shot, not a repeatable
/// engine that farms black mana until its controller is dead.
#[test]
fn elves_of_deep_shadow_tap_for_black_and_bite_their_controller() {
    let p0 = PlayerId::new(0);
    let _p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[elves_of_deep_shadow()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, elves_of_deep_shadow()).expect("the Elves are out");
    assert!(!is_tapped(&engine, elves), "they arrive untapped");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing floats on this board, so every drop afterwards is theirs"
    );

    // Ability 0 is the only line the card prints, and `activate` finds it in
    // `legal.abilities` — which is already the claim that `{T}` is affordable.
    activate(&mut engine, p0, elves_of_deep_shadow(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1, "the black mana");
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "a mana ability uses no stack (CR 605.3b): the mana and the damage \
         are already done, not waiting to resolve"
    );
    assert!(is_tapped(&engine, elves), "tapping them paid the cost");
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"This creature deals 1 damage to you\" — the same activation"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "\"to you\" is the controller and not the table"
    );

    // And it is spent: the tap symbol is a cost, so the same line is not
    // offered again for a second black mana at the price of a second life.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds priority again, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == elves),
        "a tapped Elf of Deep Shadow offers nothing: {:?}",
        legal.abilities
    );
}
