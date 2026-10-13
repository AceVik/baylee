//! `cards/lands/planets/susur_secundi_void_altar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The land enters tapped and its ordinary mana ability adds black mana.
#[test]
fn susur_secundi_void_altar_enters_tapped_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(220, forest())
        .hand(0, &[susur_secundi_void_altar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, susur_secundi_void_altar());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, susur_secundi_void_altar(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn susur_stations_then_draws_the_sacrificed_creatures_power() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut engine = kit::game(
        954,
        &[susur_secundi_void_altar(), serra_angel()],
        &[],
        &[],
        &[],
    );
    let land = kit::obj(&engine, kit::P0, susur_secundi_void_altar());
    let angel = kit::obj(&engine, kit::P0, serra_angel());
    crate::replacement::put_counters(
        engine.dev_state_mut(kit::P0).unwrap(),
        land,
        CounterKind::Charge,
        8,
    );
    kit::float(&mut engine, kit::P0, &[(ManaColor::Black, 2)]);
    assert!(!kit::offered(
        &engine,
        susur_secundi_void_altar(),
        kit::P0,
        2
    ));
    activate(&mut engine, kit::P0, susur_secundi_void_altar(), 1);
    engine
        .apply(
            kit::P0,
            PlayerAction::ChooseObjects {
                objects: vec![angel],
            },
        )
        .unwrap();
    kit::settle(&mut engine);
    assert_eq!(counters_on(&engine, land, CounterKind::Charge), 12);
    let library = library_size(&engine, kit::P0);
    let life = kit::life(&engine, kit::P0);
    activate(&mut engine, kit::P0, susur_secundi_void_altar(), 2);
    engine
        .apply(
            kit::P0,
            PlayerAction::ChooseObjects {
                objects: vec![angel],
            },
        )
        .unwrap();
    kit::settle(&mut engine);
    assert_eq!(kit::life(&engine, kit::P0), life - 2);
    assert_eq!(library_size(&engine, kit::P0), library - 4);
    assert_eq!(engine.state().object(angel).unwrap().zone, Zone::Graveyard);
}

/// "Station only as a sorcery" and "12+ | … Activate only as a sorcery": with
/// an untapped creature to tap or sacrifice, twelve charge counters, the mana
/// and the life, both abilities are on offer in p0's main phase and neither is
/// in p1's turn.
#[test]
fn susur_station_and_its_twelve_plus_ability_are_sorcery_timing_only() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut e = kit::game(
        983,
        &[susur_secundi_void_altar(), serra_angel()],
        &[],
        &[],
        &[],
    );
    let planet = kit::obj(&e, kit::P0, susur_secundi_void_altar());
    crate::replacement::put_counters(
        e.dev_state_mut(kit::P0).unwrap(),
        planet,
        CounterKind::Charge,
        12,
    );
    kit::float(&mut e, kit::P0, &[(ManaColor::Black, 2)]);
    assert!(
        planet_offered(&e, planet, 1),
        "control: Station at sorcery timing"
    );
    assert!(
        planet_offered(&e, planet, 2),
        "control: 12+ at sorcery timing"
    );

    to_instant_timing(&mut e);
    kit::float(&mut e, kit::P0, &[(ManaColor::Black, 2)]);
    assert!(
        !planet_offered(&e, planet, 1),
        "Station only as a sorcery: not in the opponent's turn"
    );
    assert!(
        !planet_offered(&e, planet, 2),
        "the 12+ ability is also sorcery-only, with its counters, mana and creature ready"
    );
    assert_activation_refused(&mut e, planet, 1);
    assert_activation_refused(&mut e, planet, 2);
}
