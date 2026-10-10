//! How much an effect does: a number, or a count read off the game.

use super::{CounterKind, Filter, ZoneSel};

/// A computed number (CR 107.1).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Amount {
    /// A fixed value.
    Fixed(u32),
    /// The value of X chosen at cast time.
    X,
    /// The negated value of X (Toxic Deluge's `-X/-X`; evaluated as a
    /// negative at use sites).
    NegX,
    /// The value of X plus the controller's commander-cast count
    /// (Commander's Insight).
    XPlusCommanderCasts,
    /// Twice X (Heliod's Intervention).
    DoubleX,
    /// Number of distinct colors among battlefield objects matching the
    /// filter (General Tazri).
    DistinctColorsAmong(&'static Filter),
    /// Number of distinct **basic land types** among battlefield objects
    /// matching the filter — the count every "domain" card spells out.
    ///
    /// Domain is an ability word and so has no rules meaning of its own
    /// (CR 207.2c): what the card actually says is "for each basic land
    /// type among lands you control", which is why this takes a filter
    /// rather than being a bare `Domain` variant with "lands you control"
    /// hidden inside the engine.
    ///
    /// It is not [`Amount::CountOf`] and not [`Amount::DistinctColorsAmong`],
    /// which is what left three cards in the pool unwritable: `CountOf`
    /// counts *objects*, so one Tundra answers 1 where the card wants 2 and
    /// two Forests answer 2 where the card wants 1; and a land is
    /// colourless, so the colour count answers 0 for any of them. The five
    /// types are CR 305.6's, and no other land type counts.
    BasicLandTypesAmong(&'static Filter),
    /// A fixed negative value (-N at use sites).
    NegXFixed(u32),
    /// The power of the first target (last known characteristics).
    TargetPower,
    /// The power of the ability's own source (Esper Sentinel's tax).
    ///
    /// Read off the *projected* characteristics, so an anthem or a counter
    /// raises it — which is the whole reason the card prints `{X}` instead
    /// of `{1}`. A source that is not a creature, or that has left the
    /// battlefield, counts as zero rather than as its printed number: an
    /// ability whose amount comes off a permanent has nothing to read when
    /// the permanent is gone.
    SourcePower,
    /// How many counters of a kind are on the ability's own source — Aether
    /// Vial's "the number of charge counters on this artifact", cumulative
    /// upkeep's "for each age counter on it" (CR 702.24a).
    ///
    /// Read off the source as it is when asked, so after the effect before
    /// it in the same list put a new counter there; a source that has left
    /// the battlefield has shed its counters (CR 122.2) and counts zero,
    /// where CR 608.2h would read the last one it had. No card in the pool
    /// loses its source in between: Aether Vial taps itself and stays, and
    /// the intervening `if` cumulative upkeep prints keeps a gone source
    /// from mattering.
    CountersOnSource(CounterKind),
    /// The mana value of the first target (Reanimate's life loss).
    TargetCmc,
    /// "That much": the amount of damage the triggering event dealt
    /// (Questing Beast's redirect). Carried from the event onto the
    /// triggered ability as it goes on the stack; 0 anywhere else.
    EventAmount,
    /// Last battlefield toughness of the event permanent, captured on its
    /// triggered ability. Negative values count as zero.
    EventLastToughness,
    /// "The sacrificed creature's mana value": the mana value, as it last
    /// existed on the battlefield (CR 608.2h), of the permanent sacrificed
    /// to pay the cost of the spell or ability that is resolving (Eldritch
    /// Evolution, Neoform, Birthing Pod).
    ///
    /// Read off the stack object, which is where the payment wrote it down:
    /// a spell's `Sacrifice` additional cost in the cast wizard, an
    /// activation's in `pay_cost`. Nothing sacrificed reads 0.
    SacrificedManaValue,
    /// "The sacrificed creature's power" (Kazuul's Fury, Life Chisel's
    /// sibling Miren): as [`Self::SacrificedManaValue`], the power it had
    /// as it last existed on the battlefield (CR 608.2h). Never negative.
    SacrificedPower,
    /// "The sacrificed creature's toughness" (Diamond Valley, Miren, the
    /// Moaning Well): likewise, its toughness. Never negative.
    SacrificedToughness,
    /// "The amount of mana spent to cast this spell" (Memory Deluge): what
    /// the cast paid in mana (CR 601.2h), read off the stack object where
    /// the payment wrote it, as [`Self::SacrificedManaValue`] is. A free
    /// cast spent none; a flashback spent its flashback cost.
    ManaSpentToCast,
    /// "The tapped creature's power": the power of the permanent a
    /// `CostPart::TapOther` tapped to pay the cost of the ability that is
    /// resolving — station's "put a number of charge counters on this
    /// permanent equal to the tapped creature's power" (CR 702.184a). Its
    /// power as the effect applies while it is still on the battlefield as
    /// the same object, and as it last existed there otherwise (CR 608.2h).
    ///
    /// The creature is not a target (station targets nothing), so hexproof
    /// does not stop it. Read off the stack object, where `pay_cost` wrote
    /// which creature it tapped; nothing tapped reads 0.
    TappedPower,
    /// "For each creature that died this turn" (Scavenging Ghoul): the
    /// creatures put into a graveyard from the battlefield this turn
    /// (CR 700.4), every player's, tokens included. A permanent counts as
    /// a creature if it was one as it left, not if the card in the graveyard
    /// is one. It is counted once, as the effect applies (CR 608.2h).
    CreaturesDiedThisTurn,
    /// Untapped lands the active player controlled as this turn began,
    /// before the untap step or phasing. This historical count is retained
    /// even if the ability's source was not on the battlefield then.
    UntappedLandsAtTurnStart,
    /// "The damage dealt to you this turn" (Simulacrum): every point of
    /// damage dealt to the ability's controller since the turn began,
    /// combat and not, from any source. Damage dealt, not life lost: a
    /// player whose life can't change is still dealt it, and life gained
    /// back since takes none of it away. Prevented damage was never dealt
    /// (CR 615.1). Read as the effect applies (CR 608.2h).
    DamageDealtToYouThisTurn,
    /// "The number of Mountains put into a graveyard this way" (Volcanic
    /// Eruption): how many of the resolving ability's targets a graveyard
    /// now holds as new objects, read after the effect that moved them. A
    /// target that was regenerated, went somewhere else instead, or was
    /// dropped as illegal (CR 608.2b) is not one. Outside a resolution, 0.
    TargetsPutIntoGraveyard,
    /// Number of objects matching a filter in a zone.
    CountOf {
        /// What to count.
        filter: &'static Filter,
        /// Where to count.
        zone: ZoneSel,
    },
    /// Another amount with a constant added to it: Muscle Burst's "3 plus
    /// the number of cards named Muscle Burst in all graveyards".
    ///
    /// A wrapper for the same reason [`Self::Negated`] is one — the offset
    /// is said once instead of doubling every counting amount there is — and
    /// it saturates rather than wraps, because a count is a count.
    ///
    /// The base is a **magnitude**: [`Self::is_negative`] reads a `Plus` as
    /// positive, because that is what the addition computes, and
    /// `amount_sign_tests::no_amount_in_the_pool_offsets_a_negative` is what
    /// keeps a negative base out rather than letting it resolve upwards.
    Plus {
        /// The amount being offset.
        base: &'static Amount,
        /// What is added to it.
        offset: u32,
    },
    /// The negation of another amount: "-1/-1 for each artifact you control"
    /// (Irradiate).
    ///
    /// [`Amount::NegX`] and [`Amount::NegXFixed`] are the two negatives the
    /// pool had before this, and they are the two whose magnitude is already
    /// a variant of its own. A *counted* quantity has no such twin — there is
    /// one `CountOf`, and a card wanting its negative had nothing to write —
    /// so this says the negation rather than doubling every amount there is.
    ///
    /// The sign is not in the evaluated number: the engine's `eval::amount`
    /// answers a magnitude, and every reader asks [`Amount::is_negative`] for
    /// the sign. Ask that function and never `matches!` on the variants: the
    /// three places that spelled the question out by hand would each have
    /// read this one as positive, which is a card that prints `-X/-X` and
    /// hands out `+X/+X`.
    Negated(&'static Amount),
    /// Subtract a constant from a nonnegative amount, with zero as the floor.
    SaturatingSub {
        /// The amount being reduced.
        base: &'static Amount,
        /// The constant subtracted.
        subtract: u32,
    },
}

impl Amount {
    /// Whether this amount counts **downwards**.
    ///
    /// The one place the question is answered, because it was four places
    /// before: `resolve::counters` spelled `matches!(a, Amount::NegX |
    /// Amount::NegXFixed(_))` in three separate closures and `baylee-ai`'s
    /// `tactics` had a fourth arm of its own. Four positive lists over an
    /// enum is four chances for the next negative amount to be read as a
    /// bonus, and nothing in a test suite reads a sign as a bug: the card
    /// resolves, the creature changes size, and only the direction is wrong.
    ///
    /// Nesting is answered by parity rather than refused, because that is
    /// what the word means. No card prints a double negative.
    #[must_use]
    pub const fn is_negative(&self) -> bool {
        match self {
            Self::NegX | Self::NegXFixed(_) => true,
            Self::Negated(inner) => !inner.is_negative(),
            _ => false,
        }
    }
}
