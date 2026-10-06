//! `cards/lands/utility/eiganjo_seat_of_the_empire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Eiganjo, Seat of the Empire prints `{{T}}: Add {{W}}.` and `Channel — {{2}}{{W}}, Discard
/// this card: It deals 4 damage to target attacking or blocking creature. This ability costs
/// {{1}} less to activate for each legendary creature you control.`
///
/// Under `Coverage::Partial`, the Channel ability from hand is omitted because the engine
/// carries no attacking/blocking filter or activation cost reduction. With `{{2}}{{W}}` floating
/// from two `plains()` and a `forest()`, Eiganjo in hand offers no activated abilities in
/// `legal.abilities`. Playing the card as an untapped legendary land allows immediately activating
/// ability index 0 for one additional white mana.
#[test]
fn eiganjo_seat_of_the_empire_taps_for_white_and_omits_channel() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[eiganjo_seat_of_the_empire()])
        .battlefield(0, &[plains(), plains(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Float {2}{W} from the lands on the battlefield.
    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 2);
    assert_eq!(pool.available(ManaColor::Green), 1);

    let card = in_hand(&engine, p0, eiganjo_seat_of_the_empire()).expect("eiganjo in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == card),
        "under `Coverage::Partial`, Channel ability is omitted despite floating {{2}}{{W}}"
    );

    let land = play_land(&mut engine, p0, eiganjo_seat_of_the_empire());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, eiganjo_seat_of_the_empire(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 3);
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert_eq!(pool.total(), 4);
    assert!(is_tapped(&engine, land));
}
