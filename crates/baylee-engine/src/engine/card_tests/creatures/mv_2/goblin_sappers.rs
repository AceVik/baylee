//! `cards/creatures/mv_2/goblin_sappers.rs`, played.
//!
//! Oracle:
//! - "{R}{R}, {T}: Target creature you control can't be blocked this turn. Destroy it and this creature at end of combat."
//! - "{R}{R}{R}{R}, {T}: Target creature you control can't be blocked this turn. Destroy it at end of combat."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn goblin_sappers() -> CardIndex {
    card_index("9e095761-cdae-4ab2-b2d3-099b0b39600a")
}

#[test]
fn goblin_sappers_cheap_ability_destroys_both_at_end_of_combat_expensive_keeps_sappers() {
    // Cheap mode ({R}{R}, {T}): destroys both target creature and Goblin Sappers at end of combat.
    {
        let mut engine = lk::game(
            SEED,
            &[goblin_sappers(), index::GRIZZLY_BEARS],
            &[],
            &[],
            &[],
        );
        let sappers = lk::obj(&engine, P0, goblin_sappers());
        let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
        lk::float(&mut engine, P0, &[(ManaColor::Red, 2)]);
        lk::use_ability(&mut engine, P0, goblin_sappers(), 0, &[bears], &[]);

        let _ = lk::attack(&mut engine, &[bears]);
        lk::block(&mut engine, &[]);
        lk::through_combat(&mut engine);
        assert_eq!(lk::zone_of(&engine, bears), Zone::Graveyard);
        assert_eq!(lk::zone_of(&engine, sappers), Zone::Graveyard);
    }

    // Expensive mode ({R}{R}{R}{R}, {T}): destroys target creature at end of combat, keeps Sappers.
    {
        let mut engine = lk::game(
            SEED,
            &[goblin_sappers(), index::GRIZZLY_BEARS],
            &[],
            &[],
            &[],
        );
        let sappers = lk::obj(&engine, P0, goblin_sappers());
        let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
        lk::float(&mut engine, P0, &[(ManaColor::Red, 4)]);
        lk::use_ability(&mut engine, P0, goblin_sappers(), 1, &[bears], &[]);

        let _ = lk::attack(&mut engine, &[bears]);
        lk::block(&mut engine, &[]);
        lk::through_combat(&mut engine);
        assert_eq!(lk::zone_of(&engine, bears), Zone::Graveyard);
        assert_eq!(lk::zone_of(&engine, sappers), Zone::Battlefield);
    }
}
