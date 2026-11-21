//! `cards/instants/mv_4/chill_to_the_bone.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "bcc53728-3f6e-48cf-a042-393a2d8098fc"
fn chill_to_the_bone() -> CardIndex {
    card_index("bcc53728-3f6e-48cf-a042-393a2d8098fc")
}

/// Chill to the Bone ({3}{B}): destroys target nonsnow creature, excluding
/// snow creatures from its target menu.
#[test]
fn chill_to_the_bone_destroys_nonsnow_creature_and_excludes_snow_creature() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS, index::FROST_AUGUR],
        &[chill_to_the_bone()],
        &[],
    );
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let snow_creature = lk::obj(&engine, P1, index::FROST_AUGUR);

    lk::float(&mut engine, P0, &[(ManaColor::Black, 4)]);
    cast_with_floating(&mut engine, P0, chill_to_the_bone());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(
        options,
        vec![bears],
        "snow creature {snow_creature:?} is not a legal target"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
    assert!(on_battlefield(&engine, P1, index::FROST_AUGUR).is_some());
}
