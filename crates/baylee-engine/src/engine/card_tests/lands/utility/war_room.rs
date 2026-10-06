//! `cards/lands/utility/war_room.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// War Room: "{T}: Add {C}." / "{3}, {T}, Pay life equal to the number of colors in your commanders' color identity: Draw a card."
/// Under `Coverage::Partial`, the draw ability with commander-identity life cost is omitted.
/// Activating the implemented mana ability adds {C} to the mana pool and taps War Room.
#[test]
fn war_room_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(113, forest())
        .battlefield(0, &[war_room()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, war_room()).expect("War Room deployed");
    activate(&mut engine, p0, war_room(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}
