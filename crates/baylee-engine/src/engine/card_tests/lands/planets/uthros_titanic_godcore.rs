//! `cards/lands/planets/uthros_titanic_godcore.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The land enters tapped. Without charge counters or a creature to station,
/// only the ordinary blue mana ability is offered after it untaps.
#[test]
fn uthros_titanic_godcore_enters_tapped_and_taps_for_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[uthros_titanic_godcore()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, uthros_titanic_godcore());
    assert!(entered_tapped(&engine, land), "enters tapped");

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "untaps on next turn");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(
        legal.abilities.iter().filter(|(id, _)| *id == land).count(),
        1,
        "only ability 0 is offered because charge counter threshold is not met"
    );

    activate(&mut engine, p0, uthros_titanic_godcore(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
    assert!(is_tapped(&engine, land));
}

#[test]
fn uthros_stations_to_twelve_and_counts_only_your_artifacts() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut engine = kit::game(
        952,
        &[
            uthros_titanic_godcore(),
            llanowar_elves(),
            sol_ring(),
            sol_ring(),
        ],
        &[sol_ring()],
        &[],
        &[],
    );
    let land = kit::obj(&engine, kit::P0, uthros_titanic_godcore());
    let elf = kit::obj(&engine, kit::P0, llanowar_elves());
    crate::replacement::put_counters(
        engine.dev_state_mut(kit::P0).unwrap(),
        land,
        CounterKind::Charge,
        11,
    );
    kit::float(&mut engine, kit::P0, &[(ManaColor::Blue, 1)]);
    assert!(!kit::offered(&engine, uthros_titanic_godcore(), kit::P0, 1));
    activate(&mut engine, kit::P0, uthros_titanic_godcore(), 2);
    engine
        .apply(kit::P0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    kit::settle(&mut engine);
    assert_eq!(counters_on(&engine, land, CounterKind::Charge), 12);
    activate(&mut engine, kit::P0, uthros_titanic_godcore(), 1);
    assert_eq!(kit::pool_of(&engine, kit::P0, ManaColor::Blue), 2);
}

/// "Station only as a sorcery" — but the 12+ ability is a mana ability, which
/// has no timing restriction: with an untapped creature to tap, twelve charge
/// counters and a {U} to pay, Station is on offer in p0's main phase and not
/// in p1's turn, while the 12+ mana ability is on offer in both.
#[test]
fn uthros_station_is_sorcery_timing_only_but_its_twelve_plus_mana_ability_is_not() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut e = kit::game(
        984,
        &[uthros_titanic_godcore(), llanowar_elves(), sol_ring()],
        &[],
        &[],
        &[],
    );
    let planet = kit::obj(&e, kit::P0, uthros_titanic_godcore());
    crate::replacement::put_counters(
        e.dev_state_mut(kit::P0).unwrap(),
        planet,
        CounterKind::Charge,
        12,
    );
    kit::float(&mut e, kit::P0, &[(ManaColor::Blue, 1)]);
    assert!(
        planet_offered(&e, planet, 2),
        "control: Station at sorcery timing"
    );
    assert!(
        planet_offered(&e, planet, 1),
        "control: 12+ at sorcery timing"
    );

    to_instant_timing(&mut e);
    kit::float(&mut e, kit::P0, &[(ManaColor::Blue, 1)]);
    assert!(
        !planet_offered(&e, planet, 2),
        "Station only as a sorcery: not in the opponent's turn"
    );
    assert_activation_refused(&mut e, planet, 2);
    assert!(
        planet_offered(&e, planet, 1),
        "the 12+ ability is a mana ability: on offer at instant timing"
    );
}
