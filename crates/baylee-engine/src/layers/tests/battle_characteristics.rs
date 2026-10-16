//! Defense is copied; protector designation and counters belong to the object.
use super::*;
use crate::object::{CounterKind, Rider};
use crate::zone::{ZoneLocation, ZonePosition};

fn battle(state: &mut GameState, name: &str, defense: u16, protector: PlayerId) -> ObjectId {
    let id = permanent(state, name, TypeSet::BATTLE, None);
    let object = state.object_mut(id).expect("created battle");
    object.base_mut().defense = Some(defense);
    object.riders.push(Rider::BattleProtector(protector));
    id
}

#[test]
fn printed_defense_is_copied_without_counters_or_protector_designation() {
    let mut state = fresh();
    let source = battle(&mut state, "Source", 7, PlayerId::new(1));
    state
        .object_mut(source)
        .unwrap()
        .counters
        .set(CounterKind::Defense, 2);
    let recipient = battle(&mut state, "Recipient", 4, me());
    state
        .object_mut(recipient)
        .unwrap()
        .counters
        .set(CounterKind::Defense, 3);
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller: me(),
        origin: crate::effects::EffectOrigin::Resolution,
        layer: Layer::Copy,
        timestamp: 1,
        duration: Duration::Indefinitely,
        filter: EffectFilter::ObjectIs(recipient, state.object(recipient).unwrap().version),
        modifier: Modifier::BecomeCopyOf(source),
    });
    let object = state.object(recipient).unwrap();
    assert_eq!(recompute(&state, object).characteristics.defense, Some(7));
    assert_eq!(copiable_values(&state, recipient).unwrap().defense, Some(7));
    assert_eq!(object.counters.get(CounterKind::Defense), 3);
    assert!(object.riders.contains(&Rider::BattleProtector(me())));
    assert!(
        !object
            .riders
            .contains(&Rider::BattleProtector(PlayerId::new(1)))
    );
    let creature = permanent(
        &mut state,
        "Copied creature",
        TypeSet::CREATURE,
        Some((2, 2)),
    );
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller: me(),
        origin: crate::effects::EffectOrigin::Resolution,
        layer: Layer::Copy,
        timestamp: 2,
        duration: Duration::Indefinitely,
        filter: EffectFilter::ObjectIs(recipient, state.object(recipient).unwrap().version),
        modifier: Modifier::BecomeCopyOf(creature),
    });
    let object = state.object(recipient).unwrap();
    let projected = recompute(&state, object).characteristics;
    assert!(!projected.types.contains(TypeSet::BATTLE));
    assert_eq!(projected.defense, None);
    assert!(object.riders.contains(&Rider::BattleProtector(me())));
    assert_eq!(object.counters.get(CounterKind::Defense), 3);
}

#[test]
fn losing_battle_type_preserves_its_printed_defense_and_raw_protector() {
    let mut state = fresh();
    let id = battle(&mut state, "Changing battle", 6, PlayerId::new(1));
    register(&mut state, 1, Modifier::RemoveType(TypeSet::BATTLE));
    let object = state.object(id).unwrap();
    let projected = recompute(&state, object).characteristics;
    assert!(!projected.types.contains(TypeSet::BATTLE));
    assert_eq!(projected.defense, Some(6));
    assert!(
        object
            .riders
            .contains(&Rider::BattleProtector(PlayerId::new(1)))
    );
    assert_eq!(copiable_values(&state, id).unwrap().defense, Some(6));
}

#[test]
fn face_down_form_has_no_printed_defense_and_departure_clears_protector() {
    let mut state = fresh();
    let id = battle(&mut state, "Hidden battle", 5, PlayerId::new(1));
    let hidden = crate::casting::SpellForm::Disguise.project(state.object(id).unwrap());
    assert_eq!(hidden.characteristics().defense, None);
    assert_eq!(hidden.characteristics().types, TypeSet::CREATURE);
    assert_eq!(state.object(id).unwrap().characteristics().defense, Some(5));
    state
        .move_object(
            id,
            ZoneLocation::Hand(me()),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    let departed = state.object(id).unwrap();
    assert!(
        !departed
            .riders
            .iter()
            .any(|rider| matches!(rider, Rider::BattleProtector(_)))
    );
    assert_eq!(departed.characteristics().defense, Some(5));
}

#[test]
fn face_construction_reads_only_that_faces_printed_defense() {
    use baylee_cards_dsl::{CardDef, FaceDef};
    static FACES: [FaceDef; 2] = [
        FaceDef {
            name: "Test siege",
            types: TypeSet::BATTLE,
            defense: Some(8),
            ..FaceDef::DEFAULT
        },
        FaceDef {
            name: "Test victor",
            types: TypeSet::CREATURE,
            power: Some(4),
            toughness: Some(4),
            castable_from_hand: false,
            ..FaceDef::DEFAULT
        },
    ];
    let original = baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6").unwrap();
    let definition = CardDef {
        faces: &FACES,
        ..*original
    };
    let mut state = fresh();
    let name = state.names.intern("Test face");
    assert_eq!(
        crate::object::Characteristics::from_face(&definition, 0, name).defense,
        Some(8)
    );
    assert_eq!(
        crate::object::Characteristics::from_face(&definition, 1, name).defense,
        None
    );
}
