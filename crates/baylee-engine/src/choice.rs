//! The choice contract: the only way the game ever advances.
//!
//! The engine never asks open questions — every [`ChoiceRequest`] carries
//! the complete set of legal answers, precomputed. Humans, AIs, and network
//! clients all answer through the same [`PlayerAction`] type.
//!
//! # Automation
//!
//! A seat can tell the engine *not* to ask it about things (see
//! [`Automation`]). This lives in the engine rather than in a client for
//! two reasons. It has to see the same board the rules do — "stop when
//! that trigger reaches the top of the stack" is a question about the
//! stack, not about a UI. And it has to be journaled: an auto-pass that a
//! client invented would replay as a different game, whereas a
//! [`PlayerAction::SetPriorityHold`] in the journal replays exactly.

use crate::win::GameResult;
use baylee_core::ids::{AbilityRef, ObjectId, PlayerId};

/// One creature that may block, and the attackers it may be assigned to.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BlockOption {
    /// The creature that may block.
    pub blocker: ObjectId,
    /// The attackers this creature may legally block.
    pub attackers: Vec<ObjectId>,
}

/// What the game is currently waiting for.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Pending {
    /// A mulligan decision (London; first is free per house rules).
    Mulligan {
        /// Deciding player.
        player: PlayerId,
        /// Mulligans already taken by this player.
        taken: u8,
        /// Whether the next mulligan is free (house rule, first only).
        next_is_free: bool,
    },
    /// A player must choose cards to put on the bottom after keeping.
    MulliganBottom {
        /// Deciding player.
        player: PlayerId,
        /// How many cards to bottom.
        count: u8,
    },
    /// A player has priority.
    Priority {
        /// The player holding priority.
        player: PlayerId,
        /// Everything they may legally do right now.
        legal: Box<LegalActions>,
    },
    /// Declare attackers (combat).
    ChooseAttackers {
        /// Attacking player.
        player: PlayerId,
        /// Which creatures may attack (CR 508.1a): untapped, unsick, no
        /// defender, and past whatever the card itself demands.
        ///
        /// Enumerated here for the same reason as the defenders below: a
        /// client that filtered its own board by "untapped and not sick"
        /// would offer Wall of Omens as an attacker and would miss every
        /// "can't attack unless…" a card prints.
        attackers: Vec<ObjectId>,
        /// What may be attacked: each surviving opponent, plus every
        /// planeswalker they control. Carried in the request because a
        /// client cannot derive "which permanents are planeswalkers I may
        /// attack" from the view without re-implementing CR 508.1a.
        defenders: Vec<baylee_core::ids::Defender>,
    },
    /// Declare blockers (combat).
    ChooseBlockers {
        /// Defending player.
        player: PlayerId,
        /// Attacking player.
        attacker: PlayerId,
        /// Which creatures may block which attackers. Evasion is a pairing
        /// question — flying, menace, protection, "can't be blocked by" —
        /// so the offer is a pairing, not two flat lists.
        blockers: Vec<BlockOption>,
    },
    /// Discard down to maximum hand size (cleanup).
    DiscardChoice {
        /// Discarding player.
        player: PlayerId,
        /// How many cards to discard.
        count: u8,
    },
    /// Legend rule: choose which copy to keep (CR 704.5j).
    LegendChoice {
        /// Choosing player.
        player: PlayerId,
        /// The duplicated permanents (choose exactly one to keep).
        options: Vec<ObjectId>,
    },
    /// Choose cards from a server-side filtered set (searches, browses).
    ChooseCards {
        /// Choosing player.
        player: PlayerId,
        /// The legal options (already filtered — e.g. only Islands/Swamps).
        options: Vec<ObjectId>,
        /// Minimum to choose.
        min: u8,
        /// Maximum to choose.
        max: u8,
        /// Why (UI hint).
        prompt: ChoicePrompt,
    },
    /// Choose targets for a spell/ability being cast/activated.
    ChooseTargets {
        /// Choosing player.
        player: PlayerId,
        /// The legal object targets.
        options: Vec<ObjectId>,
        /// The legal *player* targets, for "any target" (CR 115.4).
        ///
        /// Empty for every spec that targets objects alone, which is why
        /// [`PlayerAction::ChooseObjects`] still answers this prompt: the
        /// two lists are one choice, and `min`/`max` count across both.
        player_options: Vec<PlayerId>,
        /// Minimum to choose (0 for "up to" targets).
        min: u8,
        /// Maximum to choose.
        max: u8,
    },
    /// Choose a creature type ("the chosen type" as this enters).
    ChooseSubtype {
        /// Choosing player.
        player: PlayerId,
        /// All creature types (ids 0..=349).
        options: Vec<baylee_core::ids::SubtypeId>,
    },
    /// Choose a mana color (choice-restricted mana abilities).
    ChooseColor {
        /// Choosing player.
        player: PlayerId,
        /// Allowed colors.
        options: Vec<baylee_core::mana::ManaColor>,
    },
    /// A yes/no decision (shockland life payment, optional effects).
    YesNo {
        /// Deciding player.
        player: PlayerId,
        /// What is being decided.
        prompt: YesNoPrompt,
        /// The ability asking, when one can be named.
        ///
        /// This is the key a standing answer is stored under, and the
        /// label a client puts on the prompt. `None` for questions that
        /// come from the game itself rather than from a card (a draw
        /// offer, a mulligan-adjacent choice).
        source: Option<AbilityRef>,
    },
    /// Choose how to cast a spell (normal / alternative cost / mode).
    ChooseCastMode {
        /// Casting player.
        player: PlayerId,
        /// The legal cast options.
        options: Vec<CastModeDesc>,
    },
    /// Choose the value of X for a spell.
    ChooseNumber {
        /// Choosing player.
        player: PlayerId,
        /// Minimum value.
        min: u32,
        /// Maximum value.
        max: u32,
    },
    /// Choose a target player.
    ChoosePlayer {
        /// Choosing player.
        player: PlayerId,
        /// Candidate players.
        options: Vec<PlayerId>,
    },
    /// Order objects (top-of-library reorder, trigger ordering later).
    OrderObjects {
        /// Choosing player.
        player: PlayerId,
        /// Objects to order (index 0 = topmost after the choice).
        objects: Vec<ObjectId>,
    },
    /// The game is over.
    GameOver(GameResult),
}

/// One legal way to cast a spell (CR 601.2b).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CastModeDesc {
    /// Option index (answered via `PlayerAction::ChooseMode`).
    pub index: u8,
    /// Which kind of cast this is.
    pub kind: CastModeKind,
    /// The mana part to pay with this option.
    pub cost: baylee_core::mana::ManaCost,
}

/// The kind of a cast option.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum CastModeKind {
    /// Printed cost.
    Normal,
    /// An alternative cost (pitch, evoke, …).
    Alternative(usize),
    /// A spell mode (overload and friends).
    Mode(usize),
    /// Cast a non-front face of an MDFC (The True Scriptures; CR 712.4).
    Face(usize),
    /// Play a specific land face of an MDFC (pathways; CR 712.4a).
    PlayLandFace(usize),
    /// Miracle cast (CR 702.94).
    Miracle,
}

/// Why a [`Pending::ChooseCards`] is presented (UI hint).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum ChoicePrompt {
    /// Library search (tutor/fetch).
    SearchLibrary,
    /// Scry: choose cards for the bottom; the rest stays on top.
    ScryBottom,
    /// Put cards from your hand on top of your library (chosen order).
    PutBackOnTop,
    /// A wish: cards from outside the game, or face-up in your exile.
    Wish,
    /// Generic selection.
    Generic,
}

/// What a [`Pending::YesNo`] asks.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum YesNoPrompt {
    /// "You may pay N life; if you don't, this enters tapped" (shocklands).
    PayLifeOrEnterTapped {
        /// Life to pay.
        amount: u16,
    },
    /// Kicker/additional cost yes-or-no at cast time.
    Kicker,
    /// "Pay {N}?" for a tax trigger (Rhystic Study & co.).
    PayTax {
        /// Generic mana to pay.
        mana: u16,
    },
    /// "Reveal and cast for its miracle cost?" (CR 702.94).
    Miracle {
        /// The drawn card.
        card: baylee_core::ids::ObjectId,
    },
    /// "A draw was offered. Accept?" (CR 104.4a). Everyone still in the
    /// game has to say yes; one no and play continues where it left off.
    DrawOffer {
        /// The player who offered.
        proposer: PlayerId,
    },
    /// Generic yes/no (optional effects).
    Generic,
}

// ------------------------------------------------------------ automation

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
        /// Stack depth when the hold was set; anything above it is new.
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
    standing: Vec<(AbilityRef, StandingAnswer)>,
}

impl SeatAutomation {
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
        self.hold == PriorityHold::Always && self.standing.is_empty()
    }
}

/// Everything a player may legally do with priority (precomputed).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LegalActions {
    /// Passing is always legal.
    pub can_pass: bool,
    /// Playable lands in hand.
    pub lands: Vec<ObjectId>,
    /// Castable cards in hand (timing + mana verified).
    pub castable: Vec<ObjectId>,
    /// Mana abilities activatable on the battlefield.
    pub mana_abilities: Vec<ObjectId>,
    /// Activated abilities available on controlled permanents:
    /// `(source, ability_index)`.
    pub abilities: Vec<(ObjectId, u32)>,
    /// Cards suspendable from hand.
    pub suspendable: Vec<ObjectId>,
}

impl LegalActions {
    /// Whether passing is the only thing this seat could do.
    ///
    /// Mana abilities are excluded on purpose: the engine pays from the
    /// pool itself, so a seat that can only make mana it cannot spend has
    /// no decision to make.
    #[must_use]
    pub fn nothing_but_passing(&self) -> bool {
        self.lands.is_empty()
            && self.castable.is_empty()
            && self.abilities.is_empty()
            && self.suspendable.is_empty()
    }
}

/// A player's answer to a [`Pending`] request.
///
/// Comparable so that callers can assert on an answer without formatting it:
/// clients build actions from user input and test that they built the right
/// one, and a host can deduplicate a resent action after a reconnect.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum PlayerAction {
    /// Keep the current hand (mulligan).
    MulliganKeep,
    /// Take a mulligan (redraw, then bottom later).
    MulliganTake,
    /// Pass priority.
    PassPriority,
    /// Play a land from hand.
    PlayLand {
        /// The land card in hand.
        card: ObjectId,
    },
    /// Cast a spell from hand (S2: no modes/targets/X; auto-payment).
    CastSpell {
        /// The card in hand.
        card: ObjectId,
    },
    /// Activate a mana ability (tap a mana source).
    ActivateManaAbility {
        /// The mana source permanent.
        source: ObjectId,
    },
    /// Activate an ability of a permanent.
    ActivateAbility {
        /// The source permanent.
        source: ObjectId,
        /// Index into the card's abilities.
        ability_index: u32,
    },
    /// Declare attackers with what each one attacks.
    DeclareAttackers {
        /// `(attacker, defender)` pairs. The defender is a player or one
        /// of that player's planeswalkers (CR 508.1a).
        attackers: Vec<(ObjectId, baylee_core::ids::Defender)>,
    },
    /// Declare blockers.
    DeclareBlockers {
        /// `(blocker, blocked attacker)` pairs.
        blockers: Vec<(ObjectId, ObjectId)>,
    },
    /// Choose objects (mulligan bottoming, discards, legend rule).
    ChooseObjects {
        /// The chosen objects.
        objects: Vec<ObjectId>,
    },
    /// Choose targets where players and objects are one set ("any target",
    /// CR 115.4).
    ///
    /// [`PlayerAction::ChooseObjects`] answers the same prompt when nothing
    /// but objects is being chosen — which is every prompt a spell that
    /// says "target creature" raises. This variant exists because a player
    /// has no [`ObjectId`] to be named by, not because targeting split in
    /// two.
    ChooseTargets {
        /// The chosen object targets.
        objects: Vec<ObjectId>,
        /// The chosen player targets.
        players: Vec<PlayerId>,
    },
    /// Suspend a card from hand with time counters.
    Suspend {
        /// The card to suspend.
        card: ObjectId,
    },
    /// Order objects (index 0 = topmost).
    OrderObjects {
        /// The ordered objects.
        objects: Vec<ObjectId>,
    },
    /// Choose a mana color.
    ChooseColor(baylee_core::mana::ManaColor),
    /// Choose a creature type (Roaming Throne & co.).
    ChooseSubtype(baylee_core::ids::SubtypeId),
    /// Choose a cast option (index into `ChooseCastMode::options`).
    ChooseMode(usize),
    /// Choose a number (X values).
    ChooseNumber(u32),
    /// Choose a target player.
    ChoosePlayer(PlayerId),
    /// Answer a yes/no decision.
    YesNo(bool),
    /// Concede the game.
    Concede,
    /// Offer a draw to every other player still in the game (CR 104.4a).
    OfferDraw,
    /// Change when this seat wants to be offered priority.
    ///
    /// Legal at any time, including while another seat is being asked
    /// something: it changes nothing about the game, only about who gets
    /// interrupted. It is journaled so a replay auto-passes in exactly the
    /// places the live game did.
    SetPriorityHold(PriorityHold),
    /// Remember (or, with `None`, forget) an answer for an ability's
    /// yes/no question — "always gain the life, stop asking".
    ///
    /// Also legal at any time, for the same reason.
    SetStandingAnswer {
        /// Which ability's question.
        ability: AbilityRef,
        /// The answer to give from now on; `None` clears it.
        answer: Option<StandingAnswer>,
    },
}

impl PlayerAction {
    /// Whether the action only changes a seat's automation settings.
    ///
    /// These never touch the game: the engine applies them and re-offers
    /// whatever it was already asking, so they neither reset the priority
    /// round nor count as taking an action.
    #[must_use]
    pub const fn is_automation_setting(&self) -> bool {
        matches!(
            self,
            Self::SetPriorityHold(_) | Self::SetStandingAnswer { .. }
        )
    }
}
