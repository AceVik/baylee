//! One definition lookup for printed and granted activation paths.
use super::{AbilityDef, CardLookup, Engine, ObjectId};

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    pub(crate) fn granted_definition(
        &self,
        source: ObjectId,
        index: u32,
    ) -> Option<crate::effects::GrantedAbility> {
        self.activating_grant
            .filter(|(id, ability, _)| *id == source && *ability == index)
            .map(|(_, _, grant)| grant)
            .or_else(|| {
                crate::effects::granted_activated(&self.state, source)
                    .find(|grant| grant.id == index)
            })
    }

    pub(crate) fn activation_definition(&self, source: ObjectId, index: u32) -> Option<AbilityDef> {
        if index == crate::choice::TURN_FACE_UP {
            let (mana, parts, _) = self.turn_face_up_spec(source)?;
            Some(AbilityDef::Activated {
                cost: baylee_cards_dsl::Cost { mana, parts },
                effects: &[],
                targets: None,
                second_targets: None,
                third_targets: None,
                timing: baylee_cards_dsl::ActivationTiming::InstantSpeed,
                mana_ability: false,
                zone: baylee_cards_dsl::ActivationZone::Battlefield,
                limit: baylee_cards_dsl::ActivationLimit::Unlimited,
                x_value: baylee_cards_dsl::ActivationX::Any,
                cost_reduction: None,
            })
        } else if crate::choice::granted_slot(index).is_some() {
            self.granted_definition(source, index)
                .map(|grant| grant.definition)
        } else {
            let obj = self.state.object(source)?;
            if crate::layers::ability_is_lost(&self.state, obj, index) {
                return None;
            }
            obj.abilities(&self.lookup).get(index as usize).copied()
        }
    }
}
