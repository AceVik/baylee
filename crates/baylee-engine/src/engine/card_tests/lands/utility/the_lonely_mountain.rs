//! `cards/lands/utility/the_lonely_mountain.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Without Equipment the land enters tapped. On the following turn both mana
/// and Dwarf abilities are offered when the full five mana is available.
#[test]
fn the_lonely_mountain_enters_tapped_without_equipment_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[the_lonely_mountain()])
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, the_lonely_mountain());
    assert!(
        entered_tapped(&engine, land),
        "without an Equipment, The Lonely Mountain enters tapped"
    );

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    // Float {{5}} red mana from basic mountains while keeping The Lonely Mountain untapped.
    tap_mana_except(&mut engine, p0, land);
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        5
    );
    assert!(!is_tapped(&engine, land));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(land, 0)),
        "ability 0 ({{T}}: Add {{R}}) is offered"
    );
    assert!(
        legal.abilities.contains(&(land, 1)),
        "the Dwarf ability is affordable at its full cost without Equipment"
    );

    activate(&mut engine, p0, the_lonely_mountain(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 6);
    assert_eq!(pool.total(), 6);
    assert!(is_tapped(&engine, land));
}

#[test]
fn lonely_mountain_creates_a_dwarf_with_the_equipment_discount() {
    use crate::engine::card_tests::legends_kit as kit;
    let mut engine = kit::game(
        936,
        &[
            the_lonely_mountain(),
            lightning_greaves(),
            lightning_greaves(),
            sol_ring(),
        ],
        &[lightning_greaves()],
        &[],
        &[],
    );
    kit::float(&mut engine, kit::P0, &[(ManaColor::Red, 2)]);
    assert!(!kit::offered(&engine, the_lonely_mountain(), kit::P0, 1));
    kit::float(&mut engine, kit::P0, &[(ManaColor::Red, 1)]);
    kit::use_ability(&mut engine, kit::P0, the_lonely_mountain(), 1, &[], &[]);
    assert_eq!(kit::pool_total(&engine, kit::P0), 0);
    let dwarf = *engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .last()
        .unwrap();
    assert_eq!(pt(&engine, dwarf), (2, 2));
    assert!(
        engine
            .state()
            .object(dwarf)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::DWARF)
    );
}
