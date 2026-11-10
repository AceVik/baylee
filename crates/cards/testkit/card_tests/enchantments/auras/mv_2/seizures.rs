//! `cards/enchantments/auras/mv_2/seizures.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn seizures() -> CardIndex {
    card_index("550921f5-ec65-460d-9ada-f2c5a4c4b2c4")
}

fn prodigal_sorcerer() -> CardIndex {
    card_index("5e961d15-5972-4e4b-9385-1cd7cd7c6bbe")
}

/// Seizures (550921f5-ec65-460d-9ada-f2c5a4c4b2c4) — {1}{B} Enchantment — Aura:
/// - "Enchant creature"
/// - "Whenever enchanted creature becomes tapped, this Aura deals 3 damage to that creature's
///   controller unless that player pays {3}."
#[test]
fn seizures_deals_three_damage_on_enchanted_creature_tap_unless_controller_pays_three() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[prodigal_sorcerer(), island(), island(), island()])
        .hand(0, &[seizures()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let tim = on_battlefield(&engine, P1, prodigal_sorcerer()).unwrap();

    // P0 casts Seizures enchanting P1's Prodigal Sorcerer.
    cast_from_hand(&mut engine, P0, seizures());
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![tim],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // 1. P1 taps Prodigal Sorcerer to ping P0 -> Seizures triggers; P1 pays {3} -> takes 0 damage.
    reach_their_main_phase(&mut engine, P1);
    activate(&mut engine, P1, prodigal_sorcerer(), 0);
    engine
        .apply(
            P1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P1),
    );
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, P1);
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        20,
        "P1 paid {{3}}, so Seizures dealt no damage"
    );

    // 2. Advance to P1's next main phase (turn 4); P1 taps Prodigal Sorcerer again and declines to pay {3}
    // -> Seizures deals 3 damage to P1 (life 20 -> 17).
    pass_until(&mut engine, |e| {
        e.state().turn.number == 4 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    activate(&mut engine, P1, prodigal_sorcerer(), 0);
    engine
        .apply(
            P1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P0],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P1),
    );
    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        17,
        "P1 declined to pay {{3}}, so Seizures dealt 3 damage to P1"
    );
}
