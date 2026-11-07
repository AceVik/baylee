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
    #[cfg(feature = "test-support")]
    pub restriction: ManaRestriction,
    #[cfg(not(feature = "test-support"))]
    pub(crate) restriction: ManaRestriction,
    pub(crate) chosen_subtype: Option<SubtypeId>,
    pub(crate) source_watermark: Option<WatermarkId>,
    /// Exact exile that `ManaUse::LastLinkedExile` alone admits.
    pub(crate) last_linked_exile: Option<baylee_core::ids::DamageSourceRef>,
    /// What the snapshot hash takes the entry as. Made as [`deposit`] keeps
    /// it: an entry is never written again and a game keeps every one, so
    /// the hash after every answer reads this instead of digesting them all.
    pub(crate) digest: u64,
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
        let last_linked_exile = if restriction
            .is_some_and(|restriction| restriction.uses == ManaUse::LastLinkedExile)
        {
            object
                .and_then(|source| source.activation_mana_note(source.rules_acquisition()))
                .and_then(|(_, exile)| exile)
        } else {
            None
        };
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
            last_linked_exile,
            restriction: restriction.unwrap_or(ManaRestriction {
                uses: ManaUse::SpellsOrAbilities,
                filter: &Filter::Any,
                ability_filter: None,
                rider: baylee_cards_dsl::SpendRider::None,
                restricts: false,
                until_end_of_turn: false,
            }),
            digest: 0,
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
        if self.restriction.uses == ManaUse::LastLinkedExile {
            return !ability && self.admits_last_linked_exile(state, object);
        }
        let allowed = match self.restriction.uses {
            ManaUse::Spells | ManaUse::NonSpellPaymentsOrMatchingSpells => !ability,
            ManaUse::Abilities => ability,
            ManaUse::SpellsOrAbilities | ManaUse::CostsFromMatchingSource => true,
            ManaUse::CumulativeUpkeep | ManaUse::LastLinkedExile => false,
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

    fn admits_last_linked_exile(&self, state: &GameState, object: &GameObject) -> bool {
        let Some(exile) = self.last_linked_exile else {
            return false;
        };
        if object.id != exile.object {
            return false;
        }
        // The offer probes the exiled card before moving it; the actual cost
        // is paid after that exact incarnation has become the next stack object.
        let prospective =
            object.zone == crate::zone::Zone::Exile && object.version == exile.version;
        let announced = object.zone == crate::zone::Zone::Stack
            && object.version.checked_sub(1) == Some(exile.version);
        (prospective || announced)
            && state
                .damage_source(exile.object, Some(exile.version))
                .is_some_and(|previous| previous.zone == crate::zone::Zone::Exile)
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

impl GameState {
    /// Keeps `provenance` under `id`: the one door into `restriction_info`,
    /// so no entry is kept without the digest the hash reads.
    pub(crate) fn keep_provenance(&mut self, id: u32, mut provenance: ManaProvenance) {
        provenance.digest = crate::state::provenance_digest(id, &provenance);
        self.restriction_info.insert(id, provenance);
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
    state.keep_provenance(id, provenance);
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

#[cfg(any(test, feature = "test-support"))]
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
            last_linked_exile: None,
            digest: 0,
        }
    }
}
