//! `cards/lands/utility/hall_of_tagsin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hall of Tagsin prints `{{T}}: Add {{C}}.`, `{{1}}, {{T}}: Add one mana of any color.`, and `{{4}}, {{T}}: Create a tapped Powerstone token.`
///
/// Under `Coverage::Partial`, both mana abilities are implemented while the Powerstone token ability is omitted because tokens cannot enter tapped.
/// With `{{4}}` floating from four `forest()` lands and Hall of Tagsin untapped, ability 2 is not offered in `legal.abilities`.
/// Activating ability 1 filters one floating green mana through `Pending::ChooseColor` into one black mana.
#[test]
fn hall_of_tagsin_filters_mana_and_omits_powerstone_token_ability() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[hall_of_tagsin(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hall =
        on_battlefield(&engine, p0, hall_of_tagsin()).expect("hall of tagsin on battlefield");

    // Float {{4}} from the four Forests while keeping Hall of Tagsin untapped.
    tap_mana_except(&mut engine, p0, hall);
    assert_eq!(engine.state().players[0].mana_pool.total(), 4);
    assert!(!is_tapped(&engine, hall));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(hall, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        legal.abilities.contains(&(hall, 1)),
        "ability 1 ({{1}}, {{T}}: Add one mana of any color) is offered"
    );
    assert!(
        !legal.abilities.contains(&(hall, 2)),
        "ability 2 (Powerstone creation) is omitted under `Coverage::Partial`"
    );

    activate(&mut engine, p0, hall_of_tagsin(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 3);
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert_eq!(pool.total(), 4);
    assert!(is_tapped(&engine, hall));
}
