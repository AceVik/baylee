//! Complete granted definitions and deterministic, non-positional identities.

use super::ContinuousEffect;
use crate::copiable_abilities::AbilityProvenance;
use crate::object::Rider;
use crate::state::{CardLookup, GameState};
use crate::text_changes::TextChangeMap;
use crate::zone::Zone;
use baylee_cards_dsl::{AbilityDef, ActivatedParts, Cost, Effect, Modifier};
use baylee_core::ids::{DamageSourceRef, EffectId, ObjectId};
use std::collections::BTreeMap;

/// A complete granted ability; convenience fields are derived from its definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GrantedAbility {
    /// Stable action identity, independent of how many earlier grants still apply.
    pub id: u32,
    /// All restrictions, targets, costs and effects, including loyalty definitions.
    pub definition: AbilityDef,
    /// The ordinary mana/nonmana cost; loyalty costs remain in `definition`.
    pub cost: Cost,
    /// Instructions of the activated or loyalty definition.
    pub effects: &'static [Effect],
    /// Whether this definition is a mana ability.
    pub mana_ability: bool,
    /// Source of the granting continuous effect.
    pub source: Option<ObjectId>,
    /// Printed clause of the donor, when this is a copied definition.
    pub provenance: Option<AbilityProvenance>,
    /// Wording of the granted definition.
    pub text: TextChangeMap,
    /// Linked family acquired together from one donor (CR 607.5).
    pub acquisition: Option<u64>,
}

impl GrantedAbility {
    fn new(definition: &AbilityDef, source: Option<ObjectId>) -> Option<Self> {
        let (cost, effects) = match *definition {
            AbilityDef::Activated { cost, effects, .. }
            | AbilityDef::ActivatedConditional { cost, effects, .. } => (cost, effects),
            AbilityDef::Loyalty { effects, .. } => (Cost::FREE, effects),
            AbilityDef::ModalActivated { cost, .. } => (cost, &[] as &[_]),
            _ => return None,
        };
        Some(Self {
            id: 0,
            definition: *definition,
            cost,
            effects,
            source,
            mana_ability: definition.is_mana_ability(),
            provenance: None,
            text: TextChangeMap::default(),
            acquisition: None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Key {
    effect: EffectId,
    donor: Option<DamageSourceRef>,
    acquisition: u64,
    index: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Entry {
    key: Key,
    ability: GrantedAbility,
    active: bool,
}

/// IDs never recycle: stale choices cannot silently become another ability.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct GrantRegistry {
    entries: Vec<Entry>,
    keys: BTreeMap<Key, usize>,
    by_effect: BTreeMap<EffectId, Vec<usize>>,
    families: BTreeMap<Key, u64>,
}

impl GrantRegistry {
    fn insert(&mut self, key: Key, mut ability: GrantedAbility) {
        if let Some(&index) = self.keys.get(&key) {
            ability.id = self.entries[index].ability.id;
            self.entries[index].ability = ability;
            self.entries[index].active = true;
            return;
        }
        let index = self.entries.len();
        ability.id =
            crate::choice::granted_ability(u32::try_from(index).expect("grant identity fits"));
        self.entries.push(Entry {
            key,
            ability,
            active: true,
        });
        self.keys.insert(key, index);
        self.by_effect.entry(key.effect).or_default().push(index);
    }

    pub(super) fn register_fixed(&mut self, effect: &ContinuousEffect) {
        let definition = match effect.modifier {
            Modifier::GrantActivated {
                cost,
                effects,
                mana_ability,
            } => {
                let mut parts = ActivatedParts::new(cost, effects);
                parts.mana_ability = mana_ability;
                parts.build()
            }
            Modifier::GrantActivatedDefinition(definition) => *definition,
            _ => return,
        };
        if let Some(ability) = GrantedAbility::new(&definition, effect.source) {
            self.insert(
                Key {
                    effect: effect.id,
                    donor: None,
                    acquisition: 0,
                    index: 0,
                },
                ability,
            );
        }
    }

    pub(super) fn for_effect(&self, id: EffectId) -> impl Iterator<Item = GrantedAbility> + '_ {
        self.by_effect
            .get(&id)
            .into_iter()
            .flatten()
            .filter_map(|&index| {
                let entry = &self.entries[index];
                entry.active.then_some(entry.ability)
            })
    }
}

/// Refresh dynamic donors with the registry lookup before offers or public views.
/// The parent effect is shared by all recipients; donor definitions are stored once.
///
/// # Panics
/// Panics if one donor has more than `u32::MAX` ability clauses or the
/// registry exhausts the action-identity domain.
pub fn sync_granted_activated(state: &mut GameState, lookup: &impl CardLookup) {
    // Even a lone granted clause cannot borrow the recipient's original
    // linked choices (CR 607.5a). Assign its family on the same clock as copies.
    let mut discovered: Vec<_> = state
        .effects
        .grants
        .entries
        .iter()
        .filter(|entry| entry.key.donor.is_none() && entry.ability.acquisition.is_none())
        .map(|entry| (entry.key, entry.ability))
        .collect();
    for effect in state.effects.iter() {
        let Modifier::GrantActivatedFromLinkedExile(filter) = effect.modifier else {
            continue;
        };
        let Some(source) = effect.source.and_then(|id| state.object(id)) else {
            continue;
        };
        let link = Rider::LinkedExile {
            source: DamageSourceRef {
                object: source.id,
                version: source.version,
            },
            acquisition: source.rules_acquisition(),
        };
        for donor in state
            .arena
            .iter()
            .map(|(_, object)| object)
            .filter(|donor| {
                donor.zone == Zone::Exile
                    && donor.riders.contains(&link)
                    && crate::eval::matches(filter, state, donor, effect.controller, source.id)
            })
        {
            let list = donor.ability_list(lookup);
            for (index, &definition) in list.abilities.iter().enumerate() {
                let Some(mut ability) = GrantedAbility::new(&definition, effect.source) else {
                    continue;
                };
                ability.provenance = Some(list.origin(index));
                ability.text =
                    state.ability_text(donor.id, u32::try_from(index).expect("ability index"));
                discovered.push((
                    Key {
                        effect: effect.id,
                        donor: Some(DamageSourceRef {
                            object: donor.id,
                            version: donor.version,
                        }),
                        acquisition: list.acquisition,
                        index: u32::try_from(index).expect("ability index"),
                    },
                    ability,
                ));
            }
        }
    }
    for entry in &mut state.effects.grants.entries {
        if entry.key.donor.is_some() {
            entry.active = false;
        }
    }
    for (key, mut ability) in discovered {
        // All clauses from this donor share a family; another donor or
        // granting effect must not see its linked choices or exiled cards.
        let family = Key { index: 0, ..key };
        let acquisition = if let Some(&stamp) = state.effects.grants.families.get(&family) {
            stamp
        } else {
            let stamp = state.next_timestamp();
            state.effects.grants.families.insert(family, stamp);
            stamp
        };
        ability.acquisition = Some(acquisition);
        state.effects.grants.insert(key, ability);
    }
}

/// The captured definition belonging to an issued identity, including expired grants.
/// Stack objects outlive the effect that supplied their ability.
pub fn granted_definition(state: &GameState, id: u32) -> Option<GrantedAbility> {
    let index = usize::try_from(crate::choice::granted_slot(id)?).ok()?;
    state
        .effects
        .grants
        .entries
        .get(index)
        .map(|entry| entry.ability)
}
