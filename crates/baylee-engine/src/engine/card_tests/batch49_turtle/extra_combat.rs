#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn turtle_may_attack_in_second_combat_of_same_turn_when_previous_turn_had_no_attack() {
    let mut e = prepared(&[], &[index::GIANT_TURTLE], &[]);
    let turtle = on_battlefield(&e, P0, index::GIANT_TURTLE).unwrap();
    attack(&mut e, P0, turtle, P1);
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        e.state.turn.phase == crate::turn::Phase::SecondMain
            && matches!(e.pending(), Pending::Priority { player: P0, .. })
    });
    let turn = e.state.turn.number;
    // Transparent second-combat scheduling fixture: no implemented extra-combat card
    // exists in the pool. Prior combat, history and both attack declarations are real.
    // CombatEnd already cleared CombatState and combat_declared in normal progression.
    e.state.set_tapped(turtle, false);
    e.state.turn.phase = crate::turn::Phase::Combat;
    e.state.turn.step = crate::turn::Step::CombatBegin;
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    assert_attack_offer(&mut e, P0, turtle, true);
    assert_eq!(
        e.state.turn.number, turn,
        "earlier combat was THIS turn, not your last turn"
    );
}
