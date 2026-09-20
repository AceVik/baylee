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
    /// The effect's controller may play lands from their graveyard
    /// (Crucible of Worlds, Ramunap Excavator).
    ///
    /// A **permission**, and that is why it is a modifier of its own rather
    /// than a second reading of [`Self::GrantsFlashback`]: flashback grants
    /// *casting* a spell from a graveyard, and playing a land is not casting
    /// anything at all (CR 305.1 — "a player can't cast a land card"). The
    /// two sentences meet nothing in common in the engine, either: one goes
    /// through `casting::can_cast` and the stack, the other through
    /// `casting::play_land` and no stack.
    ///
    /// It says nothing about *how many*: the land drop is
    /// [`Self::ExtraLandDrops`], and a player with this and no land drop
    /// left may play nothing, from their graveyard or anywhere else
    /// (CR 305.2b).
    PlayLandsFromGraveyard,
    /// The effect's controller may play this many lands beyond the one the
    /// rules allow (Exploration: 1; Azusa: 2).
    ///
    /// CR 305.2 is written to be modified — "a player can normally play one
    /// land during their turn; however, continuous effects may increase this
    /// number" — so the number is the payload and not the variant, and two
    /// such effects on one player add up rather than one of them winning.
    ExtraLandDrops(u8),
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
    /// The affected permanent does not untap during the untap step of the
    /// effect's controller (Basalt Monolith, Grim Monolith).
    ///
    /// **A rule, not a characteristic.** CR 502.3 is the turn-based action
    /// this changes — "the active player determines which permanents they
    /// control will untap … effects can keep one or more of a player's
    /// permanents from untapping" — and CR 613.11 puts an effect that
    /// modifies a game rule outside the layer order entirely. So it lives in
    /// the `Layer::Text` bucket beside [`Self::NoMaxHandSize`] and the rest,
    /// which [`Modifier::layer`] documents as "no layer at all". Layer 6
    /// would be wrong on the rules: CR 613.1f is about *abilities*, and this
    /// grants none.
    ///
    /// **Whose untap step is the affected permanent's controller's**, and
    /// that is not the same as the effect's controller. Basalt Monolith says
    /// "during **your** untap step" about itself and Paralyze says "during
    /// **its controller's** untap step" about the creature it enchants; the
    /// card-script reference writes both as
    /// `ValidStepTurnToController$ You`, so its "you" is the affected card's
    /// controller and not the Aura's. The engine needs no condition for it
    /// at all: CR 502.3 untaps the permanents the active player controls and
    /// no others, so by the time `progress::untap_step` asks, that player is
    /// the only answer left. Reading it the other way would have worked on
    /// the two monoliths — an ability a permanent has about *itself* puts
    /// all three players on one seat — and done nothing at all on the 45
    /// Auras that are the commonest printing of this sentence.
    ///
    /// **Not the same as the other two sentences Magic prints here.** "You
    /// may choose not to untap" is a question CR 502.3 lets the active
    /// player answer, and "doesn't untap during your **next** untap step" is
    /// a created effect with a duration. Neither is this, and neither is
    /// spelled with this variant.
    DoesNotUntap,
    /// The affected permanent's controller may choose to leave it tapped
    /// during their untap step (the Fallen Empires storage lands, Ice Floe).
    ///
    /// The other half of CR 502.3, and the half that is a *question* rather
    /// than an effect: "the active player **determines** which permanents
    /// they control will untap". Without a card saying so that
    /// determination has one legal answer — all of them — so the engine
    /// never had to ask; this variant is what gives a permanent a second
    /// answer. It is a rules-modifying effect for the same reason
    /// [`Self::DoesNotUntap`] is (CR 613.11), and lives in the same
    /// `Layer::Text` bucket.
    ///
    /// **The question is asked, not the priority.** CR 502.4 says no player
    /// receives priority during the untap step, which forbids casting and
    /// activating there — it does not forbid the turn-based action from
    /// taking the answer CR 502.3 asks its own player for. The engine
    /// therefore suspends inside the untap step with a
    /// [`crate::choice::Pending::ChooseCards`] and never grants priority.
    ///
    /// **It is not the opposite of [`Self::DoesNotUntap`] and the two
    /// compose.** A permanent kept from untapping by an effect has nothing
    /// to decide, so it is not on the menu at all: an offer whose every
    /// answer does the same thing is the offer/apply contradiction this
    /// engine treats as its worst kind.
    MayChooseNotToUntap,
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
    /// **[`Modifier::ProtectionFrom`] and [`Modifier::GrantTriggered`] used
    /// to sit in that bucket, and it was a rules bug rather than a
    /// shorthand.** CR 613.1f is "Layer 6: Ability-adding effects, keyword
    /// counters, ability-removing effects, and effects that say an object
    /// can't have an ability are applied", and CR 702.16a opens "Protection
    /// is a static ability" — so an effect granting protection adds an
    /// ability, exactly like the [`Modifier::GrantActivated`] that was
    /// already on layer 6 beside it. Granting a *trigger* is the same claim
    /// with a different kind of ability. `Text` was never a neutral parking
    /// space for them either: it is layer **3**, which CR 613.1c gives to
    /// text-changing effects (CR 612), and neither of these changes any
    /// text.
    ///
    /// Five abilities in the pool moved with the fix, and it moved them
    /// alone: every card reaches the layer through this function or through
    /// [`static_ability!`](crate::static_ability), so not one card file had
    /// to be edited. Nothing about the *game* changed, which is the part
    /// worth being exact about — `layers::apply_modifier` has an empty arm
    /// for both, protection is read by `eval::protected_from` and a granted
    /// trigger by `trigger.rs`, and neither of those looks at a layer at
    /// all. What the layer reaches is the projection's iteration bucket, the
    /// CR 613.8 dependency sort inside it, and `state::snapshot_hash`.
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
            | Self::CantActivateArtifacts
            // CR 613.1f is the whole argument for these two: "Layer 6:
            // Ability-adding effects, keyword counters, ability-removing
            // effects, and effects that say an object can't have an ability
            // are applied." Protection is an ability — CR 702.16a opens
            // "Protection is a static ability" — so granting it is an
            // ability-adding effect, and so is granting a trigger.
            | Self::ProtectionFrom(_)
            | Self::GrantTriggered { .. } => Layer::Ability,
            // Layer 7b/7c/7e: power and toughness.
            Self::SetPT(..) => Layer::PtSet,
            Self::ModifyPT(..) | Self::ModifyPTPerCount { .. } => Layer::PtModify,
            Self::SwitchPT => Layer::PtSwitch,
            // No layer: rules-modifying effects.
            Self::LegendRuleOff
            | Self::PlayLandsFromGraveyard
            | Self::ExtraLandDrops(_)
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
            | Self::DoesNotUntap
            | Self::MayChooseNotToUntap => Layer::Text,
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
    /// Through the effect controller's next untap step and no further —
    /// "…doesn't untap during your next untap step".
    ///
    /// The third sentence CR 502.3 neighbours, and the one
    /// [`Modifier::DoesNotUntap`] names in its own docs as *not* being a
    /// static ability: ten lands in this pool print it on an activated mana
    /// ability, where the suppression is created when the land is tapped and
    /// is gone one untap step later.
    ///
    /// **It is the first duration here that ends at a step rather than at a
    /// turn boundary**, which is the whole of what makes it a new variant
    /// and not a spelling of [`Self::UntilYourNextTurn`]. Those two are one
    /// step apart in the turn structure and a whole turn apart in play: a
    /// land held until the start of its controller's next turn is a land
    /// that untaps in the untap step it was supposed to miss, because
    /// CR 500.1 puts the turn's beginning *before* its untap step. The
    /// off-by-one is invisible in the card's text and total in its effect,
    /// so both directions are pinned by a test
    /// (`baylee_engine::engine::untap_tests`).
    ///
    /// It ends **after** the step it suppresses, not as that step begins:
    /// an effect expiring on the way in would let the land untap on schedule
    /// and leave a card that compiles, claims `Implemented` and does nothing
    /// at all.
    UntilYourNextUntapStep,
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
