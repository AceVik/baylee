//! `cards/lands/planets/kavaron_memorial_world.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The land enters tapped and its ordinary mana ability adds red mana.
#[test]
fn kavaron_memorial_world_enters_tapped_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(219, forest())
        .hand(0, &[kavaron_memorial_world()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, kavaron_memorial_world());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, kavaron_memorial_world(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn kavaron_stations_by_power_then_unlocks_its_robot_and_team_pump() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut engine = kit::game(
        940,
        &[
            kavaron_memorial_world(),
            serra_angel(),
            serra_angel(),
            serra_angel(),
            forest(),
        ],
        &[],
        &[],
        &[],
    );
    let land = kit::obj(&engine, kit::P0, kavaron_memorial_world());
    let angels = all_on_battlefield(&engine, kit::P0, serra_angel());
    kit::float(&mut engine, kit::P0, &[(ManaColor::Red, 2)]);
    assert!(!kit::offered(&engine, kavaron_memorial_world(), kit::P0, 1));
    for (n, angel) in angels.iter().copied().enumerate() {
        activate(&mut engine, kit::P0, kavaron_memorial_world(), 2);
        engine
            .apply(
                kit::P0,
                PlayerAction::ChooseObjects {
                    objects: vec![angel],
                },
            )
            .unwrap();
        kit::settle(&mut engine);
        assert_eq!(
            counters_on(&engine, land, CounterKind::Charge),
            (n as u16 + 1) * 4
        );
    }
    let forest = kit::obj(&engine, kit::P0, forest());
    activate(&mut engine, kit::P0, kavaron_memorial_world(), 1);
    engine
        .apply(
            kit::P0,
            PlayerAction::ChooseObjects {
                objects: vec![forest],
            },
        )
        .unwrap();
    kit::settle(&mut engine);
    let robot = *engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .last()
        .unwrap();
    for creature in angels.into_iter().chain([robot]) {
        assert!(keywords(&engine, creature).contains(KeywordSet::HASTE));
    }
    assert_eq!(
        pt(&engine, robot),
        (3, 2),
        "the newly created Robot receives the pump too"
    );
}
