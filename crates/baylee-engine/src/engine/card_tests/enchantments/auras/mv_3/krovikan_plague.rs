//! Krovikan Plague — `{2}{B}` Enchantment — Aura (Alliances)
//!
//! Enchant non-Wall creature you control
//! When this Aura enters, draw a card at the beginning of the next turn's
//! upkeep.
//! Tap enchanted creature: This Aura deals 1 damage to any target. Put a -0/-1
//! counter on enchanted creature. Activate only if enchanted creature is
//! untapped.

#[allow(clippy::wildcard_imports)]
use super::*;

fn krovikan_plague() -> CardIndex {
    card_index("609c3b1c-d9ad-4616-b685-52b85528fde8")
}

#[test]
fn krovikan_plague_taps_enchanted_creature_for_1_damage_and_minus_zero_minus_one_counter_and_draws_next_upkeep()
 {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[krovikan_plague()])
        .battlefield(0, &[grizzly_bears(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let plague_hand = in_hand(&engine, p0, krovikan_plague()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: plague_hand })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let plague = on_battlefield(&engine, p0, krovikan_plague()).unwrap();

    // Activate Tap enchanted creature: deal 1 damage to p1 and put a -0/-1 counter on Grizzly Bears.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: plague,
                ability_index: 2,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![bears],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, bears));
    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(pt(&engine, bears), (2, 1));

    // Advance to the next turn's upkeep (p1's turn) -> p0 draws a card from the delayed trigger.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
