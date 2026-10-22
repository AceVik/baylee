//! `cards/artifacts/mv_2/altar_of_dementia.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn altar_of_dementia() -> CardIndex {
    card_index("d64e9152-ef24-4394-aeb0-9c3befc56549")
}

/// Altar of Dementia (d64e9152-ef24-4394-aeb0-9c3befc56549) — {2} Artifact:
/// "Sacrifice a creature: Target player mills cards equal to the sacrificed creature's power."
#[test]
fn altar_of_dementia_sacrifices_creature_to_mill_cards_equal_to_its_power() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[altar_of_dementia(), index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let giant = on_battlefield(&engine, P0, index::HILL_GIANT).unwrap();
    let lib_before = library_size(&engine, P1);

    activate(&mut engine, P0, altar_of_dementia(), 0);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P0, index::HILL_GIANT).is_some());
    assert_eq!(library_size(&engine, P1), lib_before - 3);
}
