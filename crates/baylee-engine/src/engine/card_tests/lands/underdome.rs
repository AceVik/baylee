//! `cards/lands/underdome.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Underdome: "{T}: Add {C}." / "{T}: Add one mana of any color. Spend this mana only to pay Un-costs."
/// The first ability produces unrestricted colorless mana. The second offers
/// all five colors and produces one separately tracked restricted unit.
#[test]
fn underdome_taps_for_colorless_or_restricted_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(120, forest())
        .battlefield(0, &[underdome()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, underdome()).expect("Underdome deployed");

    activate(&mut engine, p0, underdome(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));

    engine.state.set_tapped(land, false);
    engine.refresh_offer();
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority")
    };
    assert!(
        legal.abilities.contains(&(land, 1)),
        "the Un-cost mana action is offered when untapped"
    );
    activate(&mut engine, p0, underdome(), 1);
    let Pending::ChooseColor { options, .. } = engine.pending() else {
        panic!("any color is asked")
    };
    assert_eq!(options.len(), 5);
    assert!(!options.contains(&ManaColor::Colorless));
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    let pool = &engine.state.players[0].mana_pool;
    assert_eq!(pool.total(), 2);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "the blue unit has a spending restriction"
    );
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Blue);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert!(is_tapped(&engine, land));
}
