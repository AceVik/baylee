//! `cards/enchantments/auras/mv_2/invisibility.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Invisibility: "Enchant creature" / "Enchanted creature can't be blocked
/// except by Walls." A Wall may still be assigned to block it; a
/// non-Wall creature may not.
#[test]
fn invisibility_cannot_be_blocked_except_by_walls() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let invisibility = invisibility();
    let wall = wall_of_roots();
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), quiet_creature()])
        .battlefield(1, &[wall, festering_goblin()])
        .hand(0, &[invisibility])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let attacker = on_battlefield(&engine, p0, quiet_creature()).expect("attacker seated");

    // Tap only the Islands (not the Elves, which are meant to attack): the
    // Elves also carry a mana ability, and `cast_from_hand` would tap
    // everything that can pay, tapping the attacker along with the lands.
    let islands = all_on_battlefield(&engine, p0, island());
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: islands[0] })
        .unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: islands[1] })
        .unwrap();
    cast_with_floating(&mut engine, p0, invisibility);
    aim_at(&mut engine, p0, attacker);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, attacker),
        "the attacker stayed untapped"
    );

    let blockers = attack_and_collect_blocks(&mut engine, attacker, p1);
    let wall_id = on_battlefield(&engine, p1, wall).expect("Wall of Roots seated");
    let goblin_id = on_battlefield(&engine, p1, festering_goblin()).expect("goblin seated");
    let wall_option = blockers.iter().find(|b| b.blocker == wall_id);
    assert!(
        wall_option.is_some_and(|b| b.attackers.contains(&attacker)),
        "a Wall may still block it: {blockers:?}"
    );
    let goblin_option = blockers.iter().find(|b| b.blocker == goblin_id);
    assert!(
        goblin_option.is_none_or(|b| !b.attackers.contains(&attacker)),
        "a non-Wall creature may not: {blockers:?}"
    );
}
