//! Complete granted definitions and deterministic, non-positional identities.

use super::ContinuousEffect;
use crate::copiable_abilities::AbilityProvenance;
use crate::object::Rider;
use crate::state::{CardLookup, GameState};
use crate::text_changes::TextChangeMap;
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{
    AbilityDef, AbilityDonorZone, ActivatedParts, Cost, Effect, Filter, Modifier,
};
use baylee_core::ids::{DamageSourceRef, EffectId, ObjectId};
use baylee_core::mana::ManaColor;
use std::collections::BTreeMap;

static BASIC_WHITE: AbilityDef = ActivatedParts {
    mana_ability: true,
    ..ActivatedParts::new(Cost::TAP, &[Effect::mana(ManaColor::White, 1)])
}
.build();
static BASIC_BLUE: AbilityDef = ActivatedParts {
    mana_ability: true,
    ..ActivatedParts::new(Cost::TAP, &[Effect::mana(ManaColor::Blue, 1)])
}
.build();
static BASIC_BLACK: AbilityDef = ActivatedParts {
    mana_ability: true,
    ..ActivatedParts::new(Cost::TAP, &[Effect::mana(ManaColor::Black, 1)])
}
.build();
static BASIC_RED: AbilityDef = ActivatedParts {
    mana_ability: true,
    ..ActivatedParts::new(Cost::TAP, &[Effect::mana(ManaColor::Red, 1)])
}
.build();
static BASIC_GREEN: AbilityDef = ActivatedParts {
    mana_ability: true,
    ..ActivatedParts::new(Cost::TAP, &[Effect::mana(ManaColor::Green, 1)])
}
.build();

static EXILE_SELF_PARTS: [baylee_cards_dsl::CostPart; 1] = [baylee_cards_dsl::CostPart::ExileSelf];
static EMBALM_EFFECTS: [Effect; 1] = [Effect::CreateTokenCopyOfSource {
    mods: baylee_cards_dsl::EMBALM_MODS,
}];

fn basic_mana_definition(color: ManaColor) -> Option<(u32, &'static AbilityDef)> {
    match color {
        ManaColor::White => Some((10_000, &BASIC_WHITE)),
        ManaColor::Blue => Some((10_001, &BASIC_BLUE)),
        ManaColor::Black => Some((10_002, &BASIC_BLACK)),
        ManaColor::Red => Some((10_003, &BASIC_RED)),
        ManaColor::Green => Some((10_004, &BASIC_GREEN)),
        ManaColor::Colorless => None,
    }
}

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
    /// Original donor object from which this ability was copied, if any.
    pub origin_donor: Option<ObjectId>,
    /// Specific recipient incarnation when synthesized per card (Encore/Embalm at mana cost).
    pub recipient: Option<DamageSourceRef>,
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
            AbilityDef::ModalActivated { cost, .. }
            | AbilityDef::Encore { cost }
            | AbilityDef::Unearth { cost } => (cost, &[] as &[_]),
            _ => return None,
        };
        Some(Self {
            id: 0,
            definition: *definition,
            cost,
            effects,
            source,
            origin_donor: None,
            recipient: None,
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

fn linked_exile_donors(
    state: &GameState,
    effect: &ContinuousEffect,
    filter: &Filter,
    allow_exiled_with: bool,
) -> Vec<ObjectId> {
    let Some(source) = effect.source.and_then(|id| state.object(id)) else {
        return Vec::new();
    };
    let identity = DamageSourceRef {
        object: source.id,
        version: source.version,
    };
    let link = Rider::LinkedExile {
        source: identity,
        acquisition: source.rules_acquisition(),
    };
    let with = Rider::ExiledWith {
        host: source.id,
        version: u16::try_from(source.version).unwrap_or(u16::MAX),
    };
    state
        .arena
        .iter()
        .filter_map(|(_, donor)| {
            let linked = donor.riders.contains(&link)
                || (allow_exiled_with
                    && donor.riders.contains(&with)
                    && !donor.riders.iter().any(|rider| {
                        matches!(
                            rider,
                            Rider::LinkedExile { source: s, .. } if *s == identity
                        )
                    }));
            (donor.zone == Zone::Exile
                && linked
                && crate::eval::matches(filter, state, donor, effect.controller, source.id))
            .then_some(donor.id)
        })
        .collect()
}

fn zone_donors(
    state: &GameState,
    effect: &ContinuousEffect,
    filter: &Filter,
    zone: AbilityDonorZone,
    allow_exiled_with: bool,
) -> Vec<ObjectId> {
    let this = effect
        .source
        .or(match effect.filter {
            super::EffectFilter::ObjectIs(id, _) => Some(id),
            _ => None,
        })
        .unwrap_or(ObjectId::NO_SOURCE);
    let raw: Vec<ObjectId> = match zone {
        AbilityDonorZone::LinkedExile => {
            return linked_exile_donors(state, effect, filter, allow_exiled_with);
        }
        AbilityDonorZone::Battlefield => state.battlefield_seen().collect(),
        AbilityDonorZone::Graveyard(who) => {
            crate::eval::players_for_source(who, state, effect.controller, effect.source)
                .into_iter()
                .flatten()
                .flat_map(|p| state.zones.list(ZoneLocation::Graveyard(p)).iter().copied())
                .collect()
        }
        AbilityDonorZone::Exile(who) => {
            let Some(owners) =
                crate::eval::players_for_source(who, state, effect.controller, effect.source)
            else {
                return Vec::new();
            };
            state
                .arena
                .iter()
                .filter_map(|(_, o)| {
                    (o.zone == Zone::Exile && owners.contains(&o.owner)).then_some(o.id)
                })
                .collect()
        }
        AbilityDonorZone::TopOfYourLibrary => state
            .zones
            .list(ZoneLocation::Library(effect.controller))
            .last()
            .copied()
            .into_iter()
            .collect(),
    };
    raw.into_iter()
        .filter(|&id| {
            state
                .object(id)
                .is_some_and(|o| crate::eval::matches(filter, state, o, effect.controller, this))
        })
        .collect()
}

fn push_donor_own_abilities(
    state: &GameState,
    lookup: &impl CardLookup,
    effect: &ContinuousEffect,
    donor_id: ObjectId,
    discovered: &mut Vec<(Key, GrantedAbility)>,
) {
    let Some(donor) = state.object(donor_id) else {
        return;
    };
    let chars = donor.characteristics();
    if chars.abilities_lost.is_some() || chars.rules_text_lost {
        return;
    }
    let donor_ref = Some(DamageSourceRef {
        object: donor.id,
        version: donor.version,
    });
    let list = donor.ability_list(lookup);
    for (index, &definition) in list.abilities.iter().enumerate() {
        if definition.is_intrinsic_mana_ability() {
            continue;
        }
        let Some(mut ability) = GrantedAbility::new(&definition, effect.source) else {
            continue;
        };
        ability.origin_donor = Some(donor.id);
        ability.provenance = Some(list.origin(index));
        ability.text = state.ability_text(donor.id, u32::try_from(index).expect("ability index"));
        let key = Key {
            effect: effect.id,
            donor: donor_ref,
            acquisition: list.acquisition,
            index: u32::try_from(index).expect("ability index"),
        };
        if !discovered.iter().any(|(k, _)| *k == key) {
            discovered.push((key, ability));
        }
    }
    for color in crate::casting::intrinsic_mana_colors(state, donor.id) {
        let Some((synth_idx, def)) = basic_mana_definition(color) else {
            continue;
        };
        let Some(mut ability) = GrantedAbility::new(def, effect.source) else {
            continue;
        };
        ability.origin_donor = Some(donor.id);
        let key = Key {
            effect: effect.id,
            donor: donor_ref,
            acquisition: list.acquisition,
            index: synth_idx,
        };
        if !discovered.iter().any(|(k, _)| *k == key) {
            discovered.push((key, ability));
        }
    }
}

/// Refresh dynamic donors with the registry lookup before offers or public views.
/// The parent effect is shared by all recipients; donor definitions are stored once.
///
/// # Panics
/// Panics if one donor has more than `u32::MAX` ability clauses or the
/// registry exhausts the action-identity domain.
#[allow(clippy::too_many_lines)]
pub fn sync_granted_activated(state: &mut GameState, lookup: &impl CardLookup) {
    if !state.effects.may_grant_activated() {
        return;
    }
    // Even a lone granted clause cannot borrow the recipient's original
    // linked choices (CR 607.5a). Assign its family on the same clock as copies.
    let mut discovered: Vec<(Key, GrantedAbility)> = state
        .effects
        .grants
        .entries
        .iter()
        .filter(|entry| entry.key.donor.is_none() && entry.ability.acquisition.is_none())
        .map(|entry| (entry.key, entry.ability))
        .collect();
    if !state.effects.may_grant_from_linked_exile() && discovered.is_empty() {
        return;
    }
    let mut has_battlefield_grant = false;
    for effect in state.effects.iter() {
        if matches!(
            effect.modifier,
            Modifier::GrantsEncoreManaCost | Modifier::GrantsEmbalmManaCost
        ) {
            for p in &state.players {
                for &card_id in state.zones.list(ZoneLocation::Graveyard(p.id)) {
                    let Some(card_obj) = state.object(card_id) else {
                        continue;
                    };
                    let chars = card_obj.characteristics();
                    if chars.abilities_lost.is_some()
                        || chars.rules_text_lost
                        || !super::applies_to(state, effect, card_obj)
                    {
                        continue;
                    }
                    let Some(mana) = card_obj
                        .card
                        .and_then(|c| lookup.card(c.index))
                        .and_then(|def| def.faces.first())
                        .map(|f| f.mana_cost)
                        .filter(crate::casting::has_a_printed_cost)
                    else {
                        continue;
                    };
                    let cost = Cost {
                        mana,
                        parts: &EXILE_SELF_PARTS,
                    };
                    let (synth_idx, definition) = match effect.modifier {
                        Modifier::GrantsEncoreManaCost => (30_000_u32, AbilityDef::Encore { cost }),
                        Modifier::GrantsEmbalmManaCost => (
                            30_001_u32,
                            AbilityDef::Activated {
                                cost,
                                effects: &EMBALM_EFFECTS,
                                targets: None,
                                second_targets: None,
                                third_targets: None,
                                timing: baylee_cards_dsl::ActivationTiming::SorcerySpeed,
                                mana_ability: false,
                                zone: baylee_cards_dsl::ActivationZone::Graveyard,
                                limit: baylee_cards_dsl::ActivationLimit::Unlimited,
                                x_value: baylee_cards_dsl::ActivationX::Any,
                                cost_reduction: None,
                            },
                        ),
                        _ => unreachable!(),
                    };
                    let Some(mut ability) = GrantedAbility::new(&definition, effect.source) else {
                        continue;
                    };
                    let recipient_ref = DamageSourceRef {
                        object: card_obj.id,
                        version: card_obj.version,
                    };
                    ability.recipient = Some(recipient_ref);
                    ability.text = state.effect_text(effect);
                    let key = Key {
                        effect: effect.id,
                        donor: Some(recipient_ref),
                        acquisition: card_obj.rules_acquisition(),
                        index: synth_idx,
                    };
                    if !discovered.iter().any(|(k, _)| *k == key) {
                        discovered.push((key, ability));
                    }
                }
            }
            continue;
        }
        let (filter, zone, allow_exiled_with) = match effect.modifier {
            Modifier::GrantActivatedFromLinkedExile(filter) => {
                (filter, AbilityDonorZone::LinkedExile, false)
            }
            Modifier::GainActivatedAbilitiesOf { filter, zone } => (filter, zone, true),
            _ => continue,
        };
        if zone == AbilityDonorZone::Battlefield {
            has_battlefield_grant = true;
        }
        for donor_id in zone_donors(state, effect, filter, zone, allow_exiled_with) {
            push_donor_own_abilities(state, lookup, effect, donor_id, &mut discovered);
        }
    }
    if has_battlefield_grant {
        propagate_battlefield_grants(state, &mut discovered);
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

/// Bounded cycle-guarded transitive propagation for `AbilityDonorZone::Battlefield`
/// (`CR 613.8`, e.g. *Myr Welder* -> *Experiment Kraj* or mutual *Kraj* pairs).
fn propagate_battlefield_grants(state: &GameState, discovered: &mut Vec<(Key, GrantedAbility)>) {
    for _ in 0..8 {
        let before = discovered.len();
        for effect in state.effects.iter() {
            let Modifier::GainActivatedAbilitiesOf {
                filter,
                zone: AbilityDonorZone::Battlefield,
            } = effect.modifier
            else {
                continue;
            };
            for donor_id in zone_donors(state, effect, filter, AbilityDonorZone::Battlefield, true)
            {
                let Some(donor) = state.object(donor_id) else {
                    continue;
                };
                for prev_fx in state.effects.iter() {
                    if prev_fx.id == effect.id || !super::applies_to(state, prev_fx, donor) {
                        continue;
                    }
                    let inherited: Vec<(Key, GrantedAbility)> = if matches!(
                        prev_fx.modifier,
                        Modifier::GrantActivated { .. } | Modifier::GrantActivatedDefinition(_)
                    ) {
                        state
                            .effects
                            .grants
                            .for_effect(prev_fx.id)
                            .enumerate()
                            .map(|(idx, mut ab)| {
                                ab.source = effect.source;
                                ab.origin_donor = Some(donor.id);
                                (
                                    Key {
                                        effect: effect.id,
                                        donor: Some(DamageSourceRef {
                                            object: donor.id,
                                            version: donor.version,
                                        }),
                                        acquisition: u64::from(prev_fx.id.get()),
                                        index: 20_000 + u32::try_from(idx).unwrap_or(0),
                                    },
                                    ab,
                                )
                            })
                            .collect()
                    } else {
                        discovered
                            .iter()
                            .filter(|(k, ab)| {
                                k.effect == prev_fx.id && ab.origin_donor != Some(donor.id)
                            })
                            .map(|&(k, mut ab)| {
                                ab.source = effect.source;
                                (
                                    Key {
                                        effect: effect.id,
                                        ..k
                                    },
                                    ab,
                                )
                            })
                            .collect()
                    };
                    for (new_key, new_ab) in inherited {
                        if !discovered.iter().any(|(k, _)| *k == new_key) {
                            discovered.push((new_key, new_ab));
                        }
                    }
                }
            }
        }
        if discovered.len() == before {
            break;
        }
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
