//! `cards/lands/planar_nexus.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Planar Nexus prints `This land is every nonbasic land type.`, `{{T}}: Add {{C}}.`,
/// and `{1}, {{T}}: Add one mana of any color.`
///
/// Under `Coverage::Partial`, the nonbasic land type modifier is omitted as the
/// engine does not provide a modifier for that set of subtypes. This test floats
/// green mana from a Forest, activates ability 1 paying `{1}` and tapping the
/// land, chooses blue from `Pending::ChooseColor`, and verifies that one blue mana
/// is added to the pool.
#[test]
fn planar_nexus_filters_mana_into_any_chosen_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[planar_nexus(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana_but(&mut engine, p0, Some(planar_nexus()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );

    activate(&mut engine, p0, planar_nexus(), 1);

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("expected ChooseColor, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert!(options.contains(&ManaColor::Blue));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.available(ManaColor::Green), 0);
    assert_eq!(pool.total(), 1);

    let nexus = on_battlefield(&engine, p0, planar_nexus()).expect("nexus on battlefield");
    assert!(is_tapped(&engine, nexus));
}
