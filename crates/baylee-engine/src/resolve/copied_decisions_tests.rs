use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use crate::object::PaidRecord;

#[test]
fn copying_a_departed_spell_uses_its_recorded_incarnation_and_target_references() {
    let mut state =
        GameState::from_preset(&preset(413, &[]), &SyntheticLookup::new(vec![])).unwrap();
    let caster = PlayerId::new(0);
    let name = state.names.intern("old spell copy");
    let original = state.create_bare(caster, ObjectKind::Spell, name, ZoneLocation::Stack);
    let target = state.create_bare(
        caster,
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let replacement = state.create_bare(
        caster,
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let old_target = state.source_identity(target).unwrap();
    let old_spell = state.source_identity(original).unwrap();
    state.object_mut(original).unwrap().targets.push(target);
    state.object_mut(original).unwrap().x_value = 7;
    state.divided.push((original, vec![(old_target, 7)]));
    state
        .move_object(
            target,
            ZoneLocation::Hand(caster),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            target,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            original,
            ZoneLocation::Graveyard(caster),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            original,
            ZoneLocation::Stack,
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    state.object_mut(original).unwrap().targets = smallvec::smallvec![replacement];
    state.object_mut(original).unwrap().x_value = 2;
    assert!(!state.divided.iter().any(|(id, _)| *id == original));
    state.divided.push((
        original,
        vec![(state.source_identity(replacement).unwrap(), 2)],
    ));
    let copy = copy_spell(
        &mut state,
        old_spell,
        caster,
        &[],
        crate::text_changes::TextChangeMap::IDENTITY,
    )
    .unwrap();
    assert_eq!(state.object(copy).unwrap().x_value, 7);
    assert_eq!(state.object(copy).unwrap().targets.as_slice(), &[target]);
    assert_eq!(state.recorded_stack_target(copy, 0), Some(old_target));
    assert_ne!(state.source_identity(target), Some(old_target));
    assert_ne!(state.source_identity(original), Some(old_spell));
    assert_eq!(
        state.divided.iter().find(|(id, _)| *id == copy).unwrap().1,
        vec![(old_target, 7)]
    );
}

#[test]
fn spell_copy_keeps_damage_division_and_nonmana_cost_information_but_spends_no_mana() {
    let mut state =
        GameState::from_preset(&preset(411, &[]), &SyntheticLookup::new(vec![])).unwrap();
    let caster = PlayerId::new(0);
    let name = state.names.intern("copy decisions");
    let original = state.create_bare(caster, ObjectKind::Spell, name, ZoneLocation::Stack);
    let target = state.create_bare(
        caster,
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    state.object_mut(original).unwrap().paid = Some(Box::new(PaidRecord {
        source_after_cost: None,
        sacrificed_mana_value: Some(3),
        sacrificed: Some((target, 0)),
        mana_spent: 5,
        colors_spent: ColorSet::ALL,
        tapped: Some((target, 0)),
        ..PaidRecord::default()
    }));
    let target_ref = state.source_identity(target).unwrap();
    state.divided.push((original, vec![(target_ref, 4)]));
    let original_ref = state.source_identity(original).unwrap();
    let copy = copy_spell(
        &mut state,
        original_ref,
        PlayerId::new(1),
        &[],
        crate::text_changes::TextChangeMap::IDENTITY,
    )
    .unwrap();
    let paid = state.object(copy).unwrap().paid.as_ref().unwrap();
    assert_eq!(paid.sacrificed_mana_value, Some(3));
    assert_eq!(paid.tapped, Some((target, 0)));
    assert_eq!(paid.mana_spent, 0);
    assert_eq!(paid.colors_spent, ColorSet::EMPTY);
    assert_eq!(
        state.divided.iter().find(|(id, _)| *id == copy).unwrap().1,
        vec![(target_ref, 4)]
    );
}
