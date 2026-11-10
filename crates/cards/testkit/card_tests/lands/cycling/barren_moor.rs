//! `cards/lands/cycling/barren_moor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Barren Moor prints three lines — "This land enters tapped", "{T}: Add
/// {B}", "Cycling {B}" — and this plays all three in one game. The turn cycle
/// is what makes the mana read worth anything: a land that arrives tapped has
/// no `{T}` to offer until its controller untaps, and the Swamp beside it is
/// left standing so the black that appears in the pool can only have come off
/// the land that was played. Cycling is the line that costs mana, so it is
/// pressed with `{B}` already floating, because `legal.abilities` is filtered
/// by what the pool can pay.
#[test]
fn barren_moor_enters_tapped_cycles_itself_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(4127, swamp())
        .battlefield(0, &[swamp()])
        .hand(0, &[barren_moor(), barren_moor()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // {B} for the cycling, tapped before anything is claimed: both Moors are
    // still in hand, so the one Swamp is the whole of this board's mana.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Swamp and no other source on the table"
    );

    // The land drop, and the first printed line: a replacement effect (CR
    // 614.1), so the permanent is tapped the instant it arrives.
    let land = play_land(&mut engine, p0, barren_moor());
    assert!(entered_tapped(&engine, land), "it enters tapped");
    assert!(
        on_battlefield(&engine, p0, barren_moor()).is_some(),
        "and it is on the battlefield under the seat that played it"
    );

    // Cycling {B}, off the other copy, out of hand (CR 702.29a): the card is
    // discarded as a cost and the draw is the whole of the effect. Ability 1
    // is the cycling line; ability 0 is the mana line and is no hand
    // activation at all.
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    activate(&mut engine, p0, barren_moor(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{B}} went into paying for the cycling"
    );
    assert!(
        in_graveyard(&engine, p0, barren_moor()).is_some(),
        "the cycled card was discarded, and a discarded card goes to its \
         owner's graveyard"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\" — one card off the top of the library"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card discarded and one drawn: a draw that emptied the library \
         without ever filling the hand would satisfy the count above"
    );

    // A turn later the permanent that was played is standing, and the printed
    // `{T}: Add {B}` is read off it and nothing else — the Swamp is left
    // untouched on purpose, since a pool that could have come from either
    // source would say nothing about the land that entered tapped.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, land),
        "the untap step ran: the land that arrived tapped is back up"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the pool is empty, so the black about to appear has one source"
    );

    let the_swamp =
        on_battlefield(&engine, p0, swamp()).expect("the Swamp is still on the battlefield");
    activate(&mut engine, p0, barren_moor(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "{{T}}: Add {{B}}, off the land that was played"
    );
    assert!(is_tapped(&engine, land), "paid with its own {{T}}");
    assert!(
        !is_tapped(&engine, the_swamp),
        "and the Swamp beside it never moved"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: the mana ability uses no stack"
    );
}
