//! `cards/instants/mv_1/vendetta.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "50a16634-ebf7-477f-ad6f-23332599f838"
fn vendetta() -> CardIndex {
    card_index("50a16634-ebf7-477f-ad6f-23332599f838")
}

/// `Vendetta` (`Coverage::Implemented`):
/// "Destroy target nonblack creature. It can't be regenerated. You lose life equal to that creature's toughness."
#[test]
fn vendetta_destroys_nonblack_creature_and_loses_life_equal_to_toughness() {
    let mut engine = lk::game(
        SEED,
        &[index::SWAMP],
        &[index::GIANT_SPIDER, index::SCATHE_ZOMBIES],
        &[vendetta()],
        &[],
    );
    let spider = lk::obj(&engine, P1, index::GIANT_SPIDER); // 2/4 green
    let zombie = lk::obj(&engine, P1, index::SCATHE_ZOMBIES); // 2/2 black

    cast_from_hand(&mut engine, P0, vendetta());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("Vendetta asks for a target, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&spider),
        "Nonblack creature is a legal target"
    );
    assert!(
        !options.contains(&zombie),
        "Black creature is not a legal target for Vendetta"
    );

    lk::aim(&mut engine, &[spider], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, P1, index::GIANT_SPIDER).is_some());
    // Giant Spider has toughness 4, so P0 loses 4 life (20 -> 16).
    assert_eq!(engine.state().players[0].life, 16);
}
