//! `cards/enchantments/mv_1/root_maze.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, JAYEMDAE_TOME};

fn root_maze() -> CardIndex {
    card_index("2f4abc5e-5aad-455e-b1ec-137d7ac4a10c")
}

/// `Root Maze` ({G}, Enchantment, `Coverage::Implemented`):
/// `Artifacts and lands enter tapped.`
#[test]
fn root_maze_causes_artifacts_and_lands_to_enter_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                root_maze(),
            ],
        )
        .hand(0, &[forest(), JAYEMDAE_TOME, GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land_in_hand = in_hand(&engine, p0, forest()).unwrap();
    engine
        .apply(p0, PlayerAction::PlayLand { card: land_in_hand })
        .unwrap();
    assert!(
        engine
            .state()
            .object(land_in_hand)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    cast_from_hand(&mut engine, p0, JAYEMDAE_TOME);
    pass_until(&mut engine, stack_is_empty);
    let tome = on_battlefield(&engine, p0, JAYEMDAE_TOME).unwrap();
    assert!(
        engine
            .state()
            .object(tome)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );

    // Nonartifact creature enters untapped.
    cast_from_hand(&mut engine, p0, GRIZZLY_BEARS);
    pass_until(&mut engine, stack_is_empty);
    let bears = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    assert!(
        !engine
            .state()
            .object(bears)
            .unwrap()
            .status
            .contains(Status::TAPPED)
    );
}
