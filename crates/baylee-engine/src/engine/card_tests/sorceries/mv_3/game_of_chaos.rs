//! `cards/sorceries/mv_3/game_of_chaos.rs`, played.
//!
//! Oracle:
//! - "Flip a coin. If you win the flip, you gain 1 life and target opponent loses 1 life, and you decide whether to flip again. If you lose the flip, you lose 1 life and that opponent gains 1 life, and that player decides whether to flip again. Double the life stakes with each flip."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn game_of_chaos() -> CardIndex {
    card_index("b6756608-a618-4e26-a090-ea8fc992def1")
}

#[test]
fn game_of_chaos_flips_coin_to_transfer_one_life_with_target_opponent() {
    let mut engine = lk::game(SEED, &[], &[], &[game_of_chaos()], &[]);

    lk::float(&mut engine, P0, &[(ManaColor::Red, 3)]);
    lk::cast_modal(&mut engine, P0, game_of_chaos(), 0, 0, &[], &[P1]);

    let pair = (lk::life(&engine, P0), lk::life(&engine, P1));
    assert!(
        pair == (21, 19) || pair == (19, 21),
        "expected life totals (21, 19) or (19, 21) after flip, got {pair:?}"
    );
}
