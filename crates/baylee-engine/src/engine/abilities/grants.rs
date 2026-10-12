//! One definition lookup for printed and granted activation paths.
use super::{AbilityDef, CardLookup, Engine, ObjectId};

impl<L: CardLookup> Engine<L> {
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
        if crate::choice::granted_slot(index).is_some() {
            self.granted_definition(source, index)
                .map(|grant| grant.definition)
        } else {
            self.state
                .object(source)?
                .abilities(&self.lookup)
                .get(index as usize)
                .copied()
        }
    }
}
