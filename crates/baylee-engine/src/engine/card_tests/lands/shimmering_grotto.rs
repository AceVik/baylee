//! `cards/lands/shimmering_grotto.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Shimmering Grotto` enters untapped, taps for `{{C}}`, and filters `{{1}}` into any color
/// under `Coverage::Implemented`.
/// With a Forest providing floating mana, activating ability 1 spends the floating mana,
/// prompts for a color choice via `Pending::ChooseColor`, and adds the chosen mana.
#[test]
fn shimmering_grotto_filters_mana_of_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(2309, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[shimmering_grotto()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, shimmering_grotto());
    assert!(!entered_tapped(&engine, land));

    tap_all_mana_but(&mut engine, p0, Some(shimmering_grotto()));
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);

    activate(&mut engine, p0, shimmering_grotto(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseColor prompt, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Red));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    assert!(is_tapped(&engine, land));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
}
