//! `cards/lands/check/castle_ardenvale.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Castle Ardenvale` enters tapped unless its controller controls a Plains, taps for `{{W}}`,
/// and creates a 1/1 white Human creature token for `{{2}}{{W}}{{W}}, {{T}}` under `Coverage::Implemented`.
/// Controlling a Plains allows it to enter untapped, where activating its token ability consumes four
/// floating white mana and leaves the land tapped beside the new token.
#[test]
fn castle_ardenvale_enters_untapped_with_plains_and_creates_human_token() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1901, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[castle_ardenvale()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, castle_ardenvale());
    assert!(!entered_tapped(&engine, land));

    tap_all_mana_but(&mut engine, p0, Some(castle_ardenvale()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);

    activate(&mut engine, p0, castle_ardenvale(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, land));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(pt(&engine, tokens[0]), (1, 1));
}
