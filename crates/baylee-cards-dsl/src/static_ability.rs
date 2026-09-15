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
    /// The effect's controller controls the affected permanent (CR 613.1b,
    /// layer 2): Mind Control, Act of Treason, Sower of Temptation.
    ///
    /// A *continuous* control change, so it ends with the effect — which is
    /// the whole difference between "gain control until end of turn" and
    /// the one-shot `Effect::ChangeController` that never gives it back.
    /// Use it with `Layer::Control`; any other layer would apply it out of
    /// order with respect to the effects that read the controller.
    GainControl,
    /// The controller can't be targeted by spells or abilities (player
    /// hexproof, Everybody Lives!).
    PlayerHexproof,
    /// The controller may cast sorcery spells as though they had flash
    /// (Teferi, Time Raveler +1).
    SorceriesHaveFlash,
    /// Players may spend mana as though it were mana of any color
    /// (Mycosynth Lattice).
    ManaIsAnyColor,
    /// While an opponent searches their library, the effect's controller
    /// makes the search choices and the found cards go to exile playable
    /// by them (Opposition Agent).
    SearchTakeover,
    /// The affected object gains types while it has at least N counters
    /// of a kind (station's "artifact creature at 8+").
    AddTypeIfCountersAtLeast {
        /// Counter kind.
        kind: crate::effect::CounterKind,
        /// Threshold.
        at_least: u8,
        /// Types granted.
        types: baylee_core::types::TypeSet,
    },
    /// The affected object gains keywords while it has at least N
    /// counters of a kind (station's "8+ | Flying").
    AddKeywordIfCountersAtLeast {
        /// Counter kind.
        kind: crate::effect::CounterKind,
        /// Threshold.
        at_least: u8,
        /// Keywords granted.
        keywords: crate::KeywordSet,
    },
    /// The affected object gains an activated ability (Urza's Saga
    /// chapters, Chromatic Lantern-style grants).
    GrantActivated {
        /// Ability cost.
        cost: crate::cost::Cost,
        /// Ability effects.
        effects: &'static [crate::effect::Effect],
        /// Whether it's a mana ability.
        mana_ability: bool,
    },
    /// The affected object gains a triggered ability (class levels).
    GrantTriggered {
        /// The trigger condition.
        trigger: crate::ability::Trigger,
        /// The ability's effects.
        effects: &'static [crate::effect::Effect],
        /// Target requirement of the ability.
        target: Option<crate::effect::TargetSpec>,
    },
    /// The affected object gets +P/+T for each filter-matching permanent
    /// its controller controls (Construct tokens, "for each artifact").
    ModifyPTPerCount {
        /// What to count.
        filter: &'static crate::Filter,
        /// Power per match.
        p: i16,
        /// Toughness per match.
        t: i16,
    },
    /// Modifies power/toughness (anthems, pumps).
    ModifyPT(i16, i16),
    /// Sets power/toughness to specific values.
    SetPT(i16, i16),
    /// Switches power and toughness.
    SwitchPT,
}

impl Modifier {
    /// The layer this modifier applies in (CR 613.1).
    ///
    /// A layer is not a decision a card makes. "All permanents are
    /// artifacts" is layer 4 because it changes types, and no printing of
    /// Mycosynth Lattice could make it anything else — so the layer is a
    /// function of the modifier, and every ability that restated it was a
    /// chance to write the wrong one. Measured over the whole compiled pool
    /// before this existed: 108 `layer`/`modifier` pairings, 25 distinct
    /// modifiers, and **not one** modifier on two different layers. The
    /// table below is that measurement, which is why
    /// [`static_ability!`](crate::static_ability) can take two arguments
    /// and why `every_layer_in_the_pool_is_the_one_its_modifier_derives`
    /// keeps a hand-written literal from disagreeing with it.
    ///
    /// Two groups need reading rather than counting.
    ///
    /// **[`Layer::Text`] is doing duty as "no layer at all."** CR 613
    /// orders *characteristic-changing* effects; a rule-modifying one
    /// ([`Modifier::NoMaxHandSize`], [`Modifier::PlayersCantLose`],
    /// [`Modifier::ManaIsAnyColor`], …) changes no characteristic and has no
    /// place in that order. The engine agrees by construction —
    /// `layers::apply_modifier` has an explicit empty arm for every one of
    /// them — so their layer is read only into the effect table's iteration
    /// order and the snapshot hash. They are on `Text` because that is where
    /// the pool put them, and the honest fix is a variant that says "none",
    /// not a different layer.
    ///
    /// **Three arms are where that bucket and the rules disagree**, and the
    /// disagreement is preserved here deliberately so that adopting this
    /// function moves no card. [`Modifier::ProtectionFrom`] grants a keyword
    /// ability (CR 702.16, cited in `engine/eval.rs`), and
    /// [`Modifier::GrantTriggered`] grants a triggered one — both are
    /// ability-adding effects, which is layer 6, and the pool's own
    /// [`Modifier::GrantActivated`] is already there. Five abilities in the
    /// pool spell those two as `Text`. Moving them is a rules fix with a
    /// visible `pool-dump` diff and belongs in its own commit; doing it here
    /// would hide it inside a refactor that is supposed to change nothing.
    /// [`Modifier::GrantsFlashback`] is the third and is the mirror image —
    /// no card in the pool uses it, so it is written where the rules put it.
    #[must_use]
    pub const fn layer(&self) -> Layer {
        match self {
            // Layer 1: copy effects.
            Self::BecomeCopyOf(_) => Layer::Copy,
            // Layer 2: control-changing effects.
            Self::GainControl => Layer::Control,
            // Layer 4: type-changing effects.
            Self::AddType(_)
            | Self::RemoveType(_)
            | Self::AddSubtype(_)
            | Self::AllCreatureTypes
            | Self::AllBasicLandTypes
            | Self::AddTypeIfCountersAtLeast { .. } => Layer::Type,
            // Layer 5: color-changing effects.
            Self::AddColor(_) | Self::SetColor(_) => Layer::Color,
            // Layer 6: ability-adding and -removing effects.
            Self::AddKeyword(_)
            | Self::RemoveKeyword(_)
            | Self::LoseKeywords
            | Self::AddKeywordIfCountersAtLeast { .. }
            | Self::GrantActivated { .. }
            | Self::GrantsFlashback
            | Self::CantActivateArtifacts => Layer::Ability,
            // Layer 7b/7c/7e: power and toughness.
            Self::SetPT(..) => Layer::PtSet,
            Self::ModifyPT(..) | Self::ModifyPTPerCount { .. } => Layer::PtModify,
            Self::SwitchPT => Layer::PtSwitch,
            // No layer: rules-modifying effects, plus the three arms the
            // doc comment above names.
            Self::LegendRuleOff
            | Self::OpponentsCastAsSorcery
            | Self::PlayersCantLose
            | Self::CantLoseLife
            | Self::PreventDamageToIt
            | Self::PreventDamageFromIt
            | Self::OpponentsCantSearch
            | Self::NoMaxHandSize
            | Self::PlayerHexproof
            | Self::SorceriesHaveFlash
            | Self::ManaIsAnyColor
            | Self::SearchTakeover
            | Self::ProtectionFrom(_)
            | Self::GrantTriggered { .. } => Layer::Text,
        }
    }
}

/// A static ability on a card: `modifier` applies to objects matching
/// `filter` on `layer`, while the source is on the battlefield.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct StaticAbility {
    /// The layer the effect applies in.
    pub layer: Layer,
    /// Which objects are affected — including *where* they are.
    ///
    /// An effect that reaches past the battlefield (Maskwood Nexus:
    /// "creature cards you own that aren't on the battlefield") says so with
    /// a [`Filter::InZone`], and that is the only way to say it: the engine
    /// derives the cross-zone projection pass from the filter itself. There
    /// was a `cross_zone: bool` beside this field for exactly that purpose
    /// and nothing ever read it, so Mycosynth Lattice — the one card that
    /// relied on the flag instead of on its filter — was a `Filter::Any`
    /// that reached the stack and made instant spells into permanents, while
    /// the colourless half it was declared for never reached a library at
    /// all. One statement, in the one place that is read.
    pub filter: Filter,
    /// What changes.
    pub modifier: Modifier,
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
    /// Until the start of the effect controller's next turn (Elspeth's
    /// flying, Teferi's sorcery-flash).
    UntilYourNextTurn,
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
