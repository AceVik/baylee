//! `cards/enchantments/mv_4/lightning_cloud.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::MONS_S_GOBLIN_RAIDERS;

fn lightning_cloud() -> CardIndex {
    card_index("4a574140-0657-4f49-b497-331447f17b29")
}

/// `Lightning Cloud` ({3}{R}, Enchantment, `Coverage::Implemented`):
/// `Whenever a player casts a red spell, you may pay {R}. If you do, this
/// enchantment deals 1 damage to any target.`
#[test]
fn lightning_cloud_triggers_on_red_spell_and_deals_1_damage_when_paid() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[lightning_cloud(), MONS_S_GOBLIN_RAIDERS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, lightning_cloud());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, lightning_cloud()).is_some());

    // Cast a red spell (Mons's Goblin Raiders) and aim Lightning Cloud's trigger at P1.
    cast_from_hand(&mut engine, p0, MONS_S_GOBLIN_RAIDERS);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19);
}
