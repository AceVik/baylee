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
    /// Affected lands are every basic land type (Great Divide Guide).
    AllBasicLandTypes,
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
    /// The legend rule doesn't apply to the effect's controller (Sakashima).
    LegendRuleOff,
    /// Activated abilities of artifacts the effect's opponents control
    /// can't be activated (Karn).
    CantActivateArtifacts,
    /// The effect's opponents can cast spells only as though they were
    /// sorceries (Teferi).
    OpponentsCastAsSorcery,
    /// Players can't lose the game this turn (Everybody Lives!).
    PlayersCantLose,
    /// The controller can't lose life this turn (Everybody Lives!).
    CantLoseLife,
    /// Prevent all damage that would be dealt TO the affected object
    /// (Maze of Ith).
    PreventDamageToIt,
    /// Prevent all damage that would be dealt BY the affected object
    /// (Maze of Ith).
    PreventDamageFromIt,
    /// The effect's opponents can't search libraries (Ashiok, Dream
    /// Render).
    OpponentsCantSearch,
    /// The controller has no maximum hand size (Reliquary Tower).
    NoMaxHandSize,
    /// Protection from sources matching the filter: can't be damaged,
    /// targeted, or blocked by them (CR 702.16).
    ProtectionFrom(&'static crate::Filter),
    /// The affected object becomes a copy of the given object (layer 1
    /// copiable values; Cursed Mirror's until-EOT copy).
    BecomeCopyOf(baylee_core::ids::ObjectId),
    /// The affected card (in a graveyard) may be cast for its mana cost;
    /// exile it afterwards (flashback grant, Snapcaster Mage).
    GrantsFlashback,
    /// The controller can't be targeted by spells or abilities (player
    /// hexproof, Everybody Lives!).
    PlayerHexproof,
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

/// Replacement rules and trigger modification (CR 614; Doubling Season,
/// Panharmonicon, Elesh Norn, Roaming Throne).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ReplacementRule {
    /// "If an effect would create tokens under your control, it creates
    /// twice that many" (Doubling Season). Filter applies to the affected
    /// controller.
    DoubleTokenCreation {
        /// Which controllers' token creations are doubled.
        controller_filter: &'static Filter,
    },
    /// "If an effect would put counters on a permanent you control, it
    /// puts twice that many" (Doubling Season). Filter applies to the
    /// object receiving counters.
    DoubleCounterPlacement {
        /// Which objects' counter placements are doubled.
        object_filter: &'static Filter,
    },
    /// "…causes a triggered ability of a permanent you control to trigger,
    /// that ability triggers an additional time" (Panharmonicon).
    TriggerMultiplier {
        /// Which trigger sources are multiplied (usually permanents you
        /// control or of a type).
        source_filter: &'static Filter,
        /// Which event kind is multiplied.
        event: crate::ability::TriggerEventKind,
    },
    /// "Permanents entering the battlefield don't cause abilities of
    /// permanents your opponents control to trigger" (Elesh Norn).
    TriggerSuppress {
        /// Which trigger sources are suppressed.
        source_filter: &'static Filter,
        /// Which event kind is suppressed.
        event: crate::ability::TriggerEventKind,
    },
}
