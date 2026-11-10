//! `cards/enchantments/auras/mv_2/muzzle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "bdf497b5-166f-46a7-8e18-ae0b8f97768c"
fn muzzle() -> CardIndex {
    card_index("bdf497b5-166f-46a7-8e18-ae0b8f97768c")
}

fn prodigal_sorcerer() -> CardIndex {
    card_index("5e961d15-5972-4e4b-9385-1cd7cd7c6bbe")
}

/// `Muzzle` (`Coverage::Generated`):
/// "Enchant creature. Prevent all damage that would be dealt by enchanted creature."
///
/// Verifies that `Muzzle` prevents both combat damage and noncombat activated-ability
/// damage dealt by the enchanted creature.
#[test]
fn muzzle_prevents_combat_and_noncombat_damage_from_enchanted_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9412, plains())
        .battlefield(0, &[plains(), plains()])
        .battlefield(1, &[prodigal_sorcerer()])
        .hand(0, &[muzzle()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let tim = on_battlefield(&engine, p1, prodigal_sorcerer()).expect("Prodigal Sorcerer");
    cast_from_hand(&mut engine, p0, muzzle());
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![tim] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // P1's turn: P1 attacks with Prodigal Sorcerer -> combat damage is prevented.
    assert!(walk_to_own_main(&mut engine, p1));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(tim, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "combat damage from Muzzled creature is prevented"
    );

    // Next P1 turn: P1 activates Prodigal Sorcerer's {T}: deal 1 damage -> noncombat damage is also prevented.
    assert!(walk_to_own_main(&mut engine, p0));
    assert!(walk_to_own_main(&mut engine, p1));
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: tim,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "noncombat damage from Muzzled creature is prevented"
    );
}
