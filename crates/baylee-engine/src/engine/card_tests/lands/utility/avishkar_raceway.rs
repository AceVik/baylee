//! `cards/lands/utility/avishkar_raceway.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Avishkar Raceway prints `Start your engines!`, `{{T}}: Add {{C}}.`, and
/// `Max speed — {{3}}, {{T}}, Discard a card: Draw a card.`
///
/// Under `Coverage::Partial`, the speed mechanic and Max speed gate are omitted due to
/// lacking DSL vocabulary. With three green mana floating from `forest()` lands, Avishkar Raceway
/// standing untapped, and a card in hand to discard, ability index 1 is not offered in
/// `legal.abilities`. Activating ability 0 adds one colorless mana to `pool.available(ManaColor::Colorless)`.
#[test]
fn avishkar_raceway_taps_for_colorless_and_omits_discard_draw_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[avishkar_raceway(), forest(), forest(), forest()])
        .hand(0, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let raceway =
        on_battlefield(&engine, p0, avishkar_raceway()).expect("avishkar raceway on battlefield");

    // Float {3} from the three Forests while keeping Avishkar Raceway untapped.
    tap_mana_except(&mut engine, p0, raceway);
    assert_eq!(engine.state().players[0].mana_pool.total(), 3);
    assert!(!is_tapped(&engine, raceway));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(raceway, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        !legal.abilities.contains(&(raceway, 1)),
        "under `Coverage::Partial`, Max speed discard/draw ability is omitted despite floating {{3}} and card in hand"
    );

    activate(&mut engine, p0, avishkar_raceway(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.available(ManaColor::Green), 3);
    assert_eq!(pool.total(), 4);
    assert!(is_tapped(&engine, raceway));
}
