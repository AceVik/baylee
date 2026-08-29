//! The choice contract: the only way the game ever advances.
//!
//! The engine never asks open questions — every [`ChoiceRequest`] carries
//! the complete set of legal answers, precomputed. Humans, AIs, and network
//! clients all answer through the same [`PlayerAction`] type.

use crate::win::GameResult;
use baylee_core::ids::{ObjectId, PlayerId};

/// What the game is currently waiting for.
#[derive(Clone, Debug)]
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
    },
    /// Declare blockers (combat).
    ChooseBlockers {
        /// Defending player.
        player: PlayerId,
        /// Attacking player.
        attacker: PlayerId,
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
        /// The legal targets.
        options: Vec<ObjectId>,
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
#[derive(Clone, Debug)]
pub struct CastModeDesc {
    /// Option index (answered via `PlayerAction::ChooseMode`).
    pub index: u8,
    /// Which kind of cast this is.
    pub kind: CastModeKind,
    /// The mana part to pay with this option.
    pub cost: baylee_core::mana::ManaCost,
}

/// The kind of a cast option.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
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
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ChoicePrompt {
    /// Library search (tutor/fetch).
    SearchLibrary,
    /// Scry: choose cards for the bottom; the rest stays on top.
    ScryBottom,
    /// Put cards from your hand on top of your library (chosen order).
    PutBackOnTop,
    /// Generic selection.
    Generic,
}

/// What a [`Pending::YesNo`] asks.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
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
    /// Generic yes/no (optional effects).
    Generic,
}

/// Everything a player may legally do with priority (precomputed).
#[derive(Clone, Debug, Default)]
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

/// A player's answer to a [`Pending`] request.
#[derive(Clone, Debug)]
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
    /// Declare attackers with their defending players.
    DeclareAttackers {
        /// `(attacker, defending player)` pairs.
        attackers: Vec<(ObjectId, PlayerId)>,
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
}
