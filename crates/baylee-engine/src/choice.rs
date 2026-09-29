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

mod arrange;
pub use arrange::{
    ArrangePile, ArrangePlace, ArrangePrompt, arrangement_fault, default_arrangement,
};

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
    /// Choose a card name ("as this enters, choose a card name" — Pithing
    /// Needle), answered with [`PlayerAction::ChooseCardName`].
    ///
    /// No list rides with the question. Any card's name may be chosen, of
    /// any of its faces (CR 201.4, 201.4b–f), and the pool is the whole of
    /// what a game can mean by one (a token's name only counts when a card
    /// has it too), so the options are the card pool itself: a few thousand
    /// faces every client and agent already has, which a list here would
    /// send again with every frame that asks. The engine checks the answer
    /// against the pool it plays with.
    ChooseCardName {
        /// Choosing player.
        player: PlayerId,
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
    /// Choose a number: the value of X, how many times to pay a replicate
    /// cost, or a share of a division. `reason` says which.
    ChooseNumber {
        /// Choosing player.
        player: PlayerId,
        /// Minimum value.
        min: u32,
        /// Maximum value.
        max: u32,
        /// What the number is (UI hint). A frame without it is asking for X.
        #[serde(default)]
        reason: NumberPrompt,
    },
    /// Choose a target player.
    ChoosePlayer {
        /// Choosing player.
        player: PlayerId,
        /// Candidate players.
        options: Vec<PlayerId>,
    },
    /// Put cards into places, each place in an order: "the rest on the
    /// bottom in any order", "put them back in any order". Answered with
    /// [`PlayerAction::Arrange`]: every card exactly once, pile by pile,
    /// library piles listed top to bottom.
    Arrange {
        /// Choosing player.
        player: PlayerId,
        /// The cards being arranged, in the order they were looked at (top
        /// of the library first).
        cards: Vec<ObjectId>,
        /// Where they may go; answered pile by pile, in this order.
        piles: Vec<ArrangePile>,
        /// Why the question is asked (UI hint).
        prompt: ArrangePrompt,
    },
    /// Choose one of the piles another player separated (Fact or Fiction):
    /// the chosen pile goes into the hand and the others into the
    /// graveyard. Answered with [`PlayerAction::ChooseMode`], the pile's
    /// position; a pile may be empty and is still a pile to choose.
    ChoosePile {
        /// Choosing player.
        player: PlayerId,
        /// The piles, in the order the separation gave them.
        piles: Vec<Vec<ObjectId>>,
    },
    /// The game is over.
    GameOver(GameResult),
}
impl Pending {
    /// The seat this question is asked of; `None` once the game is over.
    ///
    /// During the opening mulligans several seats are asked at once, and
    /// this names only the one [`crate::engine::Engine::pending`] shows; ask
    /// [`crate::engine::Engine::awaited`] for all of them.
    #[must_use]
    pub const fn asked(&self) -> Option<PlayerId> {
        match self {
            Self::Mulligan { player, .. }
            | Self::MulliganBottom { player, .. }
            | Self::Priority { player, .. }
            | Self::ChooseAttackers { player, .. }
            | Self::ChooseBlockers { player, .. }
            | Self::DiscardChoice { player, .. }
            | Self::LegendChoice { player, .. }
            | Self::ChooseCards { player, .. }
            | Self::ChooseTargets { player, .. }
            | Self::ChooseSubtype { player, .. }
            | Self::ChooseCardName { player }
            | Self::ChooseColor { player, .. }
            | Self::YesNo { player, .. }
            | Self::ChooseCastMode { player, .. }
            | Self::ChooseNumber { player, .. }
            | Self::ChoosePlayer { player, .. }
            | Self::ChoosePile { player, .. }
            | Self::Arrange { player, .. } => Some(*player),
            Self::GameOver(_) => None,
        }
    }
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
    /// Printed cost plus kicker, chosen before its replacement targets.
    Kicked,
    /// An alternative cost (pitch, evoke, …).
    Alternative(usize),
    /// A spell mode (overload and friends).
    Mode(usize),
    /// Several modes of a spell that chooses more than one (CR 700.2a):
    /// bit `i` is mode `i`. Announced as one set, and carried out in the
    /// order the modes are printed (CR 608.2c). The option's cost is the
    /// spell's plus every chosen mode's own (spree, CR 702.172a).
    Modes(u8),
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
    /// Cast with prototype characteristics (CR 718).
    Prototype,
    /// Cast face down as a 2/2 creature with ward {2}.
    Disguise,
    /// Miracle cast (CR 702.94).
    Miracle,
    /// Cast from the graveyard for the card's printed flashback cost
    /// (CR 702.34a). A *granted* flashback is paid with the mana cost and is
    /// offered as [`Self::Normal`].
    Flashback,
    /// Cast for the card's dash cost rather than its mana cost
    /// (CR 702.109a): the permanent it becomes has haste and returns to its
    /// owner's hand at the beginning of the next end step.
    Dash,
    /// Cast from the owner's graveyard for the card's escape cost
    /// (CR 702.138a): its mana, and as many other cards from that graveyard
    /// exiled, asked as the cast's cost (`ChoicePrompt::CostExile`).
    Escape,
}

/// Why a [`Pending::ChooseCards`] is presented (UI hint).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum ChoicePrompt {
    /// Library search (tutor/fetch).
    SearchLibrary,
    /// Put cards from your hand on top of your library (chosen order).
    PutBackOnTop,
    /// A wish: cards from outside the game, or face-up in your exile.
    Wish,
    /// Delve: exile cards from your graveyard, each paying for {1}
    /// (CR 702.66). Not a search and not a discard — the pile is offered so
    /// the caster can spend it.
    ///
    /// The first of the six prompts in this enum that are part of a *cost*,
    /// and the only one that asks for a heap rather than one card, which is
    /// what makes it worth telling apart: `options` is the whole graveyard, but
    /// `max` is the generic mana in the spell's total cost (CR 702.66a), and
    /// answering below it leaves a cast that cannot pay. The house AI reads
    /// this variant for exactly that reason.
    Delve,
    /// "Sacrifice a creature" in an activation cost (CR 701.21a).
    ///
    /// The second of the prompts here that are part of a *cost* rather than
    /// an effect, for the reason [`Self::Delve`] gives: a question a
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
    /// The first cost prompt whose answer is not destroyed, which is why it
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
    /// "Exile a creature card from your graveyard" in an activation cost
    /// (Moorland Haunt, Mines of Moria), and escape's "exile five other
    /// cards from your graveyard" (CR 702.138a), asked once with `min` and
    /// `max` both the count.
    ///
    /// Not [`Self::CostDiscard`], though both answers end up somewhere a
    /// card is not played from: the noun has to say which pile the card
    /// leaves, and a player told "card to discard" over their graveyard
    /// would look for it in their hand. And not [`Self::Delve`], which
    /// exiles from the same pile: delve's `max` is the generic mana in a
    /// spell's cost and the house AI answers it with `max`, where this asks
    /// for exactly one card per part.
    CostExile,
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
    /// Revealed cards of one card type, one of which may be put into the
    /// hand (Atraxa, Grand Unifier: "for each card type, you may put a card
    /// of that type … into your hand"). Asked once per type, and the type
    /// is the question: the menu holds only that type's cards. Taking one is
    /// never worse than leaving it, since the rest go to the bottom, so the
    /// house AI takes the best.
    OneOfType {
        /// The card type asked about.
        card_type: baylee_core::types::TypeSet,
    },
    /// "You may reveal a matching card from your hand; if you don't, this
    /// land enters tapped."
    ///
    /// A prompt of its own rather than [`Self::Generic`] because declining
    /// is never right: nothing is spent, the card stays in hand (CR 701.20b)
    /// and the only thing given up is the information. The house AI keys on
    /// exactly that — its `ChooseCards` rule answers `min` for a prompt it
    /// does not recognise, so a generic one would have every AI reveal land
    /// enter tapped for the rest of the game.
    RevealOrEnterTapped,
    /// Looked-at cards, one or more of which go into the hand (Expressive
    /// Iteration's first question, a dig's keep). The answer is what the
    /// player keeps, so the house AI takes the best of them.
    PutIntoHand,
    /// Looked-at cards, the chosen one of which goes on the bottom of the
    /// library (Expressive Iteration's second question): the answer is what
    /// the player gives up.
    PutOnBottom,
    /// Exiled cards, the chosen one of which the player may play this turn
    /// (Dauthi Voidwalker).
    PlayFromExile,
    /// A looked-at card the player may put onto the battlefield, and which
    /// goes elsewhere if they name nothing (Risen Reef's land). The answer
    /// is what the player puts down, so the house AI puts it down.
    PutOntoBattlefield,
    /// Cards in a graveyard, one of which may be put where the effect says;
    /// naming none searches the library instead (Finale of Devastation's
    /// "search your library and/or graveyard"). The house AI takes the best.
    FromGraveyard,
    /// Cards in the hand, the chosen ones of which are discarded — not as a
    /// price but as the effect itself ("you may discard up to two cards; if
    /// you do, draw that many"). The house AI lets its least valuable card
    /// go.
    Discard,
    /// Another player's found cards, the chosen ones of which go to their
    /// owner's graveyard and the rest to that player's hand (Realms
    /// Uncharted's opponent). The chooser is denying the other player, so
    /// the house AI sends the best of them away.
    PutIntoGraveyard,
    /// Revealed cards to separate into two piles (Fact or Fiction's
    /// opponent): the chosen ones are the first pile and the rest the
    /// second, and either may be empty. The other player then takes one
    /// pile into their hand, so the house AI puts the best card alone.
    FirstPile,
    /// Creatures to tap to crew a Vehicle (CR 702.122a): any number of
    /// them, with total power `power` or greater. The engine refuses an
    /// answer whose total is short.
    CostCrew {
        /// The N of "Crew N".
        power: u8,
    },
    /// Generic selection.
    Generic,
}

/// What a [`Pending::ChooseNumber`] counts (UI hint).
///
/// Three questions share the variant because each is a bounded number the
/// player announces as the spell or ability is put on the stack (CR 601.2b,
/// 601.2d), and the answer is [`PlayerAction::ChooseNumber`] every time;
/// what they *mean* is this field, for the reason [`TargetPrompt`] gives
/// about convoke. Without it a
/// player casting Lose Focus with mana to spare was asked to "choose a
/// number (0–2)" and nothing said what for.
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, Debug, Default, serde::Serialize, serde::Deserialize,
)]
pub enum NumberPrompt {
    /// The value of X (CR 107.3): a spell's printed `{X}`, an activation's,
    /// or the X of a counter cost.
    #[default]
    X,
    /// How many times to pay a spell's replicate cost (CR 702.56a). The
    /// maximum is the most the caster's floating mana pays for, and the
    /// spell is copied once for each.
    Replicate {
        /// The cost paid each time.
        cost: baylee_core::mana::ManaCost,
    },
    /// "Damage divided as you choose" (CR 601.2d): how much of what is left
    /// goes to one target, asked target by target in the order they were
    /// chosen. The last target takes the rest and is not asked, and each
    /// target is given at least 1, which is what `min` and `max` say.
    DivideDamage {
        /// The target this share goes to.
        target: ObjectId,
        /// Its place among the targets, from 0.
        index: u8,
        /// How many targets share the damage.
        of: u8,
        /// The damage not yet given to a target.
        left: u32,
    },
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
    /// Tap permanents to help pay, each paying for {1}: creatures for
    /// convoke (CR 702.51a), artifacts and creatures for a paid waterbend
    /// (CR 701.67a). `options` says which.
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
    /// Make mana for a pact's debt. Declining loses the game.
    PayPact {
        /// The full, possibly colored cost.
        cost: baylee_core::mana::ManaCost,
    },
    /// Pay life for a resolving ability (including ward).
    PayLife {
        /// Life to pay.
        amount: u16,
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
    /// "You may cast that card without paying its mana cost" — cascade's
    /// offer (CR 702.85a), about the card it just exiled.
    CastWithoutPaying {
        /// The exiled card.
        card: baylee_core::ids::ObjectId,
    },
    /// "You may cast that card", paying its costs, as the ability resolves
    /// (Conduit of Worlds, CR 608.2g). A yes opens a payment window for its
    /// mana cost.
    CastPaying {
        /// The card to cast.
        card: baylee_core::ids::ObjectId,
    },
    /// "You may …" inside a resolving ability ([`baylee_cards_dsl::Effect::MayDo`]).
    MayDo,
    /// "Its owner puts it on their choice of the top or bottom of their
    /// library" ([`baylee_cards_dsl::Effect::OwnerPutsOnTopOrBottom`]),
    /// asked of the owner: yes is the top, no the bottom.
    TopOfLibrary {
        /// The card that is going.
        card: baylee_core::ids::ObjectId,
    },
    /// "You may cast that card without paying its mana cost. If you don't
    /// cast it, put that card into your hand." (discover, CR 701.57a),
    /// asked of the player who discovered it: yes casts it, no puts it
    /// into their hand. Asked only of a card that can be cast.
    Discover {
        /// The discovered card, in exile.
        card: baylee_core::ids::ObjectId,
    },
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

    /// Whether answering "no" does nothing, which is what a seat that ran
    /// out of time is answered with ([`timeout_answer`]).
    ///
    /// Everything a card offers is declined by not doing it: the life not
    /// paid, the kicker not paid, the tax not paid, the miracle not cast,
    /// the "you may" not done, and a draw offer not accepted. The two
    /// commander questions are not: declining leaves the commander in a
    /// graveyard, in exile, or tucked into a library, which is a loss and
    /// not a pause. The house answers those. Neither is
    /// [`Self::TopOfLibrary`]: both answers move the card, and "no" is the
    /// bottom rather than nothing.
    #[must_use]
    pub const fn declining_does_nothing(self) -> bool {
        match self {
            Self::PayLifeOrEnterTapped { .. }
            | Self::Kicker
            | Self::PayTax { .. }
            | Self::PayLife { .. }
            | Self::Miracle { .. }
            | Self::CastWithoutPaying { .. }
            | Self::CastPaying { .. }
            | Self::DrawOffer { .. }
            | Self::MayDo
            | Self::Generic => true,
            // Declining discover moves the card into the hand: a card for
            // the player, but not nothing, and a free spell is not a
            // question to answer by the clock.
            Self::CommanderZone { .. }
            | Self::CommanderReplace { .. }
            | Self::PayPact { .. }
            | Self::TopOfLibrary { .. }
            | Self::Discover { .. } => false,
        }
    }
}

impl Pending {
    /// Brings a counted choice within what its options can give, and says
    /// whether the question then has an answer at all.
    ///
    /// A choice of cards asks for no more cards than it offers: an effect
    /// that attempts the impossible does only as much as possible (CR
    /// 609.3), a player can't choose what is impossible (CR 608.2d), and a
    /// search that is short finds as many as it can (CR 701.23d). The piles
    /// of an arrangement are held to the cards left the same way. That is
    /// the whole of what fitting does; `Engine::apply` runs it over every
    /// question it hands out, so a builder that forgets the bound asks a
    /// question that still has an answer.
    ///
    /// Every other question has an answer by its shape or has none, and
    /// fitting cannot give it one. A target choice with fewer options than
    /// it must name is a spell or ability that cannot be put on the stack
    /// (CR 601.2c, CR 603.3d), which its builder refuses; shrinking its `min`
    /// here would cast a spell with fewer targets than it requires. A menu
    /// with nothing on it has no answer at all.
    ///
    /// An exhaustive match on purpose, for [`timeout_answer`]'s reason: a new
    /// question is classified here before it compiles.
    pub fn fit_to_options(&mut self) -> bool {
        match self {
            Self::ChooseCards {
                options, min, max, ..
            } => {
                let n = u8::try_from(options.len()).unwrap_or(u8::MAX);
                *max = (*max).min(n);
                *min = (*min).min(*max);
                true
            }
            Self::Arrange { cards, piles, .. } => {
                // Every card still goes somewhere, and no pile asks for more
                // than is left once the piles before it have their least.
                let total = u32::try_from(cards.len()).unwrap_or(u32::MAX);
                let mut left = total;
                let mut room = 0_u32;
                for pile in piles.iter_mut() {
                    pile.min = pile.min.min(left);
                    pile.max = pile.max.min(total);
                    left -= pile.min;
                    room = room.saturating_add(pile.max);
                }
                room >= total
            }
            Self::ChooseTargets {
                options,
                player_options,
                min,
                max,
                ..
            } => usize::from(*min) <= options.len() + player_options.len() && min <= max,
            Self::ChooseNumber { min, max, .. } => min <= max,
            Self::LegendChoice { options, .. } => !options.is_empty(),
            Self::ChooseSubtype { options, .. } => !options.is_empty(),
            Self::ChooseColor { options, .. } => !options.is_empty(),
            Self::ChooseCastMode { options, .. } => !options.is_empty(),
            Self::ChoosePlayer { options, .. } => !options.is_empty(),
            // One of the piles, and a pile may be empty.
            Self::ChoosePile { piles, .. } => !piles.is_empty(),
            // Answered by passing, keeping, declining or declaring nothing,
            // by a count of cards the engine checks against the hand, or by
            // any card name the pool has.
            Self::Mulligan { .. }
            | Self::ChooseCardName { .. }
            | Self::MulliganBottom { .. }
            | Self::Priority { .. }
            | Self::ChooseAttackers { .. }
            | Self::ChooseBlockers { .. }
            | Self::DiscardChoice { .. }
            | Self::YesNo { .. }
            | Self::GameOver(_) => true,
        }
    }
}

/// The answer that does nothing, where the question has one (#258).
///
/// What the decision clock answers for a seat that did not answer in time
/// (`baylee_gamehost::Session::answer_by_clock`), and the button a client
/// writes that clock's seconds into. One function for both ends, so a
/// countdown can never sit on a button the clock will not press.
///
/// Pass priority, attack with nothing, block with nothing, keep the hand,
/// decline what declining leaves alone ([`YesNoPrompt::declining_does_nothing`]).
/// `None` for every question with no answer that does nothing: a discard,
/// a target, a search, an ordering, a colour. The house plays those.
///
/// An exhaustive match on purpose, with no wildcard arm. A new question has
/// to be classified here before it compiles, and a wildcard would file it
/// under "the house decides" without anyone having decided that.
///
/// The answer is what the question offers, not what the rules will accept:
/// a creature that attacks each combat if able (CR 508.1d) or a lure that
/// must be blocked (CR 509.1c) can make the empty declaration illegal, and
/// the clock then falls back to the house.
#[must_use]
pub fn timeout_answer(pending: &Pending) -> Option<PlayerAction> {
    match pending {
        Pending::Priority { .. } => Some(PlayerAction::PassPriority),
        Pending::Mulligan { .. } => Some(PlayerAction::MulliganKeep),
        Pending::ChooseAttackers { .. } => Some(PlayerAction::DeclareAttackers {
            attackers: Vec::new(),
        }),
        Pending::ChooseBlockers { .. } => Some(PlayerAction::DeclareBlockers {
            blockers: Vec::new(),
        }),
        Pending::YesNo { prompt, .. } => prompt
            .declining_does_nothing()
            .then_some(PlayerAction::YesNo(false)),
        Pending::MulliganBottom { .. }
        | Pending::DiscardChoice { .. }
        | Pending::LegendChoice { .. }
        | Pending::ChooseCards { .. }
        | Pending::ChooseTargets { .. }
        | Pending::ChooseSubtype { .. }
        | Pending::ChooseCardName { .. }
        | Pending::ChooseColor { .. }
        | Pending::ChooseCastMode { .. }
        | Pending::ChooseNumber { .. }
        | Pending::ChoosePlayer { .. }
        | Pending::ChoosePile { .. }
        | Pending::Arrange { .. }
        | Pending::GameOver(_) => None,
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
/// A face-up special action, carried by the existing permanent action menu.
pub const TURN_FACE_UP: u32 = PREPARED_CAST - 1;

/// How many unlock slots there are: one per half of a Room.
pub const UNLOCK_SLOTS: u32 = 2;

/// The index unlocking `half` of a Room (0 the left, 1 the right) is
/// offered under: CR 709.5e's special action (CR 116.2m), carried by the
/// permanent's action menu as turning one face up is.
#[must_use]
pub const fn unlock_door(half: u8) -> u32 {
    // `u32::from` is not const.
    #[allow(clippy::cast_lossless)]
    let half = (half & 1) as u32;
    TURN_FACE_UP - 1 - half
}

/// Which half `index` unlocks, if it names an unlock: the decoder half of
/// [`unlock_door`], and the one place that partition is read.
#[must_use]
pub const fn door_to_unlock(index: u32) -> Option<u8> {
    let n = (TURN_FACE_UP - 1).wrapping_sub(index);
    // Below `UNLOCK_SLOTS`, so it fits.
    #[allow(clippy::cast_possible_truncation)]
    if n < UNLOCK_SLOTS {
        Some(n as u8)
    } else {
        None
    }
}

/// Whether `index` is a special action (CR 116.2): turning a permanent face
/// up, or unlocking a door. Neither is an activated ability, so what stops
/// activating one stops neither (Pithing Needle, CR 602.5; split second,
/// CR 702.61a).
#[must_use]
pub const fn is_special_action(index: u32) -> bool {
    index == TURN_FACE_UP || door_to_unlock(index).is_some()
}

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
    /// Use these targets for a bounded, currently waiting series of the
    /// same triggered ability. Each choice is validated again; this never
    /// establishes a standing policy for future triggers.
    ChooseTargetBatch {
        /// Object targets for each occurrence.
        objects: Vec<ObjectId>,
        /// Player targets for each occurrence.
        players: Vec<PlayerId>,
        /// Maximum number of occurrences the player confirmed, including
        /// the current one. New arrivals cannot increase this number.
        count: u32,
    },
    /// Suspend a card from hand with time counters.
    Suspend {
        /// The card to suspend.
        card: ObjectId,
    },
    /// Answer a [`Pending::Arrange`]: one list per pile, in the pile order
    /// the question gave, library piles listed top to bottom.
    Arrange {
        /// The cards in each pile.
        piles: Vec<Vec<ObjectId>>,
    },
    /// Choose a mana color.
    ChooseColor(baylee_core::mana::ManaColor),
    /// Choose a creature type (Roaming Throne & co.).
    ChooseSubtype(baylee_core::ids::SubtypeId),
    /// Choose a card name: face `face` of the card `card` (Pithing Needle,
    /// CR 201.4). Refused for a card the pool does not have and for a face
    /// it does not print.
    ChooseCardName {
        /// The card whose name it is.
        card: baylee_core::ids::CardIndex,
        /// Which of its faces, since each face's name may be chosen
        /// (CR 201.4b, 201.4d, 201.4f).
        face: u8,
    },
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
            Self::SetPriorityHold(_) | Self::SetAbilityYield { .. } | Self::SetAbilityPolicy { .. }
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

    // ---- what the clock answers (#258) -----------------------------------

    /// One question of every kind, each with the answer a seat that ran out
    /// of time gets. `timeout_answer` has no wildcard arm, so a new kind of
    /// question stops the build there; this table is the other half, and
    /// says what each kind was decided to be.
    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one row per kind of question, and the table is the test"
    )]
    fn a_timed_out_seat_does_nothing_wherever_the_question_allows_it() {
        let p = PlayerId::new(0);
        let pass = Some(PlayerAction::PassPriority);
        let no = Some(PlayerAction::YesNo(false));
        let yes_no = |prompt| Pending::YesNo {
            player: p,
            prompt,
            source: None,
        };
        let cases: Vec<(Pending, Option<PlayerAction>)> = vec![
            (
                Pending::Priority {
                    player: p,
                    legal: Box::default(),
                },
                pass,
            ),
            (
                Pending::Mulligan {
                    player: p,
                    taken: 1,
                    next_is_free: false,
                },
                Some(PlayerAction::MulliganKeep),
            ),
            (
                Pending::ChooseAttackers {
                    player: p,
                    attackers: vec![object()],
                    defenders: vec![baylee_core::ids::Defender::Player(PlayerId::new(1))],
                },
                Some(PlayerAction::DeclareAttackers {
                    attackers: Vec::new(),
                }),
            ),
            (
                Pending::ChooseBlockers {
                    player: p,
                    attacker: PlayerId::new(1),
                    blockers: vec![BlockOption {
                        blocker: object(),
                        attackers: vec![ObjectId::new(2, 0)],
                    }],
                },
                Some(PlayerAction::DeclareBlockers {
                    blockers: Vec::new(),
                }),
            ),
            (
                yes_no(YesNoPrompt::PayLifeOrEnterTapped { amount: 2 }),
                no.clone(),
            ),
            (yes_no(YesNoPrompt::Kicker), no.clone()),
            (yes_no(YesNoPrompt::PayTax { mana: 1 }), no.clone()),
            (yes_no(YesNoPrompt::PayLife { amount: 7 }), no.clone()),
            (yes_no(YesNoPrompt::Miracle { card: object() }), no.clone()),
            (
                yes_no(YesNoPrompt::CastWithoutPaying { card: object() }),
                no.clone(),
            ),
            (
                yes_no(YesNoPrompt::CastPaying { card: object() }),
                no.clone(),
            ),
            (yes_no(YesNoPrompt::DrawOffer { proposer: p }), no.clone()),
            (yes_no(YesNoPrompt::MayDo), no.clone()),
            (yes_no(YesNoPrompt::Generic), no),
            (yes_no(YesNoPrompt::CommanderZone { card: object() }), None),
            // Declining a pact's payment loses the game, so nobody's clock
            // declines it for them.
            (
                yes_no(YesNoPrompt::PayPact {
                    cost: baylee_core::mana::ManaCost::from_symbol_generic(2),
                }),
                None,
            ),
            (
                yes_no(YesNoPrompt::CommanderReplace {
                    card: object(),
                    to_library: false,
                }),
                None,
            ),
            (
                yes_no(YesNoPrompt::PayPact {
                    cost: baylee_core::mana::ManaCost::ZERO,
                }),
                None,
            ),
            // Both answers move the card; the house picks the end.
            (yes_no(YesNoPrompt::TopOfLibrary { card: object() }), None),
            // No still moves the card, into the hand; the house casts it.
            (yes_no(YesNoPrompt::Discover { card: object() }), None),
            (
                Pending::MulliganBottom {
                    player: p,
                    count: 1,
                },
                None,
            ),
            (
                Pending::DiscardChoice {
                    player: p,
                    count: 1,
                },
                None,
            ),
            (
                Pending::LegendChoice {
                    player: p,
                    options: vec![object()],
                },
                None,
            ),
            (
                Pending::ChooseCards {
                    player: p,
                    options: vec![object()],
                    min: 0,
                    max: 1,
                    prompt: ChoicePrompt::SearchLibrary,
                },
                None,
            ),
            (
                Pending::ChooseTargets {
                    player: p,
                    options: vec![object()],
                    player_options: vec![],
                    min: 0,
                    max: 1,
                    reason: TargetPrompt::Targets,
                },
                None,
            ),
            (
                Pending::ChooseSubtype {
                    player: p,
                    options: vec![],
                },
                None,
            ),
            (
                Pending::ChooseColor {
                    player: p,
                    options: vec![],
                },
                None,
            ),
            (Pending::ChooseCardName { player: p }, None),
            (
                Pending::ChooseCastMode {
                    player: p,
                    object: object(),
                    options: vec![],
                },
                None,
            ),
            (
                Pending::ChooseNumber {
                    player: p,
                    min: 0,
                    max: 3,
                    reason: NumberPrompt::X,
                },
                None,
            ),
            (
                Pending::ChoosePlayer {
                    player: p,
                    options: vec![p],
                },
                None,
            ),
            (
                Pending::Arrange {
                    player: p,
                    cards: vec![object()],
                    piles: vec![],
                    prompt: ArrangePrompt::Order,
                },
                None,
            ),
            (
                Pending::ChoosePile {
                    player: p,
                    piles: vec![vec![object()], vec![]],
                },
                None,
            ),
            (
                Pending::GameOver(GameResult {
                    winner: None,
                    reason: crate::win::EndReason::Draw,
                }),
                None,
            ),
        ];
        for (pending, expected) in &cases {
            assert_eq!(
                timeout_answer(pending).as_ref(),
                expected.as_ref(),
                "{pending:?}"
            );
        }
        // Every kind is in the table. The yes/no is one kind per prompt,
        // because its prompts are answered differently.
        let covered: std::collections::BTreeSet<usize> =
            cases.iter().map(|(pending, _)| kind_of(pending)).collect();
        assert_eq!(
            covered,
            (0..KINDS).collect(),
            "a kind of question has no row in this table"
        );
    }

    /// How many kinds [`kind_of`] tells apart.
    const KINDS: usize = 18 + 15;

    /// Which kind of question this is, numbered without gaps. No wildcard
    /// arm: a new `Pending` variant or yes/no prompt does not compile here
    /// until it has a number, and then the table above is missing it.
    fn kind_of(pending: &Pending) -> usize {
        match pending {
            Pending::Mulligan { .. } => 0,
            Pending::MulliganBottom { .. } => 1,
            Pending::Priority { .. } => 2,
            Pending::ChooseAttackers { .. } => 3,
            Pending::ChooseBlockers { .. } => 4,
            Pending::DiscardChoice { .. } => 5,
            Pending::LegendChoice { .. } => 6,
            Pending::ChooseCards { .. } => 7,
            Pending::ChooseTargets { .. } => 8,
            Pending::ChooseSubtype { .. } => 9,
            Pending::ChooseColor { .. } => 10,
            Pending::ChooseCastMode { .. } => 11,
            Pending::ChooseNumber { .. } => 12,
            Pending::ChoosePlayer { .. } => 13,
            Pending::Arrange { .. } => 14,
            Pending::GameOver(_) => 15,
            Pending::ChooseCardName { .. } => 16,
            Pending::YesNo { prompt, .. } => {
                17 + match prompt {
                    YesNoPrompt::PayLifeOrEnterTapped { .. } => 0,
                    YesNoPrompt::Kicker => 1,
                    YesNoPrompt::PayTax { .. } => 2,
                    YesNoPrompt::PayLife { .. } => 9,
                    YesNoPrompt::PayPact { .. } => 10,
                    YesNoPrompt::Miracle { .. } => 3,
                    YesNoPrompt::DrawOffer { .. } => 4,
                    YesNoPrompt::CommanderZone { .. } => 5,
                    YesNoPrompt::CommanderReplace { .. } => 6,
                    YesNoPrompt::MayDo => 7,
                    YesNoPrompt::Generic => 8,
                    YesNoPrompt::TopOfLibrary { .. } => 11,
                    YesNoPrompt::Discover { .. } => 12,
                    YesNoPrompt::CastWithoutPaying { .. } => 13,
                    YesNoPrompt::CastPaying { .. } => 14,
                }
            }
            Pending::ChoosePile { .. } => 17 + 15,
        }
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

    #[test]
    fn the_unlock_slots_sit_below_turning_face_up_and_decode_to_their_halves() {
        // A Room's two doors take the next two indices down. Each decodes
        // to its own half, none of them is another reserved index, and no
        // other reserved index or printed position decodes as a door: one
        // that did would unlock a door when a player pressed something else.
        assert_eq!(door_to_unlock(unlock_door(0)), Some(0));
        assert_eq!(door_to_unlock(unlock_door(1)), Some(1));
        assert_eq!(unlock_door(0), TURN_FACE_UP - 1);
        assert_eq!(unlock_door(1), TURN_FACE_UP - UNLOCK_SLOTS);
        let others = (0..GRANTED_SLOTS)
            .map(granted_ability)
            .chain([PREPARED_CAST, TURN_FACE_UP, TURN_FACE_UP - UNLOCK_SLOTS - 1])
            .chain([0, 1, 2, 7, 100, 65_535]);
        for index in others {
            assert_eq!(
                door_to_unlock(index),
                None,
                "index {index} decoded as a door"
            );
        }
        assert!(is_special_action(TURN_FACE_UP));
        assert!(is_special_action(unlock_door(1)));
        assert!(!is_special_action(PREPARED_CAST));
        assert!(!is_special_action(granted_ability(0)));
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
                | YesNoPrompt::PayLife { .. }
                | YesNoPrompt::PayPact { .. }
                | YesNoPrompt::Miracle { .. }
                | YesNoPrompt::CastWithoutPaying { .. }
                | YesNoPrompt::CastPaying { .. }
                | YesNoPrompt::DrawOffer { .. }
                | YesNoPrompt::CommanderReplace { .. }
                | YesNoPrompt::TopOfLibrary { .. }
                | YesNoPrompt::Discover { .. }
                | YesNoPrompt::Generic => false,
            }
        }
        for prompt in [
            YesNoPrompt::MayDo,
            YesNoPrompt::Generic,
            YesNoPrompt::Kicker,
            YesNoPrompt::PayLifeOrEnterTapped { amount: 1 },
            YesNoPrompt::PayTax { mana: 2 },
            YesNoPrompt::PayLife { amount: 7 },
            YesNoPrompt::Miracle { card: object() },
            YesNoPrompt::CastWithoutPaying { card: object() },
            YesNoPrompt::CastPaying { card: object() },
            YesNoPrompt::CommanderZone { card: object() },
            YesNoPrompt::TopOfLibrary { card: object() },
            YesNoPrompt::Discover { card: object() },
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

#[cfg(test)]
mod fit_to_options_tests {
    use super::*;

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    fn card(n: u32) -> ObjectId {
        ObjectId::new(n, 0)
    }

    /// A choice of cards is held to the cards it offers (CR 609.3), and a
    /// target choice is not: shrinking it would cast a spell with fewer
    /// targets than it requires (CR 601.2c).
    #[test]
    fn a_choice_of_cards_is_fitted_and_a_short_target_choice_is_not() {
        let mut cards = Pending::ChooseCards {
            player: me(),
            options: vec![card(1)],
            min: 2,
            max: 2,
            prompt: ChoicePrompt::Generic,
        };
        assert!(cards.fit_to_options());
        let Pending::ChooseCards { min, max, .. } = cards else {
            unreachable!()
        };
        assert_eq!((min, max), (1, 1));

        let mut targets = Pending::ChooseTargets {
            player: me(),
            options: vec![card(1)],
            player_options: vec![],
            min: 2,
            max: 2,
            reason: TargetPrompt::Targets,
        };
        assert!(!targets.fit_to_options());
        let Pending::ChooseTargets { min, .. } = targets else {
            unreachable!()
        };
        assert_eq!(min, 2, "a target choice keeps what it requires");

        let mut nobody = Pending::ChoosePlayer {
            player: me(),
            options: vec![],
        };
        assert!(!nobody.fit_to_options(), "an empty menu has no answer");
    }
}
