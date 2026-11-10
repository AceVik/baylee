//! `cards/lands/cycling/secluded_steppe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Secluded Steppe prints three sentences: it enters tapped, it taps for {W},
/// and it cycles for {W}. The scenario plays one copy as the turn's real land
/// drop and cycles the *other* copy off the single Plains beside it, so both
/// halves are read off one board — a `PlayLand` is what puts an enter modifier
/// to work, where `starting_battlefield` would place the permanent with no
/// replacement looking at it, and the cycled copy separately proves which of
/// the two cards the discard cost took. The white mana is asserted in the pool
/// before the cycle, because a `{W}` ability is filtered out of the offer
/// while nothing is floating and its absence would read as an unimplemented
/// card rather than as an untapped Plains.
#[test]
fn secluded_steppe_enters_tapped_and_cycles_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(88, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[secluded_steppe(), secluded_steppe()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The land drop, played rather than seeded: `EnterModifier::Tapped` is
    // read by the entry, and a board built through `starting_battlefield`
    // never runs one.
    let played = play_land(&mut engine, p0, secluded_steppe());
    assert_eq!(
        engine.state().object(played).map(|o| o.face_index),
        Some(0),
        "the front face is the one that came down"
    );
    assert!(entered_tapped(&engine, played), "this land enters tapped");

    // One Plains is the whole source of the price: the Steppe that just
    // arrived is down, so anything floating here came off the land beside it.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1,
        "one white, off the Plains alone — the Steppe that entered tapped gave nothing"
    );

    // Ability 1 is the printed `Cycling {W}` (ability 0 is the mana line).
    // With the mana floating the offer carries it; the discard is
    // `DiscardSelf`, so no question is asked about which card pays.
    let library_before = library_size(&engine, p0);
    activate(&mut engine, p0, secluded_steppe(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, secluded_steppe()).is_some(),
        "`Discard this card` is a cost, so the cycled copy is buried"
    );
    assert!(
        on_battlefield(&engine, p0, secluded_steppe()).is_some(),
        "and the copy played as the land drop is untouched — one Steppe, not both"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{W}} was spent paying for the cycle"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "`Draw a card` — exactly one card off the library"
    );
}
