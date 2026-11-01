//! Combat redirection is selective, persistent, and bound to its destination.
use super::*;
use crate::prevention::RedirectDestination;
use baylee_cards_dsl::Filter;

#[test]
fn combat_redirection_ignores_noncombat_and_other_recipients_and_repeats() {
    let mut state = state(3);
    let source = creature(&mut state, B, 3, 3);
    let target = creature(&mut state, A, 9, 9);
    let reference = state.source_identity(target).unwrap();
    let context = crate::eval::live_context(&state, target);
    shield(
        &mut state,
        A,
        ShieldKind::RedirectCombat {
            sources: &Filter::CREATURE,
            to: RedirectDestination::Object(reference),
            context,
        },
    );
    for (combat, recipient) in [(false, A), (true, PlayerId::new(2)), (true, A), (true, A)] {
        let mut assignment = assigned(source, DamageTarget::Player(recipient), 2);
        assignment.is_combat = combat;
        assert!(
            DamageWork::new(&mut state, vec![assignment])
                .advance(&mut state)
                .is_none()
        );
    }
    assert_eq!(state.players[0].life, 18);
    assert_eq!(state.players[2].life, 18);
    assert_eq!(state.object(target).unwrap().damage, 4);
    assert_eq!(state.shields.len(), 1);
    state
        .move_object(
            target,
            ZoneLocation::Exile(A),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            target,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    let mut assignment = assigned(source, DamageTarget::Player(A), 3);
    assignment.is_combat = true;
    assert!(
        DamageWork::new(&mut state, vec![assignment])
            .advance(&mut state)
            .is_none()
    );
    assert_eq!(state.players[0].life, 15);
    assert_eq!(state.object(target).unwrap().damage, 0);
}

#[test]
fn combat_redirection_reads_source_filter_when_damage_occurs() {
    let mut state = state(2);
    let source = creature(&mut state, B, 3, 3);
    let target = creature(&mut state, A, 9, 9);
    let reference = state.source_identity(target).unwrap();
    let context = crate::eval::live_context(&state, target);
    shield(
        &mut state,
        A,
        ShieldKind::RedirectCombat {
            sources: &Filter::ARTIFACT,
            to: RedirectDestination::Object(reference),
            context,
        },
    );
    for artifact in [false, true] {
        if artifact {
            state.object_mut(source).unwrap().base_mut().types =
                TypeSet::CREATURE.union(TypeSet::ARTIFACT);
        }
        let mut assignment = assigned(source, DamageTarget::Player(A), 2);
        assignment.is_combat = true;
        assert!(
            DamageWork::new(&mut state, vec![assignment])
                .advance(&mut state)
                .is_none()
        );
    }
    assert_eq!(state.players[0].life, 18);
    assert_eq!(state.object(target).unwrap().damage, 2);
}
