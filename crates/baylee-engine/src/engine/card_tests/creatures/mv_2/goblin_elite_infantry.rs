//! `cards/creatures/mv_2/goblin_elite_infantry.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GOBLIN_ELITE_INFANTRY, WALL_OF_WOOD};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Goblin Elite Infantry (ba836f90-27e0-452d-b2d1-a11d432e59b1) — {1}{R} 2/2:
/// "Whenever this creature blocks or becomes blocked, it gets -1/-1 until end
/// of turn."
#[test]
fn goblin_elite_infantry_gets_minus_one_minus_one_when_blocked() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[GOBLIN_ELITE_INFANTRY])
        .battlefield(1, &[WALL_OF_WOOD])
        .start();
    keep_mulligans(&mut engine);

    let infantry = on_battlefield(&engine, P0, GOBLIN_ELITE_INFANTRY).unwrap();
    let wall = on_battlefield(&engine, P1, WALL_OF_WOOD).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(infantry, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, infantry)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let chars = engine.state().object(infantry).unwrap().characteristics();
    assert_eq!((chars.power, chars.toughness), (Some(1), Some(1)));
}
