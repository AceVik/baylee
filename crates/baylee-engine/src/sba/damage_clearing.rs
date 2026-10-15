//! One-shot destruction replacements that clear damage without regenerating.
use crate::{
    choice::{Pending, YesNoPrompt},
    damage_history::BattlefieldDeparture,
    state::GameState,
};
use baylee_cards_dsl::{CounterKind, Modifier};
use baylee_core::ids::{DamageSourceRef, EffectId, ObjectId};
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub(crate) struct DamageClearingAttempt {
    pub(crate) reference: DamageSourceRef,
    pub(crate) effect: EffectId,
    pub(crate) regeneratable: bool,
    pub(crate) by_effect: bool,
    pub(crate) departure: Option<BattlefieldDeparture>,
}

#[derive(Clone, Debug, Default)]
struct Queue {
    entries: VecDeque<DamageClearingAttempt>,
}

/// Ordinarily empty; allocate only when another destruction replacement competes.
#[derive(Clone, Debug, Default)]
pub(crate) struct DamageClearingChoices(Option<Box<Queue>>);

impl DamageClearingChoices {
    pub(crate) fn entries(&self) -> impl Iterator<Item = &DamageClearingAttempt> {
        self.0.iter().flat_map(|queue| queue.entries.iter())
    }

    fn pop(&mut self) -> Option<DamageClearingAttempt> {
        let queue = self.0.as_mut()?;
        let attempt = queue.entries.pop_front();
        if queue.entries.is_empty() {
            self.0 = None;
        }
        attempt
    }
}

fn applies(state: &GameState, reference: DamageSourceRef, effect: EffectId) -> bool {
    state.object(reference.object).is_some_and(|object| {
        object.version == reference.version
            && object.zone == crate::zone::Zone::Battlefield
            && !object
                .characteristics()
                .keywords
                .contains(baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE)
            && state.effects.iter().any(|candidate| {
                candidate.id == effect
                    && candidate.modifier == Modifier::NextDestructionClearsDamage
                    && crate::effects::applies_to(state, candidate, object)
            })
    })
}

fn clear(state: &mut GameState, reference: DamageSourceRef, effect: EffectId) {
    state
        .effects
        .remove_where(|candidate| candidate.id == effect);
    if let Some(object) = state.object_mut(reference.object) {
        object.damage = 0;
    }
    // Deathtouch is a separate since-the-last-SBA window, closed by the SBA
    // pass itself. This replacement changes only marked damage, never tapping
    // or combat membership, and is unaffected by regeneration prohibitions.
    state.invalidate_projections();
}

pub(super) fn replace_destroy(
    state: &mut GameState,
    id: ObjectId,
    regeneration_available: bool,
    regeneratable: bool,
    by_effect: bool,
    departure: Option<&BattlefieldDeparture>,
) -> bool {
    let Some(object) = state.object(id) else {
        return false;
    };
    let Some(effect) = state
        .effects
        .iter()
        .find(|effect| {
            effect.modifier == Modifier::NextDestructionClearsDamage
                && crate::effects::applies_to(state, effect, object)
        })
        .map(|effect| effect.id)
    else {
        return false;
    };
    let reference = DamageSourceRef {
        object: id,
        version: object.version,
    };
    let competing =
        regeneration_available || (by_effect && object.counters.get(CounterKind::Shield) > 0);
    if competing {
        state
            .damage_clearing_choices
            .0
            .get_or_insert_with(Box::default)
            .entries
            .push_back(DamageClearingAttempt {
                reference,
                effect,
                regeneratable,
                by_effect,
                departure: departure.cloned(),
            });
    } else {
        clear(state, reference, effect);
    }
    true
}

pub(crate) fn damage_clearing_choice(state: &mut GameState) -> Option<Pending> {
    loop {
        // Controller changes (including concessions) are reflected at the time
        // of the choice, rather than retaining a stale APNAP seat from enqueue.
        let mut queue = state.damage_clearing_choices.0.take()?;
        let seats = state.players.len();
        let active = usize::from(state.turn.active.get());
        queue.entries.make_contiguous().sort_by_key(|attempt| {
            let controller = state
                .object(attempt.reference.object)
                .map_or(state.turn.active, |object| object.controller);
            (usize::from(controller.get()) + seats - active) % seats
        });
        let attempt = queue.entries.front().expect("nonempty queue");
        let (reference, effect) = (attempt.reference, attempt.effect);
        state.damage_clearing_choices.0 = Some(queue);
        if applies(state, reference, effect)
            && let Some(object) = state.object(reference.object)
            && !state.has_left(object.controller)
        {
            return Some(Pending::YesNo {
                player: object.controller,
                prompt: YesNoPrompt::UseDamageClearingReplacement {
                    permanent: object.id,
                },
                source: None,
            });
        }
        let attempt = state.damage_clearing_choices.pop().expect("held attempt");
        if state
            .object(attempt.reference.object)
            .is_some_and(|object| object.version == attempt.reference.version)
        {
            super::destroy_without_damage_clearing(
                state,
                attempt.reference.object,
                attempt.regeneratable,
                attempt.departure,
                attempt.by_effect,
            );
        }
    }
}

pub(crate) fn answer_damage_clearing_choice(state: &mut GameState, use_clear: bool) {
    let attempt = state
        .damage_clearing_choices
        .pop()
        .expect("offered damage-clearing choice");
    if state
        .object(attempt.reference.object)
        .is_none_or(|object| object.version != attempt.reference.version)
    {
        return;
    }
    if use_clear && applies(state, attempt.reference, attempt.effect) {
        clear(state, attempt.reference, attempt.effect);
    } else {
        super::destroy_without_damage_clearing(
            state,
            attempt.reference.object,
            attempt.regeneratable,
            attempt.departure,
            attempt.by_effect,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        combat::AttackerInfo,
        effects::{ContinuousEffect, EffectFilter, EffectOrigin},
        engine::synthetic::{SyntheticLookup, preset},
        event::Cause,
        object::{ObjectKind, Status},
        zone::{Zone, ZoneLocation, ZonePosition},
    };
    use baylee_cards_dsl::{Duration, KeywordSet};
    use baylee_core::{
        ids::{Defender, PlayerId},
        types::TypeSet,
    };

    fn setup() -> (GameState, ObjectId) {
        let mut state =
            GameState::from_preset(&preset(600, &[]), &SyntheticLookup::new(vec![])).unwrap();
        let name = state.names.intern("protected land");
        let land = state.create_bare(
            PlayerId::new(0),
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let base = std::sync::Arc::make_mut(&mut state.object_mut(land).unwrap().base);
        base.types = TypeSet::LAND.union(TypeSet::CREATURE);
        base.power = Some(2);
        base.toughness = Some(2);
        state.invalidate_projections();
        state.refresh_characteristics();
        (state, land)
    }

    fn protect(state: &mut GameState, land: ObjectId) -> EffectId {
        let modifier = Modifier::NextDestructionClearsDamage;
        let timestamp = state.next_timestamp();
        let filter = EffectFilter::object(state, land);
        state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: None,
            controller: PlayerId::new(0),
            origin: EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp,
            duration: Duration::UntilEndOfTurn,
            filter,
            modifier,
        })
    }

    fn held(state: &GameState, id: EffectId) -> bool {
        state.effects.iter().any(|effect| effect.id == id)
    }

    #[test]
    fn clearing_consumes_one_exact_effect_without_regeneration_actions() {
        let (mut state, land) = setup();
        let first = protect(&mut state, land);
        let second = protect(&mut state, land);
        state.object_mut(land).unwrap().damage = 2;
        state.combat.declare_attackers([AttackerInfo {
            creature: land,
            defending: Defender::Player(PlayerId::new(1)),
            blocked: false,
            band: None,
        }]);
        super::super::destroy_no_regen(&mut state, land);
        assert!(!held(&state, first));
        assert!(held(&state, second));
        let object = state.object(land).unwrap();
        assert_eq!(object.damage, 0);
        assert_eq!(object.zone, Zone::Battlefield);
        assert!(!object.status.contains(Status::TAPPED));
        assert!(state.combat.is_attacking(land));
        // An undamaged land still replaces destruction, consuming the second effect.
        super::super::destroy(&mut state, land);
        assert!(!held(&state, second));
        assert_eq!(state.object(land).unwrap().zone, Zone::Battlefield);
        super::super::destroy(&mut state, land);
        assert_eq!(state.object(land).unwrap().zone, Zone::Graveyard);
    }

    #[test]
    fn lethal_and_deathtouch_sbas_use_clearing_even_when_regeneration_is_forbidden() {
        for deathtouch in [false, true] {
            let (mut state, land) = setup();
            protect(&mut state, land);
            let object = state.object_mut(land).unwrap();
            object.damage = if deathtouch { 1 } else { 2 };
            object.deathtouched = deathtouch;
            let version = object.version;
            state.per_turn.cant_regenerate.push((land, version));
            super::super::run(&mut state, &SyntheticLookup::new(vec![]));
            let object = state.object(land).unwrap();
            assert_eq!(object.zone, Zone::Battlefield);
            assert_eq!(object.damage, 0);
            assert!(!object.deathtouched);
            assert!(!object.status.contains(Status::TAPPED));
            super::super::run(&mut state, &SyntheticLookup::new(vec![]));
            assert_eq!(state.object(land).unwrap().zone, Zone::Battlefield);
        }
    }

    #[test]
    fn indestructibility_preserves_effect_and_blink_does_not_preserve_protection() {
        let (mut state, land) = setup();
        let effect = protect(&mut state, land);
        std::sync::Arc::make_mut(&mut state.object_mut(land).unwrap().base).keywords =
            KeywordSet::INDESTRUCTIBLE;
        state.invalidate_projections();
        state.refresh_characteristics();
        super::super::destroy_no_regen(&mut state, land);
        assert!(held(&state, effect));
        std::sync::Arc::make_mut(&mut state.object_mut(land).unwrap().base).keywords =
            KeywordSet::EMPTY;
        state
            .move_object(
                land,
                ZoneLocation::Exile(PlayerId::new(0)),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        state
            .move_object(
                land,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        state.refresh_characteristics();
        super::super::destroy(&mut state, land);
        assert_eq!(state.object(land).unwrap().zone, Zone::Graveyard);
    }

    #[test]
    fn declining_retains_effect_destruction_and_regeneration_prohibition() {
        let (mut state, land) = setup();
        let effect = protect(&mut state, land);
        state.object_mut(land).unwrap().regeneration_shields = 1;
        crate::replacement::put_counters(&mut state, land, CounterKind::Shield, 1);
        super::super::destroy_no_regen(&mut state, land);
        assert!(damage_clearing_choice(&mut state).is_some());
        assert_eq!(
            state
                .object(land)
                .unwrap()
                .counters
                .get(CounterKind::Shield),
            1
        );
        answer_damage_clearing_choice(&mut state, false);
        let object = state.object(land).unwrap();
        assert_eq!(object.counters.get(CounterKind::Shield), 0);
        assert_eq!(object.regeneration_shields, 1);
        assert!(!object.status.contains(Status::TAPPED));
        assert!(held(&state, effect));
    }

    #[test]
    fn declining_sba_clearing_cannot_turn_shield_counter_into_sba_protection() {
        let (mut state, land) = setup();
        let effect = protect(&mut state, land);
        state.object_mut(land).unwrap().regeneration_shields = 1;
        state.object_mut(land).unwrap().damage = 2;
        crate::replacement::put_counters(&mut state, land, CounterKind::Shield, 1);
        super::super::run(&mut state, &SyntheticLookup::new(vec![]));
        assert!(damage_clearing_choice(&mut state).is_some());
        answer_damage_clearing_choice(&mut state, false);
        let object = state.object(land).unwrap();
        assert_eq!(object.counters.get(CounterKind::Shield), 1);
        assert_eq!(object.regeneration_shields, 0);
        assert!(object.status.contains(Status::TAPPED));
        assert_eq!(object.damage, 0);
        assert!(held(&state, effect));
    }

    #[test]
    fn three_way_competition_rechecks_apnap_after_each_answer() {
        let (mut state, first) = setup();
        let name = state.names.intern("other land");
        let second = state.create_bare(
            PlayerId::new(1),
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        for land in [first, second] {
            protect(&mut state, land);
            state.object_mut(land).unwrap().regeneration_shields = 1;
            crate::replacement::put_counters(&mut state, land, CounterKind::Shield, 1);
        }
        state.turn.active = PlayerId::new(1);
        super::super::destroy_all(&mut state, &[first, second], false);
        assert!(
            matches!(super::super::destruction_choice(&mut state), Some(Pending::YesNo { player, prompt: YesNoPrompt::UseDamageClearingReplacement { permanent }, .. }) if player == PlayerId::new(1) && permanent == second)
        );
        let checkpoint = state.clone();
        assert_eq!(state.snapshot_hash(), checkpoint.snapshot_hash());
        answer_damage_clearing_choice(&mut state, false);
        assert!(
            matches!(super::super::destruction_choice(&mut state), Some(Pending::YesNo { player, prompt: YesNoPrompt::UseShieldCounter { .. }, .. }) if player == PlayerId::new(1))
        );
        super::super::answer_shield_choice(&mut state, true);
        assert!(
            matches!(super::super::destruction_choice(&mut state), Some(Pending::YesNo { player, prompt: YesNoPrompt::UseDamageClearingReplacement { .. }, .. }) if player == PlayerId::new(0))
        );
        answer_damage_clearing_choice(&mut state, true);
        assert!(super::super::destruction_choice(&mut state).is_none());
        assert_eq!(state.object(first).unwrap().regeneration_shields, 1);
        assert_eq!(
            state
                .object(first)
                .unwrap()
                .counters
                .get(CounterKind::Shield),
            1
        );
    }
    #[test]
    fn queued_destruction_hashes_retained_departure_and_resumes_from_checkpoint() {
        let (mut state, land) = setup();
        protect(&mut state, land);
        state.object_mut(land).unwrap().regeneration_shields = 1;
        super::super::destroy_all(&mut state, &[land], false);
        let mut checkpoint = state.clone();
        let original_hash = state.snapshot_hash();
        let victim = state.object(land).unwrap().clone();
        let attempt = state
            .damage_clearing_choices
            .0
            .as_mut()
            .unwrap()
            .entries
            .front_mut()
            .unwrap();
        let departure = attempt.departure.as_mut().unwrap();
        departure.damage = Some(crate::damage_history::DamageDeath {
            seq: 1,
            victim,
            sources: Vec::new(),
        });
        assert_ne!(state.snapshot_hash(), original_hash);
        let before_damage_change = state.snapshot_hash();
        state
            .damage_clearing_choices
            .0
            .as_mut()
            .unwrap()
            .entries
            .front_mut()
            .unwrap()
            .departure
            .as_mut()
            .unwrap()
            .damage
            .as_mut()
            .unwrap()
            .victim
            .damage = 1;
        assert_ne!(state.snapshot_hash(), before_damage_change);
        // The unchanged checkpoint resumes through the same exact replacement.
        let mut duplicate = checkpoint.clone();
        answer_damage_clearing_choice(&mut checkpoint, true);
        answer_damage_clearing_choice(&mut duplicate, true);
        assert_eq!(checkpoint.snapshot_hash(), duplicate.snapshot_hash());
        assert_eq!(checkpoint.object(land).unwrap().regeneration_shields, 1);
    }
}
