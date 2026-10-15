//! Shield counters protect against effect destruction, independently of abilities.
use crate::{
    choice::{Pending, YesNoPrompt},
    state::GameState,
};
use baylee_cards_dsl::CounterKind;
use baylee_core::ids::{DamageSourceRef, ObjectId};
use std::collections::VecDeque;

#[derive(Clone, Debug, Default, Hash)]
pub(crate) struct ShieldChoices(
    // Rarely populated: on 64-bit targets the idle slot is 8 bytes instead
    // of the 32-byte VecDeque header in every GameState (asserted below).
    #[allow(clippy::box_collection)] Option<Box<VecDeque<(usize, DamageSourceRef)>>>,
);

/// One counter creates one replacement, regardless of the number of counters.
pub(super) fn replace_destroy(state: &mut GameState, id: ObjectId, regeneration: bool) -> bool {
    let Some(object) = state
        .object(id)
        .filter(|object| object.counters.get(CounterKind::Shield) > 0)
    else {
        return false;
    };
    if regeneration {
        let reference = DamageSourceRef {
            object: id,
            version: object.version,
        };
        let order = (usize::from(object.controller.get()) + state.players.len()
            - usize::from(state.turn.active.get()))
            % state.players.len();
        let queue = state.shield_choices.0.get_or_insert_with(Box::default);
        queue.push_back((order, reference));
        queue.make_contiguous().sort_by_key(|(order, _)| *order);
    } else {
        crate::replacement::remove_counters(state, id, CounterKind::Shield, 1);
    }
    true
}

pub(crate) fn shield_choice(state: &mut GameState) -> Option<Pending> {
    loop {
        let queue = state.shield_choices.0.as_mut()?;
        let Some((_, reference)) = queue.front().copied() else {
            state.shield_choices.0 = None;
            return None;
        };
        if let Some(object) = state.object(reference.object)
            && object.version == reference.version
            && !state.has_left(object.controller)
        {
            return Some(Pending::YesNo {
                player: object.controller,
                prompt: YesNoPrompt::UseShieldCounter {
                    permanent: object.id,
                },
                source: None,
            });
        }
        state
            .shield_choices
            .0
            .as_mut()
            .expect("held queue")
            .pop_front();
    }
}

pub(crate) fn answer_shield_choice(state: &mut GameState, shield: bool) {
    let queue = state
        .shield_choices
        .0
        .as_mut()
        .expect("offered shield choice");
    let (_, reference) = queue.pop_front().expect("offered shield choice");
    if queue.is_empty() {
        state.shield_choices.0 = None;
    }
    if state
        .object(reference.object)
        .is_none_or(|object| object.version != reference.version)
    {
        return;
    }
    if shield {
        crate::replacement::remove_counters(state, reference.object, CounterKind::Shield, 1);
    } else {
        super::destroy_with(state, reference.object, true, None, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use crate::{
        object::ObjectKind,
        zone::{Zone, ZoneLocation},
    };
    use baylee_cards_dsl::KeywordSet;
    use baylee_core::{ids::PlayerId, types::TypeSet};

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn idle_shield_choice_storage_is_one_pointer() {
        assert_eq!(size_of::<ShieldChoices>(), 8);
        assert_eq!(size_of::<VecDeque<(usize, DamageSourceRef)>>(), 32);
    }

    fn setup() -> (GameState, ObjectId) {
        let mut state =
            GameState::from_preset(&preset(572, &[]), &SyntheticLookup::new(vec![])).unwrap();
        let name = state.names.intern("shielded creature");
        let target = state.create_bare(
            PlayerId::new(0),
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let object = state.object_mut(target).unwrap();
        let base = std::sync::Arc::make_mut(&mut object.base);
        base.types = TypeSet::CREATURE;
        base.power = Some(2);
        base.toughness = Some(2);
        crate::replacement::put_counters(&mut state, target, CounterKind::Shield, 2);
        (state, target)
    }

    #[test]
    fn shield_replaces_effect_destruction_even_when_regeneration_is_forbidden() {
        let (mut state, target) = setup();
        super::super::destroy_no_regen(&mut state, target);
        assert_eq!(state.object(target).unwrap().zone, Zone::Battlefield);
        assert_eq!(
            state
                .object(target)
                .unwrap()
                .counters
                .get(CounterKind::Shield),
            1
        );
        assert_eq!(state.object(target).unwrap().regeneration_shields, 0);
        assert!(
            !state
                .object(target)
                .unwrap()
                .status
                .contains(crate::object::Status::TAPPED)
        );
    }

    #[test]
    fn indestructibility_preserves_shield_and_lethal_sba_ignores_it() {
        let (mut state, target) = setup();
        std::sync::Arc::make_mut(&mut state.object_mut(target).unwrap().base).keywords =
            KeywordSet::INDESTRUCTIBLE;
        state.invalidate_projections();
        state.refresh_characteristics();
        super::super::destroy(&mut state, target);
        assert_eq!(
            state
                .object(target)
                .unwrap()
                .counters
                .get(CounterKind::Shield),
            2
        );
        std::sync::Arc::make_mut(&mut state.object_mut(target).unwrap().base).keywords =
            KeywordSet::EMPTY;
        state.object_mut(target).unwrap().damage = 2;
        state.invalidate_projections();
        super::super::run(&mut state, &SyntheticLookup::new(vec![]));
        assert!(
            state
                .object(target)
                .is_none_or(|object| object.zone != Zone::Battlefield)
        );
    }

    #[test]
    fn affected_player_can_choose_shield_or_regeneration_without_speculation() {
        for shield in [false, true] {
            let (mut state, target) = setup();
            state.object_mut(target).unwrap().regeneration_shields = 1;
            state.object_mut(target).unwrap().damage = 1;
            super::super::destroy(&mut state, target);
            assert_eq!(state.object(target).unwrap().regeneration_shields, 1);
            assert_eq!(
                state
                    .object(target)
                    .unwrap()
                    .counters
                    .get(CounterKind::Shield),
                2
            );
            assert!(
                matches!(shield_choice(&mut state),Some(Pending::YesNo{player,..}) if player==PlayerId::new(0))
            );
            answer_shield_choice(&mut state, shield);
            assert!(shield_choice(&mut state).is_none());
            let object = state.object(target).unwrap();
            assert_eq!(object.zone, Zone::Battlefield);
            assert_eq!(
                object.counters.get(CounterKind::Shield),
                if shield { 1 } else { 2 }
            );
            assert_eq!(object.regeneration_shields, u8::from(shield));
            assert_eq!(object.damage, u16::from(shield));
            assert_eq!(
                object.status.contains(crate::object::Status::TAPPED),
                !shield
            );
        }
    }
}
