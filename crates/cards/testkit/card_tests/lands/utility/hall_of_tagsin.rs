//! `cards/lands/utility/hall_of_tagsin.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn hall_of_tagsin_filters_mana_and_offers_powerstone_token_ability() {
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
        legal.abilities.contains(&(hall, 2)),
        "token ability is payable"
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
