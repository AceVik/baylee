//! `cards/enchantments/mv_2/sacred_ground.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn sacred_ground() -> CardIndex {
    card_index("b18e773c-611b-4c18-8b2d-8d3a7e5ddc93")
}

/// Sacred Ground (`b18e773c-611b-4c18-8b2d-8d3a7e5ddc93`) — {1}{W} Enchantment (Partial):
/// "Whenever a spell or ability an opponent controls causes a land to be put into your graveyard from the battlefield, return that card to the battlefield."
#[test]
fn sacred_ground_returns_destroyed_owned_land_to_battlefield() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                sacred_ground(),
                forest(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[index::STONE_RAIN])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let target_land = on_battlefield(&engine, P0, forest()).unwrap();
    cast_from_hand(&mut engine, P0, index::STONE_RAIN);
    aim_at(&mut engine, P0, target_land);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, forest()).is_some(),
        "Sacred Ground should return the destroyed Forest to the battlefield"
    );
}
