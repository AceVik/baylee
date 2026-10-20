//! `cards/instants/mv_3/essence_vortex.rs`, played.
//!
//! Oracle:
//! - "Destroy target creature unless its controller pays life equal to its toughness. A creature destroyed this way can't be regenerated."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn essence_vortex() -> CardIndex {
    card_index("ea38a211-a7b0-49fe-b5dc-7eca7610cccb")
}

#[test]
fn essence_vortex_destroys_target_unless_controller_pays_toughness_life() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[essence_vortex()], &[]);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[
            (ManaColor::Colorless, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
        ],
    );
    cast_with_floating(&mut engine, P0, essence_vortex());
    lk::aim(&mut engine, &[bears], &[]);
    for _ in 0..20 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            other => panic!("unexpected pending {other:?}"),
        }
    }

    assert_eq!(lk::zone_of(&engine, bears), Zone::Graveyard);
}
