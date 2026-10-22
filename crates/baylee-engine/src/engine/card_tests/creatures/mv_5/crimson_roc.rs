//! `cards/creatures/mv_5/crimson_roc.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{CRIMSON_ROC, HILL_GIANT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Crimson Roc (acc7d3b8-60f2-4f3b-9cf9-b6b39627d8f6) — {4}{R} 2/2 Flying:
/// "Whenever this creature blocks a creature without flying, this creature gets
/// +1/+0 and gains first strike until end of turn."
#[test]
fn crimson_roc_gets_plus_one_power_and_first_strike_when_blocking_non_flyer() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[CRIMSON_ROC])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);

    let roc = on_battlefield(&engine, P0, CRIMSON_ROC).unwrap();
    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(roc, giant)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    // Crimson Roc became 3/2 with first strike, killing 3/3 Hill Giant without dying.
    assert!(on_battlefield(&engine, P0, CRIMSON_ROC).is_some());
    assert!(on_battlefield(&engine, P1, HILL_GIANT).is_none());
}
