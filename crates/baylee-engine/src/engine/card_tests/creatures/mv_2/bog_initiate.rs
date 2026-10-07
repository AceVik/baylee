//! `cards/creatures/mv_2/bog_initiate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bog Initiate — {1}{B}, a 1/1 Human Wizard whose entire printed text is
/// "`{1}`: Add `{B}`". The price is one mana and **not** the tap symbol, and
/// that is the half a Llanowar Elves cannot show: the shared mana helper
/// presses only an ability whose whole cost is its own `{T}`, so the three
/// lands pay for the Wizard and float the `{1}` while the Wizard itself
/// stands untouched, which leaves the black mana below with exactly one
/// possible source. A `{T}` written where the card prints `{1}` would leave
/// the creature tapped and hand out the black for free; a mana ability
/// resolves without the stack (CR 605.3b), so both halves are read the moment
/// the answer lands.
#[test]
fn bog_initiate_trades_one_floating_mana_for_a_black_without_tapping_itself() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(7717, forest())
        .battlefield(0, &[swamp(), forest(), forest()])
        .hand(0, &[bog_initiate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The card arrives the way the card arrives: {1}{B} off the three lands,
    // which leaves exactly one green floating for the ability's price.
    cast_from_hand(&mut engine, p0, bog_initiate());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let initiate = on_battlefield(&engine, p0, bog_initiate()).expect("the Wizard resolved");
    assert_eq!(pt(&engine, initiate), (1, 1), "the body the card prints");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one mana left after paying {{1}}{{B}}");
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the Forest's green — the Swamp's black was spent on the {{B}} of the cost"
    );
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "nothing black is left over, so any black below has one source"
    );
    assert!(!is_tapped(&engine, initiate), "and the Wizard is untapped");

    // Ability 0 is the printed mana ability. It is an ordinary `(source,
    // index)` entry in `abilities` and not in `mana_abilities`, because a
    // mana ability a card prints has an index to name (CR 605.1) — which is
    // also why `tap_all_mana` did not press it: `{1}` is not its own tap.
    activate(&mut engine, p0, bog_initiate(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "`{{1}}: Add {{B}}` — the black is in the pool"
    );
    assert_eq!(pool.total(), 1, "one mana paid, one mana made");
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "and the {{1}} came out of the green that was floating"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so it resolved as it was activated"
    );
    assert!(
        !is_tapped(&engine, initiate),
        "the price was one mana: the Wizard is still standing, where a {{T}} \
         cost written here would not leave it"
    );
}
