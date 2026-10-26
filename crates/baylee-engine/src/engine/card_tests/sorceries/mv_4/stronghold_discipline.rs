//! `cards/sorceries/mv_4/stronghold_discipline.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn stronghold_discipline_makes_each_player_lose_life_per_own_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[index::STRONGHOLD_DISCIPLINE])
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), swamp(), index::GRIZZLY_BEARS],
        )
        .battlefield(
            1,
            &[index::GRIZZLY_BEARS, index::GRIZZLY_BEARS, llanowar_elves()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, index::STRONGHOLD_DISCIPLINE);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(engine.state().players[1].life, 17);
}
