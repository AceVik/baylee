//! `cards/artifacts/mv_2/rishadan_pawnshop.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "9eef1d16-4405-40e0-8b8b-008e08aac4ac"
fn rishadan_pawnshop() -> CardIndex {
    card_index("9eef1d16-4405-40e0-8b8b-008e08aac4ac")
}

/// `Rishadan Pawnshop` (`Coverage::Generated`):
/// "{2}, {T}: Shuffle target nontoken permanent you control into its owner's library."
///
/// Verifies that activating `Rishadan Pawnshop` shuffles a nontoken permanent
/// you control into its owner's library and increases your library size by 1.
#[test]
fn rishadan_pawnshop_shuffles_target_nontoken_permanent_you_control_into_library() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9601, plains())
        .battlefield(
            0,
            &[
                rishadan_pawnshop(),
                index::GRIZZLY_BEARS,
                plains(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let pawnshop = on_battlefield(&engine, p0, rishadan_pawnshop()).expect("P0 Rishadan Pawnshop");
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).expect("P0 Grizzly Bears");
    let lib_before = library_size(&engine, p0);

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: pawnshop,
                ability_index: 0,
            },
        )
        .unwrap();
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

    assert!(
        on_battlefield(&engine, p0, index::GRIZZLY_BEARS).is_none(),
        "Grizzly Bears was removed from the battlefield"
    );
    assert_eq!(
        library_size(&engine, p0),
        lib_before + 1,
        "Grizzly Bears was shuffled into its owner's library"
    );
}
