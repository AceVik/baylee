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
        /// attack" from the view without re-implementing CR 506.2.
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
        /// Why (UI hint).
        reason: TargetPrompt,
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
        /// What the question is about: the card being cast or played, or the
        /// permanent whose modal trigger is picking a mode.
        ///
        /// A handle and not a name, for the reason the engine carries no card
        /// text at all — the same rule that makes an ability an
        /// [`baylee_core::ids::AbilityRef`]. Two land faces of a pathway
        /// (CR 712.12) differ in nothing a [`CastModeDesc`] carries: same
        /// kind, same empty cost, different printed name. Without this the
        /// client draws two identical buttons and the choice is blind.
        object: ObjectId,
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
    /// Cast a non-front face for its own printed cost — an MDFC's back
    /// (CR 712.11b), an adventure (CR 715), a disturb back (CR 702.146).
    ///
    /// It used to name The True Scriptures, which is none of those: a
    /// *transformed* back is reached by turning the card over and never by
    /// paying (CR 712.2), and it was on this list only because its `FaceDef`
    /// carried a cost the printing does not have. Swift Spiral, on the back
    /// of Twining Twins, is the pool's real example.
    Face(usize),
    /// Play a specific land face of an MDFC (pathways; CR 712.12).
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
    /// Surveil: choose cards for the graveyard; the rest stays on top
    /// (CR 701.25a).
    ///
    /// Not [`Self::ScryBottom`] with a different destination, because the
    /// two questions are not the same question: a card sent to the bottom
    /// is still in the library and a card sent to a graveyard is in a zone
    /// everybody can read. A player shown "which card goes to the bottom"
    /// while the answer feeds their own delirium is being asked the wrong
    /// thing.
    SurveilGraveyard,
    /// Put cards from your hand on top of your library (chosen order).
    PutBackOnTop,
    /// A wish: cards from outside the game, or face-up in your exile.
    Wish,
    /// Delve: exile cards from your graveyard, each paying for {1}
    /// (CR 702.66). Not a search and not a discard — the pile is offered so
    /// the caster can spend it.
    ///
    /// The only prompt in this enum that is part of a *cost*, which is what
    /// makes it worth telling apart: `options` is the whole graveyard, but
    /// `max` is the generic mana in the spell's total cost (CR 702.66a), and
    /// answering below it leaves a cast that cannot pay. The house AI reads
    /// this variant for exactly that reason.
    Delve,
    /// "Sacrifice a creature" in an activation cost (CR 701.21a).
    ///
    /// The second, third and fourth prompts here that are part of a *cost*
    /// rather than an effect, for the reason [`Self::Delve`] gives: a question a
    /// player is being asked in order to pay is a different question from a
    /// search, and a client that cannot tell them apart asks somebody to
    /// "choose a card" while what it means is "which one are you giving up".
    ///
    /// Not a target (CR 115.1). `Pending::ChooseCards` and not
    /// `ChooseTargets` is what says so, and it matters on the board: a
    /// creature with hexproof can be sacrificed to its own controller's
    /// outlet, and nothing "becomes the target of" an ability by being eaten
    /// by one.
    CostSacrifice,
    /// "Discard a card" in an activation cost (CR 701.9a).
    CostDiscard,
    /// "Tap an untapped creature you control" in an activation cost.
    ///
    /// The one of the three whose answer is not destroyed, which is why it
    /// needs its own word rather than sharing the sacrifice's: a player told
    /// "choose a permanent to sacrifice" over a menu of their own creatures
    /// would decline a cost that only taps one. CR 118.3 supplies the
    /// "untapped"; nothing in the rules supplies "you control", so the card
    /// prints it and `cost_wizard::options` draws the line anyway.
    CostTap,
    /// "Return a Forest you control to its owner's hand" in an activation
    /// cost (Quirion Ranger).
    ///
    /// The second whose answer survives, and the word matters for the same
    /// reason [`Self::CostTap`] earned its own: a player shown "which one
    /// are you giving up" over their own lands would decline a cost that
    /// hands the land back. What the two do not share is which permanents
    /// are on the menu — a tap wants an untapped one (CR 118.3), a return
    /// takes either, and tapping the Forest for mana *first* is the play.
    CostReturn,
    /// "Which of these do you want to leave tapped?" — the untap step's own
    /// determination (CR 502.3), on the permanents that print
    /// "you may choose not to untap".
    ///
    /// The only prompt here that is neither a cost nor an effect: it is a
    /// turn-based action asking the question its own rule gives the active
    /// player. It is phrased as what stays tapped rather than what untaps
    /// because the menu holds only the permanents with a second answer —
    /// listing everything the player controls would ask them to re-confirm
    /// the whole board every turn.
    LeaveTapped,
    /// Generic selection.
    Generic,
}

/// Why a [`Pending::ChooseTargets`] is presented (UI hint).
///
/// The convoke question is not targeting, and the only thing that ever said
/// so was the name of the variant it arrives in: a player casting a waterbend
/// spell was asked for "up to 99 targets" when what was wanted was "tap what
/// you like to help pay". Both are a bounded selection over permanents, which
/// is why they share a variant; what they *mean* is this field.
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, Debug, Default, serde::Serialize, serde::Deserialize,
)]
pub enum TargetPrompt {
    /// The targets of a spell or ability (CR 115).
    #[default]
    Targets,
    /// Convoke: tap creatures and artifacts, each paying for {1}
    /// (CR 702.51).
    Convoke,
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
    /// "A draw was offered. Accept?" (CR 104.4i). Everyone still in the
    /// game has to say yes; one no and play continues where it left off.
    DrawOffer {
        /// The player who offered.
        proposer: PlayerId,
    },
    /// "Your commander is in a graveyard or in exile — put it into the
    /// command zone instead?" (CR 903.9a).
    CommanderZone {
        /// The commander card, wherever it currently is.
        card: baylee_core::ids::ObjectId,
    },
    /// "Something is about to put your commander into your hand or library
    /// — put it into the command zone instead?" (CR 903.9b).
    ///
    /// Asked *before* the effect moves anything, because a replacement
    /// effect that asked afterwards would have already let the card land
    /// where it was not going to.
    CommanderReplace {
        /// The commander card, still where the effect found it.
        card: baylee_core::ids::ObjectId,
        /// Whether the library is the destination being replaced. The two
        /// answers are not the same question: a commander tucked into a
        /// library is gone, while one bounced to hand recasts untaxed
        /// (CR 903.8), so the prompt has to say which is happening.
        to_library: bool,
    },
    /// "You may …" inside a resolving ability ([`baylee_cards_dsl::Effect::MayDo`]).
    MayDo,
    /// Generic yes/no (optional effects).
    Generic,
}

impl YesNoPrompt {
    /// Whether a stored standing answer may answer this question for the
    /// seat.
    ///
    /// The gate is the *kind of question*, never the handle it arrives
    /// under. Every yes/no a card asks carries an [`AbilityRef`] — kicker,
    /// a shockland's two life, a tax trigger, a miracle — so a rule keyed
    /// on "it names an ability" would have let the first standing answer a
    /// player ever stored silence a decision about **cost**. A player who
    /// says "always take Ondu Cleric's life" has said nothing whatsoever
    /// about paying `{2}` for a kicker.
    ///
    /// So the list holds only the questions where saying yes costs nothing
    /// but the choice itself.
    ///
    /// [`Self::CommanderZone`] is on it because it is what
    /// `AbilityRef::COMMANDER_ZONE` was reserved for — "always put Katara
    /// back" is a preference a player is meant to be able to keep — and the
    /// `{2}` it leads to is a tax on a *later* cast rather than a cost paid
    /// here. Its sibling [`Self::CommanderReplace`] is deliberately **off**
    /// it: the right answer there depends on where the commander was going
    /// (a library is a loss, a hand is strictly better than the command
    /// zone), and a stored `Yes` cannot see the `to_library` that decides
    /// it.
    #[must_use]
    pub fn automatable(self) -> bool {
        matches!(self, Self::MayDo | Self::CommanderZone { .. })
    }
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
    /// Abilities this seat always lets resolve, in stable handle order.
    #[serde(default)]
    yields: Vec<AbilityRef>,
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

/// The ability index of a *granted* activated ability.
///
/// An ability a continuous effect handed to a permanent is not printed on
/// that permanent's card, so it has no position in the card's ability list to
/// be named by. It is offered under this index instead, and
/// [`PlayerAction::ActivateAbility`] takes it back the same way.
///
/// Named rather than written out because a client reads it: `u32::MAX` in a
/// client would look like a bug, and `index + 1` for a label is one —
/// arithmetic on a synthetic index overflows, which is exactly what happened
/// to the ability chooser.
///
/// This is slot 0; [`granted_ability`] names the rest. There used to be only
/// this one, which was not a simplification but a bug: Urza's Saga grants
/// *itself* two abilities — chapter I's `{T}: Add {C}` and chapter II's
/// `{2}, {T}: Create a Construct` — and with one slot the Construct was never
/// offered, on a card marked `Coverage::Implemented`.
pub const GRANTED_ABILITY: u32 = granted_ability(0);

/// How many granted abilities one permanent may offer at once.
///
/// A bound rather than an open range because these indices are carved out of
/// the top of the same `u32` a printed ability's position lives in: a card
/// with more abilities than this many short of `u32::MAX` is not a card.
pub const GRANTED_SLOTS: u32 = 8;

/// The index the `n`th granted ability of a permanent is offered under.
///
/// Counting *down* from `u32::MAX` so slot 0 keeps the value it always had,
/// and so the block sits where nothing else can reach: a printed ability is
/// numbered by its position in a list.
#[must_use]
pub const fn granted_ability(n: u32) -> u32 {
    u32::MAX - n
}

/// Which granted slot `index` names, if it names one.
///
/// The decoder half of [`granted_ability`], and the one place that partition
/// is written — an engine that decoded a slot differently from how the offer
/// encoded it would run a different ability than the player pressed.
#[must_use]
pub const fn granted_slot(index: u32) -> Option<u32> {
    let n = u32::MAX - index;
    if n < GRANTED_SLOTS { Some(n) } else { None }
}

/// The ability index of a prepared permanent's linked cast (Emeritus of Woe).
///
/// Same reason as [`GRANTED_ABILITY`]: what is being activated is not an
/// ability the card prints. Below the granted block, which is why it moved
/// when that block grew — these indices are per-session, chosen fresh in
/// every `LegalActions`, so nothing outside a running game holds one.
pub const PREPARED_CAST: u32 = u32::MAX - GRANTED_SLOTS;

// The indices in this module are **not** `AbilityRef` indices, and that is
// the distinction to keep before adding another one here. They name a slot in
// one `LegalActions` — chosen fresh every time it is built, held by nothing
// outside the game it was built for — while an `AbilityRef` is stored against
// an *account* and replayed into the next game. The two spaces both count
// down from `u32::MAX` and would collide if either were read as the other,
// which is why an ability the engine puts on the stack carries
// `AbilityRef::SYNTHETIC` rather than the slot it was offered under.
//
// CR 903.9a's "put your commander into the command zone?" is asked under
// `AbilityRef::COMMANDER_ZONE` for that reason: it is a question a player
// answers once and for good, so it is a wire constant and belongs in
// `baylee-core` beside the other reserved handles.

/// Everything a player may legally do with priority (precomputed).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LegalActions {
    /// Passing is always legal.
    pub can_pass: bool,
    /// Playable lands in hand.
    pub lands: Vec<ObjectId>,
    /// Castable cards in hand (timing + mana verified).
    pub castable: Vec<ObjectId>,
    /// The CR 305.6 shortcut, and **not** every mana ability on the table.
    ///
    /// A permanent is named here when it taps for mana it has by virtue of a
    /// *basic land type* (`casting::intrinsic_mana`), or when a continuous
    /// effect granted it a mana ability. Those two have no printed ability to
    /// point at, so they are answered with `ActivateManaAbility { source }`,
    /// which carries no index.
    ///
    /// A mana ability a card actually **prints** — Sol Ring's `{T}: Add
    /// {C}{C}`, a nonbasic land's own tap, Deathrite Shaman's — is an
    /// ordinary entry in [`Self::abilities`] as `(source, index)` and is
    /// pressed with `ActivateAbility`. It is a mana ability in the rules
    /// (CR 605.1) and never appears in this list.
    ///
    /// Six engine tests were written against the broader reading of this
    /// field in one batch and all six failed the same way, which is why the
    /// narrower one is spelled out here rather than left to be rediscovered.
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

    /// Whether there is any mana to make here.
    ///
    /// Read only inside a CR 605.3a payment window, where the engine has
    /// already narrowed `abilities` to mana abilities — outside one, an
    /// entry there is any activated ability at all and this would answer
    /// the wrong question. It exists so the engine can decline to open a
    /// window with nothing in it: a seat with no untapped source would
    /// otherwise be asked to make mana and have only the answer it just
    /// gave.
    #[must_use]
    pub fn has_mana_source(&self) -> bool {
        !self.mana_abilities.is_empty() || !self.abilities.is_empty()
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
        /// of that player's planeswalkers (CR 506.2).
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
    /// Offer a draw to every other player still in the game (CR 104.4i).
    OfferDraw,
    /// Change when this seat wants to be offered priority.
    ///
    /// Legal at any time, including while another seat is being asked
    /// something: it changes nothing about the game, only about who gets
    /// interrupted. It is journaled so a replay auto-passes in exactly the
    /// places the live game did.
    SetPriorityHold(PriorityHold),
    /// Automatically pass this seat's priority for one specific ability.
    ///
    /// Also legal at any time, for the same reason.
    SetAbilityYield {
        /// The individual ability whose priority windows may be skipped.
        ability: AbilityRef,
        /// Enable auto-passing; false restores manual responses.
        enabled: bool,
    },
    /// Replace both settings atomically, before the engine resumes automation.
    SetAbilityPolicy {
        /// The individual card ability.
        ability: AbilityRef,
        /// Pass priority over this ability.
        pass: bool,
        /// Optional standing answer; `None` restores asking.
        answer: Option<StandingAnswer>,
    },
    /// Change the remembered yes/no answer independently of auto-passing.
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
            Self::SetPriorityHold(_)
                | Self::SetStandingAnswer { .. }
                | Self::SetAbilityYield { .. }
                | Self::SetAbilityPolicy { .. }
        )
    }
}

#[cfg(test)]
mod choice_tests {
    use super::*;
    use baylee_core::ids::{AbilityRef, CardIndex, ObjectId};

    fn ability(n: u32) -> AbilityRef {
        AbilityRef::new(CardIndex::new(n), n)
    }

    fn object() -> ObjectId {
        ObjectId::new(1, 0)
    }

    // ---- the reserved index block -------------------------------------

    #[test]
    fn every_granted_slot_decodes_to_the_slot_it_was_encoded_from() {
        // The one property that matters about this partition: it is written
        // in two places, an encoder and a decoder, and an engine that
        // decoded a slot differently from how the offer encoded it would
        // run a different ability than the player pressed.
        for n in 0..GRANTED_SLOTS {
            assert_eq!(
                granted_slot(granted_ability(n)),
                Some(n),
                "slot {n} does not survive the round trip"
            );
        }
    }

    #[test]
    fn slot_zero_is_still_the_constant_that_was_there_before_the_block() {
        // Urza's Saga grants itself two abilities; the block grew from one
        // index to eight and slot 0 had to keep its value, or every stored
        // offer of the first granted ability would have moved.
        assert_eq!(GRANTED_ABILITY, granted_ability(0));
        assert_eq!(granted_slot(GRANTED_ABILITY), Some(0));
    }

    #[test]
    fn an_ordinary_ability_index_names_no_granted_slot() {
        // A printed ability is numbered by its position in a list, which is
        // why the reserved block counts down from the other end.
        for index in [0, 1, 2, 7, 100, 65_535] {
            assert_eq!(granted_slot(index), None, "index {index} decoded as a slot");
        }
    }

    #[test]
    fn the_block_holds_exactly_its_own_slots_and_stops() {
        // The first index below the block must not decode, or the bound is
        // decoration: `GRANTED_SLOTS` would name a size nothing enforces.
        assert_eq!(
            granted_slot(granted_ability(GRANTED_SLOTS - 1)),
            Some(GRANTED_SLOTS - 1)
        );
        assert_eq!(granted_slot(granted_ability(GRANTED_SLOTS)), None);
    }

    #[test]
    fn the_prepared_cast_sits_below_the_granted_block_and_not_inside_it() {
        // Emeritus of Woe's linked cast is not an ability the card prints
        // either, so it is carved out of the same end — and it moved when
        // the block grew. Two reserved indices that collided would offer
        // one thing and activate the other.
        assert_eq!(granted_slot(PREPARED_CAST), None);
        assert!(PREPARED_CAST < granted_ability(GRANTED_SLOTS - 1));
        for n in 0..GRANTED_SLOTS {
            assert_ne!(PREPARED_CAST, granted_ability(n));
        }
    }

    // ---- priority holds ------------------------------------------------

    #[test]
    fn a_hold_withholds_exactly_when_it_has_something_to_expire_from() {
        // Written as an exhaustive `match` rather than a list, so a new
        // variant is a compile error here instead of silently inheriting
        // one side of the answer. `PassWhenNothingToDo` is the interesting
        // one: it answers only where passing was the sole legal action, so
        // it withholds nothing and an indicator must not call it a hold.
        fn expected(hold: PriorityHold) -> bool {
            match hold {
                PriorityHold::Always | PriorityHold::PassWhenNothingToDo => false,
                PriorityHold::UntilStackEmpty { .. }
                | PriorityHold::UntilTopOfStack { .. }
                | PriorityHold::UntilEndOfTurn { .. } => true,
            }
        }
        for hold in [
            PriorityHold::Always,
            PriorityHold::PassWhenNothingToDo,
            PriorityHold::UntilStackEmpty { depth: 0 },
            PriorityHold::UntilTopOfStack { object: object() },
            PriorityHold::UntilEndOfTurn { turn: 3 },
        ] {
            assert_eq!(hold.suppresses(), expected(hold), "{hold:?}");
        }
    }

    #[test]
    fn the_default_hold_is_the_one_that_asks_every_time() {
        assert_eq!(PriorityHold::default(), PriorityHold::Always);
        assert!(!PriorityHold::default().suppresses());
    }

    // ---- standing answers ----------------------------------------------

    #[test]
    fn a_standing_answer_is_the_boolean_it_stands_for() {
        assert!(StandingAnswer::Yes.as_bool());
        assert!(!StandingAnswer::No.as_bool());
    }

    #[test]
    fn an_answer_is_found_under_its_own_ability_and_no_other() {
        let mut seat = SeatAutomation::default();
        seat.set_standing_answer(ability(2), Some(StandingAnswer::Yes));
        assert_eq!(seat.standing_answer(ability(2)), Some(StandingAnswer::Yes));
        assert_eq!(seat.standing_answer(ability(3)), None);
    }

    #[test]
    fn answering_the_same_ability_twice_replaces_rather_than_duplicates() {
        let mut seat = SeatAutomation::default();
        seat.set_standing_answer(ability(1), Some(StandingAnswer::Yes));
        seat.set_standing_answer(ability(1), Some(StandingAnswer::No));
        assert_eq!(seat.standing_answer(ability(1)), Some(StandingAnswer::No));
        assert_eq!(seat.standing_answers().count(), 1);
    }

    #[test]
    fn forgetting_removes_the_answer_and_forgetting_nothing_changes_nothing() {
        let mut seat = SeatAutomation::default();
        seat.set_standing_answer(ability(1), Some(StandingAnswer::Yes));
        seat.set_standing_answer(ability(1), None);
        assert_eq!(seat.standing_answer(ability(1)), None);
        assert_eq!(seat.standing_answers().count(), 0);

        // Forgetting one that was never remembered must not insert a hole.
        seat.set_standing_answer(ability(9), None);
        assert_eq!(seat.standing_answers().count(), 0);
        assert!(seat.is_default());
    }

    #[test]
    fn remembered_answers_come_back_in_ability_order() {
        // Iteration order is part of the determinism contract — this list
        // is cloned with the engine on every AI lookahead ply — so an
        // insertion out of order must still read back sorted.
        let mut seat = SeatAutomation::default();
        for n in [5, 1, 4, 2] {
            seat.set_standing_answer(ability(n), Some(StandingAnswer::Yes));
        }
        let order: Vec<AbilityRef> = seat.standing_answers().map(|(a, _)| a).collect();
        let mut sorted = order.clone();
        sorted.sort();
        assert_eq!(order, sorted);
        assert_eq!(order.len(), 4);
    }

    #[test]
    fn a_seat_is_default_until_it_holds_or_remembers_something() {
        assert!(SeatAutomation::default().is_default());

        let holds = SeatAutomation {
            hold: PriorityHold::UntilEndOfTurn { turn: 1 },
            ..Default::default()
        };
        assert!(!holds.is_default());

        let mut remembers = SeatAutomation::default();
        remembers.set_standing_answer(ability(1), Some(StandingAnswer::No));
        assert!(!remembers.is_default());
    }

    // ---- what a standing answer may be kept for -------------------------

    #[test]
    fn only_the_questions_whose_answer_cannot_go_stale_are_automatable() {
        // Exhaustive for the same reason as the hold test. The pair worth
        // reading is `CommanderZone` (automatable — "always put Katara
        // back", and the `{2}` is a tax on a later cast) against
        // `CommanderReplace` (not — the right answer depends on where the
        // commander was going, which a stored `Yes` cannot see).
        fn expected(prompt: YesNoPrompt) -> bool {
            match prompt {
                YesNoPrompt::MayDo | YesNoPrompt::CommanderZone { .. } => true,
                YesNoPrompt::PayLifeOrEnterTapped { .. }
                | YesNoPrompt::Kicker
                | YesNoPrompt::PayTax { .. }
                | YesNoPrompt::Miracle { .. }
                | YesNoPrompt::DrawOffer { .. }
                | YesNoPrompt::CommanderReplace { .. }
                | YesNoPrompt::Generic => false,
            }
        }
        for prompt in [
            YesNoPrompt::MayDo,
            YesNoPrompt::Generic,
            YesNoPrompt::Kicker,
            YesNoPrompt::PayLifeOrEnterTapped { amount: 1 },
            YesNoPrompt::PayTax { mana: 2 },
            YesNoPrompt::Miracle { card: object() },
            YesNoPrompt::CommanderZone { card: object() },
            YesNoPrompt::CommanderReplace {
                card: object(),
                to_library: true,
            },
            YesNoPrompt::CommanderReplace {
                card: object(),
                to_library: false,
            },
        ] {
            assert_eq!(prompt.automatable(), expected(prompt), "{prompt:?}");
        }
    }

    // ---- what a seat may do --------------------------------------------

    #[test]
    fn a_seat_offered_nothing_can_only_pass_and_has_no_mana_to_make() {
        let nothing = LegalActions::default();
        assert!(nothing.nothing_but_passing());
        assert!(!nothing.has_mana_source());
    }
}
