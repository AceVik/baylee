//! `cards/enchantments/auras/mv_1/tahngarth_s_rage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn tahngarth_s_rage() -> CardIndex {
    card_index("5b0cb089-087d-4b91-9e63-8186dc737a00")
}

/// `Tahngarth's Rage` ({R}, Enchantment — Aura, `Coverage::Implemented`):
/// `Enchant creature`
/// `Enchanted creature gets +3/+0 as long as it's attacking. Otherwise, it gets
/// -2/-1.`
#[test]
fn tahngarths_rage_gives_minus_two_minus_one_until_attacking_then_plus_three_plus_zero() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), HILL_GIANT])
        .hand(0, &[tahngarth_s_rage()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    cast_from_hand(&mut engine, p0, tahngarth_s_rage());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Not attacking -> Hill Giant (3/3) gets -2/-1 -> 1/2.
    let ch = engine.state().object(giant).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(1), Some(2)));

    // Attack with Hill Giant -> gets +3/+0 instead -> 6/3.
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
    let ch_atk = engine.state().object(giant).unwrap().characteristics();
    assert_eq!((ch_atk.power, ch_atk.toughness), (Some(6), Some(3)));
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    let ch_after = engine.state().object(giant).unwrap().characteristics();
    assert_eq!(
        (ch_after.power, ch_after.toughness),
        (Some(1), Some(2)),
        "otherwise includes after combat"
    );
}
