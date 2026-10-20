//! `cards/creatures/mv_3/gangrenous_zombies.rs`, played.
//!
//! Oracle:
//! - "{T}, Sacrifice this creature: This creature deals 1 damage to each creature and each player. If you control a snow Swamp, this creature deals 2 damage to each creature and each player instead."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn gangrenous_zombies() -> CardIndex {
    card_index("d0d844f2-6cf4-49f2-95db-64e932a15b17")
}

#[test]
fn gangrenous_zombies_deals_one_damage_or_two_with_snow_swamp() {
    // Without snow Swamp: 1 damage to each creature and each player.
    {
        let mut engine = lk::game(
            SEED,
            &[gangrenous_zombies()],
            &[index::GRIZZLY_BEARS],
            &[],
            &[],
        );
        let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
        lk::use_ability(&mut engine, P0, gangrenous_zombies(), 0, &[], &[]);
        assert_eq!(lk::life(&engine, P0), 19);
        assert_eq!(lk::life(&engine, P1), 19);
        assert_eq!(lk::damage_on(&engine, bears), 1);
    }

    // With snow Swamp: 2 damage to each creature and each player.
    {
        let mut engine = lk::game(
            SEED,
            &[gangrenous_zombies(), index::SNOW_COVERED_SWAMP],
            &[index::GRIZZLY_BEARS],
            &[],
            &[],
        );
        let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
        lk::use_ability(&mut engine, P0, gangrenous_zombies(), 0, &[], &[]);
        assert_eq!(lk::life(&engine, P0), 18);
        assert_eq!(lk::life(&engine, P1), 18);
        assert_eq!(lk::zone_of(&engine, bears), Zone::Graveyard);
    }
}
