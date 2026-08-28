//! Static (continuous) abilities and effect modifiers — the layer system.
//!
//! Static abilities on cards declare *what* changes (a [`Modifier`]), on
//! *which* layer it applies (CR 613.1), and *which* objects are affected
//! (a [`Filter`]). The engine registers matching [`crate::AbilityDef::Static`]
//! abilities into its effect table and projects characteristics through
//! them — removal when the source leaves is structural, never card code.

use crate::KeywordSet;
use crate::filter::Filter;
use baylee_core::color::ColorSet;
use baylee_core::ids::SubtypeId;
use baylee_core::types::TypeSet;

/// The characteristic layers (CR 613.1), in application order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Layer {
    /// 1: copy effects.
    Copy,
    /// 2: control-changing effects.
    Control,
    /// 3: text-changing effects.
    Text,
    /// 4: type-changing effects.
    Type,
    /// 5: color-changing effects.
    Color,
    /// 6: ability-adding/removing effects.
    Ability,
    /// 7a: power/toughness from characteristic-defining abilities.
    PtCda,
    /// 7b: effects that set power/toughness to specific values.
    PtSet,
    /// 7c: effects that modify power/toughness (anthems).
    PtModify,
    /// 7d: counters.
    PtCounters,
    /// 7e: effects that switch power/toughness.
    PtSwitch,
}

/// All layers in application order.
pub const LAYERS: [Layer; 11] = [
    Layer::Copy,
    Layer::Control,
    Layer::Text,
    Layer::Type,
    Layer::Color,
    Layer::Ability,
    Layer::PtCda,
    Layer::PtSet,
    Layer::PtModify,
    Layer::PtCounters,
    Layer::PtSwitch,
];

/// What a continuous effect changes.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Modifier {
    /// Adds types (Mycosynth Lattice: "all permanents are artifacts").
    AddType(TypeSet),
    /// Removes types.
    RemoveType(TypeSet),
    /// Adds a subtype.
    AddSubtype(SubtypeId),
    /// Affected creatures are every creature type (Maskwood Nexus).
    AllCreatureTypes,
    /// Adds colors.
    AddColor(ColorSet),
    /// Sets colors (Mycosynth Lattice: "…are colorless").
    SetColor(ColorSet),
    /// Grants keywords (Darksteel Forge: indestructible).
    AddKeyword(KeywordSet),
    /// Removes keywords.
    RemoveKeyword(KeywordSet),
    /// Removes all keyword abilities (Tishana's Tidebinder).
    LoseKeywords,
    /// Modifies power/toughness (anthems, pumps).
    ModifyPT(i16, i16),
    /// Sets power/toughness to specific values.
    SetPT(i16, i16),
    /// Switches power and toughness.
    SwitchPT,
}

/// A static ability on a card: `modifier` applies to objects matching
/// `filter` on `layer`, while the source is on the battlefield.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct StaticAbility {
    /// The layer the effect applies in.
    pub layer: Layer,
    /// Which objects are affected.
    pub filter: Filter,
    /// What changes.
    pub modifier: Modifier,
    /// Whether the effect reaches beyond the battlefield (Maskwood Nexus:
    /// "creature cards you own that aren't on the battlefield"). When any
    /// cross-zone effect is registered, the engine projects characteristics
    /// for *all* zones; without one, only battlefield + stack are projected
    /// (hot path).
    pub cross_zone: bool,
}

/// How long a created continuous effect lasts.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Duration {
    /// While the source permanent is on the battlefield.
    WhileSourceOnBattlefield,
    /// Until end of turn (cleanup).
    UntilEndOfTurn,
    /// Until end of combat.
    UntilEndOfCombat,
    /// Indefinitely (emblems, boss effects).
    Indefinitely,
}
