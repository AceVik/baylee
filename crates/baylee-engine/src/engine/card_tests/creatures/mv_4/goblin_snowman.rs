//! `cards/creatures/mv_4/goblin_snowman.rs`, played.
//!
//! Oracle:
//! - "Whenever this creature blocks, prevent all combat damage that would be dealt to and dealt by it this turn."
//! - "{T}: This creature deals 1 damage to target creature it's blocking."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn goblin_snowman() -> CardIndex {
    card_index("38e9ad30-4bbf-4b58-8bb2-47520ce351a3")
}

#[test]
fn goblin_snowman_prevents_combat_damage_when_blocking_and_pings_blocked_attacker() {
    let mut engine = lk::game(SEED, &[index::GRIZZLY_BEARS], &[goblin_snowman()], &[], &[]);
    let attacker = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let snowman = lk::obj(&engine, P1, goblin_snowman());

    let _ = lk::attack(&mut engine, &[attacker]);
    lk::block(&mut engine, &[(snowman, attacker)]);
    // Resolve the "Whenever this creature blocks" trigger.
    lk::settle(&mut engine);

    // Pass priority from P0 to P1 so P1 can tap Goblin Snowman to ping the attacker.
    lk::pass_once(&mut engine);
    lk::use_ability(&mut engine, P1, goblin_snowman(), 1, &[attacker], &[]);
    assert_eq!(lk::damage_on(&engine, attacker), 1);

    lk::through_combat(&mut engine);
    assert_eq!(
        lk::zone_of(&engine, snowman),
        Zone::Battlefield,
        "Goblin Snowman survives because combat damage to it is prevented"
    );
    assert_eq!(
        lk::damage_on(&engine, attacker),
        1,
        "attacker takes no additional combat damage from Goblin Snowman"
    );
}
