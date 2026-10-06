//! `cards/lands/utility/mariposa_military_base.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mariposa Military Base prints `You may have this land enter tapped. If you do, you get two rad counters`,
/// `{T}: Add {C}`, and `{5}, {T}: Draw a card. This ability costs {1} less to activate for each rad counter you have.`
/// The card is marked `Coverage::Partial` because player counters and cost reductions per rad counter are unsupported.
/// Mariposa Military Base enters untapped, and floating five mana allows activating ability index 1
/// for `{5}, {T}` to draw a card off the library.
#[test]
fn mariposa_military_base_enters_untapped_and_draws_card_for_five_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[mariposa_military_base()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let base = play_land(&mut engine, p0, mariposa_military_base());
    assert!(!is_tapped(&engine, base));

    tap_all_mana_but(&mut engine, p0, Some(mariposa_military_base()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        5
    );

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    activate(&mut engine, p0, mariposa_military_base(), 1);
    assert!(is_tapped(&engine, base));

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
