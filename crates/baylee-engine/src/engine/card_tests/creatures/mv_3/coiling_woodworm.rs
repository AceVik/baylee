//! `cards/creatures/mv_3/coiling_woodworm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

/// Coiling Woodworm ({2}{G} */1 Creature — Insect Worm):
/// "Coiling Woodworm's power is equal to the number of Forests on the battlefield."
#[test]
fn coiling_woodworm_power_counts_forests_controlled_by_all_players() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[index::COILING_WOODWORM, forest(), forest()])
        .hand(0, &[forest()])
        .battlefield(1, &[forest(), mountain()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let worm = on_battlefield(&engine, P0, index::COILING_WOODWORM).unwrap();
    // 2 Forests on P0's side + 1 Forest on P1's side = 3/1 (Mountain is ignored).
    assert_eq!(pt(&engine, worm), (3, 1));

    let land = in_hand(&engine, P0, forest()).unwrap();
    engine
        .apply(P0, PlayerAction::PlayLand { card: land })
        .unwrap();
    assert_eq!(pt(&engine, worm), (4, 1));
}
