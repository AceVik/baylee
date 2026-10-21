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

/// P0 enchants P1's Grizzly Bears; at P1's upkeep P1 floats two mana and,
/// asked how much to pay, pays `amount` of it. Returns the engine once the
/// trigger has resolved.
fn upkeep_paying(amount: u32) -> Engine<RegistryLookup> {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[errant_minion()], &[]);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::float(
        &mut engine,
        P0,
        &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
    );
    lk::cast_at(&mut engine, P0, errant_minion(), &[bears], &[]);

    lk::to_step(&mut engine, P1, Step::Upkeep);
    lk::float(&mut engine, P1, &[(ManaColor::Colorless, 2)]);
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseNumber { player, .. } => {
                assert_eq!(
                    player, P1,
                    "\"that player\": the enchanted creature's controller"
                );
                engine
                    .apply(player, PlayerAction::ChooseNumber(amount))
                    .unwrap();
                lk::settle(&mut engine);
                return engine;
            }
            other => panic!("unexpected pending {other:?}"),
        }
    }
    panic!("the Minion never asked for mana");
}

#[test]
fn errant_minion_deals_two_damage_at_enchanted_creatures_controllers_upkeep() {
    let engine = upkeep_paying(0);
    assert_eq!(lk::life(&engine, P1), 18, "nothing paid, nothing prevented");
    assert_eq!(
        lk::life(&engine, P0),
        20,
        "the Aura's own controller is not the one it hurts"
    );
}

/// "Prevent X of that damage, where X is the amount of mana that player paid":
/// one mana prevents one, two prevent both, and the mana is spent.
#[test]
fn errant_minion_prevents_one_damage_for_each_mana_paid() {
    for amount in 1..=2 {
        let engine = upkeep_paying(amount);
        assert_eq!(
            lk::life(&engine, P1),
            18 + i32::try_from(amount).unwrap(),
            "{amount} mana paid prevents {amount} of the 2 damage"
        );
        assert_eq!(
            lk::pool_total(&engine, P1),
            u64::from(2 - amount),
            "the mana paid is spent"
        );
    }
}
