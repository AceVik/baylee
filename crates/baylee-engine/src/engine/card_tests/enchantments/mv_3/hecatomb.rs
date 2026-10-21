//! `cards/enchantments/mv_3/hecatomb.rs`, played.
//!
//! Oracle:
//! - "When this enchantment enters, sacrifice this enchantment unless you sacrifice four creatures."
//! - "Tap an untapped Swamp you control: This enchantment deals 1 damage to any target."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn hecatomb() -> CardIndex {
    card_index("0ec153ee-2a91-4196-9105-212dd1864aea")
}

#[test]
fn hecatomb_etb_sacrifices_four_creatures_and_taps_swamp_to_deal_one_damage() {
    let mut engine = lk::game(
        SEED,
        &[
            index::SWAMP,
            index::GRIZZLY_BEARS,
            index::LLANOWAR_ELVES,
            index::WALL_OF_WOOD,
            index::HILL_GIANT,
        ],
        &[],
        &[hecatomb()],
        &[],
    );
    let swamp_id = lk::obj(&engine, P0, index::SWAMP);
    let c1 = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let c2 = lk::obj(&engine, P0, index::LLANOWAR_ELVES);
    let c3 = lk::obj(&engine, P0, index::WALL_OF_WOOD);
    let c4 = lk::obj(&engine, P0, index::HILL_GIANT);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 2)],
    );
    cast_with_floating(&mut engine, P0, hecatomb());
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                if on_battlefield(&engine, P0, hecatomb()).is_some() && stack_is_empty(&engine) {
                    break;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            Pending::ChooseCards {
                player,
                options,
                max,
                ..
            } => {
                let chosen: Vec<_> = [c1, c2, c3, c4]
                    .into_iter()
                    .filter(|id| options.contains(id))
                    .take(usize::from(max))
                    .collect();
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: chosen })
                    .unwrap();
            }
            other => panic!("unexpected pending {other:?}"),
        }
    }
    assert!(on_battlefield(&engine, P0, hecatomb()).is_some());
    for creature in [c1, c2, c3, c4] {
        assert_eq!(lk::zone_of(&engine, creature), Zone::Graveyard);
    }

    // Activate Hecatomb by tapping the untapped Swamp to deal 1 damage to P1.
    activate(&mut engine, P0, hecatomb(), 1);
    for _ in 0..10 {
        match engine.pending().clone() {
            Pending::ChooseTargets { .. } => lk::aim(&mut engine, &[], &[P1]),
            Pending::ChooseCards { player, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![swamp_id],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { .. } => break,
            other => panic!("unexpected pending {other:?}"),
        }
    }
    lk::settle(&mut engine);
    assert!(lk::tapped(&engine, swamp_id));
    assert_eq!(lk::life(&engine, P1), 19);
}

/// "…unless you sacrifice four creatures", declined with four creatures on
/// the battlefield: Hecatomb is sacrificed and the creatures stay.
#[test]
fn hecatomb_is_sacrificed_when_its_controller_keeps_the_creatures() {
    let mut engine = lk::game(
        SEED,
        &[
            index::GRIZZLY_BEARS,
            index::LLANOWAR_ELVES,
            index::WALL_OF_WOOD,
            index::HILL_GIANT,
        ],
        &[],
        &[hecatomb()],
        &[],
    );
    let creatures = [
        lk::obj(&engine, P0, index::GRIZZLY_BEARS),
        lk::obj(&engine, P0, index::LLANOWAR_ELVES),
        lk::obj(&engine, P0, index::WALL_OF_WOOD),
        lk::obj(&engine, P0, index::HILL_GIANT),
    ];
    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 1), (ManaColor::Black, 2)],
    );
    cast_with_floating(&mut engine, P0, hecatomb());
    let mut declined = false;
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                if declined && stack_is_empty(&engine) {
                    break;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo { player, .. } => {
                declined = true;
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            other => panic!("unexpected pending {other:?}"),
        }
    }
    assert!(declined, "the sacrifice of four creatures was offered");
    assert!(on_battlefield(&engine, P0, hecatomb()).is_none());
    assert!(in_graveyard(&engine, P0, hecatomb()).is_some());
    for creature in creatures {
        assert_eq!(lk::zone_of(&engine, creature), Zone::Battlefield);
    }
}
