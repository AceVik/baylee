//! `cards/creatures/mv_2/goblin_elite_infantry.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{HILL_GIANT, WALL_OF_WOOD};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn goblin_elite_infantry() -> CardIndex {
    card_index("a6a8861c-b6fe-45a6-86e4-9160c36362ff")
}

/// Goblin Elite Infantry (a6a8861c-b6fe-45a6-86e4-9160c36362ff) — {1}{R} 2/2:
/// "Whenever this creature blocks or becomes blocked, it gets -1/-1 until end
/// of turn."
#[test]
fn goblin_elite_infantry_gets_minus_one_minus_one_when_blocked() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[goblin_elite_infantry()])
        .battlefield(1, &[WALL_OF_WOOD])
        .start();
    keep_mulligans(&mut engine);

    let infantry = on_battlefield(&engine, P0, goblin_elite_infantry()).unwrap();
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

/// "blocks": when the Infantry blocks (P1's Hill Giant attacks), it gets
/// -1/-1 too.
#[test]
fn goblin_elite_infantry_gets_minus_one_minus_one_when_it_blocks() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[goblin_elite_infantry()])
        .battlefield(1, &[HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);

    let infantry = on_battlefield(&engine, P0, goblin_elite_infantry()).unwrap();
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
    assert_eq!(pt(&engine, infantry), (2, 2), "before it blocks");
    engine
        .apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(infantry, giant)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, infantry), (1, 1));
}

/// "until end of turn": after it survived being blocked by a Wall of Wood, it
/// is a 2/2 again on the opponent's turn.
#[test]
fn goblin_elite_infantry_is_two_two_again_after_the_turn() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[goblin_elite_infantry()])
        .battlefield(1, &[WALL_OF_WOOD])
        .start();
    keep_mulligans(&mut engine);

    let infantry = on_battlefield(&engine, P0, goblin_elite_infantry()).unwrap();
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
    assert_eq!(pt(&engine, infantry), (1, 1));

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, infantry), (2, 2));
}
