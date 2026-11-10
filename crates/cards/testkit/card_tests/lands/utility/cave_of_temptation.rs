//! `cards/lands/utility/cave_of_temptation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cave of Temptation prints `{T}: Add {C}`, `{1}, {T}: Add one mana of any color`, and
/// `{4}, {T}, Sacrifice this land: Put two +1/+1 counters on target creature.`
/// The card is marked `Coverage::Implemented`.
/// Activating the mana-filtering ability at index 1 spends one floating green mana and taps the cave,
/// allowing the player to choose any color such as `ManaColor::Blue` and adding it to the pool.
#[test]
fn cave_of_temptation_filters_mana_to_any_color() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[cave_of_temptation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, cave_of_temptation());
    assert!(!is_tapped(&engine, land));

    tap_all_mana_but(&mut engine, p0, Some(cave_of_temptation()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );

    activate(&mut engine, p0, cave_of_temptation(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice");
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 0);
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, land));
}
