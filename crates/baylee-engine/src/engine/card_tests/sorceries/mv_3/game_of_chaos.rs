//! `cards/sorceries/mv_3/game_of_chaos.rs`, played.
//!
//! Oracle:
//! - "Flip a coin. If you win the flip, you gain 1 life and target opponent loses 1 life, and you decide whether to flip again. If you lose the flip, you lose 1 life and that opponent gains 1 life, and that player decides whether to flip again. Double the life stakes with each flip."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::event::GameEvent;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn game_of_chaos() -> CardIndex {
    card_index("b6756608-a618-4e26-a090-ea8fc992def1")
}

/// Every coin flip the journal holds, in order, as (who flipped, won).
fn flips(engine: &Engine<RegistryLookup>) -> Vec<(PlayerId, bool)> {
    engine
        .state()
        .journal
        .entries()
        .iter()
        .filter_map(|e| match e.event {
            GameEvent::CoinFlipped { player, won } => Some((player, won)),
            _ => None,
        })
        .collect()
}

/// The branch is the one the flip names (CR 705.2): won, the caster gains 1
/// and the targeted opponent loses 1; lost, the other way round. Several
/// seeds, so that both branches are played; the card is Partial and flips
/// once, offering nobody a second flip.
#[test]
fn game_of_chaos_flips_coin_to_transfer_one_life_with_target_opponent() {
    let (mut won_seen, mut lost_seen) = (false, false);
    for seed in 0..16_u64 {
        let mut engine = lk::game(seed, &[], &[], &[game_of_chaos()], &[]);

        lk::float(&mut engine, P0, &[(ManaColor::Red, 3)]);
        lk::cast_modal(&mut engine, P0, game_of_chaos(), 0, 0, &[], &[P1]);

        let seen = flips(&engine);
        assert_eq!(seen.len(), 1, "seed {seed}: one flip, {seen:?}");
        let (flipper, won) = seen[0];
        assert_eq!(flipper, P0, "the caster flips");
        let pair = (lk::life(&engine, P0), lk::life(&engine, P1));
        if won {
            won_seen = true;
            assert_eq!(
                pair,
                (21, 19),
                "seed {seed}: won, you gain 1 and they lose 1"
            );
        } else {
            lost_seen = true;
            assert_eq!(
                pair,
                (19, 21),
                "seed {seed}: lost, you lose 1 and they gain 1"
            );
        }
    }
    assert!(won_seen && lost_seen, "both branches were played");
}
