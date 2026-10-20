//! `cards/artifacts/mv_3/goblin_lyre.rs`, played.
//!
//! Oracle:
//! - "Sacrifice this artifact: Flip a coin. If you win the flip, this artifact deals damage to target opponent or planeswalker equal to the number of creatures you control. If you lose the flip, this artifact deals damage to you equal to the number of creatures that opponent or that planeswalker's controller controls."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn goblin_lyre() -> CardIndex {
    card_index("0709e9c8-dbe7-4806-ae75-d2e2d745d6a4")
}

#[test]
fn goblin_lyre_sacrifices_and_flips_coin_to_deal_creature_count_damage() {
    let mut won_once = false;
    let mut lost_once = false;
    for seed in 1..25 {
        let mut engine = lk::game(
            seed,
            &[goblin_lyre(), index::GRIZZLY_BEARS, index::LLANOWAR_ELVES],
            &[
                index::GRIZZLY_BEARS,
                index::LLANOWAR_ELVES,
                index::WALL_OF_WOOD,
            ],
            &[],
            &[],
        );
        let lyre = lk::obj(&engine, P0, goblin_lyre());
        lk::use_ability(&mut engine, P0, goblin_lyre(), 0, &[], &[P1]);
        assert_eq!(lk::zone_of(&engine, lyre), Zone::Graveyard);
        let p0_life = lk::life(&engine, P0);
        let p1_life = lk::life(&engine, P1);
        if p1_life == 18 && p0_life == 20 {
            won_once = true;
        } else if p0_life == 17 && p1_life == 20 {
            lost_once = true;
        } else {
            panic!("unexpected life totals: p0={p0_life}, p1={p1_life}");
        }
        if won_once && lost_once {
            break;
        }
    }
    assert!(won_once && lost_once);
}
