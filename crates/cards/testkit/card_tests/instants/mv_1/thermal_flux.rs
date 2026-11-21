//! `cards/instants/mv_1/thermal_flux.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_engine::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "90fe98df-4f87-49ff-b562-0ed51d1d3a1c"
fn thermal_flux() -> CardIndex {
    card_index("90fe98df-4f87-49ff-b562-0ed51d1d3a1c")
}

fn is_snow(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    engine
        .state()
        .object(id)
        .unwrap()
        .characteristics()
        .supertypes
        .contains(SupertypeSet::SNOW)
}

/// Thermal Flux ({U}): Mode 0 makes target nonsnow permanent snow until EOT
/// and draws on next turn's upkeep; Mode 1 removes snow from target snow
/// permanent until EOT and draws on next turn's upkeep.
#[test]
fn thermal_flux_plays_both_snow_modes_until_eot_and_draws_next_upkeep() {
    let mut engine = lk::game(
        SEED,
        &[
            index::GRIZZLY_BEARS,
            index::HILL_GIANT,
            index::SNOW_COVERED_FOREST,
        ],
        &[],
        &[thermal_flux(), thermal_flux()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    let snow_forest = lk::obj(&engine, P0, index::SNOW_COVERED_FOREST);
    assert!(!is_snow(&engine, bears) && is_snow(&engine, snow_forest));

    // Mode 0: nonsnow Bears becomes snow until EOT.
    lk::float(&mut engine, P0, &[(ManaColor::Blue, 2)]);
    cast_with_floating(&mut engine, P0, thermal_flux());
    let m0 = choose_cast_kind(&engine, CastModeKind::Mode(0));
    engine.apply(P0, PlayerAction::ChooseMode(m0)).unwrap();
    let menu0 = aim_at(&mut engine, P0, bears);
    assert!(menu0.contains(&bears) && !menu0.contains(&snow_forest));
    lk::settle(&mut engine);
    assert!(is_snow(&engine, bears), "Bears became snow until EOT");

    // Mode 1: snow Forest loses snow until EOT (excludes still-nonsnow Giant).
    cast_with_floating(&mut engine, P0, thermal_flux());
    let m1 = choose_cast_kind(&engine, CastModeKind::Mode(1));
    engine.apply(P0, PlayerAction::ChooseMode(m1)).unwrap();
    let menu1 = aim_at(&mut engine, P0, snow_forest);
    assert!(menu1.contains(&snow_forest) && menu1.contains(&bears) && !menu1.contains(&giant));
    lk::settle(&mut engine);
    assert!(!is_snow(&engine, snow_forest), "Forest lost snow until EOT");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        0,
        "neither copy draws immediately"
    );

    // Next turn's upkeep: both snow changes have expired and both delayed draws fire.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);

    assert!(!is_snow(&engine, bears), "Bears snow supertype expired");
    assert!(is_snow(&engine, snow_forest), "Forest is snow again");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        2,
        "both delayed upkeep triggers drew a card"
    );
}
