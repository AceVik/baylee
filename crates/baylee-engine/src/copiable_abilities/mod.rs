//! Composable rules text with explicit provenance (CR 707.2, 707.9a).
//!
//! Definitions stay in the immutable registry. Only their ordered references
//! are composed, so nested DSL references remain static without leaking memory.

use crate::object::{AbilityList, AbilityOrigin};
use baylee_cards_dsl::{AbilityDef, CopyMod};
use std::sync::Arc;

/// Where one ability in a composed rules list was printed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AbilityProvenance {
    /// Card face or token whose text supplied the definition.
    pub origin: Option<AbilityOrigin>,
    /// Index in that original printed list, never the composed list's index.
    pub index: u32,
    /// The exception inside a copy clause, when that clause grants the ability.
    pub copy_modifier: Option<u64>,
}

impl AbilityProvenance {
    /// Stable identity of the printed clause that supplied this ability.
    /// A quoted copy exception names its enclosing clause on the original
    /// copier; token definitions have no card-backed standing-policy handle.
    #[must_use]
    pub fn ability_ref(self) -> Option<baylee_core::ids::AbilityRef> {
        let printed = self.origin?.printed()?;
        Some(baylee_core::ids::AbilityRef::new(
            printed.card(),
            self.index,
        ))
    }
}

/// An immutable ability and the printed sentence that supplied it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AbilityEntry {
    /// The canonical definition; its nested filters retain their static lifetime.
    pub definition: &'static AbilityDef,
    /// Identity used for presentation and deterministic hashing.
    pub provenance: AbilityProvenance,
    /// Wording defined when a token was created; part of its copiable text.
    pub base_text: crate::text_changes::TextChangeMap,
}

/// A static ability defined by a token-creating instruction, rather than a
/// registry ability. Its typed operands remain immutable and copiable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RuntimeStatic {
    /// Effect of this ability on the object that has it.
    pub modifier: baylee_cards_dsl::Modifier,
    /// Literal wording when the token was created (CR 612.4).
    pub base_text: crate::text_changes::TextChangeMap,
}

/// A runtime concatenation of immutable ability definitions.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AbilityBundle {
    entries: Vec<AbilityEntry>,
    runtime_statics: Vec<RuntimeStatic>,
}

impl AbilityBundle {
    /// The complete ordered list, including repeated identical abilities.
    #[must_use]
    pub fn entries(&self) -> &[AbilityEntry] {
        &self.entries
    }

    /// Number of abilities, including every independently copied instance.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the bundle contains no abilities.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// One canonical definition by its runtime index.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&'static AbilityDef> {
        self.entries.get(index).map(|entry| entry.definition)
    }

    /// Canonical definitions, retaining their registry lifetime.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &'static AbilityDef> + '_ {
        self.entries.iter().map(|entry| entry.definition)
    }
}

/// A registry list or a cheaply cloned runtime composition.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AbilityDefs {
    /// Ordinary cards need neither an allocation nor a refcount.
    Static(&'static [AbilityDef]),
    /// Copies can concatenate any number of printed abilities.
    Shared(Arc<AbilityBundle>),
}

impl AbilityDefs {
    /// No rules text.
    pub const EMPTY: Self = Self::Static(&[]);

    /// Wrap an immutable registry list without allocating.
    #[must_use]
    pub const fn from_static(abilities: &'static [AbilityDef]) -> Self {
        Self::Static(abilities)
    }

    /// The number of abilities in runtime order.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Static(abilities) => abilities.len(),
            Self::Shared(bundle) => bundle.len(),
        }
    }

    /// Whether no ability is present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a canonical definition without borrowing the runtime list.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&'static AbilityDef> {
        match self {
            Self::Static(abilities) => abilities.get(index),
            Self::Shared(bundle) => bundle.get(index),
        }
    }

    /// Iterate independently of the source object's lifetime or later changes.
    #[must_use]
    pub fn iter(&self) -> AbilityIter {
        AbilityIter {
            abilities: self.clone(),
            next: 0,
        }
    }
}

impl From<&'static [AbilityDef]> for AbilityDefs {
    fn from(abilities: &'static [AbilityDef]) -> Self {
        Self::Static(abilities)
    }
}

impl<const N: usize> From<&'static [AbilityDef; N]> for AbilityDefs {
    fn from(abilities: &'static [AbilityDef; N]) -> Self {
        Self::Static(abilities)
    }
}

impl From<&AbilityDefs> for AbilityDefs {
    fn from(abilities: &AbilityDefs) -> Self {
        abilities.clone()
    }
}

impl From<Arc<AbilityBundle>> for AbilityDefs {
    fn from(bundle: Arc<AbilityBundle>) -> Self {
        Self::Shared(bundle)
    }
}

impl From<&Arc<AbilityBundle>> for AbilityDefs {
    fn from(bundle: &Arc<AbilityBundle>) -> Self {
        Self::Shared(bundle.clone())
    }
}

impl std::ops::Index<usize> for AbilityDefs {
    type Output = AbilityDef;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("ability index in range")
    }
}

impl IntoIterator for AbilityDefs {
    type Item = &'static AbilityDef;
    type IntoIter = AbilityIter;

    fn into_iter(self) -> Self::IntoIter {
        AbilityIter {
            abilities: self,
            next: 0,
        }
    }
}

impl IntoIterator for &AbilityDefs {
    type Item = &'static AbilityDef;
    type IntoIter = AbilityIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An owning iterator whose items still point to immutable registry data.
#[derive(Clone, Debug)]
pub struct AbilityIter {
    abilities: AbilityDefs,
    next: usize,
}

impl Iterator for AbilityIter {
    type Item = &'static AbilityDef;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.abilities.get(self.next)?;
        self.next += 1;
        Some(result)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.abilities.len() - self.next;
        (left, Some(left))
    }
}

impl ExactSizeIterator for AbilityIter {}
impl std::iter::FusedIterator for AbilityIter {}

/// Compose the copied text and all ability exceptions to a copy effect.
/// Repeated instances are deliberately preserved (CR 707.9a).
#[must_use]
pub fn compose(
    copied: AbilityList,
    own: &AbilityList,
    copy_index: u32,
    mods: &[CopyMod],
    resolving: Option<AbilityEntry>,
) -> AbilityList {
    let printed = copied.printed;
    let token = copied.token;
    let AbilityBundle {
        mut entries,
        mut runtime_statics,
    } = Arc::unwrap_or_clone(copied.into_bundle());
    // An exception replaces a defining ability as well as the defined
    // value (CR 707.9d). Otherwise devoid would erase KeepColor in layer 5.
    entries.retain(|entry| !overridden_characteristic_ability(entry.definition, mods));
    for (modifier_index, modifier) in mods.iter().enumerate() {
        match modifier {
            CopyMod::KeepOtherAbilities => {
                runtime_statics.extend_from_slice(own.runtime_statics());
                entries.extend(own.entries().into_iter().filter(|entry| {
                    !matches!(
                        entry.definition,
                        AbilityDef::CopyOnEnter { .. } | AbilityDef::CopyOnEnterUntilEot { .. }
                    )
                }));
            }
            CopyMod::GrantAbility(definition) => {
                let mut provenance = own.origin(copy_index as usize);
                provenance.copy_modifier = Some(modifier_index as u64);
                entries.push(AbilityEntry {
                    definition,
                    provenance,
                    base_text: own.base_text(copy_index as usize),
                });
            }
            CopyMod::KeepResolvingAbility => entries.extend(resolving),
            _ => {}
        }
    }
    AbilityList {
        abilities: AbilityDefs::Shared(Arc::new(AbilityBundle {
            entries,
            runtime_statics,
        })),
        printed,
        token,
    }
}

fn overridden_characteristic_ability(ability: &AbilityDef, mods: &[CopyMod]) -> bool {
    use baylee_cards_dsl::{Filter, Modifier};
    let AbilityDef::Static(definition) = ability else {
        return false;
    };
    if definition.condition.is_some() || definition.filter != Filter::This {
        return false;
    }
    match definition.modifier {
        Modifier::SetColor(_) | Modifier::AddColor(_) => mods
            .iter()
            .any(|modifier| matches!(modifier, CopyMod::KeepColor | CopyMod::SetColor(_))),
        Modifier::CharacteristicPT { .. } => mods
            .iter()
            .any(|modifier| matches!(modifier, CopyMod::SetPT(_, _))),
        _ => false,
    }
}

impl AbilityList {
    /// Capture a registry list and its origin without allocating.
    #[must_use]
    pub const fn from_static(
        abilities: &'static [AbilityDef],
        printed: Option<crate::object::PrintedFace>,
        token: Option<std::num::NonZeroU16>,
    ) -> Self {
        Self {
            abilities: AbilityDefs::Static(abilities),
            printed,
            token,
        }
    }

    /// The printed sentence for a runtime ability index. An index outside
    /// the engine's `u32` ability-address domain has no printed identity;
    /// it is reported as synthetic rather than aliasing another clause.
    #[must_use]
    pub fn origin(&self, index: usize) -> AbilityProvenance {
        if let AbilityDefs::Shared(bundle) = &self.abilities
            && let Some(entry) = bundle.entries.get(index)
        {
            return AbilityProvenance {
                origin: entry
                    .provenance
                    .origin
                    .or_else(|| AbilityOrigin::new(self.printed, self.token)),
                ..entry.provenance
            };
        }
        let Ok(index) = u32::try_from(index) else {
            return AbilityProvenance {
                origin: None,
                index: baylee_core::ids::AbilityRef::SYNTHETIC,
                copy_modifier: None,
            };
        };
        AbilityProvenance {
            origin: AbilityOrigin::new(self.printed, self.token),
            index,
            copy_modifier: None,
        }
    }

    /// Capture one ability with provenance, for "it has this ability".
    #[must_use]
    pub fn entry(&self, index: usize) -> Option<AbilityEntry> {
        Some(AbilityEntry {
            definition: self.abilities.get(index)?,
            provenance: self.origin(index),
            base_text: self.base_text(index),
        })
    }

    /// Materialize references, preserving every source's original index.
    #[must_use]
    pub fn entries(&self) -> Vec<AbilityEntry> {
        self.abilities
            .iter()
            .enumerate()
            .map(|(index, definition)| AbilityEntry {
                definition,
                provenance: self.origin(index),
                base_text: self.base_text(index),
            })
            .collect()
    }

    /// Share this list with an object or a captured stack ability.
    #[must_use]
    pub fn into_bundle(self) -> Arc<AbilityBundle> {
        if let AbilityDefs::Shared(bundle) = self.abilities {
            bundle
        } else {
            Arc::new(AbilityBundle {
                entries: self.entries(),
                runtime_statics: Vec::new(),
            })
        }
    }

    /// Token-defined static abilities in their original order. Their private
    /// indices follow the canonical list and never change activation indices.
    #[must_use]
    pub fn runtime_statics(&self) -> &[RuntimeStatic] {
        match &self.abilities {
            AbilityDefs::Shared(bundle) => &bundle.runtime_statics,
            AbilityDefs::Static(_) => &[],
        }
    }

    /// Add one token-defined static without manufacturing a leaked definition.
    #[must_use]
    pub fn with_runtime_static(mut self, ability: RuntimeStatic) -> Self {
        let mut runtime_statics = self.runtime_statics().to_vec();
        runtime_statics.push(ability);
        self.abilities = AbilityDefs::Shared(Arc::new(AbilityBundle {
            entries: self.entries(),
            runtime_statics,
        }));
        self
    }

    /// Copiable wording defined by a token's creating instruction.
    #[must_use]
    pub fn base_text(&self, index: usize) -> crate::text_changes::TextChangeMap {
        match &self.abilities {
            AbilityDefs::Shared(bundle) => bundle
                .entries
                .get(index)
                .map_or(crate::text_changes::TextChangeMap::IDENTITY, |entry| {
                    entry.base_text
                }),
            AbilityDefs::Static(_) => crate::text_changes::TextChangeMap::IDENTITY,
        }
    }

    /// Record wording from a token-creating instruction as copiable text.
    #[must_use]
    pub fn with_base_text(mut self, text: crate::text_changes::TextChangeMap) -> Self {
        if text != crate::text_changes::TextChangeMap::IDENTITY {
            let mut entries = self.entries();
            for entry in &mut entries {
                entry.base_text = entry.base_text.then(text);
            }
            let mut runtime_statics = self.runtime_statics().to_vec();
            for ability in &mut runtime_statics {
                ability.base_text = ability.base_text.then(text);
            }
            self.abilities = AbilityDefs::Shared(Arc::new(AbilityBundle {
                entries,
                runtime_statics,
            }));
        }
        self
    }
}

/// Apply characteristic exceptions as part of layer 1. Counters are entry
/// events and are deliberately left to the replacement pipeline.
pub fn apply_characteristic_exceptions(
    copied: &mut crate::object::Characteristics,
    previous: &crate::object::Characteristics,
    mods: &[CopyMod],
) {
    apply_characteristic_exceptions_with_text(
        copied,
        previous,
        mods,
        crate::text_changes::TextChangeMap::IDENTITY,
    );
}

/// Apply literal words in a copy clause through its captured text map.
/// Retained or otherwise derived characteristics are not literal words.
pub fn apply_characteristic_exceptions_with_text(
    copied: &mut crate::object::Characteristics,
    previous: &crate::object::Characteristics,
    mods: &[CopyMod],
    text: crate::text_changes::TextChangeMap,
) {
    for modifier in mods {
        match *modifier {
            CopyMod::AddType(types) => copied.types = copied.types.union(types),
            CopyMod::RemoveType(types) => copied.types = copied.types.difference(types),
            CopyMod::RemoveSupertype(types) => {
                copied.supertypes = copied.supertypes.difference(types);
            }
            CopyMod::AddSubtype(subtype) => copied.subtypes.insert(text.land_type(subtype)),
            CopyMod::AddKeyword(keywords) => {
                copied.keywords = copied.keywords.union(text.keywords(keywords));
            }
            CopyMod::SetPT(power, toughness) => {
                copied.power = Some(power);
                copied.toughness = Some(toughness);
            }
            CopyMod::SetColor(colors) => copied.colors = text.color_words(colors),
            CopyMod::KeepColor => copied.colors = previous.colors,
            CopyMod::NoManaCost => copied.mana_cost = baylee_core::mana::ManaCost::ZERO,
            CopyMod::AddCounter(..)
            | CopyMod::AddCounterIf(..)
            | CopyMod::AddCounterX(..)
            | CopyMod::KeepOtherAbilities
            | CopyMod::Grant(_)
            | CopyMod::GrantAbility(_)
            | CopyMod::KeepResolvingAbility => {}
        }
    }
}

/// One application of a copy effect, shared by entry choices and resolution.
#[derive(Clone, Copy)]
pub struct CopyApplication<'a> {
    /// Permanent or entering spell becoming the copy.
    pub source: baylee_core::ids::ObjectId,
    /// Current object whose copiable values are taken now.
    pub target: baylee_core::ids::ObjectId,
    /// The copy clause's exceptions.
    pub mods: &'a [CopyMod],
    /// Index of that clause in the copier's pre-copy list.
    pub copy_index: u32,
    /// Whether this application expires in the cleanup step.
    pub until_eot: bool,
    /// Captured definition for an exception saying "it has this ability".
    pub resolving: Option<AbilityEntry>,
    /// Semantic wording of the copy clause, not the target's text changes.
    pub text: crate::text_changes::TextChangeMap,
}

/// Snapshot and install a copy without changing zones, status, damage,
/// attachments or counters (CR 707.2b, 707.4).
#[allow(clippy::too_many_lines)] // The shared copy transaction keeps snapshots ahead of every write.
pub fn apply_copy(state: &mut crate::state::GameState, application: CopyApplication<'_>) -> bool {
    use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
    use baylee_cards_dsl::{Duration, Layer, Modifier};
    use baylee_core::ids::EffectId;

    let CopyApplication {
        source,
        target,
        mods,
        copy_index,
        until_eot,
        resolving,
        text,
    } = application;
    let (Some(previous), Some(target_base), Some(own), Some(copied)) = (
        crate::layers::copiable_values(state, source),
        crate::layers::copiable_values(state, target),
        state.printed_ability_list(source),
        state.printed_ability_list(target),
    ) else {
        return false;
    };
    let Some(object) = state.object(source) else {
        return false;
    };
    let controller = object.controller;
    let x = object.x_value;
    let mut base = (*target_base).clone();
    apply_characteristic_exceptions_with_text(&mut base, &previous, mods, text);
    let became = base.types;
    let abilities = compose(copied, &own, copy_index, mods, resolving);
    let base = Arc::new(base);
    let Some(object) = state.object_mut(source) else {
        return false;
    };
    if !until_eot {
        if object.original_base.is_none() {
            object.original_base = Some(object.base.clone());
        }
        object.base = base.clone();
    }
    object.take_abilities(abilities);
    object.own_abilities_until_eot = until_eot;

    // Only the object's former printed/copied rules disappear. Effects
    // created by their earlier resolutions keep their independent duration.
    state.effects.remove_where(|effect| {
        effect.source == Some(source)
            && effect.origin == EffectOrigin::Static
            && (own.abilities.iter().any(|ability| {
                matches!(ability,
                AbilityDef::Static(printed) if printed.modifier == effect.modifier)
            }) || own
                .runtime_statics()
                .iter()
                .any(|ability| ability.modifier == effect.modifier))
    });
    state.replacement_rules.retain(|entry| {
        entry.source != source
            || !own.abilities.iter().any(
                |ability| matches!(ability, AbilityDef::Replacement(rule) if *rule == entry.rule),
            )
    });

    if until_eot {
        let timestamp = state.next_timestamp();
        let effect = state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: Some(source),
            controller,
            origin: EffectOrigin::Resolution,
            layer: Layer::Copy,
            timestamp,
            duration: Duration::UntilEndOfTurn,
            filter: EffectFilter::object(state, source),
            modifier: Modifier::BecomeCopyOf(target),
        });
        state.copy_snapshots.push((effect, base));
    } else {
        // A later permanent copy supersedes a prior temporary copy in layer 1.
        state.effects.remove_where(|effect| {
            effect.layer == Layer::Copy
                && matches!(effect.filter, EffectFilter::ObjectIs(id, _) if id == source)
        });
    }
    for modifier in mods {
        let counters = match *modifier {
            CopyMod::AddCounter(kind, amount) => Some((kind, amount)),
            CopyMod::AddCounterIf(types, kind, amount) if became.intersects(types) => {
                Some((kind, amount))
            }
            CopyMod::AddCounterX(kind) => Some((kind, u16::try_from(x).unwrap_or(u16::MAX))),
            _ => None,
        };
        if let Some((kind, amount)) = counters
            && amount > 0
        {
            crate::replacement::put_counters(state, source, kind, amount);
        }
        // Compatibility for older DSL definitions. Current copy clauses use
        // GrantAbility so that the ability is itself copiable.
        if let CopyMod::Grant(modifier) = modifier {
            let timestamp = state.next_timestamp();
            state.effects.register(ContinuousEffect {
                id: EffectId::new(0),
                source: Some(source),
                controller,
                origin: EffectOrigin::Static,
                layer: modifier.layer(),
                timestamp,
                duration: if until_eot {
                    Duration::UntilEndOfTurn
                } else {
                    Duration::WhileSourceOnBattlefield
                },
                filter: EffectFilter::object(state, source),
                modifier: **modifier,
            });
        }
    }
    state.invalidate_projections();
    true
}

#[cfg(test)]
mod tests;
