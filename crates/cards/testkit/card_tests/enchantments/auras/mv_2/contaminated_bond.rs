//! `cards/enchantments/auras/mv_2/contaminated_bond.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn contaminated_bond() -> CardIndex {
    card_index("56aeaee8-e014-485e-b7ad-e26ade663bcf")
}

/// Contaminated Bond — {1}{B} Enchantment — Aura
/// "Enchant creature"
/// "Whenever enchanted creature attacks or blocks, its controller loses 3 life."
#[test]
fn contaminated_bond_causes_controller_to_lose_three_life_on_attack_and_block() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[contaminated_bond()])
        .battlefield(0, &[swamp(), swamp(), HILL_GIANT])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, contaminated_bond());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    // P0 attacks with Hill Giant; P1 blocks with the enchanted Grizzly Bears.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bear, giant)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Blocking with the enchanted creature caused P1 to lose 3 life.
    assert_eq!(engine.state().players[1].life, 17);
}

#[test]
fn contaminated_bond_triggers_when_enchanted_creature_attacks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[contaminated_bond()])
        .battlefield(0, &[swamp(), swamp(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, contaminated_bond());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 17);
}
