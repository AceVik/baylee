//! `cards/enchantments/mv_4/lightning_cloud.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
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

/// P0 owns the Cloud; P1 holds a red spell. On P1's turn P0 has one {R}
/// floating and P1 casts Mons's Goblin Raiders; stops at P0's question about the
/// Cloud's target.
fn opponent_casts_a_red_spell() -> Engine<RegistryLookup> {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[lightning_cloud()])
        .battlefield(1, &[mountain()])
        .hand(1, &[MONS_S_GOBLIN_RAIDERS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    lk::float(&mut engine, p0, &[(ManaColor::Red, 1)]);
    cast_from_hand(&mut engine, p1, MONS_S_GOBLIN_RAIDERS);
    let Pending::ChooseTargets { player, .. } = engine.pending().clone() else {
        panic!(
            "expected the Cloud's target question, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0, "the Cloud's controller aims it");
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
    engine
}

/// "Whenever a *player* casts a red spell, *you* may pay {R}": the opponent's
/// red spell triggers it, the Cloud's controller is asked and pays the {R},
/// and the damage is dealt.
#[test]
fn lightning_cloud_triggers_on_an_opponents_red_spell_and_its_controller_pays() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = opponent_casts_a_red_spell();

    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, p0, "\"you may pay\" is the Cloud's controller");
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 19);
    assert_eq!(lk::pool_total(&engine, p0), 0, "the {{R}} was spent");
    assert!(on_battlefield(&engine, p1, MONS_S_GOBLIN_RAIDERS).is_some());
}

/// "You may pay {R}": declined, nothing is dealt and the mana stays.
#[test]
fn lightning_cloud_deals_nothing_when_its_controller_declines_to_pay() {
    let p0 = PlayerId::new(0);
    let mut engine = opponent_casts_a_red_spell();

    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[1].life, 20);
    assert_eq!(lk::pool_total(&engine, p0), 1, "the {{R}} was not paid");
}

/// "A red spell": a colorless spell does not trigger it (no target question,
/// no damage).
#[test]
fn lightning_cloud_ignores_a_spell_that_is_not_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[lightning_cloud(), mountain(), mountain()])
        .hand(0, &[sol_ring()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, sol_ring());
    // `pass_until` panics on a question it does not know, such as the Cloud's
    // target choice: reaching an empty stack means it never triggered.
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, sol_ring()).is_some());
    assert_eq!(engine.state().players[1].life, 20);
}
