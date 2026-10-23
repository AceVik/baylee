//! Restrictions capture the producing ability's words and choices once.
use crate::object::GameObject;
use crate::state::GameState;
use crate::text_changes::RuleContext;
use baylee_cards_dsl::{Filter, ManaRestriction, ManaUse};
use baylee_core::ids::{PlayerId, SubtypeId, WatermarkId};

/// Information carried by mana after its producer leaves or changes choices.
#[derive(Clone, Copy, Debug)]
pub struct ManaProvenance {
    pub(crate) context: RuleContext,
    pub(crate) source_version: Option<u32>,
    pub(crate) controller: PlayerId,
    pub(crate) timestamp: u64,
    pub(crate) restriction: ManaRestriction,
    pub(crate) chosen_subtype: Option<SubtypeId>,
    pub(crate) source_watermark: Option<WatermarkId>,
}
impl ManaProvenance {
    pub(crate) fn capture(
        state: &GameState,
        context: RuleContext,
        version: Option<u32>,
        controller: PlayerId,
        restriction: Option<ManaRestriction>,
    ) -> Self {
        let object = state.damage_source(context.source, version);
        Self {
            context,
            source_version: version.or_else(|| {
                state
                    .source_identity(context.source)
                    .map(|source| source.version)
            }),
            controller,
            timestamp: object.map_or(state.timestamp, |object| object.timestamp),
            chosen_subtype: object.and_then(|object| object.chosen_subtype),
            source_watermark: object.and_then(GameObject::watermark),
            restriction: restriction.unwrap_or(ManaRestriction {
                uses: ManaUse::SpellsOrAbilities,
                filter: &Filter::Any,
                ability_filter: None,
                rider: baylee_cards_dsl::SpendRider::None,
                restricts: false,
                until_end_of_turn: false,
            }),
        }
    }

    pub(crate) fn admits_source_cost(&self, state: &GameState, object: &GameObject) -> bool {
        self.restriction.uses == ManaUse::CostsFromMatchingSource
            && self.matches(self.restriction.filter, state, object, self.controller)
    }

    pub(crate) fn admits_cumulative_upkeep(
        &self,
        state: &GameState,
        object: Option<&GameObject>,
    ) -> bool {
        self.restriction.uses == ManaUse::CumulativeUpkeep
            && match self.restriction.filter {
                Filter::Any => true,
                filter => object
                    .is_some_and(|object| self.matches(filter, state, object, self.controller)),
            }
    }

    pub(crate) fn admits(&self, state: &GameState, object: &GameObject, ability: bool) -> bool {
        if ability && self.restriction.uses == ManaUse::NonSpellPaymentsOrMatchingSpells {
            return true;
        }
        let allowed = match self.restriction.uses {
            ManaUse::Spells | ManaUse::NonSpellPaymentsOrMatchingSpells => !ability,
            ManaUse::Abilities => ability,
            ManaUse::SpellsOrAbilities | ManaUse::CostsFromMatchingSource => true,
            ManaUse::CumulativeUpkeep => false,
        };
        let filter = if ability {
            self.restriction
                .ability_filter
                .unwrap_or(self.restriction.filter)
        } else {
            self.restriction.filter
        };
        allowed && self.matches(filter, state, object, self.controller)
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
            Filter::SharesWatermarkWithSource => self
                .source_watermark
                .is_some_and(|watermark| object.watermark() == Some(watermark)),
            _ => crate::eval::matches_with_context(filter, state, object, player, self.context),
        }
    }
}

/// Every unit retains its original producer, including ordinary intrinsic mana.
/// Ridden entries annotate the ordinary counters; they never count twice.
pub(crate) fn deposit(
    state: &mut GameState,
    player: PlayerId,
    color: baylee_core::mana::ManaColor,
    amount: u16,
    snow: bool,
    provenance: ManaProvenance,
) -> bool {
    use baylee_core::mana::{ManaFlags, RestrictedMana, RestrictionId};
    if amount == 0 {
        return true;
    }
    let restriction = provenance.restriction;
    if !restriction.restricts
        && state.players[usize::from(player.get())]
            .mana_pool
            .available(color)
            .checked_add(u32::from(amount))
            .is_none()
    {
        state.numeric_failure = Some("mana production exceeds u32 per color");
        return false;
    }
    let id = state.next_restriction_id;
    let Some(next) = id.checked_add(1) else {
        state.numeric_failure = Some("mana provenance identity exhausted");
        return false;
    };
    state.next_restriction_id = next;
    state.restriction_info.insert(id, provenance);
    let mana = RestrictedMana {
        color,
        amount,
        restriction: RestrictionId(id),
        flags: (if snow {
            ManaFlags::SNOW
        } else {
            ManaFlags::NONE
        })
        .union(if restriction.until_end_of_turn {
            ManaFlags::UNTIL_END_OF_TURN.union(ManaFlags::NO_EMPTY)
        } else {
            ManaFlags::NONE
        }),
    };
    let pool = &mut state.players[usize::from(player.get())].mana_pool;
    if restriction.restricts {
        pool.add_restricted(mana);
    } else {
        pool.add_ridden(mana);
    }
    true
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
                resolved_name: None,
                reference: None,
            },
            source_version: None,
            controller: PlayerId::new(0),
            timestamp: 0,
            restriction: ManaRestriction {
                uses: ManaUse::Spells,
                filter,
                ability_filter: None,
                rider,
                restricts: true,
                until_end_of_turn: false,
            },
            chosen_subtype: None,
            source_watermark: None,
        }
    }
}
