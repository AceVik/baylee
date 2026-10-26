//! A turn remembers both the damage source and the card a creature became.

use crate::state::GameState;
use crate::zone::Zone;
use baylee_core::ids::DamageSourceRef;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct DamagedCreatureDeath {
    pub source: DamageSourceRef,
    pub returned: DamageSourceRef,
}

impl GameState {
    /// Called only for an actual creature death, after the final destination
    /// has been selected, retaining its pre-departure identity.
    pub(crate) fn record_damaged_creature_death(&mut self, victim: DamageSourceRef) {
        let returned = DamageSourceRef {
            object: victim.object,
            version: victim.version.wrapping_add(1),
        };
        for damage in &self.per_turn.permanent_damage {
            if damage.recipient != victim.object || damage.recipient_version != victim.version {
                continue;
            }
            let death = DamagedCreatureDeath {
                source: DamageSourceRef {
                    object: damage.source,
                    version: damage.source_version,
                },
                returned,
            };
            if !self.per_turn.damaged_creature_deaths.contains(&death) {
                self.per_turn.damaged_creature_deaths.push(death);
            }
        }
    }

    /// An intervening "if" asks whether a death occurred, even when its card
    /// has since left the graveyard. This is independent of the later return.
    pub(crate) fn source_damage_caused_death(&self, source: DamageSourceRef) -> bool {
        self.per_turn
            .damaged_creature_deaths
            .iter()
            .any(|death| death.source == source)
    }

    /// Re-scan on resolution, including deaths after the ability triggered,
    /// but never follow a card out of its recorded graveyard incarnation.
    pub(crate) fn damaged_creature_cards_in_graveyard(
        &self,
        source: DamageSourceRef,
    ) -> Vec<DamageSourceRef> {
        self.per_turn
            .damaged_creature_deaths
            .iter()
            .filter_map(|death| {
                (death.source == source
                    && self.object(death.returned.object).is_some_and(|object| {
                        object.zone == Zone::Graveyard
                            && object.version == death.returned.version
                            && object.is_card()
                    }))
                .then_some(death.returned)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, creature, preset};
    use crate::event::Cause;
    use crate::zone::{ZoneLocation, ZonePosition};
    use baylee_core::ids::PlayerId;

    #[test]
    fn damaged_deaths_remember_exact_successor_even_without_a_present_damage_source() {
        let card = creature(850_000, "Synthetic damage death", 1, 4, &[]);
        let lookup = SyntheticLookup::new(vec![card]);
        let mut state = GameState::from_preset(&preset(718, &[850_000, 850_000]), &lookup).unwrap();
        let objects = state.zones.list(ZoneLocation::Battlefield).clone();
        let source = state.source_identity(objects[0]).unwrap();
        let victim = state.source_identity(objects[1]).unwrap();
        let player = PlayerId::new(0);
        state.record_permanent_damage(source.object, Some(source.version), victim.object, 1, false);
        state
            .move_object(
                source.object,
                ZoneLocation::Hand(player),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        state
            .move_object(
                victim.object,
                ZoneLocation::Graveyard(player),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        let successor = DamageSourceRef {
            object: victim.object,
            version: victim.version + 1,
        };
        assert!(state.source_damage_caused_death(source));
        assert_eq!(
            state.damaged_creature_cards_in_graveyard(source),
            vec![successor]
        );
        let mut no_death = state.clone();
        no_death.per_turn.damaged_creature_deaths.clear();
        assert_ne!(state.snapshot_hash(), no_death.snapshot_hash());
        assert_ne!(state.loop_signature(), no_death.loop_signature());
        let mut stale = state.clone();
        stale.per_turn.damaged_creature_deaths[0].returned.version += 1;
        assert_ne!(
            state.loop_signature(),
            stale.loop_signature(),
            "an unusable graveyard incarnation changes the future return"
        );
        let mut repeated = state.clone();
        repeated
            .per_turn
            .damaged_creature_deaths
            .push(repeated.per_turn.damaged_creature_deaths[0]);
        assert_eq!(
            state.loop_signature(),
            repeated.loop_signature(),
            "loop facts are canonical and deduplicated"
        );
        assert!(!state.source_damage_caused_death(state.source_identity(source.object).unwrap()));
        state
            .move_object(
                victim.object,
                ZoneLocation::Exile(player),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        state
            .move_object(
                victim.object,
                ZoneLocation::Graveyard(player),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        assert!(
            state.source_damage_caused_death(source),
            "the intervening if records the actual death"
        );
        assert!(
            state.damaged_creature_cards_in_graveyard(source).is_empty(),
            "the return cannot follow a new graveyard incarnation"
        );
        state.per_turn.reset();
        assert!(!state.source_damage_caused_death(source));
    }

    #[test]
    fn damaged_deaths_rescan_includes_a_creature_that_dies_later_in_the_turn() {
        let card = creature(850_001, "Synthetic later death", 1, 4, &[]);
        let lookup = SyntheticLookup::new(vec![card]);
        let mut state =
            GameState::from_preset(&preset(719, &[850_001, 850_001, 850_001]), &lookup).unwrap();
        let objects = state.zones.list(ZoneLocation::Battlefield).clone();
        let source = state.source_identity(objects[0]).unwrap();
        for victim in &objects[1..] {
            state.record_permanent_damage(source.object, Some(source.version), *victim, 1, false);
        }
        state
            .move_object(
                objects[1],
                ZoneLocation::Graveyard(PlayerId::new(0)),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        assert_eq!(state.damaged_creature_cards_in_graveyard(source).len(), 1);
        state
            .move_object(
                objects[2],
                ZoneLocation::Graveyard(PlayerId::new(0)),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        assert_eq!(state.damaged_creature_cards_in_graveyard(source).len(), 2);
    }

    #[test]
    fn death_trigger_freezes_graveyard_subject_even_if_victim_moves_before_collection() {
        use baylee_cards_dsl::{AbilityDef, Effect, Filter, Trigger};
        static ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::triggered!(
            Trigger::DiesAfterDamageByThis(&Filter::CREATURE),
            &[Effect::AtNextEndStepAboutSubject {
                effects: &[Effect::SacrificeEvent]
            }]
        )];
        let card = creature(850_002, "Synthetic death subject", 1, 4, &[]);
        let lookup = SyntheticLookup::new(vec![card]);
        let mut state = GameState::from_preset(&preset(720, &[850_002, 850_002]), &lookup).unwrap();
        let objects = state.zones.list(ZoneLocation::Battlefield).clone();
        let source = state.source_identity(objects[0]).unwrap();
        let victim = state.source_identity(objects[1]).unwrap();
        state.object_mut(source.object).unwrap().take_abilities(
            crate::object::AbilityList::from_static(ABILITIES, None, None),
        );
        state.record_permanent_damage(source.object, Some(source.version), victim.object, 1, false);
        state
            .move_object(
                victim.object,
                ZoneLocation::Graveyard(PlayerId::new(0)),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        state
            .move_object(
                victim.object,
                ZoneLocation::Exile(PlayerId::new(0)),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        let mut triggers = Vec::new();
        crate::damage_history::collect(&state, &lookup, 0, &mut triggers);
        assert_eq!(triggers.len(), 1);
        assert_eq!(
            triggers[0].event_subject,
            Some(DamageSourceRef {
                object: victim.object,
                version: victim.version + 1
            })
        );
        assert_eq!(
            triggers[0].event_object_identity.map(|identity| identity.0),
            Some(victim.version),
            "the event's battlefield LKI is separate from its graveyard successor"
        );
    }
}
