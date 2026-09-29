//! Game state: the complete, cloneable, hashable world.

use std::hash::Hash;
use std::sync::Arc;

use crate::arena::Arena;
use crate::event::{Cause, GameEvent, Journal, LibraryPlace, LossReason};
use crate::object::{
    CardRef, Characteristics, CounterKind, GameObject, ObjectKind, PrintedFace, Rider,
};
use crate::rng::GameRng;
use crate::turn::{DayNight, TurnInfo};
use crate::zone::{Zone, ZoneLocation, ZonePosition, Zones};
use baylee_cards_dsl::CardDef;
use baylee_core::ids::{CardIndex, Defender, NameRef, ObjectId, PlayerId};
use baylee_core::mana::{ManaColor, ManaPool, ManaSymbol};
use baylee_core::preset::{FormatId, GamePreset, PresetError};
use rustc_hash::FxHashMap;
use xxhash_rust::xxh3::Xxh3;

/// Registry seam: the engine resolves card definitions through this trait
/// and never depends on the compiled registry directly — a future runtime
/// card pack (custom cards, bosses) implements the same seam.
pub trait CardLookup {
    /// Resolves a card index to its definition.
    fn card(&self, index: CardIndex) -> Option<&'static CardDef>;
}

/// A seat's mutable state.
#[derive(Clone, Debug)]
pub struct Player {
    /// Seat handle.
    pub id: PlayerId,
    /// Life total.
    pub life: i32,
    /// Poison counters.
    pub poison: u16,
    /// Energy counters.
    pub energy: u16,
    /// Persistent designation earned through storied (CR 702.195).
    pub enduring_story: bool,
    /// The city's blessing, earned through ascend and kept for the rest of
    /// the game (CR 702.131).
    pub citys_blessing: bool,
    /// Mana pool.
    pub mana_pool: ManaPool,
    /// Maximum hand size modifier (Reliquary Tower & co.).
    pub hand_modifier: i8,
    /// Lands played this turn (CR 305.2: one per turn).
    pub lands_played_this_turn: u8,
    /// The timestamp this seat's most recent turn began at — the clock
    /// summoning sickness is measured against (CR 302.6).
    ///
    /// Per seat rather than per game, because the rule says *their* most
    /// recent turn: a creature cast on your turn is still sick through
    /// every opponent's turn that follows, and wakes when your next turn
    /// begins. One shared value said it woke as soon as anybody untapped,
    /// which handed a fresh mana creature to its controller a whole turn
    /// early — combat never saw it, because you only attack on your own
    /// turn, where the two readings agree.
    ///
    /// It holds the *last issued* stamp rather than the next one, so the
    /// comparison against it is strict: an object stamped at exactly this
    /// value was there before the turn began.
    pub turn_start_timestamp: u64,
    /// Set when a draw was attempted from an empty library (SBA loses).
    pub tried_empty_draw: bool,
    /// Combat damage this player has taken from each commander over the
    /// course of the game (CR 903.10a): twenty-one from one of them and
    /// they lose, whatever their life total says.
    ///
    /// Keyed by the commander's [`ObjectId`], which works for the same
    /// reason the marker list does — the id survives the zone changes that
    /// make the card a new object (CR 400.7). A commander that dies, goes
    /// home and comes back down is the same commander, and its tally does
    /// not start over.
    ///
    /// A `Vec` of pairs and not a map: there are at most a handful of
    /// commanders at a table, and a hashed collection would put iteration
    /// order into a hash that has to be identical on every machine.
    pub commander_damage: Vec<(ObjectId, u16)>,
    /// Why this player lost, or `None` while they are still in the game
    /// (they stay seated in multiplayer until CR 800.4 cleanup runs).
    ///
    /// Written only by [`crate::sba::eliminate_player`], and once: a player
    /// loses the game a single time, so a later concession does not rewrite
    /// how the game was lost. The same reason is in the journal's
    /// [`GameEvent::PlayerLost`], but the journal is a record of what
    /// happened and not state anyone queries; this is what a view reads.
    pub loss: Option<LossReason>,
    /// Which team this seat plays for, or `None` for a seat that plays for
    /// itself. It comes from the preset and never changes during a game,
    /// which is why it is deliberately absent from
    /// [`GameState::snapshot_hash`]: it cannot tell two states of one game
    /// apart, and hashing it would only churn every recorded hash.
    pub team: Option<u8>,
}

impl Player {
    /// Whether this player has lost the game.
    #[must_use]
    pub const fn has_lost(&self) -> bool {
        self.loss.is_some()
    }
}

/// The side a seat plays for.
///
/// A seat with no team is a side of one, and that is the whole of the rule
/// this type exists to state: "opponent" (CR 102.3) is *different side*, not
/// *different seat*, and once teams exist the two stop being the same
/// question. Written as an enum rather than an `Option<u8>` so a solo seat
/// cannot silently compare equal to another solo seat.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Side {
    /// Everyone carrying this team index.
    Team(u8),
    /// One seat, playing for nobody else.
    Solo(PlayerId),
}

/// Reserved by every game before any card name is interned.
pub(crate) const NAMELESS: NameRef = NameRef::new(0);

/// Deterministic name interner (rules identity, not display).
#[derive(Clone, Debug, Default)]
pub struct Names {
    map: FxHashMap<String, NameRef>,
    list: Vec<String>,
}

impl Names {
    /// Interns a name.
    pub fn intern(&mut self, name: &str) -> NameRef {
        if let Some(&id) = self.map.get(name) {
            return id;
        }
        let id = NameRef::new(self.list.len() as u32);
        let owned = name.to_string();
        self.list.push(owned.clone());
        self.map.insert(owned, id);
        id
    }

    /// Resolves a name.
    #[must_use]
    pub fn get(&self, id: NameRef) -> &str {
        &self.list[id.get() as usize]
    }

    /// The interned `name`, without interning it: `None` when no object of
    /// this game has ever carried it, and so none carries it now.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<NameRef> {
        self.map.get(name).copied()
    }

    /// Number of interned names.
    #[must_use]
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Whether no names are interned.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}

/// A delayed trigger registered for a future game point (suspend finishes,
/// pact payments, rebound re-casts).
#[derive(Clone, Hash, Debug)]
pub struct DelayedTrigger {
    /// Controlling player.
    pub controller: PlayerId,
    /// When it fires.
    pub when: DelayedWhen,
    /// What it does.
    pub action: DelayedAction,
}

/// When a delayed trigger fires.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DelayedWhen {
    /// At the controller's next upkeep.
    NextUpkeep,
    /// At the beginning of the next upkeep, whoever's turn it is
    /// (Archangel Avacyn's delayed transform).
    NextUpkeepOfAnyone,
    /// At the controller's next first main phase (Mana Drain).
    NextFirstMain,
    /// At the beginning of the next end step (Venser +2).
    NextEndStep,
    /// At the controller's next cleanup.
    NextCleanup,
    /// As the resolution that made it finishes — before anything else
    /// happens, not at a step. Cascade's cast is made "while the ability is
    /// resolving" (CR 702.85a), which a resolution that cannot start a cast
    /// of its own hands to the engine this way.
    AsResolutionEnds,
    /// "When that land dies or is put into exile" (earthbend, CR 701.66a):
    /// the next time `card`, as the object it was at `version`, goes from
    /// the battlefield to a graveyard or into exile. Asked of journal
    /// entries after `after` only, because a delayed trigger does not
    /// trigger on an event from before it was created (CR 603.7a); and
    /// once, on the first time that object leaves the battlefield at all
    /// (CR 603.7b) — a land bounced to its owner's hand has left, is a new
    /// object, and never comes back through this.
    ///
    /// Never polled at a step: `trigger::collect` reads it off the journal,
    /// and the scan that follows removes every one whose object is no
    /// longer on the battlefield at `version`.
    DiesOrIsExiled {
        /// The permanent watched.
        card: ObjectId,
        /// Its identity when the watch was created.
        version: u32,
        /// The journal sequence number the watch was created at.
        after: u64,
    },
}

/// What a delayed trigger does.
#[derive(Clone, Hash, Debug)]
pub enum DelayedAction {
    /// Cast a card from exile without paying its mana cost (rebound,
    /// suspend finish).
    CastFromExileWithoutPaying {
        /// The card in exile.
        card: ObjectId,
        /// Its identity on arriving there; leaving exile invalidates this
        /// permission even if the same card returns (CR 400.7).
        version: u32,
    },
    /// Cascade's cast (CR 702.85a): cast the exiled card without paying its
    /// mana cost, or, when it cannot be cast after all, put it on the bottom
    /// of its owner's library — "all cards exiled this way that weren't
    /// cast" go there.
    CastFreeOrBottom {
        /// The card in exile.
        card: ObjectId,
        /// Its identity on arriving there (CR 400.7).
        version: u32,
    },
    /// `Effect::MayCastTarget`'s cast, said yes to: a payment window for
    /// the card's mana cost, then the cast out of the pool (CR 608.2g).
    CastPaying {
        /// The card, where it was targeted.
        card: ObjectId,
        /// Its identity when it was targeted (CR 400.7).
        version: u32,
        /// "If you do, you can't cast additional spells this turn."
        then_no_more_spells: bool,
    },
    /// Pay a cost or lose the game (Pact of Negation).
    PayCostOrLose {
        /// The mana cost to pay.
        cost: baylee_core::mana::ManaCost,
    },
    /// Pay a cost or sacrifice the permanent (echo).
    PayCostOrSacrifice {
        /// The mana cost to pay.
        cost: baylee_core::mana::ManaCost,
        /// The permanent to sacrifice when not paid.
        card: ObjectId,
    },
    /// Add mana (Mana Drain's next-main-phase mana).
    AddMana {
        /// Color.
        color: baylee_core::mana::ManaColor,
        /// Amount.
        amount: u16,
    },
    /// Transform a permanent (Archangel Avacyn), unless it has left the
    /// battlefield (`version`, CR 400.7) or no longer shows `face`, the face
    /// it showed when this was created — it has transformed since, and the
    /// instruction is ignored (CR 701.27f).
    Transform {
        /// The permanent.
        card: ObjectId,
        /// Its identity when this was created.
        version: u32,
        /// The face it showed then.
        face: u8,
    },
    /// Sacrifice a permanent (Kiki-Jiki's token at the next end step), if
    /// it is still that object (`version`, CR 400.7) and still controlled
    /// by the delayed trigger's controller — a player sacrifices only a
    /// permanent they control (CR 701.21a).
    Sacrifice {
        /// The permanent.
        card: ObjectId,
        /// Its identity when this was created.
        version: u32,
    },
    /// Return an exiled card to the battlefield under its owner's control
    /// (Venser +2).
    ReturnToBattlefield {
        /// The card in exile.
        card: ObjectId,
    },
    /// Offer the cast of a discovered card (CR 701.57a) to the player who
    /// discovered it, without paying its mana cost; a card that cannot be
    /// cast, or that the player declines, goes to its owner's hand. Nothing
    /// happens if the card has left exile since (`version`, CR 400.7).
    CastDiscovered {
        /// The card in exile.
        card: ObjectId,
        /// Its identity on arriving there.
        version: u32,
    },
    /// A delayed triggered ability that goes on the stack (CR 603.7): its
    /// source and its effects. The source is the source of the ability
    /// that created it (CR 603.7e), and the object its trigger event was
    /// about is the event object its effects read ("return **it**").
    Trigger {
        /// The source of the ability that created it.
        source: ObjectId,
        /// What it does.
        effects: &'static [baylee_cards_dsl::Effect],
    },
}

/// Per-turn counters for conditional triggers (reset at every turn start).
#[derive(Clone, Hash, Debug)]
pub struct PerTurn {
    /// Noncreature spells cast this turn, per player.
    pub noncreature_spells: Vec<u32>,
    /// Cards drawn this turn, per player.
    pub draws: Vec<u32>,
    /// Whether the active player has drawn a card in this turn's draw step.
    /// Only the active player has a draw step (CR 504.1), and a turn has one,
    /// so the turn's reset starts each draw step clean. Only a draw made in
    /// that step sets it: a card drawn in the upkeep does not.
    pub drew_in_draw_step: bool,
    /// All spells cast this turn, per player (second-spell triggers).
    pub spells_cast: Vec<u32>,
    /// Players who may cast no further spells this turn (Conduit of
    /// Worlds: "If you do, you can't cast additional spells this turn").
    /// Read by `casting::may_begin_casting`, written by the cast that
    /// set it.
    pub no_more_spells: Vec<bool>,
    /// Whether each player lost life this turn (Luminarch Ascension).
    /// Written by [`GameState::change_life`] and nothing else.
    pub life_lost: Vec<bool>,
    /// Creatures that died this turn, all players (Emeritus of Woe's
    /// re-prepare condition).
    pub creatures_died: u32,
    /// What entered the battlefield this turn, in arrival order
    /// (`Filter::EnteredThisTurn`). Written where a `ZoneChanged` into the
    /// battlefield is journaled: [`GameState::move_object`] and a token's
    /// arrival.
    pub entered_battlefield: Vec<ObjectId>,
    /// Every card drawn this turn, as the object it became in its drawer's
    /// hand: the id and the version it arrived with. Sylvan Library's
    /// "cards in your hand drawn this turn" reads it; an entry whose version
    /// no longer matches the object's is a card that has left the hand since
    /// (CR 400.7) and is no longer one of them. Written by
    /// [`GameState::draw_cards`] and nothing else.
    pub drawn: Vec<(ObjectId, u32)>,
    /// Permissions to play a particular card this turn (Dauthi Voidwalker,
    /// Expressive Iteration). See [`PlayPermission`].
    pub playable: Vec<PlayPermission>,
    /// How many times each ability of each object has resolved this turn
    /// (Nissa, Resurgent Animist's "the second time this ability has
    /// resolved this turn"). Written by the stack's resolution of an
    /// ability and nothing else; see [`AbilityResolved`].
    pub resolved: Vec<AbilityResolved>,
    /// What each Muldrotha-style allowance has let its controller play from
    /// the graveyard this turn ([`GraveyardPlay`]).
    pub graveyard_plays: Vec<GraveyardPlay>,
    /// What was put into a graveyard this turn, from anywhere, in arrival
    /// order (`Filter::PutIntoGraveyardThisTurn`). Written by
    /// [`GameState::move_object`], beside `entered_battlefield`.
    pub entered_graveyard: Vec<ObjectId>,
    /// The objects that are exiled if they would die this turn, each with
    /// its version (`Effect::ExileIfDiesThisTurn`, CR 400.7): read by
    /// `replacement::graveyard_destination`.
    pub exile_if_dies: Vec<(ObjectId, u32)>,
}

/// One card played or cast from a graveyard under a
/// `Modifier::PermanentOfEachTypeFromGraveyard` allowance.
///
/// The allowance is the source's, as the object it is (CR 400.7): a Muldrotha
/// that left and came back gives a fresh one.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GraveyardPlay {
    /// Who played it.
    pub player: PlayerId,
    /// The permission's source.
    pub source: ObjectId,
    /// The source's version.
    pub version: u32,
    /// The card's permanent types: `LAND` for a land played, the rest for a
    /// spell cast, of which it used one.
    pub types: baylee_core::types::TypeSet,
}

/// One ability of one object, and how many times it has resolved this turn.
///
/// The object is its id **and** version: an object that changed zones is a
/// new object (CR 400.7), and its abilities have resolved no times yet.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AbilityResolved {
    /// The ability's source.
    pub source: ObjectId,
    /// The source's version when the ability resolved.
    pub version: u32,
    /// Index into the source's ability list.
    pub index: u32,
    /// Resolutions this turn.
    pub times: u32,
}

/// "You may play that card this turn" — a permission an effect gives one
/// player for one card, wherever it lies (CR 305.1 plays a land from the
/// hand; this widens it for one object).
///
/// It lives in [`PerTurn`] rather than on the object as a rider because
/// "this turn" is exactly the lifetime `PerTurn::reset` gives it, and it
/// names the object's `version` because the permission is for that object
/// and no later one: once the card is cast, played or moved it is a new
/// object (CR 400.7) and the permission is spent, which the version compare
/// says without anyone having to remove the entry.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PlayPermission {
    /// Who may play it.
    pub player: PlayerId,
    /// The card.
    pub card: ObjectId,
    /// The version of `card` the permission was given for.
    pub version: u32,
    /// "Without paying its mana cost" (Dauthi Voidwalker): a spell is cast
    /// through the free-cast path, and X is 0 (CR 107.3b).
    pub free: bool,
    /// "You may **cast** that card" (Ragavan, Nimble Pilferer) rather than
    /// "play": to play a card is to play it as a land or cast it as a spell
    /// (CR 601.1a), and this permission is the second half only, so a land
    /// exiled under it stays where it is (a land is never cast, CR 305.9).
    pub cast_only: bool,
}

impl PerTurn {
    /// Zeroed counters for `players` seats.
    #[must_use]
    pub fn new(players: usize) -> Self {
        Self {
            noncreature_spells: vec![0; players],
            spells_cast: vec![0; players],
            no_more_spells: vec![false; players],
            life_lost: vec![false; players],
            creatures_died: 0,
            draws: vec![0; players],
            drew_in_draw_step: false,
            entered_battlefield: Vec::new(),
            drawn: Vec::new(),
            playable: Vec::new(),
            resolved: Vec::new(),
            graveyard_plays: Vec::new(),
            entered_graveyard: Vec::new(),
            exile_if_dies: Vec::new(),
        }
    }

    /// Counts one more resolution of ability `index` of `source` as it is
    /// at `version`, and returns the count with it included.
    pub fn note_resolution(&mut self, source: ObjectId, version: u32, index: u32) -> u32 {
        if let Some(entry) = self
            .resolved
            .iter_mut()
            .find(|e| e.source == source && e.version == version && e.index == index)
        {
            entry.times += 1;
            return entry.times;
        }
        self.resolved.push(AbilityResolved {
            source,
            version,
            index,
            times: 1,
        });
        1
    }

    /// How many times ability `index` of `source` at `version` has resolved
    /// this turn.
    #[must_use]
    pub fn resolutions(&self, source: ObjectId, version: u32, index: u32) -> u32 {
        self.resolved
            .iter()
            .find(|e| e.source == source && e.version == version && e.index == index)
            .map_or(0, |e| e.times)
    }

    /// Spells `player` has cast this turn.
    #[must_use]
    pub fn spells_cast_by(&self, player: PlayerId) -> u32 {
        self.spells_cast
            .get(player.get() as usize)
            .copied()
            .unwrap_or(0)
    }

    /// Resets all counters (called at every turn start).
    pub fn reset(&mut self) {
        self.noncreature_spells.iter_mut().for_each(|v| *v = 0);
        self.draws.iter_mut().for_each(|v| *v = 0);
        self.drew_in_draw_step = false;
        self.spells_cast.iter_mut().for_each(|v| *v = 0);
        self.no_more_spells.iter_mut().for_each(|v| *v = false);
        self.life_lost.iter_mut().for_each(|v| *v = false);
        self.creatures_died = 0;
        self.entered_battlefield.clear();
        self.drawn.clear();
        self.playable.clear();
        self.resolved.clear();
        self.graveyard_plays.clear();
        self.entered_graveyard.clear();
        self.exile_if_dies.clear();
    }
}

/// What the turn before this one was, kept for CR 502.2.
///
/// Two numbers rather than a whole `PerTurn` snapshot: the untap step asks
/// exactly one question of the previous turn, and copying every counter to
/// answer it would put a per-seat allocation on every turn boundary.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PreviousTurn {
    /// Whose turn it was.
    pub active: PlayerId,
    /// How many spells that player cast during it.
    pub spells_cast: u32,
    /// How many spells every player together cast during it — the
    /// werewolves' "if no spells were cast last turn".
    pub spells_by_all: u32,
    /// The most spells any one player cast during it — "if a player cast
    /// two or more spells last turn".
    pub most_by_one: u32,
}

/// A registered replacement rule from a permanent on the battlefield.
#[derive(Clone, Copy, Hash, Debug)]
pub struct ReplacementEntry {
    /// The source permanent.
    pub source: ObjectId,
    /// The rule's controller (for "you" in its filters): whoever controls
    /// the source now (CR 109.5), which the projection keeps it as
    /// (`GameState::refresh_characteristics`).
    pub controller: PlayerId,
    /// The rule.
    pub rule: baylee_cards_dsl::ReplacementRule,
}

/// Setup failures.
#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    /// The preset is structurally invalid.
    #[error("invalid preset: {0}")]
    Preset(#[from] PresetError),
    /// A deck entry references a card the lookup cannot resolve.
    #[error("unknown card index {0}")]
    UnknownCard(CardIndex),
    /// A counter name is not in the supported setup vocabulary.
    #[error("unknown starting counter `{0}`")]
    UnknownCounter(String),
}

/// State operation failures.
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    /// The object does not exist (anymore).
    #[error("no such object: {0}")]
    NoSuchObject(ObjectId),
}

/// Printed characteristics, shared by every object that prints the same.
///
/// A [`GameObject`] holds its printed face behind an [`Arc`]. Three places
/// in the engine write a base, all through [`GameObject::base_mut`], which
/// splits the sharing before it writes; everything else only reads. So every
/// copy of a card in a deck, every token of the same kind and every
/// triggered ability waiting on the stack can point at one allocation
/// instead of carrying 256 bytes of its own.
///
/// The scale this exists for is a board of a few thousand tokens under a
/// stack of a million abilities. Without the table that is a million
/// allocations scattered across the heap, which the layer refresh then
/// chases one cache miss at a time; with it, one allocation the refresh
/// keeps in L1. The table itself sits behind an `Arc` too, so cloning a
/// state for the AI copies a pointer rather than the map.
#[derive(Clone, Debug, Default)]
pub struct BaseCache {
    /// Printed card faces, keyed by card. Only the front face is interned:
    /// [`GameState::switch_face`] rebuilds from the definition and is rare.
    cards: FxHashMap<CardIndex, Arc<Characteristics>>,
    /// The blank face a card-less, token-less object starts from — an
    /// ability on the stack, an emblem — keyed by its name.
    bare: FxHashMap<NameRef, Arc<Characteristics>>,
    /// Token faces, keyed by the definition they were printed from and the
    /// size an effect may have overridden (Skyclave Apparition's Illusion).
    /// The definition is a `&'static`, so its address is its identity —
    /// two token kinds that share a name do not share a face.
    tokens: FxHashMap<(usize, Option<i16>), Arc<Characteristics>>,
}

/// One of a seat's commanders (CR 903.3), and what it has cost so far.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Commander {
    /// The card. Its [`ObjectId`] is the marker, because it survives the
    /// zone changes that make the card a new object (CR 400.7).
    pub object: ObjectId,
    /// Times it has been cast from the command zone, which is the whole of
    /// CR 903.8's tax: `{2}` more generic for each of them.
    pub casts: u32,
    /// The zone-arrival timestamp its owner has already been asked about
    /// (CR 903.9a), or 0 while no arrival has been offered.
    ///
    /// The rule says "put into that zone *since the last time state-based
    /// actions were checked*", which sounds like one watermark on the game
    /// and is not: two commanders swept up by the same wrath arrive at two
    /// timestamps, and a single watermark moved by the first question
    /// silently swallows the second. Per commander it is exact, because
    /// SBAs run before every priority grant — the first check after an
    /// arrival is always the one that offers it, so "not offered yet" and
    /// "arrived since the last check" name the same moment.
    ///
    /// Comparing against the *arrival* rather than storing a yes/no is what
    /// makes a second question legal: a commander left in a graveyard and
    /// later exiled by someone else has arrived somewhere new, and its
    /// owner is asked again.
    pub answered: u64,
}

/// The whole game world: cloneable for AI, hashable for determinism.
#[derive(Clone, Debug)]
pub struct GameState {
    /// All game objects.
    pub arena: Arena<GameObject>,
    /// Ordered zone contents.
    pub zones: Zones,
    /// Seats in turn order.
    pub players: Vec<Player>,
    /// Turn bookkeeping.
    pub turn: TurnInfo,
    /// Combat phase state.
    pub combat: crate::combat::CombatState,
    /// Per-turn counters for conditional triggers (Esper Sentinel, Orcish
    /// Bowmasters): noncreature spells cast and cards drawn per player this
    /// turn, reset at every turn start.
    pub per_turn: PerTurn,
    /// Registered delayed triggers (suspend finishes, pact payments).
    pub delayed: Vec<DelayedTrigger>,
    /// First-of-turn drawn cards awaiting a miracle offer (CR 702.94).
    pub pending_miracle: std::collections::VecDeque<(PlayerId, ObjectId)>,
    /// Queued extra turns (CR 500.7); the front player takes the next
    /// turn instead of the normal successor.
    pub extra_turns: std::collections::VecDeque<PlayerId>,
    /// Restriction-id → (source, spell filter, spend rider) for
    /// restricted mana in players' pools (Cavern, Path of Ancestry).
    pub restriction_info: rustc_hash::FxHashMap<
        u32,
        (
            ObjectId,
            &'static baylee_cards_dsl::Filter,
            baylee_cards_dsl::SpendRider,
        ),
    >,
    /// Next restriction id to hand out (0 = unrestricted sentinel).
    pub next_restriction_id: u32,
    /// Times each player cast a commander from the command zone this
    /// game (Commander's Insight).
    ///
    /// Deliberately *not* the tax counter: CR 903.8 taxes each commander
    /// for its own previous casts, while Commander's Insight counts the
    /// player's casts of any of them. A partner deck makes the two differ,
    /// so they are two numbers — incremented together, at one place, which
    /// is what keeps them from drifting.
    pub commander_casts: Vec<u32>,
    /// Answers already given to CR 903.9b, waiting for the move they were
    /// asked about.
    ///
    /// The rule replaces an event, so the answer has to be known *before*
    /// the card moves: asking afterwards and moving a second time is a
    /// different game — "shuffle target creature into its owner's library,
    /// then that player draws a card" draws from a library the commander is
    /// in. [`Self::move_object`] is synchronous and cannot ask, so the
    /// question is put before the effect touches anything and the answer
    /// waits here for the funnel to consume it.
    ///
    /// An entry is removed by the move it belongs to, which is what keeps
    /// one list serving a whole event: CR 903.9b applies "more than once to
    /// the same event", and a wrath that bounces three commanders leaves
    /// three entries that three moves take one each.
    pub commander_redirect: Vec<(ObjectId, bool)>,
    /// Token copies waiting to be handed the printed rules text they were
    /// created owing: `(the copy, the card it copies, which face)`.
    ///
    /// CR 707.2 copies the original's abilities along with its
    /// characteristics, and for a card-backed original those live behind the
    /// card registry — which the rules kernel does not depend on and
    /// `resolve` therefore has no lookup for, the same wall the daybound
    /// checks meet in `sba::run`. So the copy is created naming the face it
    /// copied and the machine, which does hold a lookup, fills its
    /// `own_abilities` in at the top of the next pass, before anything asks
    /// what the copy can do.
    ///
    /// A queue here rather than a field on the object: a `GameObject` is
    /// copied once per object per ply of the AI's search, and eight bytes
    /// there is a `tests/footprint.rs` budget and a measurable memcpy, while
    /// a list that is empty in almost every game state costs one `Vec`
    /// header on the state itself.
    pub pending_copied_faces: Vec<(ObjectId, CardIndex, u8)>,
    /// What a copy could do as it left the battlefield (CR 603.10a).
    ///
    /// The look-back scan for leaves-the-battlefield and dies triggers reads
    /// an object that has already arrived in its graveyard, and the rule is
    /// explicit about what it must see there: the game looks back "using the
    /// existence of those abilities and the appearance of objects immediately
    /// prior to the event". Not only the appearance. [`Self::move_object`]
    /// has by then given the copy back — CR 400.7, the card in the graveyard
    /// is the printed card — so the scan was asking a Phyrexian Metamorph
    /// whether it had a dies trigger, and it does not: the copy of Solemn
    /// Simulacrum died and drew nobody a card.
    ///
    /// One entry per object at most, written on every departure and removed
    /// on every other move, so it describes the last one and no earlier one.
    /// A card that was never a copy is not in here at all, because its
    /// printed list did not change and the object answers for itself.
    ///
    /// On the state and not on the object, for the reason
    /// [`Self::pending_copied_faces`] gives: sixteen bytes on a `GameObject`
    /// is a `tests/footprint.rs` budget and a memcpy per object per ply,
    /// while a list that is empty in almost every game state is a `Vec`
    /// header once.
    pub ltb_abilities: Vec<(ObjectId, crate::object::AbilityList)>,
    /// Mana values immediately before battlefield departures. Captured by
    /// triggers before this bounded look-back store is cleared.
    pub ltb_mana_values: Vec<(ObjectId, u32)>,
    /// Who controlled a permanent immediately before it left the
    /// battlefield (CR 603.10a) — what `PlayerRel::ControllerOfEvent`
    /// reads for Massacre Wurm's "whenever a creature an opponent controls
    /// dies, **that player** loses 2 life".
    ///
    /// The object cannot answer for itself: a card in a graveyard is
    /// controlled by nobody and the refresh settles its field back to its
    /// owner, and a token that died has ceased to exist (CR 704.5d) before
    /// its trigger resolves. Same lifecycle as [`Self::ltb_mana_values`]:
    /// cleared for an object on every move and written again only on a
    /// departure from the battlefield.
    pub ltb_controllers: Vec<(ObjectId, PlayerId)>,
    /// The power each object had the moment it last left the battlefield
    /// (CR 608.2h: an effect that needs a value from an object that has
    /// left uses its last known information) — what
    /// `Effect::EventObjectDealsDamageEqualToPower` reads when "that
    /// creature" is gone by the time Pyrogoyf's trigger resolves. Same
    /// lifecycle as [`Self::ltb_controllers`].
    pub ltb_powers: Vec<(ObjectId, i16)>,
    /// What was attached to a permanent the moment it left the battlefield.
    ///
    /// The other half of CR 603.10a, and it is needed for the same reason and
    /// at the same moment. `Filter::AttachedToBySource` reads the Equipment's
    /// *live* `attached_to`, and the state-based actions have let go of the
    /// host by the time triggers are collected: CR 704.5f puts the creature
    /// in the graveyard and CR 704.5n unattaches the Equipment, both inside
    /// one `sba::run` fixpoint that runs to quiescence before
    /// `collect_triggers`. So "whenever equipped creature dies" was asked of
    /// an Equipment wearing nobody, and Skullclamp clamped a 1/1 into the
    /// graveyard and drew its controller nothing at all.
    ///
    /// Keyed by the **host** that departed, one entry at most, written on
    /// every departure from the battlefield and removed on every other move
    /// — the lifetime of [`Self::ltb_abilities`] exactly, and it is what
    /// makes the fallback safe to consult from `eval::matches` in general: an
    /// entry only ever names an object that is not on the battlefield, so no
    /// layer projection can see it, and an object that comes back has its
    /// entry cleared by the very move that brings it back.
    pub ltb_attachments: Vec<(ObjectId, Vec<ObjectId>)>,
    /// What a permanent had *on* it the moment it left the battlefield.
    ///
    /// The third half of CR 603.10a, and the one without which undying is a
    /// loop rather than a rule. "If it had no +1/+1 counters on it"
    /// (CR 702.93a, and CR 702.79a for persist) is a question about the
    /// object as it last existed on the battlefield — and `move_object`
    /// clears `counters` on every departure from there, for the good reason
    /// that a blinked creature must not bring three +1/+1 counters back. So
    /// by the time triggers are collected the card in the graveyard
    /// truthfully has none, every time, and a Wolf that returned with a
    /// counter would keep returning for ever.
    ///
    /// Written and cleared exactly where [`Self::ltb_abilities`] is, so an
    /// entry only ever names an object that is not on the battlefield and
    /// the move that brings one back removes its entry. That lifetime is
    /// also why [`Self::snapshot_hash`] reads it, as it reads the other two:
    /// an entry is not scan bookkeeping that a priority grant clears, it
    /// stays for as long as its object stays off the battlefield.
    pub ltb_counters: Vec<(ObjectId, crate::object::Counters)>,
    /// What a permanent *was* the moment it left the battlefield: its
    /// projected characteristics, the fourth half of CR 603.10a.
    ///
    /// `move_object` drops the projection on every move (CR 400.7), so by
    /// the time a leaves-the-battlefield trigger is collected the card in
    /// the graveyard answers with its printed values. A land a Living Lands
    /// had made a creature died as a Forest card and "whenever a creature
    /// you control dies" saw no creature; an Enduring Vitality that had
    /// returned as an enchantment died as an enchantment creature card and
    /// returned again. The leaves-the-battlefield trigger arms read this and
    /// nothing else does: every other question about a card off the
    /// battlefield is about the card as it is now.
    ///
    /// Written and cleared exactly where [`Self::ltb_abilities`] is.
    pub ltb_characteristics: Vec<(ObjectId, crate::object::Characteristics)>,
    /// Objects that have ceased to exist but whose triggers have not fired.
    ///
    /// CR 111.7 says it in a parenthesis, and the parenthesis is the whole
    /// rule: "if a token changes zones, applicable triggered abilities will
    /// trigger before the token ceases to exist". CR 704.5d sweeps the token
    /// away as a state-based action, and in this engine the state-based
    /// actions run to a fixpoint *before* `collect_triggers` — so by the time
    /// anything asks, the token is out of the arena and out of its zone list,
    /// and two different questions about it both answer no: its own dies
    /// trigger is not scanned, because the scan walks zone lists, and another
    /// permanent's "whenever a creature you control dies" cannot evaluate its
    /// filter, because there is no object to evaluate it against. A token
    /// died and the table saw nothing happen.
    ///
    /// So the object is *moved* here rather than dropped — `Arena::remove`
    /// hands it back and `sba::run` keeps it — and both scans consult
    /// [`Self::object_or_departed`]. The list is cleared the moment the
    /// trigger scan has passed the journal entries it is about, which is the
    /// one place that can know: nothing later can ask again, and an entry
    /// that outlived its scan would be an unbounded leak in a deck that makes
    /// thousands of tokens.
    ///
    /// The same look-back as [`Self::ltb_abilities`] and
    /// [`Self::ltb_attachments`], and deliberately a third list rather than a
    /// widening of either: those two answer *what a permanent was* to a scan
    /// that can still find it, and this one answers *that there was one at
    /// all*. Excluded from `snapshot_hash` and `loop_signature` on purpose —
    /// both enumerate their fields, and this one is scan bookkeeping that
    /// never survives a priority grant, so hashing it would make a game state
    /// depend on when it was asked.
    pub ceased: Vec<GameObject>,
    /// Reflexive triggered abilities (CR 603.12) created by the resolution
    /// in progress, waiting for the next time a player would receive
    /// priority (CR 603.3).
    ///
    /// Only `Effect::Reflexive` fills it, during a stack resolution, and
    /// `Engine::finish_resolution` empties it into the trigger queue as that
    /// resolution ends. `lints::every_reflexive_sits_where_it_can_trigger`
    /// makes the reflexive the last op of its list, and the op itself never
    /// asks anything. So no question can be published between the push and
    /// the drain, and the list is empty whenever one is out and whenever
    /// `run_machine` samples a `loop_signature`. That is why neither
    /// `snapshot_hash` nor `loop_signature` reads it. It rests on a rule the
    /// build enforces, not on which cards happen to exist.
    pub reflexive: Vec<crate::trigger::PendingTrigger>,
    /// Cards a resolving discover (CR 701.57a) exiled and stopped at:
    /// `(the discovering player, the card, its version in exile)`.
    ///
    /// Only `Effect::Discover` fills it, during a stack resolution, and
    /// `Engine::finish_resolution` hands each entry to the delayed queue as
    /// that resolution ends, where the cast is offered before anybody
    /// receives priority. Unlike [`Self::reflexive`] nothing keeps a
    /// discover last in its list, so a resolution can suspend on a later
    /// question with an entry here: `snapshot_hash` reads it, and
    /// `loop_signature`, taken only at priority grants, has nothing to read.
    pub discovered: Vec<(PlayerId, ObjectId, u32)>,
    /// The announced division of each ability on the stack that deals
    /// damage "divided as you choose" (CR 601.2d): its targets, each with
    /// its share. Written as the ability is put on the stack
    /// (`Engine::ask_trigger_division`) and read as it resolves; an entry
    /// whose object has left the stack is dropped as the next resolution
    /// ends. Off the object because no other object has one, and
    /// `tests/footprint.rs` holds `GameObject` to its size. Nothing copies a
    /// triggered ability, so no copy has to carry one (CR 115.7f).
    pub divided: Vec<(ObjectId, Vec<(ObjectId, u32)>)>,
    /// Copies of synthetic abilities made by the resolution in progress, as
    /// `(original, copy)` (CR 707.10).
    ///
    /// A synthetic ability (prowess, ward, a granted or reflexive trigger)
    /// keeps its effects beside the engine and not on its object, where the
    /// resolver cannot reach them, so `Effect::CopyTargetAbility` names the
    /// pair here and `Engine::finish_resolution` hands the copy the
    /// original's effects. The original is still on the stack below the
    /// copy then, so its effects are still there to hand over.
    ///
    /// Unlike `reflexive`, this is hashed: the copy's controller is asked
    /// about new targets (CR 707.10c) before that resolution ends, so the
    /// list is not empty while a question is out.
    pub synthetic_copies: Vec<(ObjectId, ObjectId)>,
    /// Each seat's commanders (CR 903.3), by seat index.
    ///
    /// The list is the marker, and it has to be: commander-ness belongs to
    /// the card (CR 903.3) while a zone change makes a new object
    /// (CR 400.7), so a flag on the object would have to survive every
    /// `move_object`. The command *zone* cannot serve either, which is the
    /// bug this replaces — `move_object` removes the id from the zone it
    /// left, so "my commander is on the battlefield" was unanswerable and
    /// every card that asked got `false`. `ObjectId` outlives the zone
    /// change, so this list stays true wherever the card goes.
    pub commanders: Vec<Vec<Commander>>,
    /// The monarch designation (CR 718), if any.
    pub monarch: Option<PlayerId>,
    /// The day/night designation (CR 731), if the game has one yet.
    pub day_night: Option<DayNight>,
    /// What the previous turn was, for CR 502.2's check at the untap step.
    ///
    /// The check needs the previous turn's active player and how many spells
    /// *they* cast during it, and neither survives to be read: [`PerTurn`] is
    /// reset as the next turn begins, before its untap step. So the pair is
    /// snapshotted at that turn boundary instead of counted twice.
    pub previous_turn: Option<PreviousTurn>,
    /// The player who took the first turn (Surgical Metamorph & co.).
    pub starting_player: PlayerId,
    /// How often an ability of an object has been used this turn, cleared as
    /// a turn begins.
    ///
    /// Four clauses share it because each is one ability's count of one
    /// thing it does in a turn: "this ability triggers only once each turn"
    /// (Jin-Gitaxias), "activate only once each turn" (Wall of Roots), "do
    /// this only once each turn" (The Reaper, King No More: set by the yes)
    /// and "if this is the first time this ability has resolved this turn"
    /// (Omnath, Locus of Creation: its resolutions). No ability says two of
    /// them. The key is the object and the ability index, so a permanent
    /// that leaves the battlefield and comes back starts over — CR 400.7
    /// rather than a convenience.
    ///
    /// It is hashed into [`Self::loop_signature`], because what is left of a
    /// limit decides what is offered. `Engine::loyalty_used_this_turn` is
    /// the same kind of state and is **not** hashed, because it does not
    /// live here; that is a known hole and not this field's.
    pub ability_fires: rustc_hash::FxHashMap<(ObjectId, u32), u32>,
    /// Seeded randomness.
    pub rng: GameRng,
    /// The event journal.
    pub journal: Journal,
    /// Name interner.
    pub names: Names,
    /// Printed characteristics, shared between objects that print the same.
    pub bases: Arc<BaseCache>,
    /// Monotonic timestamp source (effects ordering).
    pub timestamp: u64,
    /// Registered continuous effects (anthems, type changes, pumps).
    pub effects: crate::effects::EffectTable,
    /// Registered replacement rules (Doubling Season, Panharmonicon, …).
    pub replacement_rules: Vec<ReplacementEntry>,
    /// The effect generation the characteristic caches were computed at.
    pub characteristics_generation: u64,
    /// Scratch list reused by [`GameState::refresh_characteristics`].
    ///
    /// A refresh runs after every effect-set change, so allocating its
    /// working list each time is a per-effect malloc for the whole game.
    /// Always left empty, which keeps it free to clone.
    projection_ids: Vec<ObjectId>,
    /// Whether the preceding refresh touched off-board objects. Cache-only:
    /// the first refresh after a cross-zone effect ends must clear them too.
    projected_cross_zone: bool,
    /// The cards whose front face prints a characteristic-defining power
    /// and toughness ("Ashaya's power and toughness are each equal to the
    /// number of lands you control"), with the modifier that defines them.
    ///
    /// CR 604.3 has a characteristic-defining ability function in every
    /// zone, but a static is registered only while its source is on the
    /// battlefield, so in a library, a hand, a graveyard, exile or on the
    /// stack the card had its printed `*` as 0 — the toughness Recruiter of
    /// the Guard and the power Reveillark read. The projection applies these
    /// itself wherever the card is **not** on the battlefield
    /// (`layers::recompute_with`); on the battlefield the registered static
    /// does it, so an effect that takes the abilities away still takes this
    /// one. Written once per card in [`Self::create_card`], front face only
    /// (CR 712.8a), and never changed: a list the size of a handful of
    /// cards.
    pub printed_pt_cda: Vec<(ObjectId, baylee_cards_dsl::Modifier)>,
    /// Objects that may have become a token outside the battlefield
    /// (CR 704.5d), queued for the next state-based-action pass.
    ///
    /// The alternative — and what this replaces — is scanning the whole
    /// arena on every SBA round, which runs as a fixpoint before every
    /// priority grant. That is fine at sixty objects and fatal at a
    /// million: an Ally deck can put six-figure counts of abilities on the
    /// stack, and each of them would be visited by every round of every
    /// pass to find, almost always, nothing. A token can only *become*
    /// eligible when it leaves the battlefield, so recording that moment
    /// turns O(arena) per round into O(moves).
    ///
    /// Drained by every pass, so it is empty whenever anyone can observe
    /// the state — which is what keeps it out of [`Self::snapshot_hash`].
    token_cleanup: Vec<ObjectId>,
}

#[cfg(any(test, feature = "fuzz"))]
impl GameState {
    /// Every field, one line each, for `Engine::fingerprint`.
    ///
    /// Named without `..`, so a new field does not compile until it is here.
    /// The journal is only ever appended to, so its length and last entry
    /// stand for all of it; printing the whole of it at every decision of a
    /// long game is quadratic.
    ///
    /// `whole` false leaves out the three prints that are nearly all of the
    /// size (the arena, the base cache, the names), as empty lines in their
    /// places, so a caller can afford a light comparison at every step and
    /// the whole one where it samples.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn fingerprint(&self, whole: bool, out: &mut Vec<(&'static str, String)>) {
        let GameState {
            arena,
            zones,
            players,
            turn,
            combat,
            per_turn,
            delayed,
            pending_miracle,
            extra_turns,
            restriction_info,
            next_restriction_id,
            commander_casts,
            commander_redirect,
            pending_copied_faces,
            ltb_abilities,
            ltb_mana_values,
            ltb_controllers,
            ltb_powers,
            ltb_attachments,
            ltb_counters,
            ltb_characteristics,
            ceased,
            reflexive,
            discovered,
            divided,
            synthetic_copies,
            commanders,
            monarch,
            day_night,
            previous_turn,
            starting_player,
            ability_fires,
            rng,
            journal,
            names,
            bases,
            timestamp,
            effects,
            replacement_rules,
            characteristics_generation,
            projection_ids,
            projected_cross_zone,
            token_cleanup,
            printed_pt_cda,
        } = self;
        let mut restrictions: Vec<_> = restriction_info.iter().collect();
        restrictions.sort_by_key(|(id, _)| **id);
        let mut fires: Vec<_> = ability_fires.iter().collect();
        fires.sort_by_key(|(key, _)| **key);
        let heavy = |print: &dyn std::fmt::Debug| {
            if whole {
                format!("{print:?}")
            } else {
                String::new()
            }
        };
        out.extend([
            ("state.arena", heavy(arena)),
            ("state.zones", format!("{zones:?}")),
            ("state.players", format!("{players:?}")),
            ("state.turn", format!("{turn:?}")),
            ("state.combat", format!("{combat:?}")),
            ("state.per_turn", format!("{per_turn:?}")),
            ("state.delayed", format!("{delayed:?}")),
            ("state.pending_miracle", format!("{pending_miracle:?}")),
            ("state.extra_turns", format!("{extra_turns:?}")),
            ("state.restriction_info", format!("{restrictions:?}")),
            (
                "state.next_restriction_id",
                format!("{next_restriction_id:?}"),
            ),
            ("state.commander_casts", format!("{commander_casts:?}")),
            (
                "state.commander_redirect",
                format!("{commander_redirect:?}"),
            ),
            (
                "state.pending_copied_faces",
                format!("{pending_copied_faces:?}"),
            ),
            ("state.ltb_abilities", format!("{ltb_abilities:?}")),
            ("state.ltb_mana_values", format!("{ltb_mana_values:?}")),
            ("state.ltb_controllers", format!("{ltb_controllers:?}")),
            ("state.ltb_powers", format!("{ltb_powers:?}")),
            ("state.ltb_attachments", format!("{ltb_attachments:?}")),
            ("state.ltb_counters", format!("{ltb_counters:?}")),
            (
                "state.ltb_characteristics",
                format!("{ltb_characteristics:?}"),
            ),
            ("state.ceased", format!("{ceased:?}")),
            ("state.reflexive", format!("{reflexive:?}")),
            ("state.discovered", format!("{discovered:?}")),
            ("state.divided", format!("{divided:?}")),
            ("state.synthetic_copies", format!("{synthetic_copies:?}")),
            ("state.commanders", format!("{commanders:?}")),
            ("state.monarch", format!("{monarch:?}")),
            ("state.day_night", format!("{day_night:?}")),
            ("state.previous_turn", format!("{previous_turn:?}")),
            ("state.starting_player", format!("{starting_player:?}")),
            ("state.ability_fires", format!("{fires:?}")),
            ("state.rng", format!("{rng:?}")),
            (
                "state.journal",
                format!("{} {:?}", journal.last_seq(), journal.entries().last()),
            ),
            ("state.names", heavy(names)),
            ("state.bases", heavy(bases)),
            ("state.timestamp", format!("{timestamp:?}")),
            ("state.effects", format!("{effects:?}")),
            ("state.replacement_rules", format!("{replacement_rules:?}")),
            (
                "state.characteristics_generation",
                format!("{characteristics_generation:?}"),
            ),
            ("state.projection_ids", format!("{projection_ids:?}")),
            (
                "state.projected_cross_zone",
                format!("{projected_cross_zone:?}"),
            ),
            ("state.token_cleanup", format!("{token_cleanup:?}")),
            ("state.printed_pt_cda", format!("{printed_pt_cda:?}")),
        ]);
    }
}

/// The game as it stood before a payment began, to put back if the payment
/// cannot finish ([`GameState::checkpoint`]).
pub(crate) struct Checkpoint {
    state: Box<GameState>,
    journal: usize,
}

impl GameState {
    /// Keeps the game as it stands, for [`Self::roll_back`].
    ///
    /// CR 732.1: an action that cannot legally be completed is reversed and
    /// "any payments already made are canceled", and no ability triggers and
    /// no effect applies as a result of it. A payment is written part by
    /// part, so the one sure way to cancel whatever of it was written is to
    /// put back the game it was written into. Everything is copied except
    /// the journal, which only grows and is as long as the game: its length
    /// is kept instead, and what the payment journaled is cut off, which is
    /// also what keeps a trigger from seeing it.
    pub(crate) fn checkpoint(&mut self) -> Checkpoint {
        let journal = std::mem::take(&mut self.journal);
        let state = Box::new(self.clone());
        self.journal = journal;
        Checkpoint {
            state,
            journal: self.journal.len(),
        }
    }

    /// Puts back the game [`Self::checkpoint`] kept, the journal cut back to
    /// its length then. Nothing reads the journal while a payment runs (the
    /// trigger scan and the entry scan move only between actions), so no
    /// reader is left pointing past the cut.
    pub(crate) fn roll_back(&mut self, to: Checkpoint) {
        let mut journal = std::mem::take(&mut self.journal);
        journal.cancel_from(to.journal);
        *self = *to.state;
        self.journal = journal;
    }
}

impl GameState {
    /// The side a seat plays for (CR 102.3).
    #[must_use]
    pub fn side_of(&self, player: PlayerId) -> Side {
        self.players
            .get(player.get() as usize)
            .and_then(|p| p.team)
            .map_or(Side::Solo(player), Side::Team)
    }

    /// Whether `other` is an opponent of `player` — a different side, which
    /// in a game with no teams is simply a different seat.
    ///
    /// Every rule that says "opponent" goes through here. The ones that say
    /// "each other player" (a draw offer, a symmetrical effect) deliberately
    /// do not: a teammate is not an opponent, but they are another player.
    #[must_use]
    pub fn is_opponent(&self, other: PlayerId, player: PlayerId) -> bool {
        other != player && self.side_of(other) != self.side_of(player)
    }

    /// Whether `player` may pay `amount` life (CR 119.4).
    ///
    /// The rule is "greater than or **equal** to the amount of the payment",
    /// so a player on exactly two life may pay two and lose the game to
    /// CR 704.5a a moment later. That is their call and not the engine's:
    /// this was written three times as `life <= amount → no`, which quietly
    /// took the last point of life off the table — a shockland entered
    /// tapped without asking, and a fetchland was never offered. The margin
    /// that keeps the house AI from killing itself is the AI's own
    /// (`activate::life_ok`, `life > amount + 5`), which is where a policy
    /// belongs; a rule that refuses a legal action is not a policy.
    ///
    /// Zero is always payable whatever the life total, including a negative
    /// one, which is CR 119.4b said exactly.
    ///
    /// Two-Headed Giant pays out of the team's life total (CR 119.4a), which
    /// this does not model: teams exist here ([`Player::team`]) but life
    /// does not pool — every seat carries its own — so this asks the seat.
    ///
    /// A player who can't lose life can't pay any (CR 119.8), which
    /// [`Self::life_payable`] answers for this and for the pay-X-life cap.
    #[must_use]
    pub fn can_pay_life(&self, player: PlayerId, amount: i32) -> bool {
        amount <= 0 || self.life_payable(player) >= amount
    }

    /// How much life `player` has to pay with: their total, or none while
    /// they can't lose life (CR 119.8: "a cost that involves having that
    /// player pay life can't be paid").
    ///
    /// [`Self::can_pay_life`] and Toxic Deluge's cap on X both read this, so
    /// the two can't disagree about what a player may pay.
    #[must_use]
    pub fn life_payable(&self, player: PlayerId) -> i32 {
        if self.cant_lose_life(player) {
            return 0;
        }
        self.players
            .get(player.get() as usize)
            .map_or(0, |p| p.life)
    }

    /// Whether an effect says `player` can't lose life (Everybody Lives!).
    ///
    /// Each effect's `who` is read from that effect's own controller, the
    /// way [`Self::draw_limit`] reads its own. A relation only a resolution
    /// can answer (`Chosen`, `ControllerOfTarget`) names nobody here, and
    /// `lints::a_continuous_player_relation_is_one_the_state_can_answer` keeps
    /// the pool from printing one.
    #[must_use]
    pub fn cant_lose_life(&self, player: PlayerId) -> bool {
        self.effects.iter().any(|fx| {
            let baylee_cards_dsl::Modifier::CantLoseLife { who } = fx.modifier else {
                return false;
            };
            crate::eval::players(who, self, fx.controller).is_some_and(|p| p.contains(&player))
        })
    }

    /// Builds a game from a preset: seats, decks, shuffles, opening hands,
    /// starting battlefield, emblems.
    ///
    /// # Errors
    /// [`SetupError::Preset`] for structural violations,
    /// [`SetupError::UnknownCard`] for unresolvable deck entries.
    ///
    /// # Panics
    /// Internal invariant violations (freshly created objects are always present).
    #[allow(clippy::too_many_lines)] // setup is a linear checklist; extraction would obscure it
    pub fn from_preset(preset: &GamePreset, lookup: &impl CardLookup) -> Result<Self, SetupError> {
        preset.validate()?;
        for placement in &preset.house_rules.starting_counters {
            for counter in &placement.counters {
                if CounterKind::from_setup_name(&counter.kind).is_none() {
                    return Err(SetupError::UnknownCounter(counter.kind.clone()));
                }
            }
        }
        let default_life = match preset.format {
            FormatId::Commander => 40,
            _ => 20,
        };
        let seats = preset.seats.len() as u8;
        let mut state = Self {
            arena: Arena::with_capacity(512),
            zones: Zones::new(preset.seats.len()),
            players: preset
                .seats
                .iter()
                .enumerate()
                .map(|(i, s)| Player {
                    id: PlayerId::new(i as u8),
                    life: s.starting_life.unwrap_or(default_life),
                    poison: 0,
                    energy: 0,
                    enduring_story: false,
                    citys_blessing: false,
                    mana_pool: ManaPool::new(),
                    hand_modifier: 0,
                    lands_played_this_turn: 0,
                    turn_start_timestamp: 0,
                    tried_empty_draw: false,
                    commander_damage: Vec::new(),
                    loss: None,
                    team: s.team,
                })
                .collect(),
            turn: TurnInfo::new(PlayerId::new(0)),
            combat: crate::combat::CombatState::default(),
            per_turn: PerTurn::new(preset.seats.len()),
            delayed: Vec::new(),
            pending_miracle: std::collections::VecDeque::new(),
            extra_turns: std::collections::VecDeque::new(),
            restriction_info: rustc_hash::FxHashMap::default(),
            next_restriction_id: 1,
            commander_casts: vec![0; preset.seats.len()],
            commander_redirect: Vec::new(),
            pending_copied_faces: Vec::new(),
            ltb_mana_values: Vec::new(),
            ltb_controllers: Vec::new(),
            ltb_powers: Vec::new(),
            ltb_abilities: Vec::new(),
            ltb_counters: Vec::new(),
            ltb_characteristics: Vec::new(),
            ltb_attachments: Vec::new(),
            ceased: Vec::new(),
            reflexive: Vec::new(),
            discovered: Vec::new(),
            divided: Vec::new(),
            synthetic_copies: Vec::new(),
            commanders: vec![Vec::new(); preset.seats.len()],
            monarch: None,
            day_night: None,
            previous_turn: None,
            starting_player: PlayerId::new(0),
            ability_fires: rustc_hash::FxHashMap::default(),
            rng: GameRng::new(preset.seed),
            journal: Journal::default(),
            names: Names::default(),
            bases: Arc::default(),
            timestamp: 0,
            effects: crate::effects::EffectTable::default(),
            replacement_rules: Vec::new(),
            characteristics_generation: u64::MAX,
            projection_ids: Vec::new(),
            projected_cross_zone: false,
            printed_pt_cda: Vec::new(),
            token_cleanup: Vec::new(),
        };
        // Casting probes need the nameless face without mutating this interner.
        let nameless = state.names.intern("");
        debug_assert_eq!(nameless, NAMELESS);
        state.journal.record(GameEvent::GameStarted {
            seed: preset.seed,
            seats,
        });

        for (i, seat) in preset.seats.iter().enumerate() {
            let player = PlayerId::new(i as u8);
            // An `Open` seat is a human chair that no account has claimed yet,
            // not an absent player — every hosted game marks its human seat
            // `Open`, and that seat still needs a library and an opening hand.
            // Only a chair with nothing to set up is genuinely unoccupied,
            // which is the case `GamePreset::validate` allows an empty deck
            // for.
            let unoccupied = seat.deck.is_empty()
                && seat.starting_battlefield.is_empty()
                && seat.starting_hand.is_none()
                && seat.emblems.is_empty()
                && seat.commanders.is_empty();
            if unoccupied {
                continue;
            }
            // Emblems first (they exist from turn 0, CR 114.2).
            for emblem in &seat.emblems {
                let name = state.names.intern(emblem);
                state.create_bare(
                    player,
                    ObjectKind::Emblem,
                    name,
                    ZoneLocation::Command(player),
                );
            }
            // Commanders next (CR 903.6): they begin in the command zone,
            // and are never shuffled into the library — which is why they
            // are their own list on the seat and not a marked deck entry.
            for &entry in &seat.commanders {
                let id = state.create_card(player, entry, lookup)?;
                state
                    .move_object(
                        id,
                        ZoneLocation::Command(player),
                        ZonePosition::Top,
                        Cause::Setup,
                    )
                    .expect("freshly created object");
                state.commanders[i].push(Commander {
                    object: id,
                    casts: 0,
                    answered: 0,
                });
            }
            for (position, &entry) in seat.starting_battlefield.iter().enumerate() {
                let id = state.create_card(player, entry, lookup)?;
                state.object_mut(id).expect("freshly created object").kind = ObjectKind::Permanent;
                state
                    .move_object(
                        id,
                        ZoneLocation::Battlefield,
                        ZonePosition::Top,
                        Cause::Setup,
                    )
                    .expect("freshly created object");
                for placement in &preset.house_rules.starting_counters {
                    if placement.seat == i && placement.permanent == position {
                        for counter in &placement.counters {
                            if let Some(kind) = CounterKind::from_setup_name(&counter.kind) {
                                state
                                    .object_mut(id)
                                    .expect("freshly created object")
                                    .counters
                                    .add(kind, counter.amount);
                            }
                        }
                    }
                }
            }
            for &entry in &seat.deck {
                let id = state.create_card(player, entry, lookup)?;
                state
                    .move_object(
                        id,
                        ZoneLocation::Library(player),
                        ZonePosition::Top,
                        Cause::Setup,
                    )
                    .expect("freshly created object");
            }
            state.shuffle_library(player);
            // The sideboard is created but never shuffled in. These cards are
            // outside the game (CR 400.11a) until a wish reaches them; folding
            // them into the library would silently make every deck bigger
            // than the one the player registered.
            for &entry in &seat.sideboard {
                let id = state.create_card(player, entry, lookup)?;
                state
                    .move_object(
                        id,
                        ZoneLocation::OutsideGame(player),
                        ZonePosition::Top,
                        Cause::Setup,
                    )
                    .expect("freshly created object");
            }
            match &seat.starting_hand {
                Some(hand) => {
                    for &entry in hand {
                        let id = state.create_card(player, entry, lookup)?;
                        state
                            .move_object(
                                id,
                                ZoneLocation::Hand(player),
                                ZonePosition::Top,
                                Cause::Setup,
                            )
                            .expect("freshly created object");
                    }
                }
                None => {
                    state.draw_cards(player, 7);
                }
            }
        }
        Ok(state)
    }

    /// The shared printed face of a card, interned on first use.
    ///
    /// Front face only: an MDFC that turns over goes through
    /// [`GameState::switch_face`], which builds a face of its own.
    fn card_base(&mut self, def: &CardDef, index: CardIndex) -> Arc<Characteristics> {
        if let Some(base) = self.bases.cards.get(&index) {
            return Arc::clone(base);
        }
        let name = self.names.intern(def.name());
        let base = Arc::new(Characteristics::from_face(def, 0, name));
        Arc::make_mut(&mut self.bases)
            .cards
            .insert(index, Arc::clone(&base));
        base
    }

    /// The shared blank face behind every card-less, token-less object:
    /// an ability on the stack, an emblem. A name and nothing else.
    ///
    /// This is the one that carries the million-ability stack: a deck that
    /// puts six figures of triggers up puts up a handful of *distinct*
    /// ones, so they all end up pointing here.
    pub fn bare_base(&mut self, name: NameRef) -> Arc<Characteristics> {
        if let Some(base) = self.bases.bare.get(&name) {
            return Arc::clone(base);
        }
        let base = Arc::new(Characteristics {
            name,
            mana_cost: baylee_core::mana::ManaCost::ZERO,
            colors: baylee_core::color::ColorSet::EMPTY,
            types: baylee_core::types::TypeSet::EMPTY,
            supertypes: baylee_core::types::SupertypeSet::EMPTY,
            subtypes: baylee_core::types::SubtypeSet::EMPTY,
            keywords: baylee_cards_dsl::KeywordSet::EMPTY,
            power: None,
            toughness: None,
            loyalty: None,
            color_identity: baylee_core::color::ColorSet::EMPTY,
            produced_colors: baylee_core::color::ColorSet::EMPTY,
            produced_colorless: false,
            produced_chosen: false,
            abilities_lost: None,
            front_mana_value: None,
        });
        Arc::make_mut(&mut self.bases)
            .bare
            .insert(name, Arc::clone(&base));
        base
    }

    /// The shared printed face of a token, at the size the effect asked for.
    ///
    /// `size` is `None` for the printed size and `Some(x)` when the effect
    /// computed one (Skyclave Apparition's Illusion is "X/X"); the two are
    /// different faces of the same definition and are interned apart.
    pub fn token_base(
        &mut self,
        token: &'static baylee_cards_dsl::TokenDef,
        size: Option<i16>,
    ) -> Arc<Characteristics> {
        // The definition is a `&'static` handed out by the card registry,
        // so its address identifies it. Nothing hashes or orders on this —
        // it is a lookup key inside one process — so a different address in
        // a different run cannot change a game.
        let key = (std::ptr::from_ref(token) as usize, size);
        if let Some(base) = self.bases.tokens.get(&key) {
            return Arc::clone(base);
        }
        let name = self.names.intern(token.name);
        let base = Arc::new(Characteristics {
            name,
            mana_cost: baylee_core::mana::ManaCost::ZERO,
            colors: token.colors,
            types: token.types,
            supertypes: token.supertypes,
            subtypes: baylee_core::types::SubtypeSet::from_slice(token.subtypes),
            keywords: token.keywords,
            power: size.or(token.power),
            toughness: size.or(token.toughness),
            loyalty: None,
            color_identity: baylee_core::color::ColorSet::EMPTY,
            produced_colors: baylee_core::color::ColorSet::EMPTY,
            produced_colorless: false,
            produced_chosen: false,
            abilities_lost: None,
            front_mana_value: None,
        });
        Arc::make_mut(&mut self.bases)
            .tokens
            .insert(key, Arc::clone(&base));
        base
    }

    fn create_card(
        &mut self,
        owner: PlayerId,
        entry: baylee_core::preset::DeckEntry,
        lookup: &impl CardLookup,
    ) -> Result<ObjectId, SetupError> {
        let def = lookup
            .card(entry.card)
            .ok_or(SetupError::UnknownCard(entry.card))?;
        let base = self.card_base(def, entry.card);
        let card = CardRef {
            index: entry.card,
            print: entry.print,
        };
        self.timestamp += 1;
        let ts = self.timestamp;
        let id = self.arena.insert_with(|id| {
            let mut obj = GameObject::new_card(id, owner, card, base);
            obj.timestamp = ts;
            obj
        });
        if let Some(modifier) = printed_pt_cda(def) {
            self.printed_pt_cda.push((id, modifier));
        }
        Ok(id)
    }

    /// Creates a card-less object (tokens, emblems).
    ///
    /// # Panics
    /// On zone insertion failure (internal invariant).
    pub fn create_bare(
        &mut self,
        owner: PlayerId,
        kind: ObjectKind,
        name: NameRef,
        loc: ZoneLocation,
    ) -> ObjectId {
        let base = self.bare_base(name);
        self.timestamp += 1;
        let ts = self.timestamp;
        let id = self.arena.insert_with(|id| {
            let mut obj = GameObject::new_bare(id, owner, kind, base);
            obj.timestamp = ts;
            obj
        });
        self.zones.insert(
            id,
            loc,
            ZonePosition::Top,
            kind != ObjectKind::AbilityOnStack,
        );
        {
            let obj = self.arena.get_mut(id).expect("fresh object");
            obj.zone = loc.zone();
            obj.zone_owner = loc.player();
        }
        // Same rule as `move_object`: a card-less object created straight
        // into a zone that is not the battlefield or the stack is a
        // cleanup candidate. Emblems are card-less and live in the command
        // zone, which is why `sba::run` — not this call — decides.
        if !matches!(loc.zone(), Zone::Battlefield | Zone::Stack) {
            self.watch_token_cleanup(id);
        }
        id
    }

    /// Monotonic timestamp (effects ordering, summoning sickness).
    pub fn next_timestamp(&mut self) -> u64 {
        self.timestamp += 1;
        self.timestamp
    }

    /// CR 302.6: a creature must have been controlled continuously since
    /// its controller's most recent turn began, so *any* control change
    /// makes it summoning-sick again — including the one at end of turn
    /// that hands a stolen creature back.
    pub(crate) fn restart_summoning_sickness(&mut self, id: ObjectId) {
        let ts = self.next_timestamp();
        if let Some(obj) = self.object_mut(id) {
            obj.timestamp = ts;
        }
    }

    /// Turns a permanent over (CR 701.27) and says so in the journal.
    ///
    /// [`Self::switch_face`] is the other half of this and stays silent,
    /// because its callers are not transforms: choosing which face of a
    /// modal card to cast or to play as a land puts a card onto the stack
    /// or the battlefield face up (CR 712.11b for the cast, CR 712.12 for
    /// the land play) — nothing turned over, and a
    /// "transformed" entry there would fire every trigger that watches for
    /// one. Anything that turns an existing permanent over comes here.
    ///
    /// Returns whether the face actually changed, which is what the
    /// daybound/nightbound fixpoint step reads: a permanent already showing
    /// the face it should show is not progress, and reporting it as such
    /// would keep the fixpoint spinning.
    pub fn transform(&mut self, id: ObjectId, def: &CardDef, face: usize) -> bool {
        let face = face.min(def.faces.len() - 1);
        if self
            .object(id)
            .is_none_or(|o| o.face_index as usize == face)
        {
            return false;
        }
        self.switch_face(id, def, face);
        self.journal.record(GameEvent::Transformed {
            object: id,
            face: face as u8,
        });
        true
    }

    /// Switches an object to another face of its card (MDFC cast/land
    /// play, CR 712.11b and CR 712.12): rebuilds base characteristics from
    /// the face and
    /// invalidates the layered projection cache.
    ///
    /// This is the mechanism, not the rules action — see [`Self::transform`]
    /// for the one that journals.
    pub fn switch_face(&mut self, id: ObjectId, def: &CardDef, face: usize) {
        let face = face.min(def.faces.len() - 1);
        let name = self.names.intern(def.faces[face].name);
        let base = crate::object::Characteristics::from_face(def, face, name);
        if let Some(obj) = self.object_mut(id) {
            obj.face_index = face as u8;
            obj.base = Arc::new(base);
            obj.cache.clear();
        }
        // The projection is now stale for this object; the next refresh
        // has to rebuild it even if no effect was added or removed.
        self.characteristics_generation = u64::MAX;
    }

    /// Gives a permanent with a shared type line the unlocked designations
    /// `unlocked` names (bit 0 the left half, bit 1 the right; CR 709.5c),
    /// and the characteristics they leave it: the name, mana cost and rules
    /// text of its unlocked halves only (CR 709.5), and the shared types
    /// either way (CR 709.5a). The rules text is read off the doors
    /// (`CardDef::door_abilities`); this writes the rest.
    ///
    /// One half unlocked is that half's face ([`Self::switch_face`]). Both
    /// is the left face with both halves' mana cost, so both colours and
    /// their sum for a mana value, and both halves' keywords; the name stays
    /// the left half's, because an object here has one name and such a Room
    /// has two (CR 709.4a), and nothing in the pool reads a Room's name.
    /// Neither is the left face with no name, no mana cost, no colour and no
    /// keyword. Keywords are rules text, so each state sets them from the
    /// halves it has and never from the card-level fallback a front face
    /// reads (`CardDef::keywords_for_face`). The printed front is set aside
    /// in `original_base`, which the move off the battlefield restores: off
    /// the battlefield the card is the card again (CR 400.7).
    ///
    /// What the Room's statics and replacement rules did under the old
    /// doors ends here, and the next scan (`Engine::sync_static_effects`)
    /// registers what the new doors print. That scan only ever adds for a
    /// permanent still on the battlefield, and a Room put there uncast was
    /// scanned as its left half before it was given no doors: without this,
    /// Walk-In Closet's static outlived the door that prints it.
    pub fn set_doors(&mut self, id: ObjectId, def: &CardDef, unlocked: u8) {
        let unlocked = unlocked & 0b11;
        self.effects.remove_where(|fx| {
            fx.source == Some(id) && fx.origin == crate::effects::EffectOrigin::Static
        });
        self.replacement_rules.retain(|r| r.source != id);
        let front = {
            let name = self.names.intern(def.faces[0].name);
            Arc::new(crate::object::Characteristics::from_face(def, 0, name))
        };
        self.switch_face(id, def, usize::from(unlocked == 0b10));
        let nameless = self.names.intern("");
        let Some(obj) = self.object_mut(id) else {
            return;
        };
        obj.doors = crate::object::Doors::room(unlocked);
        obj.original_base.get_or_insert(front);
        let keywords = def
            .faces
            .iter()
            .take(2)
            .enumerate()
            .filter(|(half, _)| unlocked & (1 << half) != 0)
            .fold(baylee_cards_dsl::KeywordSet::EMPTY, |all, (_, f)| {
                all.union(f.keywords)
            });
        let c = obj.base_mut();
        c.keywords = keywords;
        match unlocked {
            0b11 => {
                let cost = def.faces[0].mana_cost.combine(&def.faces[1].mana_cost);
                c.mana_cost = cost;
                c.colors = cost
                    .colors()
                    .union(def.faces[0].color_indicator)
                    .union(def.faces[1].color_indicator);
            }
            0 => {
                c.name = nameless;
                c.mana_cost = baylee_core::mana::ManaCost::ZERO;
                c.colors = baylee_core::color::ColorSet::EMPTY;
            }
            _ => {}
        }
    }

    /// Forces the next [`GameState::refresh_characteristics`] to rebuild
    /// every projection, even though the effect table did not change.
    ///
    /// The generation compare that guards the refresh tracks *effects*, and
    /// counters are the other input the projection reads (CR 613.4c for
    /// +1/+1 and -1/-1, CR 122.1b for keyword counters). Changing a counter
    /// on a board where nothing else moved therefore leaves the projection
    /// stale — which is how an amass token, a 0/0 that is only alive because
    /// of the counter placed on it a moment later, used to die to state-based
    /// actions before anything ever recomputed its toughness. Anything that
    /// writes a counter, or puts a new permanent on the battlefield or a new
    /// spell on the stack ([`Self::put_new_spell_on_stack`]), calls this.
    pub const fn invalidate_projections(&mut self) {
        self.characteristics_generation = u64::MAX;
    }

    /// Puts `id`, a spell that has never been in a zone (a copy of a spell,
    /// CR 707.10), on top of the stack, and has the projection take it in.
    ///
    /// A spell on the stack is projected like a permanent (a creature spell
    /// under Maskwood Nexus is every creature type), and `move_object` is
    /// what invalidates for a spell that is cast. A copy is not moved: it is
    /// made there. Without the invalidation the copy answered its copied
    /// printed values until some unrelated change refreshed the board, and
    /// for everything that read it in between — a cast trigger, "target Ally
    /// spell" — the Nexus did not apply to it.
    pub fn put_new_spell_on_stack(&mut self, id: ObjectId) {
        self.zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top, true);
        self.invalidate_projections();
    }

    /// Changes a player's life total by `by`. This is **the** door, for the
    /// same reason [`Self::set_tapped`] is one.
    ///
    /// "If you didn't lose life this turn" (Luminarch Ascension) reads
    /// [`PerTurn::life_lost`], and the flag is only true if every loss sets
    /// it. A loss can be damage (CR 119.2), an effect (CR 119.3) or a
    /// payment (CR 119.4: "in other words, the player loses that much
    /// life"). Eleven sites used to adjust the total and journal the change
    /// themselves, and none of them set the flag (#241). The journal entry
    /// comes through here too, because every one of those sites recorded it
    /// and every life trigger reads it.
    /// `card_tests::rules::nothing_changes_a_life_total_except_the_one_door`
    /// counts the writers.
    ///
    /// A loss for a player who can't lose life (Everybody Lives!) doesn't
    /// happen: no change, no record, no flag. That holds whatever caused
    /// it, which is why the check is here and not at each cause (#244).
    /// Damage is still dealt, because the damage sites record `DamageDealt`
    /// themselves. A payment never gets this far: [`Self::can_pay_life`]
    /// refuses it first (CR 119.8).
    ///
    /// Two decisions stay with the caller: whether a change of nothing is
    /// worth recording, and the `Cause`.
    pub fn change_life(&mut self, player: PlayerId, by: i32, cause: Cause) {
        if by < 0 && self.cant_lose_life(player) {
            return;
        }
        let seat = player.get() as usize;
        let p = &mut self.players[seat];
        let old = p.life;
        p.life += by;
        let new = p.life;
        if new < old {
            self.per_turn.life_lost[seat] = true;
        }
        self.journal.record(GameEvent::LifeChanged {
            player,
            old,
            new,
            cause,
        });
    }

    /// Taps or untaps a permanent — **the** door, and the reason it is one.
    ///
    /// A tap is an input to the layer projection wherever an effect's filter
    /// reads it, and the generation compare in
    /// [`Self::refresh_characteristics`] tracks the *effect table* rather
    /// than the board: writing `Status::TAPPED` on an object therefore
    /// leaves every projection that depends on it stale. Spectral Cloak
    /// ("enchanted creature has shroud as long as it's untapped") kept its
    /// shroud through the tap that was supposed to take it away, and so did
    /// Castle, Giant Tortoise and the six storage lands that read
    /// `Filter::Tapped`.
    ///
    /// Seventeen sites wrote that bit and each of them would have had to
    /// remember — which is the positive list that goes quiet the day an
    /// eighteenth is added. They all call this instead. The journal entry
    /// stays with the caller: not every tap is an `ObjectTapped` event (a
    /// cost-paid tap and a replacement-written one are recorded
    /// differently), and widening this door to the journal would be a
    /// second question wearing the first one's answer.
    ///
    /// Returns whether the status actually changed, which is what an untap
    /// needs in order to record an event only for a permanent that was
    /// tapped.
    pub fn set_tapped(&mut self, id: ObjectId, tapped: bool) -> bool {
        let Some(obj) = self.object_mut(id) else {
            return false;
        };
        let was = obj.status.contains(crate::object::Status::TAPPED);
        if was == tapped {
            return false;
        }
        if tapped {
            obj.status.insert(crate::object::Status::TAPPED);
        } else {
            obj.status.remove(crate::object::Status::TAPPED);
        }
        self.board_state_changed();
        true
    }

    /// Tells the projection that the board moved under it — something tapped,
    /// something began or stopped attacking.
    ///
    /// The gate rather than a bare [`Self::invalidate_projections`]:
    /// `refresh_characteristics` makes the same scan for cross-zone effects,
    /// and a full re-projection per land tap is the cost this refuses. On a
    /// board where no effect reads tap or attack status — nearly every board
    /// — this is one walk of a short table and nothing else.
    pub fn board_state_changed(&mut self) {
        if self
            .effects
            .iter()
            .any(|fx| matches!(fx.filter, crate::effects::EffectFilter::Dsl(f) if filter_reads_board_state(f)))
        {
            self.invalidate_projections();
        }
    }

    /// Whether [`crate::zone::Zones::stack_projectable`] still agrees with
    /// the stack: same objects, abilities excluded.
    ///
    /// Read-only, and only ever called from a `debug_assert!` — it walks
    /// the stack, which is precisely the walk the subset exists to avoid.
    #[must_use]
    pub fn stack_projection_set_is_consistent(&self) -> bool {
        let mut expected: Vec<ObjectId> = self
            .zones
            .list(ZoneLocation::Stack)
            .iter()
            .copied()
            .filter(|id| {
                self.object(*id)
                    .is_some_and(|o| o.kind != ObjectKind::AbilityOnStack)
            })
            .collect();
        let mut actual = self.zones.stack_projectable().to_vec();
        expected.sort_unstable();
        actual.sort_unstable();
        expected == actual
    }

    /// Notes that `id` may now be a token outside the battlefield, so the
    /// next state-based-action pass looks at it (CR 704.5d).
    ///
    /// Deliberately over-inclusive: the caller checks only the two cheap
    /// facts it already has in hand (card-less, destination is neither the
    /// battlefield nor the stack) and [`crate::sba`] applies the real rule.
    /// A false positive costs one arena lookup; a false negative would be
    /// a token that never ceases to exist.
    pub(crate) fn watch_token_cleanup(&mut self, id: ObjectId) {
        self.token_cleanup.push(id);
    }

    /// Takes the pending cleanup candidates, leaving the buffer empty and
    /// allocated for reuse.
    pub(crate) fn take_token_cleanup(&mut self) -> Vec<ObjectId> {
        std::mem::take(&mut self.token_cleanup)
    }

    /// Returns the drained buffer so the next pass allocates nothing.
    pub(crate) fn return_token_cleanup(&mut self, mut buffer: Vec<ObjectId>) {
        if self.token_cleanup.is_empty() {
            buffer.clear();
            self.token_cleanup = buffer;
        }
    }

    /// Hot path: one generation compare. When stale, permanents and stack
    /// objects are re-projected through the layer system (CR 613).
    ///
    /// # Panics
    /// Internal invariant violations (zone objects always exist).
    pub fn refresh_characteristics(&mut self) {
        // Both halves of the stack shortcut below fail silently — an id
        // left in the subset is projected after its object is gone, a spell
        // missing from it quietly stops being affected by anthems — and the
        // sites that fill it are spread over five files. Checked here, on
        // every engine step rather than only when the effect set moved, so
        // that every test on every code path is a test of the invariant.
        debug_assert!(
            self.stack_projection_set_is_consistent(),
            "stack_projectable drifted from the stack"
        );
        if self.characteristics_generation == self.effects.generation {
            return;
        }
        let generation = self.effects.generation;
        // Bucket and dependency-order the effect table ONCE for the whole
        // pass; the ordering does not depend on the object being projected
        // (see `layers::LayerPlan`).
        let plan = crate::layers::LayerPlan::build(&self.effects);
        // Cross-zone effects (Maskwood Nexus & co.) reach into library,
        // hand, graveyard — then every object must be projected, not only
        // battlefield + stack.
        let cross_zone = self
            .effects
            .iter()
            .any(|fx| matches!(fx.filter, crate::effects::EffectFilter::Dsl(f) if filter_reaches_other_zones(f)));
        // Scratch buffers live in the state so a refresh — which runs
        // after every single effect-set change — allocates nothing.
        let mut ids = std::mem::take(&mut self.projection_ids);
        ids.clear();
        // Cached off-board projections must be revisited after the last
        // cross-zone effect disappears (#120).
        //
        // A phased-out permanent is not projected at all: it can't be
        // affected by anything (CR 702.26b), so what it was as it phased out
        // is what it stays, and so is its controller, the player it phases
        // in under (CR 502.1). `phase_in` invalidates, and the next refresh
        // takes it back in.
        if cross_zone || self.projected_cross_zone {
            ids.extend(
                self.arena
                    .iter()
                    .filter(|(_, o)| !o.status.contains(crate::object::Status::PHASED_OUT))
                    .map(|(id, _)| id),
            );
        } else {
            // The stack contributes only its *spells*. An ability on the
            // stack has no characteristic a layer can touch, and this pass
            // runs once per engine step — walking six figures of Ally
            // triggers to establish that, every time a counter moves, is
            // the difference between a long game and no game at all.
            ids.extend(
                self.battlefield_seen()
                    .chain(self.zones.stack_projectable().iter().copied()),
            );
            // And the cards that define their own power and toughness,
            // wherever else they are (CR 604.3).
            ids.extend(self.printed_pt_cda.iter().map(|(id, _)| *id).filter(|id| {
                self.object(*id)
                    .is_some_and(|o| !matches!(o.zone, Zone::Battlefield | Zone::Stack))
            }));
        }
        // Layer 2 decides the controller every later layer reads, of this
        // object and of every other one (CR 613.1b before 613.1c–f), and a
        // static ability's "you" is whoever controls its source now
        // (CR 109.5). A walk projects one object at a time, so what it read
        // of a controller it had not reached yet, or of the object it was
        // projecting, was the last refresh's: a creature just taken was not
        // pumped by its taker's anthem. So: point each static at its
        // source's controller, walk, and walk again while a walk moved a
        // controller. The walk that moves none read exactly the controllers
        // it wrote. That is one walk when no control changed and two when
        // one did; a static that gives control of something (Control Magic)
        // whose source changed hands is a third (CR 613.8a). The bound only
        // stops a dependency loop, which CR 613.8b settles by timestamp and
        // this by stopping where it is.
        let mut was: Vec<(ObjectId, PlayerId)> = Vec::new();
        // Empty, and so unallocated, on every board where nothing counts.
        let mut readers: Vec<ObjectId> = Vec::new();
        let mut settled = false;
        for _ in 0..plan.control_effects() + 2 {
            self.follow_static_sources();
            readers.clear();
            if !self.project_all(&ids, &plan, generation, &mut was, &mut readers) {
                settled = true;
                break;
            }
        }
        debug_assert!(
            !settled || self.statics_follow_their_sources(),
            "a settled refresh left a static ability's controller behind its source's"
        );
        // The same for what an object counts (CR 613.1: the layers in their
        // order, so a count in layer 7 sees every type layer 4 gave). Ashaya
        // counts the lands you control, and an Elf the walk had not reached
        // was still the last refresh's Elf and not yet this one's Forest, so
        // Ashaya came out one short and stayed so. The board is finished
        // now: project what counted again, and again while that moved a
        // count another counter reads. Each pass settles at least one more
        // counter; the bound stops counters that count each other round in
        // a circle.
        for _ in 0..readers.len() {
            if !self.project_readers(&readers, &plan, generation, &mut was) {
                break;
            }
        }
        // CR 302.6 wants control held continuously since the turn began. A
        // permanent a walk moved and a later one moved back never changed
        // hands, so the comparison is with the controller before the first.
        for (id, before) in was {
            if self.object(id).is_some_and(|o| o.controller != before) {
                self.restart_summoning_sickness(id);
            }
        }
        ids.clear();
        self.projection_ids = ids;
        self.projected_cross_zone = cross_zone;
        self.characteristics_generation = generation;
    }

    /// One walk of the refresh: projects `ids` through every layer and
    /// writes each controller, noting in `was` the controller an object had
    /// before a walk first moved it, and in `readers` every object whose
    /// projection counted others (`layers::Projection::read_board`).
    /// Whether any controller moved.
    fn project_all(
        &mut self,
        ids: &[ObjectId],
        plan: &crate::layers::LayerPlan,
        generation: u64,
        was: &mut Vec<(ObjectId, PlayerId)>,
        readers: &mut Vec<ObjectId>,
    ) -> bool {
        let mut moved = false;
        for &id in ids {
            let Some(obj) = self.object(id) else {
                continue;
            };
            if crate::layers::needs_projection(plan, obj) || self.defines_pt_off_battlefield(obj) {
                let projection = crate::layers::recompute_with(self, obj, plan);
                if projection.read_board {
                    readers.push(id);
                }
                let obj = self.object_mut(id).expect("zone object exists");
                moved |= settle_controller(obj, projection.controller, was);
                // `cache` and `base` are disjoint fields, so this is one
                // mutable borrow and one shared borrow of the same object.
                let crate::object::GameObject { cache, base, .. } = obj;
                cache.store(generation, projection.characteristics, base);
            } else {
                // Nothing can change this object's characteristics, so the
                // base is the projection. Dropping the cache is not just
                // cheaper than recomputing it — it is what keeps an
                // untouched board's per-object projection memory at zero.
                let obj = self.object_mut(id).expect("checked above");
                obj.cache.clear();
                // No effects means no layer 2 either: whoever the base
                // says controls it does, which is how a "gain control
                // until end of turn" hands the permanent back.
                let base = obj.base_controller;
                moved |= settle_controller(obj, base, was);
            }
        }
        moved
    }

    /// Projects again the objects a walk found counting others, now that
    /// every one of those others is this refresh's. Whether any of them
    /// came out different, which is what another counter may have read.
    fn project_readers(
        &mut self,
        readers: &[ObjectId],
        plan: &crate::layers::LayerPlan,
        generation: u64,
        was: &mut Vec<(ObjectId, PlayerId)>,
    ) -> bool {
        let mut changed = false;
        for &id in readers {
            let Some(obj) = self.object(id) else {
                continue;
            };
            let projection = crate::layers::recompute_with(self, obj, plan);
            let obj = self.object_mut(id).expect("zone object exists");
            changed |= settle_controller(obj, projection.controller, was);
            changed |= *obj.characteristics() != projection.characteristics;
            let crate::object::GameObject { cache, base, .. } = obj;
            cache.store(generation, projection.characteristics, base);
        }
        changed
    }

    /// Gives each static ability's effect, and each replacement rule, the
    /// controller its source has now (CR 109.5).
    ///
    /// Only for a source on the battlefield. One that has left keeps the
    /// controller it last had there until `sync_static_effects` drops what
    /// it registered: its last-known information, so that a rule consulted
    /// after its source moved in the middle of one event does not change
    /// sides to the owner.
    fn follow_static_sources(&mut self) {
        let Self {
            effects,
            replacement_rules,
            arena,
            ..
        } = self;
        let now = |source: ObjectId| {
            arena
                .get(source)
                .filter(|o| o.zone == Zone::Battlefield)
                .map(|o| o.controller)
        };
        effects.follow_sources(now);
        for entry in replacement_rules.iter_mut() {
            if let Some(controller) = now(entry.source) {
                entry.controller = controller;
            }
        }
    }

    /// Whether every static ability's effect and replacement rule names the
    /// player who controls its source on the battlefield now: what a
    /// settled refresh leaves behind.
    fn statics_follow_their_sources(&self) -> bool {
        let now = |source: ObjectId| {
            self.object(source)
                .filter(|o| o.zone == Zone::Battlefield)
                .map(|o| o.controller)
        };
        self.effects.iter().all(|fx| {
            fx.origin != crate::effects::EffectOrigin::Static
                || fx.source.and_then(now).is_none_or(|c| c == fx.controller)
        }) && self
            .replacement_rules
            .iter()
            .all(|entry| now(entry.source).is_none_or(|c| c == entry.controller))
    }

    /// Whether `player` has left the game (CR 800.4a). Nothing is created
    /// for them any more, and nothing they control triggers (CR 800.4d).
    #[must_use]
    pub fn has_left(&self, player: PlayerId) -> bool {
        self.players[usize::from(player.get())].has_lost()
    }

    /// Object access.
    #[must_use]
    pub fn object(&self, id: ObjectId) -> Option<&GameObject> {
        self.arena.get(id)
    }

    /// The object an event was about, even if it has ceased to exist.
    ///
    /// The narrow door onto [`Self::ceased`], and it is deliberately not
    /// [`Self::object`]: an object that has ceased to exist is not in the
    /// game, and every rule that asks "is this here?" has to keep getting no
    /// for an answer. The callers are the ones asking a different question —
    /// "what was this, at the moment of the event?" — which is the trigger
    /// scan and nothing else (CR 111.7, CR 603.10a).
    #[must_use]
    pub fn object_or_departed(&self, id: ObjectId) -> Option<&GameObject> {
        self.arena
            .get(id)
            .or_else(|| self.ceased.iter().find(|o| o.id == id))
    }

    /// The characteristic-defining P/T `obj` applies itself, which it does
    /// wherever it is but on the battlefield ([`Self::printed_pt_cda`]).
    #[must_use]
    pub fn off_battlefield_pt_cda(&self, obj: &GameObject) -> Option<baylee_cards_dsl::Modifier> {
        if obj.zone == Zone::Battlefield {
            return None;
        }
        self.printed_pt_cda
            .iter()
            .find(|(card, _)| *card == obj.id)
            .map(|(_, modifier)| *modifier)
    }

    fn defines_pt_off_battlefield(&self, obj: &GameObject) -> bool {
        self.off_battlefield_pt_cda(obj).is_some()
    }

    /// What `id` was as it last left the battlefield, if that is the last
    /// move it made ([`Self::ltb_characteristics`]).
    #[must_use]
    pub fn last_known_characteristics(
        &self,
        id: ObjectId,
    ) -> Option<&crate::object::Characteristics> {
        self.ltb_characteristics
            .iter()
            .find(|(other, _)| *other == id)
            .map(|(_, was)| was)
    }

    /// The game becomes day, or day becomes night's opposite (CR 730.1).
    ///
    /// Every route to the designation goes through this door and
    /// [`Self::become_night`], so that "it becomes day" is journalled once
    /// and in one place — CR 730.1a's "night becomes day" is the same
    /// event, the game losing one designation and gaining the other, and a
    /// card that triggers on it has one entry to read. Setting the field
    /// directly would record nothing.
    pub fn become_day(&mut self) {
        self.set_designation(DayNight::Day);
    }

    /// The game becomes night (CR 730.1). See [`Self::become_day`].
    pub fn become_night(&mut self) {
        self.set_designation(DayNight::Night);
    }

    fn set_designation(&mut self, now: DayNight) {
        if self.day_night == Some(now) {
            return;
        }
        self.day_night = Some(now);
        self.journal.record(GameEvent::DayNightChanged { now });
    }

    /// Sets the monarch (CR 724.3), and ends every exile that lasted "until
    /// an opponent becomes the monarch" (Palace Jailer) for which the new
    /// monarch is such an opponent.
    ///
    /// "An opponent" of the player who controlled the exiling ability, which
    /// the exile wrote down: not of whoever controls the Jailer now, and not
    /// of anybody at all when the Jailer is gone. Every other linked exile
    /// is left alone. Skyclave Apparition's has no end, and Safe Haven's
    /// ends when Safe Haven says so.
    pub fn set_monarch(&mut self, player: PlayerId) {
        let previous = self.monarch;
        self.monarch = Some(player);
        if previous == Some(player) {
            return;
        }
        self.return_linked(|state, _, until| {
            matches!(
                until,
                Some(crate::object::LinkUntil::OpponentBecomesMonarch { of })
                    if state.is_opponent(player, of)
            )
        });
    }

    /// `player` has left the game (CR 800.4a) and, if they were the monarch,
    /// the designation passes on at the same time (CR 724.4): to the active
    /// player, or, when the active player is the one leaving, to the next
    /// player in turn order still in the game. With nobody left the game goes
    /// on with no monarch.
    ///
    /// `sba::eliminate_player` calls this once `player` is marked as having
    /// left, so the leaver is never the heir. Through [`Self::set_monarch`],
    /// so an exile that waited for an opponent to become the monarch (Palace
    /// Jailer) ends if the heir is such an opponent. The rule's "if there is
    /// no active player" never arises here, because the engine always has one
    /// (`TurnInfo::active`), and nothing in the engine keeps a player still
    /// in the game from becoming the monarch.
    pub(crate) fn monarch_leaves(&mut self, player: PlayerId) {
        if self.monarch != Some(player) {
            return;
        }
        let seats = self.players.len();
        let active = usize::from(self.turn.active.get());
        let heir = (0..seats)
            .map(|offset| PlayerId::new(((active + offset) % seats) as u8))
            .find(|&p| !self.has_left(p));
        match heir {
            Some(heir) => self.set_monarch(heir),
            None => self.monarch = None,
        }
    }

    /// Returns every exiled card whose link `ends` says has ended to the
    /// battlefield, under its owner's control, and forgets the link.
    ///
    /// The one way back for a card exiled with a link, whatever ended it:
    /// an effect of the host (`Effect::ReturnLinkedToBattlefield`), a new
    /// monarch ([`Self::set_monarch`]), or the host leaving the battlefield
    /// ([`Self::return_what_departed_hosts_held`]). `ends` is asked with the
    /// host and the link's `until`.
    ///
    /// Under its owner's control because every sentence that reaches here
    /// says so or says nothing (CR 610.3c), and written where it arrives: the
    /// default the card last had on the battlefield is whoever put it there,
    /// which after a reanimation or a blink "under your control" may not be
    /// its owner.
    pub(crate) fn return_linked(
        &mut self,
        ends: impl Fn(&Self, ObjectId, Option<crate::object::LinkUntil>) -> bool,
    ) {
        let mut returning = Vec::new();
        for seat in 0..self.players.len() {
            let p = PlayerId::new(seat as u8);
            for &card in self.zones.list(ZoneLocation::Exile(p)) {
                let ended = self.object(card).and_then(|o| {
                    o.riders.iter().find_map(|r| match *r {
                        crate::object::Rider::Linked { host, until } if ends(self, host, until) => {
                            Some(host)
                        }
                        _ => None,
                    })
                });
                if let Some(host) = ended {
                    returning.push((card, host));
                }
            }
        }
        for (card, host) in returning {
            if let Some(obj) = self.object_mut(card) {
                obj.kind = crate::object::ObjectKind::Permanent;
                obj.riders.retain(
                    |r| !matches!(r, crate::object::Rider::Linked { host: h, .. } if *h == host),
                );
                obj.set_controller(obj.owner);
            }
            let _ = self.move_object(
                card,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            );
        }
    }

    /// Returns what was exiled "until this creature leaves the battlefield"
    /// by a host that is no longer on it (CR 610.3).
    ///
    /// The return is the second one-shot effect CR 610.3 creates
    /// "immediately after the specified event", and not a triggered ability,
    /// so it is done at the event itself: [`Self::move_object`] calls this
    /// as a permanent leaves the battlefield, before anything else can
    /// happen (a state-based action, a trigger, the rest of the resolution
    /// that moved it), and `sba::eliminate_player` calls it once what the
    /// departed player owned has left the game with them (CR 800.4a). Those
    /// are the only two ways off the battlefield, which is what makes asking
    /// "is the host still there" enough: a host that was blinked is asked
    /// in the moment it is in exile, before it comes back as a new object.
    pub(crate) fn return_what_departed_hosts_held(&mut self) {
        self.return_linked(|state, host, until| {
            until == Some(crate::object::LinkUntil::HostLeaves)
                && state
                    .object(host)
                    .is_none_or(|h| h.zone != Zone::Battlefield)
        });
    }

    /// The battlefield as rules see it: phased-out permanents are treated
    /// as though they don't exist (CR 702.26b).
    #[must_use]
    pub fn battlefield_view(&self) -> Vec<ObjectId> {
        self.battlefield_seen().collect()
    }

    /// [`Self::battlefield_view`] without the allocation, for a walk that
    /// only counts or filters.
    pub fn battlefield_seen(&self) -> impl Iterator<Item = ObjectId> + '_ {
        // phasing: the one walk that filters phased-out permanents for
        // everybody else.
        self.zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|id| {
                self.object(*id)
                    .is_some_and(|o| !o.status.contains(crate::object::Status::PHASED_OUT))
            })
    }

    /// Mutable object access.
    #[must_use]
    pub fn object_mut(&mut self, id: ObjectId) -> Option<&mut GameObject> {
        self.arena.get_mut(id)
    }

    fn flashback_destination(&self, id: ObjectId, from: Zone, to: ZoneLocation) -> ZoneLocation {
        if from == Zone::Stack
            && to.zone() != Zone::Stack
            && let Some(obj) = self.object(id)
            && obj.riders.contains(&crate::object::Rider::Flashback)
        {
            ZoneLocation::Exile(obj.owner)
        } else {
            to
        }
    }

    /// Writes down what `id` was, one statement before the move erases it.
    ///
    /// The three look-back stores are one job and are done in one place
    /// because they share the moment: a leaves-the-battlefield or dies
    /// trigger fires from the zone the permanent has *arrived* in, and the
    /// game looks back at what was true "immediately prior to the event"
    /// (CR 603.10a). That is here — before the block at the bottom of
    /// [`move_object`] gives the copy back, empties `counters` and clears
    /// every other field only a permanent can have, and before the
    /// state-based actions that would unattach the Equipment.
    ///
    /// Each store is cleared for this object on **every** move and written
    /// again only on a departure from the battlefield, so each describes the
    /// last such departure and no earlier one.
    fn record_last_known(&mut self, id: ObjectId, from_zone: Zone) {
        self.ltb_mana_values.retain(|(other, _)| *other != id);
        self.ltb_controllers.retain(|(other, _)| *other != id);
        self.ltb_powers.retain(|(other, _)| *other != id);
        if from_zone == Zone::Battlefield
            && let Some(object) = self.object(id)
        {
            let controller = object.controller;
            let characteristics = object.characteristics();
            let (mana_value, power) = (characteristics.mana_value(), characteristics.power);
            self.ltb_mana_values.push((id, mana_value));
            if let Some(power) = power {
                self.ltb_powers.push((id, power));
            }
            self.ltb_controllers.push((id, controller));
        }
        if from_zone == Zone::Battlefield {
            let power = self
                .object(id)
                .and_then(|o| o.characteristics().power)
                .unwrap_or(0);
            let stack = self.zones.list(ZoneLocation::Stack).clone();
            for waiting in stack {
                if let Some(obj) = self.object_mut(waiting)
                    && obj.ability.is_some_and(|loc| loc.source == id)
                    && obj.source_power_lki.is_none()
                {
                    obj.source_power_lki = Some(power);
                }
            }
        }
        // What the object could *do*.
        let departing = self.object(id).and_then(|o| {
            o.own_abilities.map(|abilities| crate::object::AbilityList {
                abilities,
                printed: o.own_face,
            })
        });
        self.ltb_abilities.retain(|(other, _)| *other != id);
        if from_zone == Zone::Battlefield
            && let Some(abilities) = departing
        {
            self.ltb_abilities.push((id, abilities));
        }
        // What it wore. Undying and persist ask this of a creature that is
        // already in the graveyard, where the answer no longer exists.
        let departing_counters = self.object(id).map(|o| o.counters.clone());
        self.ltb_counters.retain(|(other, _)| *other != id);
        if from_zone == Zone::Battlefield
            && let Some(counters) = departing_counters
            && !counters.is_empty()
        {
            self.ltb_counters.push((id, counters));
        }
        // What it was.
        self.ltb_characteristics.retain(|(other, _)| *other != id);
        if from_zone == Zone::Battlefield
            && let Some(was) = self.object(id).map(|o| o.characteristics().clone())
        {
            self.ltb_characteristics.push((id, was));
        }
        // What was attached to it.
        self.ltb_attachments.retain(|(other, _)| *other != id);
        if from_zone == Zone::Battlefield {
            // A phased-out Aura was attached to nothing the rules can
            // see (CR 702.26b), so nothing looks back at it.
            let worn: Vec<ObjectId> = self
                .battlefield_seen()
                .filter(|other| {
                    self.object(*other)
                        .is_some_and(|o| o.attached_to == Some(id))
                })
                .collect();
            if !worn.is_empty() {
                self.ltb_attachments.push((id, worn));
            }
        }
    }

    /// Who controlled `id`, the way CR 608.2h reads an object an effect
    /// needs information from: the controller it had as it last existed on
    /// the battlefield if that is where it last left, and its controller now
    /// otherwise. `None` for an object that is gone and left no record.
    ///
    /// "Exile target creature. Its controller creates …" (Crib Swap) and
    /// "Exile target creature. Its controller gains life …" (Swords to
    /// Plowshares) read the second sentence after the first has moved the
    /// creature, and the field on the exiled card is no answer: nothing
    /// controls a card in exile, the field holds whatever the last refresh
    /// left there, and a refresh that reaches every zone (any effect whose
    /// filter names another zone) settles it to the card's default. After a
    /// steal that is the player it was stolen from.
    ///
    /// [`Self::ltb_controllers`] is cleared at every move and written only
    /// by a departure from the battlefield, so an entry is always the last
    /// word: a creature still on the battlefield, or a spell on the stack,
    /// has none and answers with the controller it has now.
    #[must_use]
    pub fn last_known_controller(&self, id: ObjectId) -> Option<PlayerId> {
        self.ltb_controllers
            .iter()
            .find(|(object, _)| *object == id)
            .map(|(_, seat)| *seat)
            .or_else(|| self.object(id).map(|o| o.controller))
    }

    /// Whether a static permission makes this player's library top public.
    #[must_use]
    pub fn library_top_revealed(&self, player: PlayerId) -> bool {
        self.effects.iter().any(|fx| {
            fx.controller == player
                && matches!(fx.modifier, baylee_cards_dsl::Modifier::RevealLibraryTop)
                && fx.source.is_none_or(|source| {
                    self.object(source)
                        .is_some_and(|o| o.zone == Zone::Battlefield)
                })
        })
    }

    /// Moves an object between zones (CR 400.7: `version` bumps — it
    /// becomes a new object for rules that track identity).
    ///
    /// # Errors
    /// [`StateError::NoSuchObject`] for stale or unknown handles.
    ///
    /// # Panics
    /// Internal invariant violations (existence is checked above).
    #[allow(clippy::too_many_lines)] // one reset per field CR 400.7 clears
    pub fn move_object(
        &mut self,
        id: ObjectId,
        to: ZoneLocation,
        pos: ZonePosition,
        cause: Cause,
    ) -> Result<ObjectId, StateError> {
        let (from_zone, from_player) = {
            let obj = self.object(id).ok_or(StateError::NoSuchObject(id))?;
            (obj.zone, obj.zone_owner.unwrap_or(obj.owner))
        };
        // CR 903.9b, applied here because here is the only place it can be
        // applied: the card must never reach the hand or the library it was
        // headed for. The answer was taken before the effect ran; all that
        // is left is to spend it.
        let to = self.flashback_destination(id, from_zone, to);
        let to = self.take_commander_redirect(id, to);
        let (to, exile_counter) = crate::replacement::graveyard_destination(self, id, to);
        let from_loc = ZoneLocation::of(from_zone, from_player);
        // Record each exposed card before it leaves, including each draw of
        // a multi-card draw. Only the top was public, never the whole library.
        if from_zone == Zone::Library
            && self.library_top_revealed(from_player)
            && self.zones.list(from_loc).last() == Some(&id)
        {
            self.journal.record(GameEvent::Revealed {
                player: from_player,
                cards: vec![id],
            });
        }
        self.zones.remove(id, from_loc);
        self.timestamp += 1;
        let ts = self.timestamp;
        self.record_last_known(id, from_zone);
        // CR 400.7 for the turn's per-ability tally: what the old object
        // used this turn is not the new object's. An id is stable for the
        // whole game and only `version` moves, so the tally keyed by id kept
        // counting across a blink — Omnath returned by Ephemerate took its
        // second landfall for the second time this turn, not the first.
        self.ability_fires.retain(|(object, _), _| *object != id);
        {
            let obj = self.object_mut(id).expect("checked above");
            obj.zone = to.zone();
            obj.zone_owner = to.player();
            obj.timestamp = ts;
            obj.version = obj.version.wrapping_add(1);
            // CR 400.7: it becomes a new object. The old projection must
            // not survive the move — the refresh pass only revisits the
            // battlefield and the stack, so a creature that died under an
            // anthem would otherwise sit in the graveyard still pumped,
            // and every filter reading its power would agree.
            obj.cache.clear();
            // And the rest of what the old object remembered, for the half
            // of it that only a *permanent* can have. Nothing cleared this,
            // so a permanent that left the battlefield carried its tapped
            // bit, its marked damage, its counters and what it was attached
            // to into the next zone and back out again: Ephemerate on a
            // tapped land returned it tapped, and a creature blinked with
            // three +1/+1 counters came back with them.
            //
            // `from_zone` and not "entering the battlefield", because these
            // fields are written *before* the arrival in more than one
            // place — `SearchDest::Battlefield` taps the land it fetched
            // and then moves it, so a reset on the way in would undo the
            // half of "put it onto the battlefield tapped" that does the
            // work.
            //
            // What stays: `riders` (a card exiled from the battlefield is
            // linked to whatever exiled it, which is the exception this
            // rule is written around, until it leaves exile: below, with
            // everything else it was in exile), and the spell-shaped fields
            // (`x_value`, `kicked`, `targets`), which a permanent resolving
            // off the stack still needs and which no permanent writes.
            if from_zone == Zone::Battlefield {
                obj.status = crate::object::Status::NONE;
                obj.damage = 0;
                obj.deathtouched = false;
                obj.regeneration_shields = 0;
                obj.attached_to = None;
                // A name chosen as it entered belongs to that permanent
                // (CR 400.7): a Pithing Needle bounced and cast again names
                // again, and nothing in between names anything.
                obj.chosen_name = None;
                // A Room's designations are the permanent's (CR 709.5c), and
                // its face was the half they left showing. The card that
                // arrives is the card, left half first, and `original_base`
                // below brings its printed characteristics back.
                if obj.doors.is_room() {
                    obj.face_index = 0;
                }
                obj.doors = crate::object::Doors::NONE;
            }
            if matches!(from_zone, Zone::Battlefield | Zone::Exile) {
                obj.counters = crate::object::Counters::default();
            }
            // How a spell was cast belongs to the spell and to the permanent
            // it becomes, and to no later object (CR 400.7): a dashed
            // creature blinked or bounced and put back has had no dash cost
            // paid for it, and one that escaped and was blinked did not
            // escape. The cast writes the rider before the card moves to the
            // stack, so that move keeps it too.
            if to.zone() != Zone::Stack
                && !(from_zone == Zone::Stack && to.zone() == Zone::Battlefield)
            {
                obj.riders.retain(|r| {
                    !matches!(
                        r,
                        crate::object::Rider::Dashed | crate::object::Rider::Escaped
                    )
                });
            }
            // What the card was in exile lasts only as long as the exile. A
            // card that leaves exile any other way than the return its host
            // makes (cast, put into a hand, shuffled away) is a new object
            // with no relation to the exile it left: not "exiled with" its
            // host, not on an adventure, not suspended, not castable from
            // exile by anybody (`Rider::ends_as_it_leaves_exile`). The link
            // was kept, so a card cast out of Safe Haven's exile and later
            // hit by Swords to Plowshares came back when Safe Haven was
            // sacrificed. Not on a move from exile to exile, which is the one
            // move that is no leaving; `ExileLinked` and the other writers
            // put the rider on before they move the card.
            if from_zone == Zone::Exile && to.zone() != Zone::Exile {
                obj.riders.retain(|r| !r.ends_as_it_leaves_exile());
            }
            // What was paid is the spell's and no later object's (a flashback
            // is a new payment); nothing on the battlefield reads it yet, so
            // a resolved permanent spell gives it up too (`PaidRecord`).
            obj.paid = None;
            // The same rule for the other thing a copy replaced. Copiable
            // values are fixed while the copy exists (CR 707.2a) and the
            // new object has none of them: the card in the graveyard is
            // the card that was printed. Both halves go back together,
            // because a copy took both — `base` for the characteristics
            // and `own_abilities` for the rules text. A Glasspool Mimic
            // that copied a Wizard and then died is the case: left as a
            // copy it would be recast as one, and the enters-as-a-copy
            // ability it needs to be anything at all would be gone.
            //
            // Only for a card-backed object, because for every other kind
            // `own_abilities` *is* the object — an emblem (CR 114.2) and a
            // triggered ability on the stack (CR 113.7a, which is why it
            // captured its list) have no card to fall back to.
            if obj.card.is_some()
                && !((obj.prototyped || obj.status.contains(crate::object::Status::FACE_DOWN))
                    && from_zone == Zone::Stack
                    && to.zone() == Zone::Battlefield)
            {
                obj.prototyped = false;
                obj.status.remove(crate::object::Status::FACE_DOWN);
                if let Some(original) = obj.original_base.take() {
                    obj.base = original;
                }
                obj.drop_own_abilities();
                // And the flag that says when the field it just cleared was
                // due back, because it describes that copy and the copy ends
                // here. A Cursed Mirror that bounced and was recast without
                // copying anything would otherwise be swept at the next
                // cleanup as though it were still one — harmless for a turn
                // and a lie in the meantime.
                obj.own_abilities_until_eot = false;
            }
        }
        let projectable = self
            .object(id)
            .is_none_or(|o| o.kind != ObjectKind::AbilityOnStack);
        self.zones.insert(id, to, pos, projectable);
        // A card-less object that lands anywhere but the battlefield or the
        // stack is a cleanup candidate (CR 704.5d), and so is a copy of a
        // spell, which carries a card and would otherwise be invisible here
        // (CR 704.5e). The stack is excluded because a copy of a spell
        // legitimately lives there and must be allowed to resolve — the bug
        // this rule already caused once, recorded in `sba::run`.
        if !matches!(to.zone(), Zone::Battlefield | Zone::Stack)
            && self.object(id).is_some_and(|o| {
                o.card.is_none() || o.riders.contains(&crate::object::Rider::SpellCopy)
            })
        {
            self.watch_token_cleanup(id);
        }
        // CR 506.4: a permanent that leaves the battlefield is removed from
        // combat, and this is the one place every departure passes through —
        // a death, a bounce, an exile, and the exile-and-return that is a
        // blink. The id is no help on its own: it is an arena handle and the
        // same one comes back, so a creature that was blinked mid-combat was
        // still declared as an attacker, still counted as blocked, and still
        // traded damage with a blocker it had never met.
        if from_zone == crate::zone::Zone::Battlefield {
            self.combat.remove_from_combat(id);
        }
        // Creature deaths this turn (Emeritus of Woe's re-prepare).
        if from_zone == crate::zone::Zone::Battlefield && to.zone() == crate::zone::Zone::Graveyard
        {
            let is_creature = self.object(id).is_some_and(|o| {
                o.characteristics()
                    .types
                    .contains(baylee_core::types::TypeSet::CREATURE)
            });
            if is_creature {
                self.per_turn.creatures_died = self.per_turn.creatures_died.saturating_add(1);
            }
        }
        // The projected set just changed, and the generation compare that
        // guards a refresh counts *effects* — so nothing would have
        // recomputed this object, or the ones that count it.
        //
        // Both directions, and both zones. An arrival keeps its cleared
        // cache, which reads as the printed card: a creature cast into a
        // board that already had an anthem on it stood there at its printed
        // power, and — the way this was found — a creature cast after a
        // Toxic Deluge resolved was the one creature on the battlefield the
        // Deluge did not shrink. A departure is the other half, because a
        // projection may count the board: `Modifier::ModifyPTPerCount` is
        // "+1/+1 for each artifact you control", so a permanent leaving
        // changes what a permanent that stayed projects to.
        //
        // The battlefield and the stack, which is exactly what the refresh
        // pass revisits. And the graveyards, because a permanent's
        // projection may count them: Pyrogoyf is as big as the card types
        // among cards in all graveyards (`PtCount::CardTypesInAllGraveyards`),
        // so a card milled or discarded grows a permanent that never moved.
        // And exile, for the same reason one zone over: Unlicensed Hearse is
        // as big as the cards exiled with it (`PtCount::ExiledWithThis`), and
        // one of them leaving exile shrinks it.
        //
        // And every zone at all while a cross-zone effect is registered
        // (Maskwood Nexus: "Creatures you control are every creature type.
        // The same is true for creature spells you control and creature
        // cards you own that aren't on the battlefield."). The pass that
        // honours one projects every object, but it runs only when the
        // generation moved, and a card drawn, tutored, wished for or put
        // back moves none: the move above cleared its cache, so a creature
        // card drawn under a Nexus read as its printed self in hand until
        // something else moved. `projected_cross_zone` says whether the last
        // pass was such a pass.
        if matches!(
            from_zone,
            Zone::Battlefield | Zone::Stack | Zone::Graveyard | Zone::Exile
        ) || matches!(
            to.zone(),
            Zone::Battlefield | Zone::Stack | Zone::Graveyard | Zone::Exile
        ) || self.projected_cross_zone
        {
            self.invalidate_projections();
        }
        // A card that defines its own power and toughness is projected in
        // every zone (CR 604.3), and the move just cleared its cache: a
        // drawn Ashaya would read its printed 0/0 until something else
        // moved.
        if self.printed_pt_cda.iter().any(|(card, _)| *card == id) {
            self.invalidate_projections();
        }
        if to.zone() == Zone::Battlefield {
            self.per_turn.entered_battlefield.push(id);
        }
        // "For as long as you control [this]" ends as its source leaves
        // (CR 611.2b), here rather than at the next pass over the effect
        // table: a blink is back before any pass runs, and what returns is a
        // new object (CR 400.7) that the effect never named.
        if from_zone == Zone::Battlefield {
            self.effects.remove_where(|fx| {
                matches!(
                    fx.duration,
                    baylee_cards_dsl::Duration::WhileYouControlSource
                ) && fx.source == Some(id)
            });
        }
        if to.zone() == Zone::Graveyard {
            self.per_turn.entered_graveyard.push(id);
        }
        // Read off `to` after the redirects above, so a commander that went
        // to the command zone instead names no place in a library.
        let place = match to {
            ZoneLocation::Library(_) => Some(library_place(pos, self.zones.list(to).len())),
            _ => None,
        };
        self.journal.record(GameEvent::ZoneChanged {
            object: id,
            from: from_zone,
            to: to.zone(),
            cause,
            place,
        });
        if let Some(kind) = exile_counter {
            crate::replacement::put_counters(self, id, kind, 1);
        }
        // What this permanent held "until it leaves the battlefield" comes
        // back now, immediately after the event and before anything else
        // (CR 610.3).
        if from_zone == Zone::Battlefield {
            self.return_what_departed_hosts_held();
        }
        Ok(id)
    }

    /// Whether CR 903.9b has anything to say about this move: is the object
    /// somebody's commander, and is it on its way to a hand or a library?
    ///
    /// Being a commander is [`Commander::object`], not a characteristic: the
    /// id survives the zone changes that make the card a new object
    /// (CR 400.7), which is the whole reason the marker is an id.
    #[must_use]
    pub fn commander_owner(&self, id: ObjectId, to: ZoneLocation) -> Option<PlayerId> {
        if !matches!(to, ZoneLocation::Hand(_) | ZoneLocation::Library(_)) {
            return None;
        }
        // Nothing here excludes a move from a library back into the same
        // library, because nothing has to. CR 400.7 makes that no zone
        // change at all, and it is the *callers* that enforce it: scry,
        // dig and every other library-internal reorder move their cards
        // without going near `ask_commander_replace`. A guard here would
        // have read as load-bearing while no call site could reach it.
        self.commanders
            .iter()
            .enumerate()
            .find(|(_, list)| list.iter().any(|c| c.object == id))
            .and_then(|(seat, _)| u8::try_from(seat).ok())
            .map(PlayerId::new)
    }

    /// Consumes the answer recorded for `id` and says where it really goes.
    ///
    /// A missing entry means nobody was asked — every non-commander move,
    /// and the sites listed in `docs/engine-internals.md` that this rule
    /// does not reach yet — so the destination stands unchanged.
    fn take_commander_redirect(&mut self, id: ObjectId, to: ZoneLocation) -> ZoneLocation {
        let Some(at) = self.commander_redirect.iter().position(|(o, _)| *o == id) else {
            return to;
        };
        let (_, home) = self.commander_redirect.remove(at);
        if !home {
            return to;
        }
        // The *owner's* command zone, for CR 903.9a's reason: a commander
        // stolen and then bounced goes home to whoever brought it.
        let Some(obj) = self.object_mut(id) else {
            return to;
        };
        let owner = obj.owner;
        // It stops being a permanent on the way, exactly as it would have
        // stopped being one on the way to a hand.
        obj.kind = ObjectKind::Card;
        ZoneLocation::Command(owner)
    }

    /// Shuffles a player's library (journaled).
    pub fn shuffle_library(&mut self, player: PlayerId) {
        let loc = ZoneLocation::Library(player);
        let rng = &mut self.rng;
        rng.shuffle(self.zones.list_mut(loc).as_mut_slice());
        self.journal.record(GameEvent::Shuffled {
            player,
            zone: Zone::Library,
        });
    }

    /// Shuffles a player's library with a stream other than the table's: a
    /// mulligan's, which is the seat's own ([`GameRng::for_seat`]).
    pub fn shuffle_library_with(&mut self, player: PlayerId, rng: &mut GameRng) {
        rng.shuffle(
            self.zones
                .list_mut(ZoneLocation::Library(player))
                .as_mut_slice(),
        );
        self.journal.record(GameEvent::Shuffled {
            player,
            zone: Zone::Library,
        });
    }

    /// Moves the top `n` cards of a player's library to their hand.
    /// Drawing from an empty library flags [`Player::tried_empty_draw`] —
    /// the loss is a state-based action (CR 704.5b).
    pub fn draw_cards(&mut self, player: PlayerId, n: usize) -> Vec<ObjectId> {
        let mut drawn = Vec::with_capacity(n);
        let first_of_turn = self
            .per_turn
            .draws
            .get(player.get() as usize)
            .copied()
            .unwrap_or(0)
            == 0;
        let already = self
            .per_turn
            .draws
            .get(player.get() as usize)
            .copied()
            .unwrap_or(0);
        let limit = self.draw_limit(player);
        for _ in 0..n {
            // CR 121.2b, and the loop is where it belongs: the effect
            // "applies to individual card draws", so "draw three cards"
            // under a limit of one is partially carried out rather than
            // refused. `already` is read once because `per_turn.draws` is
            // not written until the loop is over.
            if limit.is_some_and(|cap| already + drawn.len() as u32 >= cap) {
                break;
            }
            let Some(&top) = self.zones.list(ZoneLocation::Library(player)).last() else {
                if let Some(p) = self.players.get_mut(player.get() as usize) {
                    p.tried_empty_draw = true;
                }
                break;
            };
            if self
                .move_object(
                    top,
                    ZoneLocation::Hand(player),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .is_ok()
            {
                drawn.push(top);
                if let Some(version) = self.object(top).map(|o| o.version) {
                    self.per_turn.drawn.push((top, version));
                }
            }
        }
        // Miracle (CR 702.94): the first card drawn this turn may be
        // revealed and cast for its miracle cost — the engine offers it.
        if first_of_turn && let Some(&card) = drawn.first() {
            self.pending_miracle.push_back((player, card));
        }
        if !drawn.is_empty() {
            if let Some(v) = self.per_turn.draws.get_mut(player.get() as usize) {
                *v = v.saturating_add(drawn.len() as u32);
            }
            // "The first one they draw in each of their draw steps" is a
            // fact about this draw, so it is written down now. The turn's
            // count cannot stand in for it: a card drawn in the upkeep
            // would make the draw step's first card the turn's second.
            let in_own_draw_step =
                self.turn.active == player && self.turn.step == crate::turn::Step::Draw;
            let first_in_draw_step = in_own_draw_step && !self.per_turn.drew_in_draw_step;
            if in_own_draw_step {
                self.per_turn.drew_in_draw_step = true;
            }
            self.journal.record(crate::event::GameEvent::CardsDrawn {
                player,
                count: drawn.len() as u16,
                first_in_draw_step,
            });
        }
        drawn
    }

    /// How many cards `player` may draw this turn, or `None` for no limit.
    ///
    /// The **lowest** limit wins rather than the newest or the sum, because
    /// "can't" is not a number being modified: two Spirits of the Labyrinth
    /// are one limit of one, and a limit of one beside a limit of two is a
    /// limit of one (CR 101.2).
    ///
    /// The relation is read from each effect's own controller, so one
    /// Leovold limits that seat's opponents and says nothing about its
    /// controller, while one Spirit limits the table including whoever
    /// played it.
    #[must_use]
    pub fn draw_limit(&self, player: PlayerId) -> Option<u32> {
        self.effects
            .iter()
            .filter_map(|fx| {
                let baylee_cards_dsl::Modifier::DrawLimitPerTurn { who, limit } = fx.modifier
                else {
                    return None;
                };
                crate::eval::players(who, self, fx.controller)?
                    .contains(&player)
                    .then_some(u32::from(limit))
            })
            .min()
    }

    /// Streaming xxh3 hash over the entire deterministic state.
    ///
    /// Every struct on the way is taken apart by name, here and in the
    /// helpers it calls, so a field added tomorrow does not compile until it
    /// is either hashed or bound to `_` beside the reason it is left out. The
    /// list this replaced was written by hand and went blind to every field
    /// added after it, among them the X, the kicker and the chosen mode of a
    /// spell on the stack (#122). A type whose every field counts derives
    /// `Hash` instead, which reaches a new field without being asked.
    ///
    /// Nothing here depends on the order a map iterates in (`hash_unordered`)
    /// or on an address: a `&'static` definition hashes what it says, or the
    /// printed face that names it (`hash_ability_list`).
    #[must_use]
    #[allow(clippy::too_many_lines)] // one line per field: the list is the guard
    pub fn snapshot_hash(&self) -> u64 {
        let Self {
            arena,
            zones,
            players,
            turn,
            combat,
            per_turn,
            delayed,
            pending_miracle,
            extra_turns,
            restriction_info,
            next_restriction_id,
            commander_casts,
            commander_redirect,
            pending_copied_faces,
            ltb_mana_values,
            ltb_controllers,
            ltb_powers,
            ltb_abilities,
            ltb_attachments,
            ltb_counters,
            ltb_characteristics,
            // Empty again before any question is out; the field says why.
            ceased: _,
            // Empty whenever a question is out, by a rule the build
            // enforces; the field says which.
            reflexive: _,
            discovered,
            divided,
            synthetic_copies,
            commanders,
            monarch,
            day_night,
            previous_turn,
            starting_player,
            ability_fires,
            rng,
            // A record of what happened, not an input to what happens next.
            // The two rules that used to read this turn's entries now read
            // `per_turn` instead (#241). The engine's remaining readers are
            // anchored to state the `Engine` holds: `trigger_scan_seq`,
            // `entry_scan_seq`, and the resolution in progress
            // (`resolve::reflexive`). Hashing those is #238.
            journal: _,
            names,
            // Printed faces shared between objects. Each object's face is
            // hashed with the object, whether or not it is shared.
            bases: _,
            timestamp,
            effects,
            replacement_rules,
            characteristics_generation,
            // Scratch, always left empty.
            projection_ids: _,
            // A cache flag, derived from the effects hashed below.
            projected_cross_zone: _,
            // Read off the cards at setup and never written again; each
            // object's card is hashed with the object.
            printed_pt_cda: _,
            // Drained by every pass, before anyone can look.
            token_cleanup: _,
        } = self;
        let mut h = Hasher::new();
        h.u64(*timestamp);
        h.u64(*characteristics_generation);
        hash_effects(&mut h, effects);
        replacement_rules.hash(&mut h);
        turn.hash(&mut h);
        day_night.hash(&mut h);
        previous_turn.hash(&mut h);
        monarch.hash(&mut h);
        starting_player.hash(&mut h);
        per_turn.hash(&mut h);
        delayed.hash(&mut h);
        pending_miracle.hash(&mut h);
        extra_turns.hash(&mut h);
        rng.hash(&mut h);
        // The interner's order is the game's history, and every object
        // hashes the `NameRef` it carries; the count keeps apart two
        // histories that interned a different number of names.
        h.usize(names.len());
        h.usize(players.len());
        for player in players {
            hash_player(&mut h, player);
        }
        for (slot, generation, value) in arena.slots() {
            h.u32(slot);
            h.u8(generation);
            h.boolean(value.is_some());
            if let Some(obj) = value {
                hash_object(&mut h, obj);
            }
        }
        zones.hash(&mut h);
        combat.hash(&mut h);
        // The commander counters, which no zone can stand in for: a
        // commander cast and then returned leaves the command zone exactly
        // as it found it, and the only difference between the two states is
        // what the next cast costs (CR 903.8). `Commander::answered` rides
        // along: two states that differ only in whether the owner has said
        // no yet (CR 903.9a) are two states.
        commander_casts.hash(&mut h);
        commanders.hash(&mut h);
        // Answers taken but not yet spent (CR 903.9b), and copies created
        // but not yet handed the rules text they copied. A resolution can
        // suspend on a choice with either outstanding, which is a moment
        // this hash is taken at.
        commander_redirect.hash(&mut h);
        pending_copied_faces.hash(&mut h);
        // A discovered card waiting for the resolution that found it to end.
        discovered.hash(&mut h);
        // How an ability on the stack divides its damage.
        divided.hash(&mut h);
        // The look-back lists are not scan bookkeeping that a priority
        // grant clears: an entry stays until its object moves again, and
        // `eval::matches` consults `ltb_attachments` in general.
        h.usize(ltb_abilities.len());
        for (object, list) in ltb_abilities {
            object.hash(&mut h);
            hash_ability_list(&mut h, list.abilities, list.printed);
        }
        ltb_attachments.hash(&mut h);
        ltb_counters.hash(&mut h);
        ltb_mana_values.hash(&mut h);
        h.usize(ltb_characteristics.len());
        for (object, was) in ltb_characteristics {
            object.hash(&mut h);
            hash_characteristics(&mut h, was);
        }
        ltb_controllers.hash(&mut h);
        ltb_powers.hash(&mut h);
        synthetic_copies.hash(&mut h);
        hash_unordered(
            &mut h,
            restriction_info.iter(),
            |(id, (source, filter, rider))| {
                let mut one = Hasher::new();
                id.hash(&mut one);
                source.hash(&mut one);
                filter_hash(&mut one, filter);
                rider.hash(&mut one);
                one.finish()
            },
        );
        next_restriction_id.hash(&mut h);
        // Thirteen bytes an entry, digested in one call: a streaming state
        // set up per entry costs more than the entry does.
        hash_unordered(&mut h, ability_fires.iter(), |((object, index), uses)| {
            let mut entry = [0u8; 13];
            entry[..4].copy_from_slice(&object.slot().to_le_bytes());
            entry[4] = object.generation();
            entry[5..9].copy_from_slice(&index.to_le_bytes());
            entry[9..].copy_from_slice(&uses.to_le_bytes());
            xxhash_rust::xxh3::xxh3_64(&entry)
        });
        h.finish()
    }

    /// A hash of the *rules-visible situation*, blind to object identity and
    /// to time.
    ///
    /// [`Self::snapshot_hash`] answers "is this the same game state?" — it is
    /// what resync and replay compare, and it deliberately hashes object
    /// slots, generations and timestamps. That makes it useless for the
    /// question the loop detector asks. Slots are never recycled and
    /// timestamps only go up, so a permanent that dies and comes back is a
    /// different object at a later time: a genuine endless loop never hashes
    /// the same twice.
    ///
    /// This hashes what a player would see instead — who is where, with what
    /// characteristics, counters, damage and status — and reduces every
    /// object reference (attachments, targets, combat, an ability's source)
    /// to a position in a canonical ordering of the zones, so that a
    /// re-created permanent looks like the one it replaced.
    ///
    /// The turn *number* is left out for the same reason: a loop that spans a
    /// turn boundary would otherwise look different every time round.
    ///
    /// See [`crate::loops`] for how the detector uses it.
    #[must_use]
    pub fn loop_signature(&self) -> u64 {
        let zones = self.signature_zones();

        // Canonical position of every object, so references can be hashed
        // without their ids. Sorted by slot for a binary search; slots are
        // unique, so the mapping is exact.
        let mut by_slot: Vec<(u32, u32)> = Vec::new();
        for loc in &zones {
            for id in self.zones.list(*loc) {
                let position = by_slot.len() as u32;
                by_slot.push((id.slot(), position));
            }
        }
        by_slot.sort_unstable_by_key(|(slot, _)| *slot);
        let position = |id: ObjectId| -> u32 {
            by_slot
                .binary_search_by_key(&id.slot(), |(slot, _)| *slot)
                .map_or(u32::MAX, |i| by_slot[i].1)
        };

        let mut h = Hasher::new();
        h.u8(self.turn.active.get());
        h.u8(self.turn.phase as u8);
        h.u8(self.turn.step as u8);
        h.u8(self.monarch.map_or(255, PlayerId::get));
        // The designation is rules-visible and a loop that flips it is a
        // loop that changes what daybound permanents are (CR 731). The
        // previous turn belongs here for a subtler reason: it is what the
        // *next* untap step will read, so two states alike in everything
        // else but disagreeing about it are not the same state.
        h.u8(self.day_night.map_or(255, |d| d as u8));
        h.u8(self.previous_turn.map_or(255, |p| p.active.get()));
        h.u32(self.previous_turn.map_or(0, |p| p.spells_cast));
        h.u32(self.previous_turn.map_or(0, |p| p.spells_by_all));
        h.u32(self.previous_turn.map_or(0, |p| p.most_by_one));
        h.usize(self.players.len());
        for p in &self.players {
            h.u8(p.id.get());
            h.i32(p.life);
            h.u16(p.poison);
            h.u16(p.energy);
            h.boolean(p.enduring_story);
            h.boolean(p.citys_blessing);
            h.i8(p.hand_modifier);
            h.boolean(p.has_lost());
            for color in ManaColor::ALL {
                h.u16(p.mana_pool.available(color));
                h.u16(p.mana_pool.snow_available(color));
            }
            // The commander tax belongs here even though nothing else that
            // only grows does. It is rules-visible — a player can see what
            // the next cast costs — and it only ever rises (CR 903.8), so a
            // "loop" that casts a commander is a game still making progress
            // and must not be called a draw. The ids need no canonical
            // position: the list is fixed for the whole game, so its order
            // already identifies each commander.
            //
            // Commander damage belongs here for the same reason and by the
            // same argument (CR 903.10a): it is rules-visible, it only ever
            // rises, and a "loop" that lands another swing from a commander
            // is a game walking towards a loss rather than standing still.
            // The order is the order it was first dealt in, which is the
            // same on every machine.
            h.usize(p.commander_damage.len());
            for (source, amount) in &p.commander_damage {
                h.u32(source.slot());
                h.u16(*amount);
            }
            // `Commander::answered` deliberately does *not* join it, in
            // either form. As a timestamp it would be the tax bug in
            // reverse — a number that only grows makes every situation
            // unique and no loop detectable — and as a "has this arrival
            // been offered" bit it is a constant here: this hash is taken
            // at priority grants, the SBA fixpoint has finished by then,
            // and a commander sitting in a graveyard has therefore always
            // been offered already. `commander_redirect` stays out for the
            // second of those reasons: it is filled and spent inside one
            // resolution, so it is empty every time this hash is taken.
            // `snapshot_hash` carries it because that one runs at any
            // moment, including a suspended one.
            let seat = p.id.get() as usize;
            h.u32(self.commander_casts.get(seat).copied().unwrap_or(0));
            for c in self.commanders.get(seat).into_iter().flatten() {
                h.u32(c.casts);
            }
        }
        for loc in &zones {
            let list = self.zones.list(*loc);
            h.usize(list.len());
            for id in list {
                match self.object(*id) {
                    Some(obj) => hash_object_situation(&mut h, obj, &position),
                    None => h.u8(0),
                }
            }
        }
        h.usize(self.combat.attackers().len());
        for a in self.combat.attackers() {
            h.u32(position(a.creature));
            hash_defender(&mut h, a.defending, position);
            h.boolean(a.blocked);
        }
        h.usize(self.combat.blockers.len());
        for b in &self.combat.blockers {
            h.u32(position(b.blocker));
            h.u32(position(b.attacker));
        }
        // What has already been used this turn, which is rules-visible: an
        // ability that prints "activate only once each turn" is *offered* in
        // one of these states and not in the other, so two boards alike in
        // everything else are not the same state. Left out, a loop detector
        // would call them one and could declare a draw on a game that still
        // had a move in it.
        //
        // Sorted, because iterating the map in its own order would make the
        // signature depend on insertion — the rule this engine keeps
        // everywhere for determinism. The object is hashed by its canonical
        // position for the reason every other reference here is.
        let mut used: Vec<(u32, u32, u32)> = self
            .ability_fires
            .iter()
            .map(|((id, index), n)| (position(*id), *index, *n))
            .collect();
        used.sort_unstable();
        h.usize(used.len());
        for (obj, index, n) in used {
            h.u32(obj);
            h.u32(index);
            h.u32(n);
        }
        h.finish()
    }

    /// Every zone, in a fixed order — the canonical ordering object
    /// positions in [`Self::loop_signature`] are taken from.
    fn signature_zones(&self) -> Vec<ZoneLocation> {
        let mut locs = Vec::with_capacity(2 + self.players.len() * 6);
        locs.push(ZoneLocation::Battlefield);
        locs.push(ZoneLocation::Stack);
        for p in &self.players {
            locs.push(ZoneLocation::Library(p.id));
            locs.push(ZoneLocation::Hand(p.id));
            locs.push(ZoneLocation::Graveyard(p.id));
            locs.push(ZoneLocation::Exile(p.id));
            locs.push(ZoneLocation::Command(p.id));
            locs.push(ZoneLocation::OutsideGame(p.id));
        }
        locs
    }
}

/// A loss as [`GameState::snapshot_hash`] folds it in: `0` for none, which
/// is what the boolean it replaced wrote for `false`, and one value per
/// reason after it.
///
/// Spelled out rather than `reason as u8`, so reordering the enum cannot
/// move a hash and a new reason has to be given a byte here on purpose.
const fn loss_byte(loss: Option<LossReason>) -> u8 {
    match loss {
        None => 0,
        Some(LossReason::Life) => 1,
        Some(LossReason::EmptyDraw) => 2,
        Some(LossReason::Poison) => 3,
        Some(LossReason::CommanderDamage) => 4,
        Some(LossReason::Conceded) => 5,
        Some(LossReason::Effect) => 6,
    }
}

/// Where `pos` put a card in a library that holds `len` cards with it.
///
/// `Index` counts from the bottom and is clamped the way `Zones::insert`
/// clamps it.
fn library_place(pos: ZonePosition, len: usize) -> LibraryPlace {
    match pos {
        ZonePosition::Top => LibraryPlace::Top,
        ZonePosition::Bottom => LibraryPlace::Bottom,
        ZonePosition::Index(i) => {
            let at = i.min(len.saturating_sub(1));
            match len - at {
                1 => LibraryPlace::Top,
                _ if at == 0 => LibraryPlace::Bottom,
                n => LibraryPlace::FromTop(u32::try_from(n).unwrap_or(u32::MAX)),
            }
        }
    }
}

struct Hasher {
    inner: Xxh3,
}

// Fixed byte order and word width keep structural DSL hashing deterministic
// across native and wasm builds. References hash their contents, never addresses.
impl std::hash::Hasher for Hasher {
    fn finish(&self) -> u64 {
        self.inner.digest()
    }
    fn write(&mut self, bytes: &[u8]) {
        self.inner.update(bytes);
    }
    fn write_u8(&mut self, value: u8) {
        self.inner.update(&value.to_le_bytes());
    }
    fn write_u16(&mut self, value: u16) {
        self.inner.update(&value.to_le_bytes());
    }
    fn write_u32(&mut self, value: u32) {
        self.inner.update(&value.to_le_bytes());
    }
    fn write_u64(&mut self, value: u64) {
        self.inner.update(&value.to_le_bytes());
    }
    fn write_u128(&mut self, value: u128) {
        self.inner.update(&value.to_le_bytes());
    }
    fn write_i8(&mut self, value: i8) {
        self.inner.update(&value.to_le_bytes());
    }
    fn write_i16(&mut self, value: i16) {
        self.inner.update(&value.to_le_bytes());
    }
    fn write_i32(&mut self, value: i32) {
        self.inner.update(&value.to_le_bytes());
    }
    fn write_i64(&mut self, value: i64) {
        self.inner.update(&value.to_le_bytes());
    }
    fn write_i128(&mut self, value: i128) {
        self.inner.update(&value.to_le_bytes());
    }
    fn write_usize(&mut self, value: usize) {
        self.inner.update(&(value as u64).to_le_bytes());
    }
    fn write_isize(&mut self, value: isize) {
        self.inner.update(&(value as i64).to_le_bytes());
    }
}

impl Hasher {
    fn new() -> Self {
        Self { inner: Xxh3::new() }
    }
    fn finish(self) -> u64 {
        self.inner.digest()
    }
    fn bytes(&mut self, b: &[u8]) {
        self.inner.update(b);
    }
    fn u8(&mut self, v: u8) {
        self.bytes(&[v]);
    }
    fn i8(&mut self, v: i8) {
        self.bytes(&v.to_le_bytes());
    }
    fn u16(&mut self, v: u16) {
        self.bytes(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.bytes(&v.to_le_bytes());
    }
    fn i16(&mut self, v: i16) {
        self.bytes(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.bytes(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.bytes(&v.to_le_bytes());
    }
    fn u128(&mut self, v: u128) {
        self.bytes(&v.to_le_bytes());
    }
    fn usize(&mut self, v: usize) {
        self.bytes(&(v as u64).to_le_bytes());
    }
    fn boolean(&mut self, v: bool) {
        self.u8(u8::from(v));
    }
    fn option_u32(&mut self, v: Option<u32>) {
        match v {
            Some(x) => {
                self.u8(1);
                self.u32(x);
            }
            None => self.u8(0),
        }
    }
}

/// Hashes a defender. `locate` maps an object to whatever identity the
/// caller's hash is built on — the arena slot for the snapshot, a
/// canonical position for the loop signature.
///
/// The discriminant is hashed first so that a planeswalker in slot 3 and
/// the player with id 3 cannot collide.
fn hash_defender(h: &mut Hasher, defender: Defender, locate: impl Fn(ObjectId) -> u32) {
    match defender {
        Defender::Player(p) => {
            h.u8(0);
            h.u32(u32::from(p.get()));
        }
        Defender::Planeswalker(id) => {
            h.u8(1);
            h.u32(locate(id));
        }
    }
}

/// Hashes one object as a *situation*: everything a player could observe
/// about it, with object references reduced to canonical positions.
///
/// The identity fields `hash_object` includes — slot, generation — are
/// exactly what has to be left out here; see
/// [`GameState::loop_signature`].
fn hash_object_situation(h: &mut Hasher, obj: &GameObject, position: &impl Fn(ObjectId) -> u32) {
    h.u8(1);
    h.u8(obj.owner.get());
    h.u8(obj.controller.get());
    // Two boards that look identical but differ in who gets the permanent
    // back when a control effect ends are different situations.
    h.u8(obj.base_controller.get());
    h.u8(obj.zone as u8);
    h.u8(obj.zone_owner.map_or(255, PlayerId::get));
    h.u8(obj.kind as u8);
    h.u8(obj.face_index);
    h.u8(obj.doors.bits());
    h.boolean(obj.prototyped);
    match &obj.card {
        Some(c) => {
            h.u8(1);
            h.u32(c.index.get());
        }
        None => h.u8(0),
    }
    let b = &obj.base;
    h.u32(b.name.get());
    hash_mana_cost(h, &b.mana_cost);
    h.u8(b.colors.bits());
    h.u16(b.types.bits());
    h.u8(b.supertypes.bits());
    for word in b.subtypes.words() {
        h.u64(*word);
    }
    h.u128(b.keywords.bits());
    h.option_u32(b.power.map(|v| v as u32));
    h.option_u32(b.toughness.map(|v| v as u32));
    h.option_u32(b.loyalty.map(u32::from));
    let counters: Vec<_> = obj.counters.iter().collect();
    h.usize(counters.len());
    for (kind, n) in counters {
        hash_counter(h, kind);
        h.u16(n);
    }
    h.u16(obj.damage);
    // Status and the deathtouch mark share one word; bit 8 is out of the
    // status byte, so packing them cannot collide.
    h.u16(u16::from(obj.status.bits()) | (u16::from(obj.deathtouched) << 8));
    // Its own byte rather than a third thing packed into the word above:
    // a shield is a count and not a flag, so there is no width to argue
    // about, and two boards that differ only in how many destructions a
    // creature will survive are different situations.
    h.u8(obj.regeneration_shields);
    h.option_u32(obj.source_power_lki.map(|p| p as u32));
    // What was paid is part of what the spell will do: Neoform after a
    // two-drop and after a five-drop are two different futures.
    h.option_u32(obj.paid.as_ref().and_then(|p| p.sacrificed_mana_value));
    h.u32(obj.paid.as_ref().map_or(0, |p| p.mana_spent));
    h.u8(obj.paid.as_ref().map_or(0, |p| p.colors_spent.bits()));
    // And which creature station tapped: its power is what the counters
    // will be.
    let tapped = obj.paid.as_ref().and_then(|p| p.tapped);
    h.option_u32(tapped.map(|(id, _)| position(id)));
    h.option_u32(tapped.map(|(_, version)| version));
    h.option_u32(obj.attached_to.map(position));
    h.usize(obj.targets.len());
    for t in &obj.targets {
        h.u32(position(*t));
    }
    h.usize(obj.second_targets().len());
    for t in obj.second_targets() {
        h.u32(position(*t));
    }
    match &obj.ability {
        Some(loc) => {
            h.u8(1);
            h.option_u32(loc.card.map(baylee_core::ids::CardIndex::get));
            h.u32(loc.index);
            h.u32(position(loc.source));
        }
        None => h.u8(0),
    }
}

/// Hashes a map's entries without depending on the order it iterates in.
///
/// `digest` hashes one entry on its own and the digests are summed, so any
/// iteration order gives one total and nothing is allocated. Sorting the
/// entries first would put a `Vec` on every call, and the harness takes
/// this hash after every action (#213).
fn hash_unordered<T>(
    h: &mut Hasher,
    entries: impl ExactSizeIterator<Item = T>,
    digest: impl Fn(T) -> u64,
) {
    h.usize(entries.len());
    let sum = entries.fold(0u64, |sum, entry| sum.wrapping_add(digest(entry)));
    h.u64(sum);
}

/// Hashes an ability list by what names it, which is never its address.
///
/// A printed list is named by its face: [`AbilityList`] carries the two
/// together, and every place that builds one takes both from the same face,
/// so the face stands for the list. A token's or an emblem's has no face,
/// and there the list's content is hashed, because an address is no name:
/// it differs between builds, and the compiler merges identical lists into
/// one ([`PrintedFace`]).
///
/// [`AbilityList`]: crate::object::AbilityList
fn hash_ability_list(
    h: &mut Hasher,
    abilities: &[baylee_cards_dsl::AbilityDef],
    printed: Option<PrintedFace>,
) {
    h.usize(abilities.len());
    if let Some(face) = printed {
        h.u8(1);
        face.hash(h);
    } else {
        h.u8(0);
        abilities.hash(h);
    }
}

fn hash_effects(h: &mut Hasher, table: &crate::effects::EffectTable) {
    let (effects, parked, next_id, generation) = table.hashed_parts();
    next_id.hash(h);
    generation.hash(h);
    h.usize(effects.len());
    for fx in effects {
        hash_effect(h, fx);
    }
    // The statics parked while their sources are phased out. Nothing is
    // written while nothing is parked, so a game without phasing hashes as
    // it did before the table could park; the length above already moves
    // when an effect is parked.
    if !parked.is_empty() {
        h.usize(parked.len());
        for fx in parked {
            hash_effect(h, fx);
        }
    }
}

fn hash_effect(h: &mut Hasher, fx: &crate::effects::ContinuousEffect) {
    let crate::effects::ContinuousEffect {
        id,
        source,
        controller,
        origin,
        layer,
        timestamp,
        duration,
        filter,
        modifier,
    } = fx;
    id.hash(h);
    source.hash(h);
    controller.hash(h);
    origin.hash(h);
    layer.hash(h);
    timestamp.hash(h);
    duration.hash(h);
    match filter {
        crate::effects::EffectFilter::Dsl(filter) => {
            h.u8(0);
            filter_hash(h, filter);
        }
        crate::effects::EffectFilter::ObjectIs(id, version) => {
            h.u8(1);
            id.hash(h);
            version.hash(h);
        }
    }
    hash_modifier(h, modifier);
}

fn hash_player(h: &mut Hasher, player: &Player) {
    let Player {
        id,
        life,
        poison,
        energy,
        enduring_story,
        citys_blessing,
        mana_pool,
        hand_modifier,
        lands_played_this_turn,
        turn_start_timestamp,
        tried_empty_draw,
        commander_damage,
        loss,
        // Preset-constant, so it tells no two states of one game apart
        // (`docs/engine-internals.md`).
        team: _,
    } = player;
    id.hash(h);
    life.hash(h);
    poison.hash(h);
    energy.hash(h);
    enduring_story.hash(h);
    citys_blessing.hash(h);
    mana_pool.hash(h);
    hand_modifier.hash(h);
    lands_played_this_turn.hash(h);
    turn_start_timestamp.hash(h);
    tried_empty_draw.hash(h);
    // Commander damage (CR 903.10a), which no life total records: two
    // seats on the same life with twelve and twenty points from the same
    // commander are one attack apart from different games.
    commander_damage.hash(h);
    // Why, and not only whether: two engines that eliminated the same seat
    // by different state-based actions ran different rules, and once that
    // seat's objects have left the game the reason is the only trace of
    // which one fired.
    h.u8(loss_byte(*loss));
}

fn hash_characteristics(h: &mut Hasher, characteristics: &Characteristics) {
    let Characteristics {
        name,
        mana_cost,
        colors,
        types,
        supertypes,
        subtypes,
        keywords,
        power,
        toughness,
        loyalty,
        color_identity,
        produced_colors,
        produced_colorless,
        produced_chosen,
        abilities_lost,
        front_mana_value,
    } = characteristics;
    name.hash(h);
    hash_mana_cost(h, mana_cost);
    colors.hash(h);
    types.hash(h);
    supertypes.hash(h);
    subtypes.hash(h);
    keywords.hash(h);
    power.hash(h);
    toughness.hash(h);
    loyalty.hash(h);
    color_identity.hash(h);
    produced_colors.hash(h);
    produced_colorless.hash(h);
    produced_chosen.hash(h);
    abilities_lost.hash(h);
    front_mana_value.hash(h);
}

#[allow(clippy::too_many_lines)] // one line per field: the list is the guard
fn hash_object(h: &mut Hasher, obj: &GameObject) {
    let GameObject {
        id,
        owner,
        controller,
        base_controller,
        zone,
        zone_owner,
        kind,
        card,
        base,
        // The layer projection of `base` under the effect table, both of
        // which are hashed; it is recomputed whenever the table moves.
        cache: _,
        counters,
        damage,
        deathtouched,
        regeneration_shields,
        status,
        attached_to,
        timestamp,
        version,
        riders,
        targets,
        target_req,
        second,
        original_base,
        ability,
        source_power_lki,
        paid,
        x_value,
        kicked,
        replicated,
        alt_cast,
        prototyped,
        chosen_player,
        target_players,
        mode_index,
        modes,
        chosen_subtype,
        chosen_color,
        chosen_name,
        doors,
        face_index,
        own_abilities,
        own_abilities_until_eot,
        own_face,
        token,
        pending_face_change,
        event_object,
        event_amount,
        cast_from_hand,
    } = obj;
    id.hash(h);
    owner.hash(h);
    controller.hash(h);
    // Not derivable from the projected controller: it is who the permanent
    // goes back to when a control effect ends, so a resync that lost it
    // would hand the permanent to the wrong seat later.
    base_controller.hash(h);
    zone.hash(h);
    zone_owner.hash(h);
    kind.hash(h);
    card.hash(h);
    // Base characteristics (copiable values), and the ones a copy gives
    // back when it changes zones (CR 400.7).
    hash_characteristics(h, base);
    h.boolean(original_base.is_some());
    if let Some(original) = original_base {
        hash_characteristics(h, original);
    }
    counters.hash(h);
    damage.hash(h);
    deathtouched.hash(h);
    regeneration_shields.hash(h);
    status.hash(h);
    attached_to.hash(h);
    timestamp.hash(h);
    version.hash(h);
    // Exile riders.
    h.usize(riders.len());
    for rider in riders {
        match rider {
            Rider::Linked { host, until } => {
                h.u8(1);
                host.hash(h);
                match until {
                    None => h.u8(0),
                    Some(crate::object::LinkUntil::HostLeaves) => h.u8(1),
                    Some(crate::object::LinkUntil::OpponentBecomesMonarch { of }) => {
                        h.u8(2);
                        h.u8(of.get());
                    }
                }
            }
            Rider::Rebound => h.u8(2),
            Rider::Adventure => h.u8(3),
            Rider::Foretold => h.u8(4),
            Rider::Plotted => h.u8(5),
            Rider::Suspend => h.u8(6),
            Rider::Flashback => h.u8(7),
            Rider::Uncounterable => h.u8(8),
            Rider::PlayableFromExileFor(p) => {
                h.u8(9);
                h.u8(p.get());
            }
            Rider::Prepared => h.u8(10),
            Rider::SpellCopy => h.u8(11),
            Rider::ExileInsteadOfGraveyard => h.u8(12),
            Rider::ExiledWith { host, version } => {
                h.u8(13);
                host.hash(h);
                version.hash(h);
            }
            Rider::Dashed => h.u8(14),
            Rider::EventPlayer(p) => {
                h.u8(15);
                h.u8(p.get());
            }
            Rider::Escaped => h.u8(16),
        }
    }
    // What the spell or ability on the stack was cast or put there with:
    // its targets, what it may retarget to, which ability it is, and every
    // choice made on the way — two copies of one spell with X 3 and X 0 are
    // two different futures.
    targets.hash(h);
    target_req.hash(h);
    second.hash(h);
    ability.hash(h);
    source_power_lki.hash(h);
    paid.hash(h);
    x_value.hash(h);
    kicked.hash(h);
    replicated.hash(h);
    alt_cast.hash(h);
    prototyped.hash(h);
    chosen_player.hash(h);
    target_players.hash(h);
    mode_index.hash(h);
    modes.hash(h);
    chosen_subtype.hash(h);
    chosen_color.hash(h);
    chosen_name.hash(h);
    doors.hash(h);
    face_index.hash(h);
    pending_face_change.hash(h);
    event_object.hash(h);
    event_amount.hash(h);
    cast_from_hand.hash(h);
    // What the object can do when it is not what its card says: a copy's
    // list, an emblem's, an ability's captured one. `own_face` names it.
    h.boolean(own_abilities.is_some());
    if let Some(list) = own_abilities {
        hash_ability_list(h, list, *own_face);
    }
    own_face.hash(h);
    own_abilities_until_eot.hash(h);
    // A token's definition, hashed by what it says for the reason
    // `hash_ability_list` gives.
    token.hash(h);
}

/// Whether a DSL filter reads **board state** rather than a characteristic:
/// whether an object is tapped, or whether it is attacking.
///
/// The sibling of [`filter_reaches_other_zones`], and it exists for the same
/// reason: the projection cache is keyed on the *effect* generation, so an
/// input that is not an effect has to announce itself. These two are such
/// inputs — `Filter::Untapped` is Spectral Cloak's whole sentence
/// ("enchanted creature has shroud as long as it's untapped"), `Filter::
/// Attacking` is Orcish Oriflamme's — and a board where nothing reads them
/// must not pay for the announcement, because tapping a land is the most
/// frequent thing that happens in a game.
///
/// `Filter::EnteredThisTurn` is the third of this kind and is deliberately
/// **not** here: no card in this pool reads it from a `static_ability!`, and
/// the two moments it flips at — a permanent arriving, a turn beginning —
/// both invalidate for their own reasons already. The day a static prints it,
/// this is the list it joins and the turn boundary is what needs the door.
pub(crate) fn filter_reads_board_state(filter: &baylee_cards_dsl::Filter) -> bool {
    use baylee_cards_dsl::Filter;
    match filter {
        Filter::Tapped | Filter::Untapped | Filter::Attacking => true,
        Filter::And(parts) | Filter::Or(parts) => parts.iter().any(filter_reads_board_state),
        Filter::Not(f) => filter_reads_board_state(f),
        _ => false,
    }
}

/// Whether a DSL filter mentions non-battlefield zones (then its effect
/// needs cross-zone projection).
/// The characteristic-defining power and toughness a card's front face
/// prints (CR 604.3a): a static on the card itself, unconditional, that
/// defines P/T in layer 7a. Front face only, because a card off the
/// battlefield and the stack has only its front face's characteristics
/// (CR 712.8a).
fn printed_pt_cda(def: &CardDef) -> Option<baylee_cards_dsl::Modifier> {
    def.abilities_for_face(0)
        .iter()
        .find_map(|ability| match ability {
            baylee_cards_dsl::AbilityDef::Static(sa)
                if sa.filter == baylee_cards_dsl::Filter::This
                    && sa.condition.is_none()
                    && matches!(
                        sa.modifier,
                        baylee_cards_dsl::Modifier::CharacteristicPT { .. }
                    ) =>
            {
                Some(sa.modifier)
            }
            _ => None,
        })
}

pub(crate) fn filter_reaches_other_zones(filter: &baylee_cards_dsl::Filter) -> bool {
    use baylee_cards_dsl::{Filter, ZoneRef};
    match filter {
        Filter::InZone(z) => !matches!(z, ZoneRef::Battlefield),
        Filter::And(parts) | Filter::Or(parts) => parts.iter().any(filter_reaches_other_zones),
        Filter::Not(f) => filter_reaches_other_zones(f),
        _ => false,
    }
}

/// Deterministic structural hash of a DSL filter (modifier payloads).
fn filter_hash(h: &mut Hasher, f: &baylee_cards_dsl::Filter) {
    use baylee_cards_dsl::Filter as F;
    match f {
        F::Any => h.u8(0),
        F::This => h.u8(1),
        F::Another => h.u8(2),
        F::And(parts) | F::Or(parts) => {
            h.u8(if matches!(f, F::And(_)) { 3 } else { 4 });
            for p in *parts {
                filter_hash(h, p);
            }
        }
        F::Not(inner) => {
            h.u8(5);
            filter_hash(h, inner);
        }
        F::HasType(t) => {
            h.u8(6);
            h.u16(t.bits());
        }
        F::LacksType(t) => {
            h.u8(7);
            h.u16(t.bits());
        }
        F::HasSupertype(s) => {
            h.u8(8);
            h.u8(s.bits());
        }
        F::HasSubtype(s) => {
            h.u8(9);
            h.u16(s.get());
        }
        F::HasColor(c) => {
            h.u8(10);
            h.u8(c.bits());
        }
        F::IsColorless => h.u8(11),
        F::Monocolored => h.u8(12),
        F::IsToken => h.u8(13),
        F::ControlledByYou => h.u8(14),
        F::ControlledByOpponent => h.u8(15),
        F::OwnedByYou => h.u8(16),
        F::Tapped => h.u8(17),
        F::Untapped => h.u8(18),
        F::Attacking => h.u8(19),
        F::MatchesChosenTypeOfSource => h.u8(20),
        F::AttachedToBySource => h.u8(25),
        F::SharesSubtypeWithCommander => h.u8(27),
        F::ToughnessAtMost(n) => {
            h.u8(26);
            h.i16(*n);
        }
        // Three tags and not one with a direction byte: the hash is what
        // tells two continuous effects apart, and a shared tag would make
        // "power at least 4" and "power at most 4" the same effect to the
        // cache — which is the one pair of filters on this list that a
        // single board satisfies on opposite sides.
        F::ToughnessAtLeast(n) => {
            h.u8(31);
            h.i16(*n);
        }
        F::PowerAtLeast(n) => {
            h.u8(32);
            h.i16(*n);
        }
        F::PowerAtMost(n) => {
            h.u8(33);
            h.i16(*n);
        }
        F::HasKeyword(k) => {
            h.u8(21);
            h.u128(k.bits());
        }
        // Game state rather than a characteristic, and hashed all the same:
        // two continuous effects that differ only in this filter are two
        // different effects, and a tag table that left it out would make
        // them one.
        F::EnteredThisTurn => h.u8(30),
        F::PutIntoGraveyardThisTurn => h.u8(35),
        F::HasCounter(kind) => {
            h.u8(36);
            hash_counter(h, *kind);
        }
        F::WithSingleTarget => h.u8(34),
        // Its own tag rather than a payload on `CmcAtMost`: the bound is
        // read from the source at match time, so two filters that differ
        // only in *where* the number comes from are different filters.
        F::CmcAtMostX => h.u8(28),
        F::CmcAtMostColorsSpent => h.u8(37),
        F::CmcAtMost(n) | F::CmcAtLeast(n) => {
            h.u8(if matches!(f, F::CmcAtMost(_)) { 22 } else { 23 });
            h.u32(*n);
        }
        F::InZone(z) => {
            h.u8(24);
            h.u8(*z as u8);
        }
        // Length-prefixed, so `Named("a") + Named("bc")` inside an `And`
        // cannot hash as `Named("ab") + Named("c")`.
        F::Named(name) => {
            h.u8(29);
            h.u32(u32::try_from(name.len()).unwrap_or(u32::MAX));
            h.bytes(name.as_bytes());
        }
    }
}

fn hash_modifier(h: &mut Hasher, modifier: &baylee_cards_dsl::Modifier) {
    // Derived Hash walks every modifier payload, including granted costs,
    // effects, triggers and counter kinds which the old tag table omitted.
    std::hash::Hash::hash(modifier, h);
}

/// Writes a counter kind into a hash.
///
/// A function rather than the one-byte tag it used to be, because a P/T
/// counter carries two numbers: `Plus { power: 0, toughness: 1 }` and
/// `Plus { power: 1, toughness: 0 }` are different counters and folding
/// them onto one byte would make two different boards hash alike.
fn hash_counter(h: &mut Hasher, kind: CounterKind) {
    match kind {
        CounterKind::Plus { power, toughness } => {
            h.u8(1);
            h.u8(power);
            h.u8(toughness);
        }
        CounterKind::Minus { power, toughness } => {
            h.u8(2);
            h.u8(power);
            h.u8(toughness);
        }
        CounterKind::Loyalty => h.u8(3),
        CounterKind::Lore => h.u8(4),
        CounterKind::Time => h.u8(5),
        CounterKind::Charge => h.u8(6),
        CounterKind::Poison => h.u8(7),
        CounterKind::Energy => h.u8(8),
        CounterKind::Rad => h.u8(9),
        CounterKind::Lifelink => h.u8(10),
        CounterKind::Level => h.u8(11),
        // The id is hashed whole. Folding it modulo 100 was safe while every
        // tag was one byte and is not worth keeping now that the function
        // writes as many as it likes.
        CounterKind::Custom(id) => {
            h.u8(12);
            h.u16(id);
        }
    }
}

pub(crate) fn mana_cost_fingerprint(cost: &baylee_core::mana::ManaCost) -> u64 {
    let mut h = Hasher::new();
    hash_mana_cost(&mut h, cost);
    h.finish()
}

fn hash_mana_cost(h: &mut Hasher, cost: &baylee_core::mana::ManaCost) {
    // The cost is counted (`ManaCost`): how many kinds it holds, then one
    // five-byte record per kind, its tag, what names it and how many of it,
    // rather than one per symbol. The count fits a byte, and a byte is what
    // an empty cost, most objects' (tokens'), costs the stream: a wider
    // prefix measured 7 % slower on `snapshot_hash_3k_tokens`.
    h.u8(cost.kinds());
    for (symbol, count) in cost.runs() {
        let [count_lo, count_hi] = count.to_le_bytes();
        let (tag, first, second) = match symbol {
            ManaSymbol::Generic(amount) => {
                // Always one generic symbol: its amount says it all.
                let [w0, w1, w2, w3] = amount.to_le_bytes();
                h.bytes(&[0, w0, w1, w2, w3]);
                continue;
            }
            ManaSymbol::Colorless => (1, 0, 0),
            ManaSymbol::White => (2, 0, 0),
            ManaSymbol::Blue => (3, 0, 0),
            ManaSymbol::Black => (4, 0, 0),
            ManaSymbol::Red => (5, 0, 0),
            ManaSymbol::Green => (6, 0, 0),
            ManaSymbol::Hybrid(pair) => (7, pair.first() as u8, pair.second() as u8),
            ManaSymbol::TwoOrColor(color) => (8, color as u8, 0),
            ManaSymbol::Phyrexian(color) => (9, color as u8, 0),
            ManaSymbol::HybridPhyrexian(pair) => (10, pair.first() as u8, pair.second() as u8),
            ManaSymbol::Snow => (11, 0, 0),
            ManaSymbol::Variable(variable) => (12, variable as u8, 0),
            ManaSymbol::HalfGeneric => (13, 0, 0),
            ManaSymbol::Infinite => (14, 0, 0),
        };
        h.bytes(&[tag, first, second, count_lo, count_hi]);
    }
}

/// Writes the controller a walk of the refresh projected, noting in `was`
/// the one the object had before a walk first moved it. Whether it moved.
fn settle_controller(
    obj: &mut GameObject,
    controller: PlayerId,
    was: &mut Vec<(ObjectId, PlayerId)>,
) -> bool {
    if obj.controller == controller {
        return false;
    }
    if !was.iter().any(|(moved, _)| *moved == obj.id) {
        was.push((obj.id, obj.controller));
    }
    obj.controller = controller;
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::ids::PrintRef;
    use baylee_core::preset::{
        AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, SeatController, SeatSpec,
    };

    struct RegistryLookup;

    impl CardLookup for RegistryLookup {
        fn card(&self, index: CardIndex) -> Option<&'static CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn card_index(oracle_id: &str) -> CardIndex {
        baylee_cards::by_oracle_id(oracle_id)
            .expect("acceptance registry contains the card")
            .index
    }

    fn forest() -> CardIndex {
        card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
    }

    fn force_of_will() -> CardIndex {
        card_index("956381ba-6d37-4a8a-846c-bad79222dbee")
    }

    fn make_preset(seed: u64) -> GamePreset {
        let deck: Vec<DeckEntry> = (0..60)
            .map(|i| DeckEntry {
                card: if i % 3 == 0 {
                    force_of_will()
                } else {
                    forest()
                },
                print: PrintRef::new(0),
            })
            .collect();
        GamePreset {
            format: FormatId::Freeform,
            seed,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![baylee_core::preset::PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: baylee_core::preset::Finish::Normal,
            }],
            seats: (0..2)
                .map(|_| SeatSpec {
                    controller: SeatController::Ai(AIProfile::default()),
                    capabilities: baylee_core::preset::SeatCapabilities::default(),
                    deck: deck.clone(),
                    sideboard: vec![],
                    commanders: vec![],
                    starting_life: None,
                    starting_hand: None,
                    starting_battlefield: vec![],
                    emblems: vec![],
                    team: None,
                })
                .collect(),
        }
    }

    /// A draw limit, registered by hand rather than played off a card.
    ///
    /// Spirit of the Labyrinth is the card and it has a test of its own; this
    /// is the rule, asked without one, because what CR 121.2b actually says
    /// is about the *loop* and not about the card: "such an effect applies to
    /// individual card draws. Instructions to draw multiple cards may still
    /// be partially carried out." A card test proves the limit exists; only
    /// this shape proves that draw-three under a limit of one draws one
    /// rather than nothing.
    fn limit_draws(state: &mut GameState, who: baylee_cards_dsl::PlayerRel, limit: u8) {
        let timestamp = state.next_timestamp();
        state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: PlayerId::new(0),
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Layer::Text,
            timestamp,
            duration: baylee_cards_dsl::Duration::Indefinitely,
            filter: crate::effects::EffectFilter::Dsl(&baylee_cards_dsl::Filter::Any),
            modifier: baylee_cards_dsl::Modifier::DrawLimitPerTurn { who, limit },
        });
    }

    /// A state at the start of a turn.
    ///
    /// `from_preset` has already dealt the opening hands and those went
    /// through `draw_cards`, so `per_turn.draws` reads seven before a turn
    /// has begun. `progress.rs` zeroes it at every untap step, which is why
    /// this is a fixture and not a finding.
    fn at_a_fresh_turn(seed: u64) -> GameState {
        let mut state = GameState::from_preset(&make_preset(seed), &RegistryLookup).unwrap();
        state.per_turn.reset();
        state
    }

    #[test]
    fn room_counters_seed_only_the_selected_permanent_and_reject_unknown_kinds() {
        use baylee_core::preset::{StartingCounter, StartingCounters};
        let mut preset = make_preset(77);
        let entry = preset.seats[0].deck[0];
        preset.seats[0].starting_battlefield = vec![entry, entry];
        preset.house_rules.starting_counters = vec![StartingCounters {
            seat: 0,
            permanent: 1,
            counters: vec![
                StartingCounter {
                    kind: "charge".into(),
                    amount: 3,
                },
                StartingCounter {
                    kind: "+2/+1".into(),
                    amount: 2,
                },
                StartingCounter {
                    kind: "custom:42".into(),
                    amount: 1,
                },
            ],
        }];
        let state = GameState::from_preset(&preset, &RegistryLookup).unwrap();
        let cards: Vec<_> = state
            .zones
            .list(ZoneLocation::Battlefield)
            .iter()
            .map(|id| state.object(*id).unwrap())
            .collect();
        assert!(cards[0].counters.is_empty());
        assert_eq!(cards[1].counters.get(CounterKind::Charge), 3);
        assert_eq!(
            cards[1].counters.get(CounterKind::Plus {
                power: 2,
                toughness: 1
            }),
            2
        );
        assert_eq!(cards[1].counters.get(CounterKind::Custom(42)), 1);
        preset.house_rules.starting_counters[0].counters[0].kind = "made-up".into();
        assert!(matches!(
            GameState::from_preset(&preset, &RegistryLookup),
            Err(SetupError::UnknownCounter(_))
        ));
        preset.house_rules.starting_counters[0].permanent = 2;
        assert!(matches!(
            preset.validate(),
            Err(PresetError::StartingCounters)
        ));
    }

    #[test]
    fn a_draw_limit_is_partially_carried_out_rather_than_refused() {
        let mut state = at_a_fresh_turn(11);
        let me = PlayerId::new(0);
        assert_eq!(state.draw_limit(me), None, "no effect, no limit");
        assert_eq!(state.draw_cards(me, 3).len(), 3);

        let mut state = at_a_fresh_turn(11);
        limit_draws(&mut state, baylee_cards_dsl::PlayerRel::EachPlayer, 1);
        assert_eq!(state.draw_limit(me), Some(1));
        assert_eq!(
            state.draw_cards(me, 3).len(),
            1,
            "CR 121.2b: the instruction is partially carried out, not refused"
        );
        assert_eq!(
            state.draw_cards(me, 1).len(),
            0,
            "and the second instruction this turn draws nothing at all"
        );
    }

    /// Whether a draw opened its player's draw step is written as the draw
    /// is made (CR 504.1). The active player's first card in their draw step
    /// is flagged; a card drawn in their upkeep is not and does not use the
    /// flag up; the step's next card is not; the other player's card in that
    /// step is not; and the next turn's draw step starts clean.
    #[test]
    fn a_draw_says_whether_it_is_the_first_of_its_players_draw_step() {
        use crate::turn::Step;
        fn opened(state: &mut GameState, player: PlayerId, n: usize) -> bool {
            assert_eq!(state.draw_cards(player, n).len(), n);
            match state.journal.entries().last().map(|e| &e.event) {
                Some(crate::event::GameEvent::CardsDrawn {
                    player: drew,
                    first_in_draw_step,
                    ..
                }) if *drew == player => *first_in_draw_step,
                other => panic!("the draw's entry is last: {other:?}"),
            }
        }
        let (me, them) = (PlayerId::new(0), PlayerId::new(1));
        let mut state = at_a_fresh_turn(14);
        state.turn.active = me;

        state.turn.step = Step::Upkeep;
        assert!(!opened(&mut state, me, 1), "an upkeep card");
        state.turn.step = Step::Draw;
        assert!(opened(&mut state, me, 1), "the step's first card");
        assert!(!opened(&mut state, me, 2), "the step's next cards");
        assert!(!opened(&mut state, them, 1), "the other player's card");

        state.per_turn.reset();
        assert!(!opened(&mut state, them, 1), "not their draw step");
        assert!(opened(&mut state, me, 3), "the next turn's first card");
    }

    /// The relation is read from the **effect's** controller, so the same
    /// modifier limits the table or only the other side of it.
    #[test]
    fn a_draw_limit_on_the_opponents_leaves_its_own_controller_alone() {
        let (me, them) = (PlayerId::new(0), PlayerId::new(1));
        let mut state = at_a_fresh_turn(12);
        limit_draws(&mut state, baylee_cards_dsl::PlayerRel::EachOpponent, 1);

        assert_eq!(state.draw_limit(me), None, "Leovold does not limit Leovold");
        assert_eq!(state.draw_limit(them), Some(1));
        assert_eq!(state.draw_cards(me, 3).len(), 3);
        assert_eq!(state.draw_cards(them, 3).len(), 1);
    }

    /// Two limits are not a sum and not a newest-wins: "can't" is CR 101.2,
    /// so the lowest number is the one that holds.
    #[test]
    fn the_lowest_draw_limit_is_the_one_that_holds() {
        let mut state = at_a_fresh_turn(13);
        let me = PlayerId::new(0);
        limit_draws(&mut state, baylee_cards_dsl::PlayerRel::EachPlayer, 2);
        limit_draws(&mut state, baylee_cards_dsl::PlayerRel::EachPlayer, 1);
        assert_eq!(state.draw_limit(me), Some(1));
        assert_eq!(state.draw_cards(me, 4).len(), 1);
    }

    #[test]
    fn setup_is_deterministic() {
        let a = GameState::from_preset(&make_preset(42), &RegistryLookup).unwrap();
        let b = GameState::from_preset(&make_preset(42), &RegistryLookup).unwrap();
        assert_eq!(a.snapshot_hash(), b.snapshot_hash());
        let lib_a = a.zones.list(ZoneLocation::Library(PlayerId::new(0)));
        let lib_b = b.zones.list(ZoneLocation::Library(PlayerId::new(0)));
        assert_eq!(lib_a, lib_b);
        // 60-card deck minus 7 opening cards.
        assert_eq!(lib_a.len(), 53);
        assert_eq!(a.zones.list(ZoneLocation::Hand(PlayerId::new(0))).len(), 7);
    }

    /// Regression: every hosted game marks its human seat `Open` (the gateway
    /// and the dev server both do), and setup used to skip those seats
    /// entirely — the human started with no library, no opening hand, and lost
    /// to an empty draw on turn one.
    #[test]
    fn an_open_seat_is_dealt_in_like_any_other() {
        let mut preset = make_preset(7);
        preset.seats[0].controller = baylee_core::preset::SeatController::Open;
        let state = GameState::from_preset(&preset, &RegistryLookup).expect("game starts");

        let human = PlayerId::new(0);
        assert_eq!(
            state.zones.list(ZoneLocation::Hand(human)).len(),
            7,
            "an unclaimed human chair still gets an opening hand"
        );
        assert_eq!(state.zones.list(ZoneLocation::Library(human)).len(), 53);

        // And the seat opposite is unaffected.
        let other = PlayerId::new(1);
        assert_eq!(state.zones.list(ZoneLocation::Hand(other)).len(), 7);
    }

    /// The case an empty deck on an `Open` seat is actually for: a chair in a
    /// lobby that nobody has sat down in yet.
    #[test]
    fn a_genuinely_empty_chair_is_still_skipped() {
        let mut preset = make_preset(7);
        preset.seats[0].controller = baylee_core::preset::SeatController::Open;
        preset.seats[0].deck.clear();
        let state = GameState::from_preset(&preset, &RegistryLookup).expect("game starts");

        let empty = PlayerId::new(0);
        assert!(state.zones.list(ZoneLocation::Library(empty)).is_empty());
        assert!(state.zones.list(ZoneLocation::Hand(empty)).is_empty());
    }

    #[test]
    fn different_seeds_differ() {
        let a = GameState::from_preset(&make_preset(42), &RegistryLookup).unwrap();
        let b = GameState::from_preset(&make_preset(43), &RegistryLookup).unwrap();
        assert_ne!(a.snapshot_hash(), b.snapshot_hash());
    }

    #[test]
    fn draw_moves_top_card_and_bumps_version() {
        let mut state = GameState::from_preset(&make_preset(7), &RegistryLookup).unwrap();
        let hand = ZoneLocation::Hand(PlayerId::new(0));
        let before = state.zones.list(hand).len();
        let top = *state
            .zones
            .list(ZoneLocation::Library(PlayerId::new(0)))
            .last()
            .unwrap();
        let version_before = state.object(top).unwrap().version;
        let drawn = state.draw_cards(PlayerId::new(0), 1);
        assert_eq!(drawn, vec![top]);
        assert_eq!(state.zones.list(hand).len(), before + 1);
        assert_eq!(state.object(top).unwrap().version, version_before + 1);
        assert_eq!(state.object(top).unwrap().zone, crate::zone::Zone::Hand);
        // Twin state must draw the identical card.
        let mut twin = GameState::from_preset(&make_preset(7), &RegistryLookup).unwrap();
        assert_eq!(twin.draw_cards(PlayerId::new(0), 1), drawn);
        assert_eq!(state.snapshot_hash(), twin.snapshot_hash());
    }

    /// The attachment look-back is written on a departure and gone on the
    /// way back.
    ///
    /// The behaviour it exists for is a card test — Skullclamp drawing two
    /// when the creature it clamped dies — and that test cannot see the half
    /// that matters here. `eval::matches` consults `ltb_attachments` for
    /// *every* `Filter::AttachedToBySource`, not only during a trigger scan,
    /// so an entry that outlived its departure would make an unattached
    /// Equipment go on granting to a host that came back: the same
    /// `ObjectId` returns to the battlefield (CR 400.7 makes it a new object,
    /// not a new id) and the stale pairing would answer for it.
    ///
    /// Both halves are struck, which is the whole of the test: the entry is
    /// there after the host leaves, and it is gone after the host moves
    /// again.
    #[test]
    fn what_a_permanent_wore_is_remembered_across_one_departure_and_no_further() {
        let mut state = GameState::from_preset(&make_preset(11), &RegistryLookup).unwrap();
        let p0 = PlayerId::new(0);
        let host = state.draw_cards(p0, 1)[0];
        let worn = state.draw_cards(p0, 1)[0];
        for id in [host, worn] {
            state
                .move_object(
                    id,
                    ZoneLocation::Battlefield,
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .unwrap();
        }
        state.object_mut(worn).unwrap().attached_to = Some(host);
        assert!(
            state.ltb_attachments.is_empty(),
            "nothing has left the battlefield yet"
        );

        state
            .move_object(
                host,
                ZoneLocation::Graveyard(p0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        assert_eq!(
            state.ltb_attachments,
            vec![(host, vec![worn])],
            "the host left wearing something and the look-back says what"
        );

        state
            .move_object(
                host,
                ZoneLocation::Battlefield,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        assert!(
            state.ltb_attachments.is_empty(),
            "and the move that brings it back clears the pairing, or an \
             Equipment attached to nobody would go on granting to it"
        );
    }

    /// A move into a library says where in it the card went, and nothing
    /// else says a place (#300). `FromTop` is checked against where the card
    /// actually sits, and an index past either end reads as that end.
    #[test]
    fn a_move_into_a_library_names_where_in_it_the_card_went() {
        use crate::zone::ZonePosition as At;
        let mut state = GameState::from_preset(&make_preset(11), &RegistryLookup).unwrap();
        let p0 = PlayerId::new(0);
        let library = ZoneLocation::Library(p0);
        let card = state.draw_cards(p0, 1)[0];
        let place_of = |state: &mut GameState, to: ZoneLocation, at: At| {
            state
                .move_object(card, to, at, crate::event::Cause::Effect)
                .unwrap();
            match state.journal.entries().last().map(|e| &e.event) {
                Some(GameEvent::ZoneChanged { place, .. }) => *place,
                other => panic!("the move is journaled last, got {other:?}"),
            }
        };
        let hand = ZoneLocation::Hand(p0);
        assert_eq!(
            place_of(&mut state, library, At::Top),
            Some(LibraryPlace::Top)
        );
        place_of(&mut state, hand, At::Top);
        assert_eq!(
            place_of(&mut state, library, At::Bottom),
            Some(LibraryPlace::Bottom)
        );
        place_of(&mut state, hand, At::Top);
        let place = place_of(&mut state, library, At::Index(2));
        let list = state.zones.list(library);
        let from_top = list.len() - list.iter().position(|&o| o == card).unwrap();
        assert!(
            from_top > 1 && from_top < list.len(),
            "a card between the ends"
        );
        assert_eq!(
            place,
            Some(LibraryPlace::FromTop(u32::try_from(from_top).unwrap()))
        );
        place_of(&mut state, hand, At::Top);
        assert_eq!(
            place_of(&mut state, library, At::Index(0)),
            Some(LibraryPlace::Bottom)
        );
        place_of(&mut state, hand, At::Top);
        assert_eq!(
            place_of(&mut state, library, At::Index(usize::MAX)),
            Some(LibraryPlace::Top)
        );
        assert_eq!(
            place_of(&mut state, hand, At::Top),
            None,
            "a hand has no place"
        );
    }

    #[test]
    fn journal_records_setup() {
        let state = GameState::from_preset(&make_preset(1), &RegistryLookup).unwrap();
        assert!(matches!(
            state.journal.entries().first().map(|e| &e.event),
            Some(GameEvent::GameStarted { seed: 1, seats: 2 })
        ));
        assert!(
            state
                .journal
                .entries()
                .iter()
                .any(|e| matches!(e.event, GameEvent::Shuffled { .. }))
        );
        assert!(state.journal.entries().iter().any(|e| matches!(
            e.event,
            GameEvent::ZoneChanged {
                to: crate::zone::Zone::Hand,
                ..
            }
        )));
    }

    #[test]
    fn emblems_and_starting_battlefield_are_seeded() {
        let mut preset = make_preset(5);
        preset.seats[0].emblems = vec!["boss:test-emblem".to_string()];
        preset.seats[0].starting_battlefield = vec![DeckEntry {
            card: forest(),
            print: PrintRef::new(0),
        }];
        let state = GameState::from_preset(&preset, &RegistryLookup).unwrap();
        assert_eq!(
            state
                .zones
                .list(ZoneLocation::Command(PlayerId::new(0)))
                .len(),
            1
        );
        assert_eq!(state.zones.list(ZoneLocation::Battlefield).len(), 1);
        assert_eq!(
            state
                .object(state.zones.list(ZoneLocation::Battlefield)[0])
                .unwrap()
                .kind,
            ObjectKind::Permanent
        );
    }
    /// Two counters a one-byte tag would have collapsed hash apart.
    ///
    /// `CounterKind` says every +X/+Y counter in one variant, so the pair of
    /// numbers *is* the counter (CR 122.1a): a creature wearing a -0/-1 and
    /// one wearing a -1/-0 are two boards, and a determinism hash that could
    /// not tell them apart would let a replay diverge in silence. The
    /// equalities are the half that makes the inequalities worth anything —
    /// a hash that answered "different" to everything would pass the first
    /// three assertions on its own.
    #[test]
    fn a_counter_is_hashed_by_its_two_numbers_and_not_by_a_tag() {
        use baylee_cards_dsl::CounterKind as K;

        let with = |kind: K| {
            let mut state =
                GameState::from_preset(&make_preset(7), &RegistryLookup).expect("game starts");
            let owner = PlayerId::new(0);
            let name = state.names.intern("Test Permanent");
            let id = state.create_bare(
                owner,
                ObjectKind::Permanent,
                name,
                ZoneLocation::Battlefield,
            );
            state
                .object_mut(id)
                .expect("just created")
                .counters
                .add(kind, 1);
            state.snapshot_hash()
        };

        let toughness = with(K::Minus {
            power: 0,
            toughness: 1,
        });
        assert_ne!(
            toughness,
            with(K::Minus {
                power: 1,
                toughness: 0
            }),
            "-0/-1 and -1/-0 take different numbers off and are different counters"
        );
        assert_ne!(
            toughness,
            with(K::Plus {
                power: 0,
                toughness: 1
            }),
            "and the sign is part of the counter, not a way of reading it"
        );
        assert_ne!(
            toughness,
            with(K::Charge),
            "a counter with a word for a name is not one with numbers"
        );
        assert_eq!(
            toughness,
            with(K::Minus {
                power: 0,
                toughness: 1
            }),
            "the same counter on the same board is the same state"
        );
        assert_eq!(
            with(K::M1M1),
            with(K::Minus {
                power: 1,
                toughness: 1
            }),
            "and the constant is a spelling of the general form, not a second counter"
        );
    }

    /// A mana cost is hashed by every symbol it holds and how many of it,
    /// and by nothing else.
    ///
    /// The cost is counted (`ManaCost`), so the hash walks kinds, not
    /// symbols; each inequality is a way that walk could lose a cost: the
    /// count's high byte, the generic amount, which pair a hybrid names, the
    /// tag between two symbols that name one color, and no mana cost against
    /// `{0}` (an unpayable cost against a free one, CR 202.1b). The
    /// equalities hold the other half: one cost, however it was put
    /// together, is one state.
    #[test]
    fn a_mana_cost_is_hashed_by_its_symbols_and_their_counts() {
        use baylee_core::mana::ManaCost;
        let hash = |cost: &ManaCost| mana_cost_fingerprint(cost);
        let text = |text: &str| hash(&ManaCost::parse(text));
        let blue = ManaCost::parse("{U}");
        let blues = |n: u32| hash(&ManaCost::ZERO.combine_n(&blue, n));

        assert_ne!(blues(1), blues(257), "257 is 1 in its low byte");
        assert_ne!(blues(1), blues(2));
        assert_ne!(text("{1}"), text("{2}"));
        assert_ne!(text("{W/U}"), text("{U/B}"));
        assert_ne!(text("{2/W}"), text("{W/P}"));
        assert_ne!(text("{W}{U}"), text("{W/U}"));
        assert_ne!(hash(&ManaCost::ZERO), text("{0}"));

        assert_eq!(text("{U}{1}"), text("{1}{U}"), "written in any order");
        assert_eq!(
            text("{1}{1}{U}"),
            text("{2}{U}"),
            "generic mana is one amount"
        );
        assert_eq!(
            hash(&ManaCost::parse("{1}{U}").combine_n(&blue, 20)),
            text("{1}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}{U}"),
            "twenty payments added at once are the twenty written out"
        );
    }

    /// What an ability has already been used for this turn is part of the
    /// state a loop detector compares.
    ///
    /// "Activate only once each turn" makes the tally decide what is
    /// *offered*, so two boards alike in everything else are not the same
    /// board. Left out of [`GameState::loop_signature`], Brent's algorithm
    /// would call them one and could declare a draw on a game that still had
    /// a move in it.
    #[test]
    fn what_has_been_used_this_turn_is_part_of_the_loop_signature() {
        let mut state =
            GameState::from_preset(&make_preset(9), &RegistryLookup).expect("game starts");
        let owner = PlayerId::new(0);
        let name = state.names.intern("Test Permanent");
        let id = state.create_bare(
            owner,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );

        let untouched = state.loop_signature();
        state.ability_fires.insert((id, 0), 1);
        let spent = state.loop_signature();
        assert_ne!(
            untouched, spent,
            "an ability used once this turn is a different state from one used none"
        );
        state.ability_fires.insert((id, 0), 2);
        assert_ne!(
            spent,
            state.loop_signature(),
            "and the count matters, not merely the presence — `PerTurn(2)` exists"
        );
        state.ability_fires.clear();
        assert_eq!(
            untouched,
            state.loop_signature(),
            "cleared is back to where it started, which is what a turn boundary does"
        );
    }

    /// A two-seat game with one bare permanent on the battlefield: the
    /// object the snapshot-hash tests below change one thing about.
    fn hash_fixture() -> (GameState, ObjectId) {
        let mut state =
            GameState::from_preset(&make_preset(3), &RegistryLookup).expect("game starts");
        let name = state.names.intern("Test Permanent");
        let id = state.create_bare(
            PlayerId::new(0),
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        (state, id)
    }

    fn fixture_object(state: &mut GameState, id: ObjectId) -> &mut GameObject {
        state.object_mut(id).expect("the fixture's permanent")
    }

    /// Every field the snapshot hash was blind to until #122, one at a time.
    ///
    /// Each entry changes exactly one thing a later rule reads: an X on the
    /// stack, a land drop, a queued extra turn, a card outside the game. A
    /// hash that stays put across one of them calls two different games the
    /// same, which is what a replay or a cross-machine comparison then
    /// believes. The misses are collected rather than asserted one at a
    /// time, so a red run names every field it could not see.
    #[test]
    #[allow(clippy::too_many_lines)] // one entry per field
    fn every_field_that_decides_the_future_moves_the_snapshot_hash() {
        use crate::effects::{ContinuousEffect, EffectFilter};
        use crate::object::{AbilityList, Counters, PrintedFace, SecondInstance};
        use baylee_cards_dsl::{
            Filter, ReplacementRule, SpendRider, TargetReq, TargetSpec, TokenDef,
        };
        use baylee_core::color::ColorSet;
        use baylee_core::ids::SubtypeId;

        static TOKEN: TokenDef = TokenDef {
            name: "Test Token",
            ..TokenDef::DEFAULT
        };
        static REQ: TargetReq = TargetReq::one(TargetSpec::Object(&Filter::Any));

        type Mutation = (&'static str, fn(&mut GameState, ObjectId));
        let mutations: &[Mutation] = &[
            ("per_turn", |s, _| s.per_turn.creatures_died += 1),
            ("per_turn.life_lost", |s, _| s.per_turn.life_lost[0] = true),
            ("per_turn.no_more_spells", |s, _| {
                s.per_turn.no_more_spells[0] = true;
            }),
            ("per_turn.entered_graveyard", |s, id| {
                s.per_turn.entered_graveyard.push(id);
            }),
            ("per_turn.exile_if_dies", |s, id| {
                s.per_turn.exile_if_dies.push((id, 0));
            }),
            ("per_turn.entered_battlefield", |s, id| {
                s.per_turn.entered_battlefield.push(id);
            }),
            ("per_turn.drawn", |s, id| s.per_turn.drawn.push((id, 0))),
            ("per_turn.resolved", |s, id| {
                s.per_turn.note_resolution(id, 0, 0);
            }),
            ("per_turn.graveyard_plays", |s, id| {
                s.per_turn.graveyard_plays.push(GraveyardPlay {
                    player: PlayerId::new(0),
                    source: id,
                    version: 0,
                    types: baylee_core::types::TypeSet::LAND,
                });
            }),
            ("per_turn.playable", |s, id| {
                s.per_turn.playable.push(PlayPermission {
                    player: PlayerId::new(0),
                    card: id,
                    version: 0,
                    free: true,
                    cast_only: false,
                });
            }),
            ("delayed", |s, _| {
                s.delayed.push(DelayedTrigger {
                    controller: PlayerId::new(0),
                    when: DelayedWhen::NextUpkeep,
                    action: DelayedAction::AddMana {
                        color: ManaColor::Green,
                        amount: 1,
                    },
                });
            }),
            ("pending_miracle", |s, id| {
                s.pending_miracle.push_back((PlayerId::new(0), id));
            }),
            ("extra_turns", |s, _| {
                s.extra_turns.push_back(PlayerId::new(1));
            }),
            ("restriction_info", |s, id| {
                s.restriction_info
                    .insert(1, (id, &Filter::Any, SpendRider::None));
            }),
            ("next_restriction_id", |s, _| s.next_restriction_id += 1),
            ("ltb_abilities", |s, id| {
                s.ltb_abilities.push((id, AbilityList::NONE));
            }),
            ("ltb_attachments", |s, id| {
                s.ltb_attachments.push((id, Vec::new()));
            }),
            ("ltb_mana_values", |s, id| {
                s.ltb_mana_values.push((id, 3));
            }),
            ("ltb_controllers", |s, id| {
                s.ltb_controllers.push((id, PlayerId::new(1)));
            }),
            ("ltb_powers", |s, id| {
                s.ltb_powers.push((id, 4));
            }),
            ("synthetic_copies", |s, id| {
                s.synthetic_copies.push((id, id));
            }),
            ("ltb_counters", |s, id| {
                s.ltb_counters.push((id, Counters::default()));
            }),
            ("ltb_characteristics", |s, id| {
                let was = s
                    .object(id)
                    .expect("the test's object")
                    .base
                    .as_ref()
                    .clone();
                s.ltb_characteristics.push((id, was));
            }),
            ("monarch", |s, _| s.monarch = Some(PlayerId::new(1))),
            ("starting_player", |s, _| {
                s.starting_player = PlayerId::new(1);
            }),
            ("ability_fires", |s, id| {
                s.ability_fires.insert((id, 0), 1);
            }),
            ("replacement_rules", |s, id| {
                s.replacement_rules.push(ReplacementEntry {
                    source: id,
                    controller: PlayerId::new(0),
                    rule: ReplacementRule::DoubleTokenCreation {
                        controller_filter: &Filter::Any,
                    },
                });
            }),
            // An effect that came and went leaves the table as it found it
            // but for the next id it hands out, which is what the next
            // effect is then called.
            ("the effect table's next id", |s, _| {
                s.effects.register(ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: None,
                    controller: PlayerId::new(0),
                    origin: crate::effects::EffectOrigin::Resolution,
                    layer: baylee_cards_dsl::Layer::Text,
                    timestamp: 0,
                    duration: baylee_cards_dsl::Duration::Indefinitely,
                    filter: EffectFilter::Dsl(&Filter::Any),
                    modifier: baylee_cards_dsl::Modifier::ManaIsAnyColor,
                });
                s.effects.remove_where(|_| true);
            }),
            ("a card outside the game", |s, id| {
                s.zones.insert(
                    id,
                    ZoneLocation::OutsideGame(PlayerId::new(0)),
                    ZonePosition::Top,
                    false,
                );
            }),
            ("lands_played_this_turn", |s, _| {
                s.players[0].lands_played_this_turn += 1;
            }),
            ("tried_empty_draw", |s, _| {
                s.players[0].tried_empty_draw = true;
            }),
            ("x_value", |s, id| fixture_object(s, id).x_value = 3),
            ("kicked", |s, id| fixture_object(s, id).kicked = true),
            ("replicated", |s, id| fixture_object(s, id).replicated = 2),
            ("alt_cast", |s, id| fixture_object(s, id).alt_cast = true),
            ("chosen_player", |s, id| {
                fixture_object(s, id).chosen_player = Some(PlayerId::new(1));
            }),
            ("target_players", |s, id| {
                fixture_object(s, id)
                    .target_players
                    .insert(PlayerId::new(1));
            }),
            ("mode_index", |s, id| {
                fixture_object(s, id).mode_index = Some(1);
            }),
            ("modes", |s, id| {
                fixture_object(s, id).modes = 0b101;
            }),
            ("chosen_subtype", |s, id| {
                fixture_object(s, id).chosen_subtype = Some(SubtypeId::new(1));
            }),
            ("chosen_color", |s, id| {
                fixture_object(s, id).chosen_color = Some(ManaColor::Blue);
            }),
            ("chosen_name", |s, id| {
                fixture_object(s, id).chosen_name =
                    crate::object::PrintedFace::new(CardIndex::new(7), 0);
            }),
            ("doors", |s, id| {
                fixture_object(s, id).doors = crate::object::Doors::room(0b01);
            }),
            ("face_index", |s, id| fixture_object(s, id).face_index = 1),
            ("own_abilities", |s, id| {
                fixture_object(s, id).own_abilities = Some(&[]);
            }),
            ("own_abilities_until_eot", |s, id| {
                fixture_object(s, id).own_abilities_until_eot = true;
            }),
            ("own_face", |s, id| {
                fixture_object(s, id).own_face = PrintedFace::new(CardIndex::new(1), 0);
            }),
            ("token", |s, id| fixture_object(s, id).token = Some(&TOKEN)),
            ("pending_face_change", |s, id| {
                fixture_object(s, id).pending_face_change = Some(1);
            }),
            ("event_object", |s, id| {
                fixture_object(s, id).event_object = Some(id);
            }),
            ("event_amount", |s, id| {
                fixture_object(s, id).event_amount = core::num::NonZeroU16::new(3);
            }),
            ("cast_from_hand", |s, id| {
                let object = fixture_object(s, id);
                object.cast_from_hand = !object.cast_from_hand;
            }),
            ("target_req", |s, id| {
                fixture_object(s, id).target_req = Some(REQ);
            }),
            ("the second instance's requirement", |s, id| {
                fixture_object(s, id).second = Some(Box::new(SecondInstance {
                    targets: smallvec::SmallVec::new(),
                    req: Some(REQ),
                }));
            }),
            ("original_base", |s, id| {
                let object = fixture_object(s, id);
                object.original_base = Some(Arc::clone(&object.base));
            }),
            ("color_identity", |s, id| {
                fixture_object(s, id).base_mut().color_identity = ColorSet::ALL;
            }),
            ("produced_colors", |s, id| {
                fixture_object(s, id).base_mut().produced_colors = ColorSet::ALL;
            }),
            ("produced_colorless", |s, id| {
                fixture_object(s, id).base_mut().produced_colorless = true;
            }),
            ("produced_chosen", |s, id| {
                fixture_object(s, id).base_mut().produced_chosen = true;
            }),
            ("abilities_lost", |s, id| {
                fixture_object(s, id).base_mut().abilities_lost = std::num::NonZeroU32::new(7);
            }),
        ];

        let (base, id) = hash_fixture();
        let before = base.snapshot_hash();
        let blind: Vec<&str> = mutations
            .iter()
            .filter_map(|(field, mutate)| {
                let mut state = base.clone();
                mutate(&mut state, id);
                (state.snapshot_hash() == before).then_some(*field)
            })
            .collect();
        assert!(blind.is_empty(), "the snapshot hash cannot see {blind:?}");
    }

    /// A list of abilities no card prints (a token's, an emblem's) is
    /// hashed by what it says, because nothing else names it: the address
    /// of a `&'static` differs between builds and is shared between lists
    /// the compiler merged.
    #[test]
    fn a_list_no_card_prints_is_hashed_by_what_it_says() {
        let (mut state, id) = hash_fixture();
        fixture_object(&mut state, id).own_abilities = Some(&[]);
        let empty = state.snapshot_hash();
        let said = baylee_cards::by_index(force_of_will())
            .expect("the registry has Force of Will")
            .abilities;
        assert!(!said.is_empty(), "the list has to say something to differ");
        fixture_object(&mut state, id).own_abilities = Some(said);
        assert_ne!(
            state.snapshot_hash(),
            empty,
            "two unprinted lists that say different things are two states"
        );
    }

    /// The two hashed maps are summed entry by entry, so the order a map
    /// happens to iterate in cannot reach the hash. Laid out at two
    /// capacities the same entries iterate in a different order, which
    /// the test checks first: an equality between two maps that iterate
    /// alike would prove nothing.
    #[test]
    fn the_snapshot_hash_does_not_depend_on_the_order_a_map_iterates_in() {
        let (base, id) = hash_fixture();
        let entries: Vec<((ObjectId, u32), u32)> = (0..40).map(|i| ((id, i), i + 1)).collect();

        let mut small = base.clone();
        for (key, n) in &entries {
            small.ability_fires.insert(*key, *n);
        }
        let mut large = base.clone();
        large.ability_fires =
            rustc_hash::FxHashMap::with_capacity_and_hasher(4096, rustc_hash::FxBuildHasher);
        for (key, n) in entries.iter().rev() {
            large.ability_fires.insert(*key, *n);
        }

        let order = |s: &GameState| s.ability_fires.keys().copied().collect::<Vec<_>>();
        assert_ne!(
            order(&small),
            order(&large),
            "the two layouts must iterate differently, or this proves nothing"
        );
        assert_eq!(small.snapshot_hash(), large.snapshot_hash());
        assert_eq!(
            small.snapshot_hash(),
            small.snapshot_hash(),
            "and one state hashed twice is one hash"
        );
        let (again, _) = hash_fixture();
        assert_eq!(
            base.snapshot_hash(),
            again.snapshot_hash(),
            "two games built from one preset are one state"
        );
    }

    /// A four-seat table where seats 0 and 1 are a team, seat 2 is on a team
    /// of its own and seat 3 is on none at all.
    fn teamed_state() -> GameState {
        let mut preset = make_preset(5);
        let seat = preset.seats[0].clone();
        preset.seats = vec![seat.clone(), seat.clone(), seat.clone(), seat];
        preset.seats[0].team = Some(1);
        preset.seats[1].team = Some(1);
        preset.seats[2].team = Some(2);
        preset.seats[3].team = None;
        GameState::from_preset(&preset, &RegistryLookup).expect("a four-seat table")
    }

    /// CR 119.4 says "greater than or **equal** to the amount", so a player
    /// on exactly two life may pay two and lose to CR 704.5a a moment
    /// later. That is their call: this was written three times as
    /// `life <= amount → no`, which quietly took the last point of life off
    /// the table — a shockland entered tapped without asking and a
    /// fetchland was never offered. The margin belongs to the AI's own
    /// policy, not to a rule.
    #[test]
    fn the_last_point_of_life_is_still_payable() {
        let mut state = GameState::from_preset(&make_preset(1), &RegistryLookup).expect("a game");
        let me = PlayerId::new(0);
        state.players[0].life = 2;

        assert!(state.can_pay_life(me, 2), "exactly enough is enough");
        assert!(state.can_pay_life(me, 1));
        assert!(!state.can_pay_life(me, 3));

        state.players[0].life = 0;
        assert!(!state.can_pay_life(me, 1));
        assert!(
            state.can_pay_life(me, 0),
            "CR 119.4b: paying nothing is always possible"
        );
        state.players[0].life = -5;
        assert!(
            state.can_pay_life(me, 0),
            "including at a life total the game has not swept up yet"
        );
        assert!(
            state.can_pay_life(me, -1),
            "and a negative payment is not a payment"
        );
    }

    /// Every rule that says "opponent" goes through one predicate, and it
    /// asks the **side** rather than the seat: a teammate is another player
    /// and is not an opponent, which is the difference between "each
    /// opponent loses 1 life" and "each other player".
    #[test]
    fn a_teammate_is_another_player_and_not_an_opponent() {
        let state = teamed_state();
        let (a, b, c, d) = (
            PlayerId::new(0),
            PlayerId::new(1),
            PlayerId::new(2),
            PlayerId::new(3),
        );

        assert!(!state.is_opponent(a, a), "nobody is their own opponent");
        assert!(!state.is_opponent(b, a), "and neither is a teammate");
        assert!(!state.is_opponent(a, b), "which is true both ways round");
        assert!(state.is_opponent(c, a), "another team is");
        assert!(state.is_opponent(d, a), "and so is a seat on no team");
        assert!(
            state.is_opponent(d, c),
            "two seats that share no side are opponents however they got there"
        );
    }

    /// A seat with no team is a side of one, which is what makes a game
    /// with no teams at all a table of opponents without anything having to
    /// say so. Two such seats are two different sides even though both are
    /// `None`.
    #[test]
    fn a_seat_on_no_team_is_a_side_of_one() {
        let state = teamed_state();
        assert_eq!(state.side_of(PlayerId::new(0)), Side::Team(1));
        assert_eq!(state.side_of(PlayerId::new(1)), Side::Team(1));
        assert_eq!(state.side_of(PlayerId::new(2)), Side::Team(2));
        assert_eq!(
            state.side_of(PlayerId::new(3)),
            Side::Solo(PlayerId::new(3))
        );
        assert_ne!(
            state.side_of(PlayerId::new(3)),
            state.side_of(PlayerId::new(2)),
            "a lone seat is not on the team of every other lone seat"
        );

        let plain = GameState::from_preset(&make_preset(2), &RegistryLookup).expect("a game");
        assert!(plain.is_opponent(PlayerId::new(1), PlayerId::new(0)));
    }

    /// Names are rules identity rather than display text: the same spelling
    /// interns to one handle, so "is this the same name" is an integer
    /// compare — which is what a legend rule and a `Filter::NamedLike` both
    /// do thousands of times a game.
    #[test]
    fn one_spelling_is_one_name() {
        let mut names = Names::default();
        assert!(names.is_empty());

        let bolt = names.intern("Lightning Bolt");
        let again = names.intern("Lightning Bolt");
        let other = names.intern("Lightning Helix");

        assert_eq!(bolt, again, "one spelling, one handle");
        assert_ne!(bolt, other);
        assert_eq!(names.len(), 2, "and the second interning stored nothing");
        assert_eq!(names.get(bolt), "Lightning Bolt");
        assert_eq!(names.get(other), "Lightning Helix");
        assert!(!names.is_empty());

        // Case and whitespace are part of the spelling: this is identity,
        // not a search box.
        assert_ne!(names.intern("lightning bolt"), bolt);
        assert_ne!(names.intern("Lightning Bolt "), bolt);
    }
}
