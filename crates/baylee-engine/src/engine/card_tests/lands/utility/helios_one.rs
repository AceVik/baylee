//! `cards/lands/utility/helios_one.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// HELIOS One prints three lines and `Coverage::Partial` writes one: a
/// nonbasic land's own "{T}: Add {C}". The two energy clauses are the gap —
/// nothing in the DSL puts a counter on a player and no cost part pays one —
/// so the only thing to play is the tap, and the board is built to prove it is
/// *this* land's tap: twenty Forests sit in the library and not one of them on
/// the battlefield, so the single colorless that appears can have come from
/// nowhere else. The offer is read through `deeds`, which sees both lists a
/// mana ability can arrive in — a printed `{T}: Add {C}` on a land with no
/// basic land type is an ordinary `(source, index)` entry and not the CR 305.6
/// shortcut — and it holds exactly one line, so nothing the card file leaves
/// unwritten is being offered either.
#[test]
fn helios_one_taps_for_one_colorless_and_offers_nothing_else() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4102, forest()).hand(0, &[helios_one()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, helios_one());
    assert!(
        !entered_tapped(&engine, land),
        "nothing on the card enters it tapped"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "a land drop spends nothing, so whatever is in the pool next is the land's"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("the seat holds a quiet main phase: {:?}", engine.pending());
    };
    let offered = deeds(&legal, &[land]);
    assert!(
        matches!(offered[..], [(0, Deed::Ability(0))]),
        "one line, and it is the printed {{T}}: Add {{C}} — the two energy \
         abilities are the `Coverage::Partial` gap and are offered by nobody: \
         {offered:?}"
    );

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(
        taken, 1,
        "the only mana source on the battlefield is HELIOS One"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "{{C}}, as printed");
    assert_eq!(pool.total(), 1, "one tap, one mana");
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}
