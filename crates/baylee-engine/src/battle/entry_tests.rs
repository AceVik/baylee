//! Battle entry runs the actual deferred movement and replacement transaction.

use super::*;
use crate::choice::{Pending, PlayerAction};
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use crate::event::{Cause, GameEvent, LossReason};
use crate::object::ObjectKind;
use crate::state::{CardLookup, ReplacementEntry};
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::{CounterKind, Duration, Filter, Modifier, ReplacementRule};
use baylee_core::ids::{CardIndex, EffectId};
use baylee_core::preset::{FormatId, GamePreset, HouseRules, SeatController, SeatSpec};

const A: PlayerId = PlayerId::new(0);
const B: PlayerId = PlayerId::new(1);
const C: PlayerId = PlayerId::new(2);
struct NoCards;
impl CardLookup for NoCards {
    fn card(&self, _: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        None
    }
}
fn state(seats: usize) -> GameState {
    let seat = || SeatSpec {
        controller: SeatController::Open,
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: vec![],
        sideboard: vec![],
        commanders: vec![],
        starting_life: Some(20),
        starting_hand: None,
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    GameState::from_preset(
        &GamePreset {
            rules: baylee_core::card_metadata::RulesMetadata::default(),
            format: FormatId::Freeform,
            seed: 4,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![],
            seats: (0..seats).map(|_| seat()).collect(),
        },
        &NoCards,
    )
    .unwrap()
}
fn battle(state: &mut GameState, siege: bool) -> ObjectId {
    let name = state.names.intern("Battle entry fixture");
    let id = state.create_bare(A, ObjectKind::Card, name, ZoneLocation::Hand(A));
    let base = state.object_mut(id).unwrap().base_mut();
    base.types = TypeSet::BATTLE;
    base.defense = Some(4);
    if siege {
        base.subtypes
            .insert(baylee_core::generated::subtypes::battle::SIEGE);
    }
    id
}
fn effect(state: &mut GameState, id: ObjectId, modifier: Modifier) {
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller: A,
        origin: EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp: 2,
        duration: Duration::UntilEndOfTurn,
        filter: EffectFilter::ObjectIs(id, state.object(id).unwrap().version),
        modifier,
    });
}
fn global_effect(state: &mut GameState, modifier: Modifier) {
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller: A,
        origin: EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp: 2,
        duration: Duration::UntilEndOfTurn,
        filter: EffectFilter::Dsl(&Filter::Any),
        modifier,
    });
}
fn begin(state: &mut GameState, id: ObjectId) -> Option<Pending> {
    // Each helper call is a new instruction, not a simultaneous co-arrival.
    state.entries.begin_event(state.journal.last_seq(), Some(A));
    state
        .move_object(
            id,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert_ne!(
        state.object(id).unwrap().zone,
        Zone::Battlefield,
        "entry is deferred"
    );
    crate::entry::advance(state)
}
fn answer(state: &mut GameState, pending: &Pending, action: PlayerAction) -> Option<Pending> {
    assert_eq!(pending.answer_fault(&action), None);
    crate::entry::answer(state, &action);
    crate::entry::advance(state)
}

#[test]
fn siege_enters_with_defense_and_sole_opponent_protector() {
    let mut state = state(2);
    let id = battle(&mut state, true);
    assert!(begin(&mut state, id).is_none());
    assert_eq!(state.object(id).unwrap().zone, Zone::Battlefield);
    assert_eq!(
        state.object(id).unwrap().counters.get(CounterKind::Defense),
        4
    );
    assert_eq!(protector(&state, id), Some(B));
    assert_eq!(
        state
            .journal
            .entries()
            .iter()
            .filter(|entry| matches!(entry.event,
        GameEvent::CounterChanged { object, kind: CounterKind::Defense, .. } if object == id))
            .count(),
        1
    );
}

#[test]
fn multiplayer_protector_and_counter_choices_publish_no_partial_arrival() {
    for add_first in [true, false] {
        let mut state = state(3);
        let id = battle(&mut state, true);
        global_effect(&mut state, Modifier::AdditionalCounterPlacement { n: 1 });
        state.replacement_rules.push(ReplacementEntry {
            source: id,
            controller: A,
            rule: ReplacementRule::DoubleCounterPlacement {
                object_filter: &Filter::Any,
            },
        });
        let before = state.journal.len();
        let pending = begin(&mut state, id).unwrap();
        assert!(
            matches!(&pending, Pending::ChoosePlayer { player: A, options } if options == &[B, C])
        );
        assert_eq!(state.journal.len(), before);
        assert_eq!(designated_protector(&state, id), None);
        let pending = answer(&mut state, &pending, PlayerAction::ChoosePlayer(C)).unwrap();
        assert!(matches!(pending, Pending::YesNo { player: A, .. }));
        assert_eq!(state.journal.len(), before);
        assert_eq!(
            state.object(id).unwrap().counters.get(CounterKind::Defense),
            0
        );
        assert_ne!(state.object(id).unwrap().zone, Zone::Battlefield);
        let mut replay = state.clone();
        assert!(answer(&mut state, &pending, PlayerAction::YesNo(add_first)).is_none());
        assert!(answer(&mut replay, &pending, PlayerAction::YesNo(add_first)).is_none());
        assert_eq!(protector(&state, id), Some(C));
        let expected = if add_first { 10 } else { 9 };
        assert_eq!(
            state.object(id).unwrap().counters.get(CounterKind::Defense),
            expected
        );
        assert_eq!(
            replay
                .object(id)
                .unwrap()
                .counters
                .get(CounterKind::Defense),
            expected
        );
        assert_eq!(
            crate::state::structural_fingerprint(&state.entries),
            crate::state::structural_fingerprint(&replay.entries)
        );
    }
}

#[test]
fn typeless_battle_protects_itself_and_counter_prohibition_applies_before_entry() {
    let mut state = state(3);
    let id = battle(&mut state, false);
    global_effect(&mut state, Modifier::AdditionalCounterPlacement { n: 1 });
    state.replacement_rules.push(ReplacementEntry {
        source: id,
        controller: A,
        rule: ReplacementRule::DoubleCounterPlacement {
            object_filter: &Filter::Any,
        },
    });
    global_effect(
        &mut state,
        Modifier::CounterMaximum {
            kind: CounterKind::Defense,
            maximum: 0,
        },
    );
    assert!(begin(&mut state, id).is_none());
    assert_eq!(protector(&state, id), Some(A));
    assert_eq!(
        state.object(id).unwrap().counters.get(CounterKind::Defense),
        0
    );
    assert!(state.journal.entries().iter().all(|entry| !matches!(entry.event,
        GameEvent::CounterChanged { object, kind: CounterKind::Defense, new: 1.., .. } if object == id)));
}

#[test]
fn protector_choice_refreshes_when_an_opponent_leaves() {
    let mut state = state(3);
    let id = battle(&mut state, true);
    assert!(begin(&mut state, id).is_some());
    state.players[usize::from(B.get())].loss = Some(LossReason::Conceded);
    crate::entry::refresh_after_departure(&mut state);
    assert!(crate::entry::advance(&mut state).is_none());
    assert_eq!(protector(&state, id), Some(C));
    assert_eq!(
        state.object(id).unwrap().counters.get(CounterKind::Defense),
        4
    );
}

#[test]
fn designation_survives_type_and_copy_changes_but_not_a_new_incarnation() {
    let mut state = state(3);
    let id = battle(&mut state, true);
    let pending = begin(&mut state, id).unwrap();
    assert!(answer(&mut state, &pending, PlayerAction::ChoosePlayer(B)).is_none());
    let reference = state.source_identity(id).unwrap();
    effect(&mut state, id, Modifier::RemoveType(TypeSet::BATTLE));
    state.refresh_characteristics();
    assert_eq!(protector(&state, id), None);
    assert_eq!(designated_protector(&state, id), Some(B));
    let copy = battle(&mut state, false);
    effect(&mut state, id, Modifier::BecomeCopyOf(copy));
    state.refresh_characteristics();
    assert_eq!(designated_protector(&state, id), Some(B));
    state
        .move_object(
            id,
            ZoneLocation::Graveyard(A),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    assert_eq!(designated_protector(&state, id), None);
    set_protector(&mut state, reference, C);
    assert_eq!(designated_protector(&state, id), None);
}

#[test]
fn copying_a_siege_chooses_its_own_protector_and_uses_copied_printed_defense() {
    use crate::object::AbilityList;
    use baylee_cards_dsl::{AbilityDef, TargetSpec};
    static COPY: &[AbilityDef] = &[AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&Filter::HasType(TypeSet::BATTLE)),
        mods: &[],
    }];
    let mut state = state(3);
    let original = battle(&mut state, true);
    let pending = begin(&mut state, original).unwrap();
    assert!(answer(&mut state, &pending, PlayerAction::ChoosePlayer(B)).is_none());
    crate::replacement::remove_counters(&mut state, original, CounterKind::Defense, 3);
    let name = state.names.intern("Battle copying fixture");
    let copier = state.create_bare(A, ObjectKind::Card, name, ZoneLocation::Hand(A));
    state.object_mut(copier).unwrap().base_mut().types = TypeSet::CREATURE;
    state
        .object_mut(copier)
        .unwrap()
        .take_abilities(AbilityList {
            acquisition: 0,
            abilities: COPY.into(),
            printed: None,
            token: None,
        });
    let pending = begin(&mut state, copier).unwrap();
    let pending = answer(
        &mut state,
        &pending,
        PlayerAction::ChooseTargets {
            objects: vec![original],
            players: vec![],
        },
    )
    .unwrap();
    assert!(matches!(&pending, Pending::ChoosePlayer { options, .. } if options == &[B, C]));
    assert!(answer(&mut state, &pending, PlayerAction::ChoosePlayer(C)).is_none());
    assert_eq!(protector(&state, original), Some(B));
    assert_eq!(protector(&state, copier), Some(C));
    assert_eq!(
        state
            .object(copier)
            .unwrap()
            .counters
            .get(CounterKind::Defense),
        4
    );
}

#[test]
fn prospective_controller_chooses_protector_among_its_own_opponents() {
    let mut state = state(3);
    let id = battle(&mut state, true);
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller: B,
        origin: EffectOrigin::Resolution,
        layer: Modifier::GainControl.layer(),
        timestamp: 2,
        duration: Duration::UntilEndOfTurn,
        filter: EffectFilter::Dsl(&Filter::HasType(TypeSet::BATTLE)),
        modifier: Modifier::GainControl,
    });
    let pending = begin(&mut state, id).unwrap();
    assert!(matches!(&pending, Pending::ChoosePlayer { player: B, options } if options == &[A, C]));
    assert!(answer(&mut state, &pending, PlayerAction::ChoosePlayer(A)).is_none());
    state.refresh_characteristics();
    assert_eq!(state.object(id).unwrap().controller, B);
    assert_eq!(protector(&state, id), Some(A));
}

#[test]
fn losing_intrinsic_entry_ability_does_not_cancel_protector_designation() {
    let mut state = state(2);
    let id = battle(&mut state, true);
    global_effect(&mut state, Modifier::LoseAllAbilities);
    assert!(begin(&mut state, id).is_none());
    assert_eq!(protector(&state, id), Some(B));
    assert_eq!(
        state.object(id).unwrap().counters.get(CounterKind::Defense),
        0
    );
}

#[test]
fn basic_land_typing_preserves_intrinsic_battle_defense() {
    let mut state = state(2);
    let id = battle(&mut state, true);
    state.object_mut(id).unwrap().base_mut().types = TypeSet::BATTLE.union(TypeSet::LAND);
    global_effect(
        &mut state,
        Modifier::SetLandType(baylee_core::generated::subtypes::land::ISLAND),
    );
    assert!(begin(&mut state, id).is_none());
    state.refresh_characteristics();
    let object = state.object(id).unwrap();
    assert!(object.characteristics().rules_text_lost);
    assert!(object.characteristics().abilities_lost.is_none());
    assert_eq!(object.counters.get(CounterKind::Defense), 4);
    assert_eq!(protector(&state, id), Some(B));
}

#[test]
fn battlefield_only_copy_uses_prospective_defense_metadata() {
    static COPY_FILTER: Filter = Filter::And(&[
        Filter::InZone(baylee_cards_dsl::ZoneRef::Battlefield),
        Filter::HasType(TypeSet::CREATURE),
    ]);
    let mut state = state(2);
    let original = battle(&mut state, true);
    assert!(begin(&mut state, original).is_none());
    crate::replacement::remove_counters(&mut state, original, CounterKind::Defense, 3);
    let name = state.names.intern("Prospective battlefield copy");
    let copier = state.create_bare(A, ObjectKind::Card, name, ZoneLocation::Hand(A));
    state.object_mut(copier).unwrap().base_mut().types = TypeSet::CREATURE;
    let modifier = Modifier::BecomeCopyOf(original);
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller: A,
        origin: EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp: 2,
        duration: Duration::UntilEndOfTurn,
        filter: EffectFilter::Dsl(&COPY_FILTER),
        modifier,
    });
    assert_eq!(
        crate::layers::copiable_values(&state, copier)
            .unwrap()
            .defense,
        None
    );
    assert!(begin(&mut state, copier).is_none());
    state.refresh_characteristics();
    assert_eq!(
        state
            .object(copier)
            .unwrap()
            .counters
            .get(CounterKind::Defense),
        4
    );
    assert_eq!(protector(&state, copier), Some(B));
}
