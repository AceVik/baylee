//! Where mana comes from, and what spending it may do.

use super::{Amount, Filter, ManaColor};

/// Which colors a mana effect may produce.
///
/// The colors, the amount and the spend restriction are three independent
/// questions, and they used to be answered by seven separate `Effect`
/// variants that each fixed all three — `AddMana`, `AddManaDynamic`,
/// `AddManaChoice`, `AddManaCommanderIdentity`,
/// `AddManaRestrictedCommanderIdentity`, `AddManaRestricted` and
/// `AddManaLandColor`. Anything the printed cards combined differently had
/// no way to be said.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ManaSource {
    /// CR 305.6 abilities supplied by the source's current basic land types.
    /// This provenance is distinct from printed mana symbols.
    IntrinsicBasicLandTypes,
    /// One color among matching objects in the selected zone.
    ColorsAmong {
        /// Which objects contribute their current colors.
        filter: &'static Filter,
        /// The zone to read, relative to the ability's controller.
        zone: super::ZoneSel,
    },
    /// One named color, or colorless.
    Fixed(ManaColor),
    /// A choice among the listed colors, made on resolution.
    Choice(&'static [ManaColor]),
    /// A color in your commander's color identity (Command Tower).
    CommanderIdentity,
    /// A color some land could produce: yours (Reflecting Pool) or an
    /// opponent's (Exotic Orchard).
    LandColor {
        /// Which lands contribute their possible mana types.
        filter: &'static Filter,
        /// `true` = your lands, `false` = opponents' lands.
        mine: bool,
        /// `true` for "any **type**" (Reflecting Pool), which colorless mana
        /// is (CR 106.1b); `false` for "any **color**" (Exotic Orchard,
        /// Fellwar Stone), which colorless is not (CR 106.1a) — so a Wastes
        /// across the table puts nothing on an Orchard's menu.
        any_type: bool,
    },
    /// Colors of cards still exiled by this acquisition of the source's linked ability.
    ExiledWithSource,
    /// The colors circled on this particular acquired ability at deck construction.
    CircledColors,
    /// The color chosen as this permanent entered (Uncharted Haven).
    ///
    /// Read off the *source* object rather than off the card, which is why
    /// `resolve::colors_of` takes the object at all: two Thriving Moors on
    /// one battlefield are two different colours, and a card cannot say
    /// which. A permanent with no chosen colour produces nothing, the same
    /// answer [`Self::LandColor`] gives when it finds no land.
    Chosen,
    /// `Add {W} or one mana of the chosen color.` — the chosen colour, or
    /// one of the colours printed beside it (the Thriving cycle).
    ///
    /// Its own variant rather than a flag on [`Self::Chosen`] for the reason
    /// `EnterModifier::ChooseColorExcept` is one: "Add one mana of the
    /// chosen color" names no alternative, and a card never restates a
    /// default.
    ChosenOr(&'static [ManaColor]),
}

/// How an instruction distributes its amount among the available colors.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ManaDistribution {
    /// Choose one color for the entire amount.
    OneColor,
    /// Choose independently for each mana, allowing repeated colors.
    AnyCombination,
    /// Choose once per mana, without repeating a color.
    DifferentColors,
}

/// Whether a restriction names the spell being cast or an activated ability's source.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ManaUse {
    /// Only casting matching spells.
    Spells,
    /// Only activating abilities of matching sources, in any zone.
    Abilities,
    /// Either matching spells or abilities of matching sources.
    SpellsOrAbilities,
    /// Any nonspell payment, or casting a spell matching the filter.
    /// Powerstone mana may pay abilities, ward, echo and other nonspell costs.
    NonSpellPaymentsOrMatchingSpells,
    /// Any cost imposed by a matching spell, ability source, or other identified source.
    CostsFromMatchingSource,
    /// Only paying cumulative upkeep costs (CR 702.24a).
    CumulativeUpkeep,
}

/// What produced mana may be spent on (Cavern of Souls), or what spending
/// it on something does (Path of Ancestry), CR 106.6.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ManaRestriction {
    /// Which kind of payment the filter admits.
    pub uses: ManaUse,
    /// The spell or activated source that must match for restricted payments.
    /// A rider alone fires only when the admitted payment matches.
    pub filter: &'static Filter,
    /// Optional separate filter for an activated ability's source.
    /// `None` uses `filter` for every admitted payment kind.
    pub ability_filter: Option<&'static Filter>,
    /// What happens when it is spent on a matching spell.
    pub rider: SpendRider,
    /// "Spend this mana only …". False for a rider alone ("When that mana
    /// is spent to cast …"): that mana is ordinary mana and pays for
    /// anything, and only a spell `filter` matches sets the rider off
    /// (#232).
    pub restricts: bool,
    /// This mana survives step and phase boundaries until end of turn.
    pub until_end_of_turn: bool,
}

/// What happens when mana is spent on a spell its filter matches.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SpendRider {
    /// Nothing extra (restriction only).
    None,
    /// The spell can't be countered (Cavern of Souls).
    Uncounterable,
    /// The producing ability's controller scries N (Path of Ancestry).
    Scry(u8),
    /// The permanent enters with additional +1/+1 counters (CR 106.6a).
    EnterCounters(u32),
    /// Extra entry counters equal to this commander's command-zone casts.
    CommanderEnterCounters,
    /// Scry equal to this commander's command-zone casts when its owner casts it.
    CommanderScry,
    /// The creature spell gains haste indefinitely (CR 400.7a, 611.2a).
    Haste,
    /// The creature spell gains haste until end of turn.
    HasteUntilEndOfTurn,
    /// Copy the spell or nonmana activated ability this unit pays for.
    /// A separate delayed trigger belongs to each unit (CR 106.6a).
    Copy,
}

/// How a card's mana value is compared with a [`ManaValueBound`]'s amount.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ManaValueCmp {
    /// "With mana value X or less."
    AtMost,
    /// "With mana value equal to …."
    Exactly,
}

/// "A creature card with mana value equal to 1 plus the sacrificed
/// creature's mana value": a bound whose number the resolution computes.
///
/// A `Filter` cannot carry it — a filter is asked with no resolution in
/// hand, and the sacrificed creature is written on the stack object that is
/// resolving, not on the source a filter can see.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ManaValueBound {
    /// How the card's mana value is compared.
    pub cmp: ManaValueCmp,
    /// What it is compared with.
    pub amount: Amount,
}
