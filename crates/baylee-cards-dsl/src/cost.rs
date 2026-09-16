//! Ability costs: mana plus non-mana parts (see `docs/cost-model.md`).

use crate::filter::Filter;
use baylee_core::mana::ManaCost;

/// A non-mana cost part (tap, sacrifice, life, …).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CostPart {
    /// Tap the source (`{T}`).
    TapSelf,
    /// Untap the source (`{Q}`).
    UntapSelf,
    /// Sacrifice the source.
    SacrificeSelf,
    /// Sacrifice a permanent matching the filter.
    Sacrifice(&'static Filter),
    /// Pay life.
    PayLife(u16),
    /// Discard a card matching the filter (choice at payment).
    Discard(&'static Filter),
    /// Tap an untapped permanent matching the filter (choice at payment).
    ///
    /// The source is not it: `{T}` is [`CostPart::TapSelf`], and most of the
    /// cards that print this print both — "{T}, Tap an untapped creature you
    /// control: Add one mana of any color". Earthcraft is the exception that
    /// makes it worth two variants rather than a flag on one: it taps a
    /// creature and never itself.
    ///
    /// The filter carries both printed halves, what kind and whose; what the
    /// *rule* supplies is the word "untapped", because CR 118.3 says a
    /// permanent that is already tapped cannot be tapped to pay a cost
    /// whether or not the card bothered to say so.
    ///
    /// **One permanent, named by a filter**, which is where this stops.
    /// Counted by hand over the `//! Oracle:` headers on 2026-09-16,
    /// thirteen cards in the pool print "tap an untapped …": nine are this
    /// shape; Grove of the Guardian taps *two* and Secluded Starforge taps
    /// *X*, which want a count this variant does not carry; and Command
    /// Bridge and Public Thoroughfare print it inside "sacrifice this unless
    /// you tap …", which is a replacement on a trigger and not an activation
    /// cost at all.
    TapOther(&'static Filter),
    /// Discard the source card itself (cycling).
    DiscardSelf,
    /// Exile the source.
    ExileSelf,
    /// Return the source to its owner's hand (Recurring Nightmare).
    ReturnSelfToHand,
    /// Exile a card from your hand matching the filter (pitch costs).
    ExileFromHand(&'static Filter),
    /// Pay life equal to the spell's X value (Toxic Deluge).
    PayLifeX,
    /// Remove N counters of a kind from the source.
    ///
    /// **The counters come off the source and nowhere else**, which is not
    /// a simplification but what the pool prints. Counted over every
    /// `//! Oracle:` header on 2026-09-16, 32 activated abilities in this
    /// pool name a counter in their cost; 31 of them take it off the
    /// permanent whose ability it is, and the odd one out is Tayam,
    /// Luminous Enigma — "Remove three counters from among creatures you
    /// control" — which is a question to the player and belongs with the
    /// [`CostPart::Sacrifice`] family rather than here.
    ///
    /// So this asks nobody anything: `can_afford` is
    /// `counters.get(kind) >= n` and the payment is arithmetic. That is
    /// the whole reason it is its own variant — writing it as a chooser
    /// with one legal answer would put a prompt in front of a player for a
    /// decision the card never offered.
    ///
    /// It is a **cost**, so the counters leave without a replacement
    /// effect getting a word in — the doubling rules are about counters
    /// being *placed* (CR 614.16), and there is no rule that multiplies a
    /// removal. The counters a permanent arrives with are the opposite
    /// case and take the opposite door: [`crate::EnterModifier::WithCounters`].
    RemoveCounterSelf {
        /// Which counter.
        kind: crate::effect::CounterKind,
        /// How many.
        n: u16,
    },
}

/// A conditional cost reduction printed on a card (Surgical Metamorph).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CostReduction {
    /// Costs {N} less if you weren't the starting player.
    NotStartingPlayer(u32),
}

/// When an alternative cost may be used.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AltCondition {
    /// Always (Force of Will, evoke).
    Always,
    /// Only when it is not your turn (Force of Negation).
    NotYourTurn,
    /// Only while you control your commander (Fierce Guardianship).
    CommanderControlled,
}

/// An alternative way to pay a spell's cost (CR 601.2b).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AlternativeCost {
    /// What you pay instead of the mana cost.
    pub cost: Cost,
    /// When it may be used.
    pub condition: AltCondition,
}

/// A complete cost: a mana part plus non-mana parts.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Cost {
    /// The mana part (`ManaCost::ZERO` for none).
    pub mana: baylee_core::mana::ManaCost,
    /// The non-mana parts.
    pub parts: &'static [CostPart],
}

impl Cost {
    /// Free.
    pub const FREE: Cost = Cost {
        mana: ManaCost::ZERO,
        parts: &[],
    };

    /// Tap-only (`{T}: …`).
    pub const TAP: Cost = Cost {
        mana: ManaCost::ZERO,
        parts: &[CostPart::TapSelf],
    };
}
