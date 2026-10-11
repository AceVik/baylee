//! Restrictions capture the producing ability's words and choices once.
use crate::object::GameObject;
use crate::state::GameState;
use crate::text_changes::RuleContext;
use baylee_cards_dsl::{Filter, ManaRestriction, ManaUse};
use baylee_core::ids::{PlayerId, SubtypeId};

/// Information carried by mana after its producer leaves or changes choices.
#[derive(Clone, Copy, Debug)]
pub struct ManaProvenance {
    pub(crate) context: RuleContext,
    pub(crate) source_version: Option<u32>,
    pub(crate) controller: PlayerId,
    pub(crate) timestamp: u64,
    pub(crate) restriction: ManaRestriction,
    pub(crate) chosen_subtype: Option<SubtypeId>,
}
impl ManaProvenance {
    pub(crate) fn admits(&self, state: &GameState, object: &GameObject, ability: bool) -> bool {
        if ability && self.restriction.uses == ManaUse::NonSpellPaymentsOrMatchingSpells {
            return true;
        }
        let allowed = match self.restriction.uses {
            ManaUse::Spells | ManaUse::NonSpellPaymentsOrMatchingSpells => !ability,
            ManaUse::Abilities => ability,
            ManaUse::SpellsOrAbilities => true,
        };
        allowed && self.matches(self.restriction.filter, state, object, self.controller)
    }

    fn matches(
        &self,
        filter: &Filter,
        state: &GameState,
        object: &GameObject,
        player: PlayerId,
    ) -> bool {
        match filter {
            Filter::And(parts) => parts
                .iter()
                .all(|part| self.matches(part, state, object, player)),
            Filter::Or(parts) => parts
                .iter()
                .any(|part| self.matches(part, state, object, player)),
            Filter::Not(part) => !self.matches(part, state, object, player),
            Filter::MatchesChosenTypeOfSource => self
                .chosen_subtype
                .is_some_and(|subtype| object.characteristics().subtypes.contains(subtype)),
            _ => crate::eval::matches_with_context(filter, state, object, player, self.context),
        }
    }
}

#[cfg(test)]
impl
    From<(
        baylee_core::ids::ObjectId,
        &'static Filter,
        baylee_cards_dsl::SpendRider,
    )> for ManaProvenance
{
    fn from(
        (source, filter, rider): (
            baylee_core::ids::ObjectId,
            &'static Filter,
            baylee_cards_dsl::SpendRider,
        ),
    ) -> Self {
        Self {
            context: RuleContext {
                source,
                text: crate::text_changes::TextChangeMap::IDENTITY,
            },
            source_version: None,
            controller: PlayerId::new(0),
            timestamp: 0,
            restriction: ManaRestriction {
                uses: ManaUse::Spells,
                filter,
                rider,
                restricts: true,
                until_end_of_turn: false,
            },
            chosen_subtype: None,
        }
    }
}
