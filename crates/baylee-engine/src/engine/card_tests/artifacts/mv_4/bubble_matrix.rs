//! `cards/artifacts/mv_4/bubble_matrix.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn bubble_matrix() -> CardIndex {
    card_index("50261dca-bfeb-40b3-95b6-9057491ef3ad")
}

/// `Bubble Matrix` ({4}, Artifact) prevents all damage that would be dealt to
/// creatures.
#[test]
fn bubble_matrix_prevents_all_damage_to_creatures() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[bubble_matrix(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, bubble_matrix());
    pass_until(&mut engine, stack_is_empty);

    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, GRIZZLY_BEARS).is_some());
}
