#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch49_turtle_cannot_attack_after_attacking_previous_own_turn_then_recovers() {
    let mut e = prepared(&[], &[index::GIANT_TURTLE], &[]);
    let turtle = on_battlefield(&e, P0, index::GIANT_TURTLE).unwrap();
    attack(&mut e, P0, turtle, P1);
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    let turn = e.state.turn.number;
    main_after(&mut e, P0, turn);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P0, .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = e.pending() else {
        unreachable!()
    };
    assert!(!attackers.contains(&turtle));
    let before = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(turtle, Defender::Player(P1))]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), before);
    e.apply(P0, PlayerAction::DeclareAttackers { attackers: vec![] })
        .unwrap();
    let turn = e.state.turn.number;
    main_after(&mut e, P0, turn);
    attack(&mut e, P0, turtle, P1);
}
