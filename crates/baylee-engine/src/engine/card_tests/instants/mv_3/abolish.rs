//! `cards/instants/mv_3/abolish.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn abolish() -> CardIndex {
    card_index("d6d7faa8-ecc9-408b-9c46-a0f62c74f567")
}

#[test]
fn abolish_destroys_target_artifact_and_enchantment() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[abolish(), abolish()])
        .battlefield(0, &[plains(); 6])
        .battlefield(1, &[index::SOL_RING, index::CRUSADE, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ring = on_battlefield(&engine, p1, index::SOL_RING).unwrap();
    let crusade = on_battlefield(&engine, p1, index::CRUSADE).unwrap();
    let bears = on_battlefield(&engine, p1, index::GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, abolish());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets");
    };
    assert!(
        !options.contains(&bears),
        "Abolish must not target a non-artifact, non-enchantment creature"
    );
    assert!(options.contains(&ring) && options.contains(&crusade));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ring],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, index::SOL_RING).is_some());

    cast_from_hand(&mut engine, p0, abolish());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![crusade],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, index::CRUSADE).is_some());
}
