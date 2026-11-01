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

fn wall_of_roots() -> CardIndex {
    card_index("3a21a6ae-b2f2-4f0c-acfd-5f3e8d63fd2f")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// "Enchant non-Wall creature you control": a Wall of your own and a creature of the opponent's are
/// refused, the Grizzly Bears is accepted.
#[test]
fn krovikan_plague_enchants_only_a_non_wall_creature_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[krovikan_plague()])
        .battlefield(
            0,
            &[grizzly_bears(), wall_of_roots(), swamp(), swamp(), swamp()],
        )
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let wall = on_battlefield(&engine, p0, wall_of_roots()).unwrap();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let plague_hand = in_hand(&engine, p0, krovikan_plague()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: plague_hand })
        .unwrap();
    for illegal in [wall, giant] {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![illegal],
                        players: vec![],
                    },
                )
                .is_err(),
            "{illegal:?} is a Wall or not yours"
        );
    }
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
    assert!(on_battlefield(&engine, p0, krovikan_plague()).is_some());
}

/// "Activate only if enchanted creature is untapped" (tapping it is the cost): used once the
/// ability is not offered again until the creature untaps, each use adds another -0/-1 counter, and
/// the second one leaves a 2/0 that dies.
#[test]
fn krovikan_plague_works_once_per_untap_and_its_counters_add_up() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
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

    let ping = |engine: &mut Engine<RegistryLookup>| {
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
        pass_until(engine, stack_is_empty);
    };

    ping(&mut engine);
    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(pt(&engine, bears), (2, 1));
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == plague),
        "the enchanted creature is tapped, so it cannot be tapped again: {:?}",
        legal.abilities
    );

    // Its controller's next turn untaps it; the second use leaves a 2/0.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    ping(&mut engine);
    assert_eq!(engine.state().players[1].life, 18);
    assert!(
        on_battlefield(&engine, p0, grizzly_bears()).is_none(),
        "two -0/-1 counters make a 2/0"
    );
}
