//! `cards/sorceries/mv_3/fiery_justice.rs`, played.
//!
//! Oracle:
//! - "Fiery Justice deals 5 damage divided as you choose among any number of targets. Target opponent gains 5 life."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn fiery_justice() -> CardIndex {
    card_index("333809cb-e196-45f2-8a67-31374438e56e")
}

#[test]
fn fiery_justice_deals_five_divided_damage_and_opponent_gains_five_life() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS, index::LLANOWAR_ELVES],
        &[fiery_justice()],
        &[],
    );
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let elves = lk::obj(&engine, P1, index::LLANOWAR_ELVES);

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
            (ManaColor::White, 1),
        ],
    );
    cast_with_floating(&mut engine, P0, fiery_justice());
    lk::aim(&mut engine, &[bears, elves], &[]);
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::ChooseNumber { player, max, .. } => {
                engine
                    .apply(player, PlayerAction::ChooseNumber(max.min(3)))
                    .unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending {other:?}"),
        }
    }
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, bears), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, elves), Zone::Graveyard);
    assert_eq!(lk::life(&engine, P1), 25);
}
