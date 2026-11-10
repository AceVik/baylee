//! `cards/creatures/mv_1/fyndhorn_elves.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fyndhorn Elves prints one line — `{T}: Add {G}` — and *where* that line is
/// offered is the whole of the test. A mana ability a card actually prints is
/// a mana ability in the rules (CR 605.1) and an ordinary `(source, index)`
/// entry in `LegalActions::abilities`, because it has an index to name; the
/// CR 305.6 shortcut `mana_abilities` carries only what a basic land type
/// grants and "never" carries a printed one (#159). The empty pool is what
/// makes the offer readable — a `{T}`-only price is affordable with nothing
/// floating, so this is the printed ability and not a mana payment — and the
/// green lands with the stack still empty (CR 605.3b).
#[test]
fn fyndhorn_elves_offers_its_printed_mana_ability_and_the_tap_lands_one_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(911, forest())
        .battlefield(0, &[fyndhorn_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, fyndhorn_elves()).expect("the Elves are out");
    assert_eq!(pt(&engine, elves), (1, 1), "a printed 1/1 for {{G}}");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing floats: the board is one creature and nothing else"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered = deeds(&legal, &[elves]);
    assert!(
        matches!(offered[..], [(0, Deed::Ability(0))]),
        "the printed {{T}}: Add {{G}} is an ordinary ability entry and not the \
         CR 305.6 shortcut, which has no index to name: {offered:?}"
    );
    assert!(
        !legal.mana_abilities.contains(&elves),
        "and it is not on the shortcut list as well — one line, one entry: {:?}",
        legal.mana_abilities
    );

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 1, "the Elves are the only mana source on the board");
    assert!(
        is_tapped(&engine, elves),
        "the whole price was its own {{T}}"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "one green, in the pool"
    );
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
}
