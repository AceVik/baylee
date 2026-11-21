//! `cards/instants/mv_2/early_frost.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "9f9cdbc2-d2f2-4390-8506-350fabf3f89a"
fn early_frost() -> CardIndex {
    card_index("9f9cdbc2-d2f2-4390-8506-350fabf3f89a")
}

/// Early Frost ({1}{U}): targets ONLY lands (excluding creatures and
/// artifacts), allows choosing 0..=3 lands, and taps every chosen land.
#[test]
fn early_frost_taps_up_to_three_target_lands_only() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[
            plains(),
            forest(),
            mountain(),
            swamp(),
            index::GRIZZLY_BEARS,
            sol_ring(),
        ],
        &[early_frost(), early_frost()],
        &[],
    );
    let l1 = lk::obj(&engine, P1, plains());
    let l2 = lk::obj(&engine, P1, forest());
    let l3 = lk::obj(&engine, P1, mountain());
    let l4 = lk::obj(&engine, P1, swamp());
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let ring = lk::obj(&engine, P1, sol_ring());

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 4)]);
    cast_with_floating(&mut engine, P0, early_frost());
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!((min, max), (0, 3), "up to three target lands");
    assert!(options.contains(&l1) && options.contains(&l2) && options.contains(&l3));
    assert!(
        !options.contains(&bears) && !options.contains(&ring),
        "nonlands excluded: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![l1, l2, l3],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert!(is_tapped(&engine, l1) && is_tapped(&engine, l2) && is_tapped(&engine, l3));
    assert!(!is_tapped(&engine, l4), "fourth land untouched");

    // Choosing 0 targets is also legal ("up to three").
    cast_with_floating(&mut engine, P0, early_frost());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);
    assert!(!is_tapped(&engine, l4));
}
