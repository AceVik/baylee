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
        &[
            index::GRIZZLY_BEARS,
            index::SERRA_ANGEL,
            index::LLANOWAR_ELVES,
        ],
        &[giant_trap_door_spider()],
        &[],
        &[],
    );
    let attacker = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let flier = lk::obj(&engine, P0, index::SERRA_ANGEL);
    let at_home = lk::obj(&engine, P0, index::LLANOWAR_ELVES);
    let spider = lk::obj(&engine, P1, giant_trap_door_spider());

    let _ = lk::attack(&mut engine, &[attacker, flier]);
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
    activate(&mut engine, P1, giant_trap_door_spider(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&attacker),
        "a creature without flying attacking you is a target"
    );
    assert!(
        !options.contains(&flier),
        "an attacker with flying is not a target"
    );
    assert!(
        !options.contains(&at_home),
        "a creature that is not attacking is not a target"
    );
    lk::aim(&mut engine, &[attacker], &[]);
    lk::settle(&mut engine);
    assert_eq!(lk::zone_of(&engine, spider), Zone::Exile);
    assert_eq!(lk::zone_of(&engine, attacker), Zone::Exile);
    assert_eq!(
        lk::zone_of(&engine, flier),
        Zone::Battlefield,
        "only the target goes with the Spider"
    );
}
