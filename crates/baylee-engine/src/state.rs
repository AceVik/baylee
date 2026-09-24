//! Game state: the complete, cloneable, hashable world.

use std::hash::Hash;
use std::sync::Arc;

use crate::arena::Arena;
use crate::event::{Cause, GameEvent, Journal, LossReason};
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
    /// At the controller's next first main phase (Mana Drain).
    NextFirstMain,
    /// At the beginning of the next end step (Venser +2).
    NextEndStep,
    /// At the controller's next cleanup.
    NextCleanup,
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
    /// Return an exiled card to the battlefield under its owner's control
    /// (Venser +2).
    ReturnToBattlefield {
        /// The card in exile.
        card: ObjectId,
    },
}

/// Per-turn counters for conditional triggers (reset at every turn start).
#[derive(Clone, Hash, Debug)]
pub struct PerTurn {
    /// Noncreature spells cast this turn, per player.
    pub noncreature_spells: Vec<u32>,
    /// Cards drawn this turn, per player.
    pub draws: Vec<u32>,
    /// All spells cast this turn, per player (second-spell triggers).
    pub spells_cast: Vec<u32>,
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
}

impl PerTurn {
    /// Zeroed counters for `players` seats.
    #[must_use]
    pub fn new(players: usize) -> Self {
        Self {
            noncreature_spells: vec![0; players],
            spells_cast: vec![0; players],
            life_lost: vec![false; players],
            creatures_died: 0,
            draws: vec![0; players],
            entered_battlefield: Vec::new(),
        }
    }

    /// Resets all counters (called at every turn start).
    pub fn reset(&mut self) {
        self.noncreature_spells.iter_mut().for_each(|v| *v = 0);
        self.draws.iter_mut().for_each(|v| *v = 0);
        self.spells_cast.iter_mut().for_each(|v| *v = 0);
        self.life_lost.iter_mut().for_each(|v| *v = false);
        self.creatures_died = 0;
        self.entered_battlefield.clear();
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
}

/// A registered replacement rule from a permanent on the battlefield.
#[derive(Clone, Copy, Hash, Debug)]
pub struct ReplacementEntry {
    /// The source permanent.
    pub source: ObjectId,
    /// The rule's controller (for "you" in its filters).
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
    /// Two clauses share it because they are the same count: "this ability
    /// triggers only once each turn" (Jin-Gitaxias) and "activate only once
    /// each turn" (Wall of Roots). The key is the object and the ability
    /// index, so a permanent that leaves the battlefield and comes back
    /// starts over — CR 400.7 rather than a convenience.
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
            ltb_abilities: Vec::new(),
            ltb_counters: Vec::new(),
            ltb_attachments: Vec::new(),
            ceased: Vec::new(),
            reflexive: Vec::new(),
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
            token_cleanup: Vec::new(),
        };
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
            for &entry in &seat.starting_battlefield {
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
    fn restart_summoning_sickness(&mut self, id: ObjectId) {
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
    /// writes a counter, or puts a new permanent on the battlefield, calls
    /// this.
    pub const fn invalidate_projections(&mut self) {
        self.characteristics_generation = u64::MAX;
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
        if cross_zone || self.projected_cross_zone {
            ids.extend(self.arena.iter().map(|(id, _)| id));
        } else {
            // The stack contributes only its *spells*. An ability on the
            // stack has no characteristic a layer can touch, and this pass
            // runs once per engine step — walking six figures of Ally
            // triggers to establish that, every time a counter moves, is
            // the difference between a long game and no game at all.
            ids.extend(
                self.zones
                    .list(ZoneLocation::Battlefield)
                    .iter()
                    .chain(self.zones.stack_projectable().iter())
                    .copied(),
            );
        }
        for &id in &ids {
            let Some(obj) = self.object(id) else {
                continue;
            };
            if !crate::layers::needs_projection(&plan, obj) {
                // Nothing can change this object's characteristics, so the
                // base is the projection. Dropping the cache is not just
                // cheaper than recomputing it — it is what keeps an
                // untouched board's per-object projection memory at zero.
                let obj = self.object_mut(id).expect("checked above");
                obj.cache.clear();
                // No effects means no layer 2 either: whoever the base
                // says controls it does, which is how a "gain control
                // until end of turn" hands the permanent back.
                let moved = obj.controller != obj.base_controller;
                obj.controller = obj.base_controller;
                if moved {
                    self.restart_summoning_sickness(id);
                }
                continue;
            }
            let projection = crate::layers::recompute_with(self, obj, &plan);
            let obj = self.object_mut(id).expect("zone object exists");
            let moved = obj.controller != projection.controller;
            obj.controller = projection.controller;
            // `cache` and `base` are disjoint fields, so this is one
            // mutable borrow and one shared borrow of the same object.
            let crate::object::GameObject { cache, base, .. } = obj;
            cache.store(generation, projection.characteristics, base);
            if moved {
                self.restart_summoning_sickness(id);
            }
        }
        ids.clear();
        self.projection_ids = ids;
        self.projected_cross_zone = cross_zone;
        self.characteristics_generation = generation;
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

    /// Sets the monarch and releases monarch-linked exiles: when a player
    /// becomes monarch, cards exiled "until an opponent becomes monarch"
    /// (Palace Jailer) return if the new monarch is an opponent of the
    /// jailer's controller.
    pub fn set_monarch(&mut self, player: PlayerId) {
        let previous = self.monarch;
        self.monarch = Some(player);
        if previous == Some(player) {
            return;
        }
        // Monarch-link releases (Palace Jailer): return cards whose host's
        // controller is not the new monarch.
        let mut returning = Vec::new();
        for seat in 0..self.players.len() {
            let p = PlayerId::new(seat as u8);
            for &card in self.zones.list(ZoneLocation::Exile(p)) {
                if let Some(host) = self.object(card).and_then(|o| {
                    o.riders.iter().find_map(|r| match r {
                        crate::object::Rider::Linked { host } => Some(host),
                        _ => None,
                    })
                }) {
                    let host_controller = self.object(*host).map_or(player, |h| h.controller);
                    if host_controller != player {
                        returning.push(card);
                    }
                }
            }
        }
        for card in returning {
            if let Some(obj) = self.object_mut(card) {
                obj.kind = crate::object::ObjectKind::Permanent;
            }
            let _ = self.move_object(
                card,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            );
        }
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
        // What was attached to it.
        self.ltb_attachments.retain(|(other, _)| *other != id);
        if from_zone == Zone::Battlefield {
            let worn: Vec<ObjectId> = self
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .filter(|other| {
                    self.object(**other)
                        .is_some_and(|o| o.attached_to == Some(id))
                })
                .copied()
                .collect();
            if !worn.is_empty() {
                self.ltb_attachments.push((id, worn));
            }
        }
    }

    /// Moves an object between zones (CR 400.7: `version` bumps — it
    /// becomes a new object for rules that track identity).
    ///
    /// # Errors
    /// [`StateError::NoSuchObject`] for stale or unknown handles.
    ///
    /// # Panics
    /// Internal invariant violations (existence is checked above).
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
        let from_loc = ZoneLocation::of(from_zone, from_player);
        self.zones.remove(id, from_loc);
        self.timestamp += 1;
        let ts = self.timestamp;
        self.record_last_known(id, from_zone);
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
            // rule is written around), and the spell-shaped fields
            // (`x_value`, `kicked`, `targets`), which a permanent resolving
            // off the stack still needs and which no permanent writes.
            if from_zone == Zone::Battlefield {
                obj.status = crate::object::Status::NONE;
                obj.damage = 0;
                obj.deathtouched = false;
                obj.regeneration_shields = 0;
                obj.counters = crate::object::Counters::default();
                obj.attached_to = None;
            }
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
            if obj.card.is_some() {
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
        // Only the battlefield and the stack, which is exactly what the
        // refresh pass revisits — a card drawn changes no projection unless
        // a cross-zone effect is registered, and that pass projects every
        // zone anyway.
        if matches!(from_zone, Zone::Battlefield | Zone::Stack)
            || matches!(to.zone(), Zone::Battlefield | Zone::Stack)
        {
            self.invalidate_projections();
        }
        if to.zone() == Zone::Battlefield {
            self.per_turn.entered_battlefield.push(id);
        }
        self.journal.record(GameEvent::ZoneChanged {
            object: id,
            from: from_zone,
            to: to.zone(),
            cause,
        });
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
            self.journal.record(crate::event::GameEvent::CardsDrawn {
                player,
                count: drawn.len() as u16,
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
            ltb_abilities,
            ltb_attachments,
            ltb_counters,
            // Empty again before any question is out; the field says why.
            ceased: _,
            // Empty whenever a question is out, by a rule the build
            // enforces; the field says which.
            reflexive: _,
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
        h.usize(self.players.len());
        for p in &self.players {
            h.u8(p.id.get());
            h.i32(p.life);
            h.u16(p.poison);
            h.u16(p.energy);
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
        h.usize(self.combat.attackers.len());
        for a in &self.combat.attackers {
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
    let (effects, next_id, generation) = table.hashed_parts();
    next_id.hash(h);
    generation.hash(h);
    h.usize(effects.len());
    for fx in effects {
        let crate::effects::ContinuousEffect {
            id,
            source,
            controller,
            layer,
            timestamp,
            duration,
            filter,
            modifier,
        } = fx;
        id.hash(h);
        source.hash(h);
        controller.hash(h);
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
}

fn hash_player(h: &mut Hasher, player: &Player) {
    let Player {
        id,
        life,
        poison,
        energy,
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
        x_value,
        kicked,
        alt_cast,
        chosen_player,
        target_players,
        mode_index,
        chosen_subtype,
        chosen_color,
        face_index,
        own_abilities,
        own_abilities_until_eot,
        own_face,
        token,
        pending_face_change,
        event_object,
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
            Rider::Linked { host } => {
                h.u8(1);
                host.hash(h);
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
    x_value.hash(h);
    kicked.hash(h);
    alt_cast.hash(h);
    chosen_player.hash(h);
    target_players.hash(h);
    mode_index.hash(h);
    chosen_subtype.hash(h);
    chosen_color.hash(h);
    face_index.hash(h);
    pending_face_change.hash(h);
    event_object.hash(h);
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
        // Its own tag rather than a payload on `CmcAtMost`: the bound is
        // read from the source at match time, so two filters that differ
        // only in *where* the number comes from are different filters.
        F::CmcAtMostX => h.u8(28),
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

fn hash_mana_cost(h: &mut Hasher, cost: &baylee_core::mana::ManaCost) {
    h.u8(cost.len());
    for s in cost.symbols() {
        match s {
            ManaSymbol::Generic(n) => {
                h.u8(0);
                h.u32(n);
            }
            ManaSymbol::Colorless => h.u8(1),
            ManaSymbol::White => h.u8(2),
            ManaSymbol::Blue => h.u8(3),
            ManaSymbol::Black => h.u8(4),
            ManaSymbol::Red => h.u8(5),
            ManaSymbol::Green => h.u8(6),
            ManaSymbol::Hybrid(p) => {
                h.u8(7);
                h.u8(p.first() as u8);
                h.u8(p.second() as u8);
            }
            ManaSymbol::TwoOrColor(c) => {
                h.u8(8);
                h.u8(c as u8);
            }
            ManaSymbol::Phyrexian(c) => {
                h.u8(9);
                h.u8(c as u8);
            }
            ManaSymbol::HybridPhyrexian(p) => {
                h.u8(10);
                h.u8(p.first() as u8);
                h.u8(p.second() as u8);
            }
            ManaSymbol::Snow => h.u8(11),
            ManaSymbol::Variable(v) => {
                h.u8(12);
                h.u8(v as u8);
            }
            ManaSymbol::HalfGeneric => h.u8(13),
            ManaSymbol::Infinite => h.u8(14),
        }
    }
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
            ("per_turn.entered_battlefield", |s, id| {
                s.per_turn.entered_battlefield.push(id);
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
            ("ltb_counters", |s, id| {
                s.ltb_counters.push((id, Counters::default()));
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
            ("chosen_subtype", |s, id| {
                fixture_object(s, id).chosen_subtype = Some(SubtypeId::new(1));
            }),
            ("chosen_color", |s, id| {
                fixture_object(s, id).chosen_color = Some(ManaColor::Blue);
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
