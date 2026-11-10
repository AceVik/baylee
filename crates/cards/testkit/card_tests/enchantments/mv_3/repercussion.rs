//! `cards/enchantments/mv_3/repercussion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn repercussion() -> CardIndex {
    card_index("f6069a4f-744e-43e8-9f8b-7c13b8b0187f")
}

/// Repercussion (f6069a4f-744e-43e8-9f8b-7c13b8b0187f) — {1}{R}{R} Enchantment:
/// "Whenever a creature is dealt damage, this enchantment deals that much
/// damage to that creature's controller."
///
/// The trigger is symmetric: dealing 3 damage to an opponent's creature deals
/// 3 damage to that opponent, and dealing 3 damage to your own creature deals
/// 3 damage to you. Damage dealt directly to a player does not trigger it.
#[test]
fn repercussion_deals_damage_to_damaged_creatures_controller_on_both_sides() {
    use baylee_core::generated::index;

    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                repercussion(),
                mountain(),
                mountain(),
                mountain(),
                index::HILL_GIANT,
            ],
        )
        .battlefield(1, &[index::HILL_GIANT])
        .hand(0, &[lightning_bolt(), lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_giant = on_battlefield(&engine, p0, index::HILL_GIANT).unwrap();
    let their_giant = on_battlefield(&engine, p1, index::HILL_GIANT).unwrap();

    // 1. Lightning Bolt deals 3 damage to opponent's creature -> Repercussion deals 3 to P1.
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, their_giant);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        17,
        "opponent takes 3 damage when their creature takes 3 damage"
    );
    assert_eq!(engine.state().players[0].life, 20);

    // 2. Symmetric own-side check: Lightning Bolt deals 3 damage to own creature -> Repercussion deals 3 to P0.
    cast_from_hand(&mut engine, p0, lightning_bolt());
    aim_at(&mut engine, p0, my_giant);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        17,
        "Repercussion's controller takes 3 damage when their own creature takes 3 damage"
    );
    assert_eq!(engine.state().players[1].life, 17);

    // 3. Negative boundary: Lightning Bolt directly at a player does not trigger Repercussion.
    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        14,
        "direct player damage deals only the 3 from Lightning Bolt, not doubled by Repercussion"
    );
}
