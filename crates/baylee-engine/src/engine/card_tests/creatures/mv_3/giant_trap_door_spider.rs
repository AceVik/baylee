//! `cards/creatures/mv_3/giant_trap_door_spider.rs`, played.
//!
//! Oracle:
//! - "{1}{R}{G}, {T}: Exile this creature and target creature without flying that's attacking you."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn giant_trap_door_spider() -> CardIndex {
    card_index("2c87565b-3902-4b11-b894-6922f66008ba")
}

#[test]
fn giant_trap_door_spider_exiles_itself_and_nonflying_attacker() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS],
        &[giant_trap_door_spider()],
        &[],
        &[],
    );
    let attacker = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let spider = lk::obj(&engine, P1, giant_trap_door_spider());

    let _ = lk::attack(&mut engine, &[attacker]);
    lk::block(&mut engine, &[]);
    lk::pass_once(&mut engine);
    lk::float(
        &mut engine,
        P1,
        &[
            (ManaColor::Colorless, 1),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
        ],
    );
    lk::use_ability(
        &mut engine,
        P1,
        giant_trap_door_spider(),
        0,
        &[attacker],
        &[],
    );
    assert_eq!(lk::zone_of(&engine, spider), Zone::Exile);
    assert_eq!(lk::zone_of(&engine, attacker), Zone::Exile);
}
