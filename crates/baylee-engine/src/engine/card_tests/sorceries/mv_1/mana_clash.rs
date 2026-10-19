//! `cards/sorceries/mv_1/mana_clash.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::event::GameEvent;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mana_clash() -> CardIndex {
    card_index("a2f5a5fd-14dc-48e8-bdb0-984a83288023")
}

/// Every coin flip the journal holds, in order, as (who, came up heads).
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

/// Seat 0 casts Mana Clash at seat 1 on `seed`, and the spell resolves.
fn clash(seed: u64) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(seed, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[mana_clash()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    tap_all_mana(&mut engine, P0);
    lk::announce(&mut engine, P0, mana_clash(), 0, 0, &[], &[P1]);
    lk::settle(&mut engine);
    engine
}

/// Mana Clash (a2f5a5fd-14dc-48e8-bdb0-984a83288023) — "You and target
/// opponent each flip a coin. Mana Clash deals 1 damage to each player whose
/// coin comes up tails. Repeat this process until both players' coins come
/// up heads on the same flip."
///
/// The coins are random, so what is read is the journal of flips and the life
/// totals against it, over several seeds: the flips come in rounds of one
/// coin each (you, then the opponent); every round but the last has a tails;
/// the last has two heads; and each player lost 1 life per tails of theirs.
#[test]
fn mana_clash_flips_in_rounds_until_both_heads_and_damages_each_tails() {
    let mut longest = 0;
    let mut damaged = false;
    for seed in 1..=16 {
        let engine = clash(seed);
        let flips = flips(&engine);
        assert!(
            !flips.is_empty() && flips.len().is_multiple_of(2),
            "seed {seed}: rounds of two coins"
        );
        let rounds: Vec<(bool, bool)> = flips
            .chunks(2)
            .map(|pair| {
                assert_eq!(pair[0].0, P0, "seed {seed}: you flip first");
                assert_eq!(pair[1].0, P1, "seed {seed}: then the target opponent");
                (pair[0].1, pair[1].1)
            })
            .collect();
        let (last, earlier) = rounds.split_last().expect("at least one round");
        assert_eq!(*last, (true, true), "seed {seed}: it ends on two heads");
        assert!(
            earlier.iter().all(|(a, b)| !(*a && *b)),
            "seed {seed}: no earlier round was two heads"
        );
        let mine = rounds.iter().filter(|(a, _)| !a).count();
        let theirs = rounds.iter().filter(|(_, b)| !b).count();
        assert_eq!(
            lk::life(&engine, P0),
            20 - i32::try_from(mine).unwrap(),
            "seed {seed}"
        );
        assert_eq!(
            lk::life(&engine, P1),
            20 - i32::try_from(theirs).unwrap(),
            "seed {seed}"
        );
        assert!(in_graveyard(&engine, P0, mana_clash()).is_some());
        longest = longest.max(rounds.len());
        damaged |= mine + theirs > 0;
    }
    assert!(longest > 1, "some seed had to repeat the process");
    assert!(damaged, "some seed had a tails, and so some damage");
}
