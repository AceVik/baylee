use super::{RegistryLookup, me, permanent, state, them};
use crate::event::Cause;
use crate::object::AbilityList;
use crate::state::GameState;
use crate::trigger::PendingTrigger;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_cards_dsl::{AbilityDef, Effect};
use baylee_core::ids::ObjectId;

fn source(state: &mut GameState) -> ObjectId {
    static ABILITIES: &[AbilityDef] = &[AbilityDef::StartYourEngines];
    let source = permanent(state, me(), "Start your engines");
    state
        .object_mut(source)
        .unwrap()
        .take_abilities(AbilityList::from_static(ABILITIES, None, None));
    source
}

fn start(state: &mut GameState) -> ObjectId {
    let source = source(state);
    crate::sba::run(state, &RegistryLookup);
    source
}

fn triggers(state: &GameState, from: u64) -> Vec<PendingTrigger> {
    crate::trigger::collect(state, &RegistryLookup, from)
        .into_iter()
        .filter(|trigger| matches!(trigger.synthetic_effects, Some([Effect::IncreaseSpeed])))
        .collect()
}

#[test]
fn speed_loss_before_start_does_not_retroactively_trigger() {
    let mut state = state();
    state.turn.active = me();
    let from = state.journal.last_seq();
    state.change_life(them(), -1, Cause::Effect);
    start(&mut state);
    assert!(triggers(&state, from).is_empty());
    state.change_life(them(), -1, Cause::Effect);
    let pending = triggers(&state, from);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].controller, me());
    assert_eq!(pending[0].source, ObjectId::NO_SOURCE);
}

#[test]
fn speed_survives_its_permanent_and_only_triggers_once_per_own_turn() {
    let mut state = state();
    state.turn.active = me();
    let source = start(&mut state);
    state
        .move_object(
            source,
            ZoneLocation::Exile(me()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    let from = state.journal.last_seq();
    state.change_life(them(), -1, Cause::Effect);
    state.change_life(them(), -1, Cause::Effect);
    assert_eq!(triggers(&state, from).len(), 1);
    state.per_turn.reset();
    state.turn.active = them();
    let from = state.journal.last_seq();
    state.change_life(them(), -1, Cause::Effect);
    state.change_life(me(), -1, Cause::Effect);
    assert!(triggers(&state, from).is_empty());
    state.per_turn.reset();
    state.turn.active = me();
    state.change_life(them(), -1, Cause::Effect);
    assert_eq!(triggers(&state, from).len(), 1);
}

#[test]
fn speed_self_loss_gain_and_max_speed_do_not_trigger() {
    let mut state = state();
    state.turn.active = me();
    start(&mut state);
    let from = state.journal.last_seq();
    state.change_life(me(), -1, Cause::Effect);
    state.change_life(them(), 1, Cause::Effect);
    state.change_life(them(), 0, Cause::Effect);
    assert!(triggers(&state, from).is_empty());
    state.players[0].speed = 4;
    state.change_life(them(), -1, Cause::Effect);
    assert!(triggers(&state, from).is_empty());
    assert!(!state.per_turn.speed_triggered[0]);
}

#[test]
fn speed_trigger_consumption_changes_loop_signature() {
    let mut state = state();
    start(&mut state);
    let before = state.loop_signature();
    state.per_turn.speed_triggered[0] = true;
    assert_ne!(before, state.loop_signature());
}

#[test]
fn speed_starts_for_the_new_controller_without_an_entry_event() {
    let mut state = state();
    let source = start(&mut state);
    state.object_mut(source).unwrap().set_controller(them());
    crate::sba::run(&mut state, &RegistryLookup);
    assert_eq!(state.players[0].speed, 1);
    assert_eq!(state.players[1].speed, 1);
}

#[test]
fn speed_ignores_a_phased_out_source_until_it_phases_in() {
    let mut state = state();
    let source = source(&mut state);
    state
        .object_mut(source)
        .unwrap()
        .status
        .insert(crate::object::Status::PHASED_OUT);
    crate::sba::run(&mut state, &RegistryLookup);
    assert_eq!(state.players[0].speed, 0);
    state
        .object_mut(source)
        .unwrap()
        .status
        .remove(crate::object::Status::PHASED_OUT);
    crate::sba::run(&mut state, &RegistryLookup);
    assert_eq!(state.players[0].speed, 1);
}

#[test]
fn speed_does_not_start_while_a_continuous_effect_removes_the_ability() {
    let mut state = state();
    let source = source(&mut state);
    let modifier = baylee_cards_dsl::Modifier::LoseAllAbilities;
    let effect = crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: me(),
        origin: crate::effects::EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp: state.next_timestamp(),
        duration: baylee_cards_dsl::Duration::Indefinitely,
        filter: crate::effects::EffectFilter::object(&state, source),
        modifier,
    };
    let id = state.effects.register(effect);
    state.refresh_characteristics();
    crate::sba::run(&mut state, &RegistryLookup);
    assert_eq!(state.players[0].speed, 0);
    state.effects.remove_where(|effect| effect.id == id);
    state.refresh_characteristics();
    crate::sba::run(&mut state, &RegistryLookup);
    assert_eq!(state.players[0].speed, 1);
}
