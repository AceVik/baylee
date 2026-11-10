//! `cards/lands/utility/daily_bugle_building.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Daily Bugle Building prints `{T}: Add {C}`, `{1}, {T}: Add one mana of any color`, and `Smear Campaign —
/// {1}, {T}: Target legendary creature gains menace until end of turn. Activate only as a sorcery.`
/// The card is marked `Coverage::Implemented`.
/// With one floating mana from a `forest` and controlled `thorin_oakenshield` on the battlefield, activating
/// ability index 2 grants menace to the legendary creature until end of turn.
#[test]
fn daily_bugle_building_grants_menace_to_target_legendary_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), thorin_oakenshield()])
        .hand(0, &[daily_bugle_building()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let building = play_land(&mut engine, p0, daily_bugle_building());
    let thorin = on_battlefield(&engine, p0, thorin_oakenshield()).expect("thorin on battlefield");
    assert!(!keywords(&engine, thorin).contains(KeywordSet::MENACE));

    tap_all_mana_but(&mut engine, p0, Some(daily_bugle_building()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1
    );

    activate(&mut engine, p0, daily_bugle_building(), 2);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice");
    };
    assert!(options.contains(&thorin));

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![thorin],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, thorin).contains(KeywordSet::MENACE));
    assert!(is_tapped(&engine, building));
}
