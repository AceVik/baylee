//! `cards/enchantments/auras/mv_2/decomposition.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{BOG_IMP, GRIZZLY_BEARS, LIGHTNING_BOLT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn decomposition() -> CardIndex {
    card_index("62817ef5-9b8d-4bca-bdd4-ab3bc4e063d0")
}

/// Decomposition (62817ef5-9b8d-4bca-bdd4-ab3bc4e063d0) — {1}{G} Aura:
/// "Enchant black creature. Enchanted creature has 'Cumulative upkeep—Pay 1
/// life.' When enchanted creature dies, its controller loses 2 life."
#[test]
fn decomposition_triggers_life_loss_for_controller_when_enchanted_black_creature_dies() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), mountain(), mountain()])
        .battlefield(1, &[BOG_IMP])
        .hand(0, &[decomposition(), LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let imp = on_battlefield(&engine, P1, BOG_IMP).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, decomposition());
    aim_at(&mut engine, P0, imp);
    pass_until(&mut engine, stack_is_empty);

    cast_with_floating(&mut engine, P0, LIGHTNING_BOLT);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![imp],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, BOG_IMP).is_none());
    assert_eq!(engine.state().players[1].life, 18);
}

/// P0 casts Decomposition on P1's Bog Imp (and P1 holds Grizzly Bears too);
/// returns the engine with the Aura resolved, P0's turn 1 main phase.
fn imp_enchanted() -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest()])
        .battlefield(1, &[BOG_IMP, GRIZZLY_BEARS])
        .hand(0, &[decomposition()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let imp = on_battlefield(&engine, P1, BOG_IMP).unwrap();
    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, decomposition());
    let menu = aim_at(&mut engine, P0, imp);
    assert!(menu.contains(&imp), "{menu:?}");
    assert!(
        !menu.contains(&bears),
        "\"Enchant black creature\": a green creature is not a target: {menu:?}"
    );
    pass_until(&mut engine, stack_is_empty);
    (engine, imp)
}

/// "Cumulative upkeep—Pay 1 life", at the enchanted creature's controller's
/// upkeep only: one age counter and 1 life the first time, two counters and 2
/// life the second, and declining sacrifices it ("dies": its controller loses
/// 2 more). P0's own upkeep in between asks nothing.
#[test]
fn decomposition_gives_cumulative_upkeep_pay_life_and_sacrifices_when_unpaid() {
    let (mut engine, imp) = imp_enchanted();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(engine.state().turn.number, 2, "P1's upkeep, not P0's");
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, P1);
    assert_eq!(prompt, crate::choice::YesNoPrompt::PayLife { amount: 1 });
    engine.apply(P1, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(engine.state().players[1].life, 19);
    assert!(on_battlefield(&engine, P1, BOG_IMP).is_some());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(engine.state().turn.number, 4, "nothing was asked at turn 3");
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, P1);
    assert_eq!(prompt, crate::choice::YesNoPrompt::PayLife { amount: 2 });
    engine.apply(P1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, BOG_IMP).is_none(), "{imp:?}");
    assert_eq!(engine.state().players[1].life, 17, "19 - 2 for dying");
}
