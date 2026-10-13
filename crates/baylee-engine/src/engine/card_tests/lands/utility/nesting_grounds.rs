//! `cards/lands/utility/nesting_grounds.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nesting Grounds: "{T}: Add {C}." / "{1}, {T}: Move a counter from target permanent you control onto a second target permanent. Activate only as a sorcery."
/// Activating ability 0 adds one colorless mana to the pool and taps the land.
#[test]
fn nesting_grounds_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(133, forest())
        .battlefield(0, &[nesting_grounds()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, nesting_grounds()).expect("Nesting Grounds deployed");
    activate(&mut engine, p0, nesting_grounds(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

/// "{1}, {T}: Move a counter from target permanent you control onto a second
/// target permanent. Activate only as a sorcery." With a counter to move and
/// the {1} floating, the move is on offer in the own main phase and in
/// neither the stack-holding window nor the opponent's turn.
#[test]
fn nesting_grounds_counter_move_is_sorcery_timing_only() {
    use crate::engine::card_tests::legends_kit as kit;
    use baylee_core::generated::index;
    let mut e = kit::game(
        951,
        &[nesting_grounds(), index::GRIZZLY_BEARS, index::SOL_RING],
        &[],
        &[llanowar_elves()],
        &[],
    );
    let grounds = kit::obj(&e, kit::P0, nesting_grounds());
    let bear = kit::obj(&e, kit::P0, index::GRIZZLY_BEARS);
    crate::replacement::put_counters(
        e.dev_state_mut(kit::P0).unwrap(),
        bear,
        CounterKind::Charge,
        1,
    );
    assert_sorcery_timing_only(&mut e, grounds, 1, &[(ManaColor::Colorless, 1)]);
}
