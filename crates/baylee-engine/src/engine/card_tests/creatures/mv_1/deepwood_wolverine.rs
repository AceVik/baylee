//! `cards/creatures/mv_1/deepwood_wolverine.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn subject_card() -> CardIndex {
    card_index("ca8060a7-30e7-49a4-a720-da5b995dd31f")
}

/// Deepwood Wolverine — {G} 1/1 Creature — Wolverine:
/// "Whenever this creature becomes blocked, it gets +2/+0 until end of turn."
#[test]
fn deepwood_wolverine_gets_plus_two_plus_zero_when_blocked() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[subject_card()])
        .battlefield(1, &[index::WALL_OF_WOOD])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let wolverine = on_battlefield(&engine, P0, subject_card()).unwrap();
    let wall = on_battlefield(&engine, P1, index::WALL_OF_WOOD).unwrap();
    assert_eq!(pt(&engine, wolverine), (1, 1));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(wolverine, Defender::Player(P1))],
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
                blockers: vec![(wall, wolverine)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(pt(&engine, wolverine), (3, 1));
    assert!(on_battlefield(&engine, P1, index::WALL_OF_WOOD).is_none());
    assert!(in_graveyard(&engine, P1, index::WALL_OF_WOOD).is_some());
}
