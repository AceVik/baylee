//! `cards/instants/mv_1/skred.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "026318be-e3dd-4d5c-bd77-53ce23a2e7cc"
fn skred() -> CardIndex {
    card_index("026318be-e3dd-4d5c-bd77-53ce23a2e7cc")
}

/// Skred ({R}): deals damage to target creature equal to the number of snow
/// permanents YOU control (counting snow lands and snow creatures, ignoring
/// nonsnow permanents and opponent's snow permanents).
#[test]
fn skred_deals_damage_equal_to_your_snow_permanents_to_target_creature() {
    let mut engine = lk::game(
        SEED,
        &[
            index::SNOW_COVERED_MOUNTAIN,
            index::SNOW_COVERED_MOUNTAIN,
            index::FROST_AUGUR,
            mountain(),
        ],
        &[
            index::SNOW_COVERED_FOREST,
            index::SNOW_COVERED_FOREST,
            index::SERRA_ANGEL,
        ],
        &[skred()],
        &[],
    );
    let angel = lk::obj(&engine, P1, index::SERRA_ANGEL);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut engine, P0, skred());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(
        player_options.is_empty(),
        "Skred targets creatures only, not players"
    );
    assert!(options.contains(&angel));
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![angel],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(
        engine.state().object(angel).unwrap().damage,
        3,
        "3 own snow permanents (2 Snow-Covered Mountains + Frost Augur) -> 3 damage"
    );
}
