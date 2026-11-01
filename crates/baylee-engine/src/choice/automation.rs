//! Seat automation: priority holds, yields and standing answers.

use baylee_core::ids::{AbilityRef, ObjectId};

/// A seat's standing instruction for when it wants to be offered priority.
///
/// Every variant is *self-cancelling*: it names a condition that the game
/// reaches on its own, and the engine drops back to [`Self::Always`] the
/// moment it does. There is deliberately no "never ask me again" — a hold
/// that could outlive its reason is a hold that loses a game quietly.
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, Debug, Default, serde::Serialize, serde::Deserialize,
)]
pub enum PriorityHold {
    /// Offer priority every time (full manual control).
    #[default]
    Always,
    /// Don't offer priority while the seat's only legal action is to pass.
    ///
    /// Strictly safe: it fires only when passing is the sole thing the
    /// seat *could* have done. Mana abilities on their own do not count as
    /// something to do — the engine pays costs from the pool itself, so
    /// floating mana with nothing to spend it on is not a decision.
    ///
    /// Unlike the other variants this one does not expire; it never
    /// suppresses a decision, so there is nothing for it to expire from.
    PassWhenNothingToDo,
    /// "Let the stack resolve": don't offer priority until the stack is
    /// empty.
    ///
    /// Cancelled the moment anything is *added* to the stack — which is
    /// exactly the moment a player wants to be asked again, because
    /// somebody just responded to what they were letting through.
    UntilStackEmpty {
        /// Current response boundary, initialized from the depth when set.
        /// Lowered as the stack shrinks; anything above it is a new response.
        depth: u16,
    },
    /// Don't offer priority until a specific spell or ability is the next
    /// thing to resolve.
    ///
    /// This is "I don't care about the rest of the stack, wake me when
    /// *that* one is up". Cancelled if the object leaves the stack without
    /// ever reaching the top (it was countered, or its source left).
    UntilTopOfStack {
        /// The stack object to stop for.
        object: ObjectId,
    },
    /// Don't offer priority for the rest of this turn.
    UntilEndOfTurn {
        /// The turn the hold was set on; it expires when the number moves.
        turn: u32,
    },
}

impl PriorityHold {
    /// Whether this hold can keep a decision from being offered at all.
    ///
    /// [`Self::PassWhenNothingToDo`] cannot: it answers only where passing
    /// was the seat's sole legal action, so nothing is ever withheld. That is
    /// the same reason it is the one variant with nothing to expire from, and
    /// the reason an indicator built on this reports it as no hold — a player
    /// told "you are holding priority" by a setting that never withholds
    /// anything would learn to ignore the light.
    #[must_use]
    pub const fn suppresses(self) -> bool {
        match self {
            Self::Always | Self::PassWhenNothingToDo => false,
            Self::UntilStackEmpty { .. }
            | Self::UntilTopOfStack { .. }
            | Self::UntilEndOfTurn { .. } => true,
        }
    }
}

/// A remembered answer to a yes/no question a particular ability asks.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum StandingAnswer {
    /// Always accept.
    Yes,
    /// Always decline.
    No,
}

impl StandingAnswer {
    /// The boolean this answer stands for.
    #[must_use]
    pub const fn as_bool(self) -> bool {
        matches!(self, Self::Yes)
    }
}

/// One seat's automation settings.
///
/// Kept as a sorted `Vec` rather than a map: it holds a handful of entries,
/// it is cloned with the engine on every AI lookahead ply, and iteration
/// order is part of the determinism contract.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SeatAutomation {
    /// When to offer this seat priority.
    pub hold: PriorityHold,
    /// Remembered yes/no answers, keyed by ability and kept sorted.
    pub(super) standing: Vec<(AbilityRef, StandingAnswer)>,
    /// Abilities this seat always lets resolve, in stable handle order.
    #[serde(default)]
    pub(super) yields: Vec<AbilityRef>,
    /// A requested stack boundary outranks standing yields until manual input.
    #[serde(default)]
    pub(crate) priority_paused: bool,
}

impl SeatAutomation {
    /// Whether priority over this particular ability is passed automatically.
    #[must_use]
    pub fn yields_to(&self, ability: AbilityRef) -> bool {
        self.yields.binary_search(&ability).is_ok()
    }

    /// Enable or remove a standing yield without changing yes/no answers.
    pub fn set_yield(&mut self, ability: AbilityRef, enabled: bool) {
        match (self.yields.binary_search(&ability), enabled) {
            (Err(i), true) => self.yields.insert(i, ability),
            (Ok(i), false) => {
                self.yields.remove(i);
            }
            _ => {}
        }
    }

    /// Yielded abilities in deterministic order.
    pub fn yielded_abilities(&self) -> impl Iterator<Item = AbilityRef> + '_ {
        self.yields.iter().copied()
    }

    /// The remembered answer for an ability, if any.
    #[must_use]
    pub fn standing_answer(&self, ability: AbilityRef) -> Option<StandingAnswer> {
        self.standing
            .binary_search_by_key(&ability, |(a, _)| *a)
            .ok()
            .map(|i| self.standing[i].1)
    }

    /// Remembers (or, with `None`, forgets) an answer for an ability.
    pub fn set_standing_answer(&mut self, ability: AbilityRef, answer: Option<StandingAnswer>) {
        match (
            self.standing.binary_search_by_key(&ability, |(a, _)| *a),
            answer,
        ) {
            (Ok(i), Some(a)) => self.standing[i].1 = a,
            (Ok(i), None) => {
                self.standing.remove(i);
            }
            (Err(i), Some(a)) => self.standing.insert(i, (ability, a)),
            (Err(_), None) => {}
        }
    }

    /// Every remembered answer, in ability order.
    pub fn standing_answers(&self) -> impl Iterator<Item = (AbilityRef, StandingAnswer)> + '_ {
        self.standing.iter().copied()
    }

    /// Whether this seat automates nothing (the default).
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.hold == PriorityHold::Always
            && self.standing.is_empty()
            && self.yields.is_empty()
            && !self.priority_paused
    }
}
