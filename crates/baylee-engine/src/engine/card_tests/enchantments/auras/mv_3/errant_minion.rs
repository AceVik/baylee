//! `cards/enchantments/auras/mv_3/errant_minion.rs`, played.
//!
//! Oracle:
//! - "Enchant creature"
//! - "At the beginning of the upkeep of enchanted creature's controller, that player may pay any amount of mana. This Aura deals 2 damage to that player. Prevent X of that damage, where X is the amount of mana that player paid this way."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn errant_minion() -> CardIndex {
    card_index("80459049-bd89-4736-82f8-03c603644033")
}

#[test]
fn errant_minion_deals_two_damage_at_enchanted_creatures_controllers_upkeep() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[errant_minion()], &[]);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
    );
    lk::cast_at(&mut engine, P0, errant_minion(), &[bears], &[]);

    lk::to_step(&mut engine, P1, Step::Upkeep);
    for _ in 0..20 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseNumber { player, .. } => {
                engine.apply(player, PlayerAction::ChooseNumber(0)).unwrap();
            }
            other => panic!("unexpected pending {other:?}"),
        }
    }

    assert_eq!(lk::life(&engine, P1), 18);
}
