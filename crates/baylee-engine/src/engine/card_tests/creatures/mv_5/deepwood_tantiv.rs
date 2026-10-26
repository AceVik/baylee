//! `cards/creatures/mv_5/deepwood_tantiv.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn subject_card() -> CardIndex {
    card_index("3b31a9a7-6c97-4e82-bf72-de83034f81a9")
}

/// Deepwood Tantiv — {4}{G} 2/4 Creature — Beast:
/// "Whenever this creature becomes blocked, you gain 2 life."
#[test]
fn deepwood_tantiv_gains_two_life_when_blocked() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[subject_card()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let tantiv = on_battlefield(&engine, P0, subject_card()).unwrap();
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(tantiv, Defender::Player(P1))],
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
                blockers: vec![(bears, tantiv)],
            },
        )
        .unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert_eq!(engine.state().players[0].life, 22);
    assert!(on_battlefield(&engine, P0, subject_card()).is_some());
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
}
