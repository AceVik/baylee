//! `cards/creatures/mv_4/pious_warrior.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

// oracle_id = "163e5482-700c-4d91-b70c-9170cc8f1ad2"
fn pious_warrior() -> CardIndex {
    card_index("163e5482-700c-4d91-b70c-9170cc8f1ad2")
}

/// `Pious Warrior` (`Coverage::Generated`):
/// "Whenever this creature is dealt combat damage, you gain that much life."
///
/// Verifies that `Pious Warrior` triggers and gains its controller life equal
/// to the combat damage dealt to it, and does not trigger when dealt noncombat
/// damage.
#[test]
fn pious_warrior_gains_life_equal_to_combat_damage_dealt_to_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9512, plains())
        .battlefield(0, &[pious_warrior(), mountain()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[index::SHOCK])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let warrior = on_battlefield(&engine, p0, pious_warrior()).expect("P0 Pious Warrior");
    let bear = on_battlefield(&engine, p1, grizzly_bears()).expect("P1 Grizzly Bears");

    // Noncombat damage (Shock dealing 2 damage to Pious Warrior) does NOT trigger Pious Warrior.
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, index::SHOCK);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![warrior],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "noncombat damage dealt to Pious Warrior does not trigger life gain"
    );

    // On P1's turn, P1 attacks with 2/2 Grizzly Bears and P0 blocks with 2/3 Pious Warrior.
    assert!(walk_to_own_main(&mut engine, p1));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(warrior, bear)],
            },
        )
        .unwrap();
    assert!(walk_to_own_main(&mut engine, p0));

    assert_eq!(
        engine.state().players[0].life,
        22,
        "Pious Warrior gained 2 life when dealt 2 combat damage by Grizzly Bears"
    );
}
