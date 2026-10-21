//! `cards/sorceries/mv_3/essence_filter.rs`, played.
//!
//! Oracle:
//! - "Destroy all enchantments or all nonwhite enchantments."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn essence_filter() -> CardIndex {
    card_index("b9f63388-d1ac-4895-84c5-e5b1db187463")
}

#[test]
fn essence_filter_can_destroy_only_nonwhite_enchantments() {
    let mut engine = lk::game(
        SEED,
        &[index::CRUSADE],
        &[index::FASTBOND],
        &[essence_filter()],
        &[],
    );
    let white_ench = lk::obj(&engine, P0, index::CRUSADE);
    let green_ench = lk::obj(&engine, P1, index::FASTBOND);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Green, 2)],
    );
    cast_with_floating(&mut engine, P0, essence_filter());
    for _ in 0..20 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCastMode { player, .. } => {
                engine.apply(player, PlayerAction::ChooseMode(1)).unwrap();
            }
            other => panic!("unexpected pending {other:?}"),
        }
    }

    assert_eq!(lk::zone_of(&engine, white_ench), Zone::Battlefield);
    assert_eq!(lk::zone_of(&engine, green_ench), Zone::Graveyard);
}

/// The other answer, "destroy all enchantments": the white Crusade goes too.
#[test]
fn essence_filter_can_destroy_every_enchantment() {
    let mut engine = lk::game(
        SEED,
        &[index::CRUSADE],
        &[index::FASTBOND],
        &[essence_filter()],
        &[],
    );
    let white_ench = lk::obj(&engine, P0, index::CRUSADE);
    let green_ench = lk::obj(&engine, P1, index::FASTBOND);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Green, 2)],
    );
    cast_with_floating(&mut engine, P0, essence_filter());
    let mut asked = false;
    for _ in 0..20 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseCastMode { player, .. } => {
                asked = true;
                engine.apply(player, PlayerAction::ChooseMode(0)).unwrap();
            }
            other => panic!("unexpected pending {other:?}"),
        }
    }

    assert!(asked, "the caster chose between the two");
    assert_eq!(lk::zone_of(&engine, white_ench), Zone::Graveyard);
    assert_eq!(lk::zone_of(&engine, green_ench), Zone::Graveyard);
}
