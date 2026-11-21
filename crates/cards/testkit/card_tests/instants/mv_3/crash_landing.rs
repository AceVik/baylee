//! `cards/instants/mv_3/crash_landing.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "5d36e5ee-1e23-48fe-ad13-081cc153342b"
fn crash_landing() -> CardIndex {
    card_index("5d36e5ee-1e23-48fe-ad13-081cc153342b")
}

/// Crash Landing ({2}{G}): targets ONLY a creature with flying (excluding
/// non-flyers), removes flying until EOT, and deals damage equal to the number
/// of Forests YOU control (including Snow-Covered Forest, ignoring opponent's).
#[test]
fn crash_landing_grounds_flyer_and_deals_damage_by_your_forest_count() {
    let mut engine = lk::game(
        SEED,
        &[forest(), forest(), index::SNOW_COVERED_FOREST, mountain()],
        &[forest(), forest(), index::SERRA_ANGEL, index::GRIZZLY_BEARS],
        &[crash_landing()],
        &[],
    );
    let angel = lk::obj(&engine, P1, index::SERRA_ANGEL);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    assert!(keywords(&engine, angel).contains(KeywordSet::FLYING));

    lk::float(&mut engine, P0, &[(ManaColor::Green, 3)]);
    cast_with_floating(&mut engine, P0, crash_landing());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(
        options,
        vec![angel],
        "only creature with flying is legal (excluded ground {bears:?})"
    );
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

    assert!(
        !keywords(&engine, angel).contains(KeywordSet::FLYING),
        "Serra Angel lost flying until EOT"
    );
    assert_eq!(
        engine.state().object(angel).unwrap().damage,
        3,
        "dealt 3 damage for 3 own Forests (2 Forest + 1 Snow-Covered Forest)"
    );

    reach_their_main_phase(&mut engine, P1);
    assert!(
        keywords(&engine, angel).contains(KeywordSet::FLYING),
        "Serra Angel regained flying after EOT"
    );
}
