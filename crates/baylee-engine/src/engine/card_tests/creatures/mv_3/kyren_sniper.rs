//! `cards/creatures/mv_3/kyren_sniper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "7661dda6-4e7b-42d0-8374-0a1fe65b83e4"
fn kyren_sniper() -> CardIndex {
    card_index("7661dda6-4e7b-42d0-8374-0a1fe65b83e4")
}

/// `Kyren Sniper` (`Coverage::Generated`):
/// "At the beginning of your upkeep, you may have this creature deal 1 damage to target player or planeswalker."
///
/// Verifies that at the beginning of your upkeep `Kyren Sniper` deals 1 damage to
/// the targeted player when accepted and 0 damage when declined.
#[test]
fn kyren_sniper_pings_target_player_on_upkeep_when_accepted() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9406, mountain())
        .battlefield(0, &[kyren_sniper()])
        .start();
    keep_mulligans(&mut engine);

    // P0's first upkeep: target P1 and accept MayDo -> P1 goes 20 -> 19.
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
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 19);

    // Advance to P0's next upkeep and decline -> P1 stays at 19.
    assert!(walk_to_own_main(&mut engine, p0));
    assert!(walk_to_own_main(&mut engine, p1));
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
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 19);
}
