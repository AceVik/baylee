//! Game state: the complete, cloneable, hashable world.

use std::hash::Hash;
use std::sync::Arc;

use crate::arena::Arena;
use crate::event::{Cause, GameEvent, Journal, LibraryPlace, LossReason};
use crate::hasher::Hasher;
use crate::object::{
    CardRef, Characteristics, CounterKind, GameObject, ObjectKind, PrintedFace, Rider,
};
use crate::rng::GameRng;
use crate::turn::{DayNight, TurnInfo};
use crate::zone::{Zone, ZoneLocation, ZonePosition, Zones};
use baylee_cards_dsl::CardDef;
use baylee_core::ids::{CardIndex, Defender, NameRef, ObjectId, PlayerId};
use baylee_core::mana::{ManaPool, ManaSymbol};
use baylee_core::preset::{FormatId, GamePreset, PresetError};
use rustc_hash::FxHashMap;

mod delayed;
mod discard;
mod draw_offer;
mod entry_history;
pub use draw_offer::{DrawReplacement, FrontDraw};
mod draw_wish;
mod hash;
mod hidden_exile;
mod lifetimes;
mod merged;
mod moves;
mod objects;
mod per_turn;
mod player_counters;
mod players;
mod projection;
mod reads;
mod setup;
#[cfg(test)]
mod tests;
mod upkeep_history;

pub(crate) use hash::{
    SnapshotMemo, characteristics_fingerprint, mana_cost_fingerprint, structural_fingerprint,
};

pub use delayed::{DelayedAction, DelayedTrigger, DelayedWhen, PlayerTurnControl};
pub use per_turn::{
    AbilityResolved, BlockRecord, DamageLogEntry, GraveyardPlay, PerTurn, PlayLifetime,
    PlayPermission, PreviousTurn,
};
use reads::printed_pt_cda;
pub(crate) use reads::{
    filter_reaches_other_zones, filter_reads_board_state, filter_reads_reference,
    modifier_reads_combat,
};

/// Registry seam: the engine resolves card definitions through this trait
/// and never depends on the compiled registry directly — a future runtime
/// card pack (custom cards, bosses) implements the same seam.
pub trait CardLookup {
    /// Resolves a card index to its definition.
    fn card(&self, index: CardIndex) -> Option<&'static CardDef>;
    /// Stable presentation id of a registry token; `u16::MAX` is reserved.
    /// Custom lookups may omit identities without affecting token rules.
    fn token_id(&self, _token: &baylee_cards_dsl::TokenDef) -> Option<u16> {
        None
    }
}

/// A seat's mutable state.
#[derive(Clone, Debug)]
#[allow(clippy::struct_excessive_bools)] // Independent per-seat facts, not a state machine.
pub struct Player {
    /// Seat handle.
    pub id: PlayerId,
    /// Life total.
    pub life: i32,
    /// Poison counters.
    pub poison: u16,
    /// Energy counters.
    pub energy: u16,
    /// Persistent speed designation: zero means not started, four is maximum.
    pub speed: u8,
    /// Radiation counters (CR 728).
    pub rad: u16,
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
    /// Number of this player's turns that actually began.
    pub turns_started: u32,
    /// Independent obligations to skip the next turn; one is consumed per omitted turn.
    pub turns_to_skip: u32,
    /// Exact attackers from this seat's most recently completed actual turn.
    pub last_turn_attackers: Vec<baylee_core::ids::DamageSourceRef>,
    /// Exact incarnations that blocked or became blocked since this player's latest upkeep.
    pub blocked_since_upkeep: Vec<baylee_core::ids::DamageSourceRef>,
    /// The completed interval retained during the player's current upkeep.
    pub blocked_before_upkeep: Vec<baylee_core::ids::DamageSourceRef>,
    /// Whether this player cast a spell or put a nontoken permanent onto the
    /// battlefield during their most recently completed turn (Arboria's
    /// "during their last turn"); false before their first turn ends.
    pub last_turn_acted: bool,
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
    /// Active dungeon and its command-zone object; absent outside a dungeon.
    pub dungeon: Option<(baylee_core::dungeon::Dungeon, ObjectId)>,
    /// One-based room index; zero outside a dungeon.
    pub dungeon_room: u8,
    /// Number of dungeons this player has completed this game.
    pub dungeons_completed: u8,
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
///
/// It only grows, and a clone shares it: a decision checkpoint clones the
/// state on every answer, and copying every interned name twice (map and
/// list) was over a third of that clone in self-play. A name it has never
/// seen copies the table once, then appends in place.
#[derive(Clone, Debug, Default)]
pub struct Names(Arc<NameTable>);

#[derive(Clone, Debug, Default)]
struct NameTable {
    map: FxHashMap<Arc<str>, NameRef>,
    list: Vec<Arc<str>>,
}

impl Names {
    /// Interns a name.
    pub fn intern(&mut self, name: &str) -> NameRef {
        if let Some(&id) = self.0.map.get(name) {
            return id;
        }
        let table = Arc::make_mut(&mut self.0);
        let id = NameRef::new(table.list.len() as u32);
        let owned: Arc<str> = Arc::from(name);
        table.list.push(Arc::clone(&owned));
        table.map.insert(owned, id);
        id
    }

    /// Resolves a name.
    #[must_use]
    pub fn get(&self, id: NameRef) -> &str {
        &self.0.list[id.get() as usize]
    }

    /// The interned `name`, without interning it: `None` when no object of
    /// this game has ever carried it, and so none carries it now.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<NameRef> {
        self.0.map.get(name).copied()
    }

    /// Number of interned names.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.list.len()
    }

    /// Whether no names are interned.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.list.is_empty()
    }
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
    /// Immutable definitions already admitted through the registry seam.
    rules: FxHashMap<CardIndex, &'static CardDef>,
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

impl CardLookup for BaseCache {
    fn card(&self, index: CardIndex) -> Option<&'static CardDef> {
        self.rules.get(&index).copied()
    }
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
    /// Format-specific rules, including access to cards outside the game.
    pub format: FormatId,
    /// All game objects.
    pub arena: Arena<GameObject>,
    /// Ordered zone contents.
    pub zones: Zones,
    /// Choices owed for simultaneous graveyard arrivals.
    pub(crate) graveyard_order: crate::graveyard_order::Ordering,
    /// Moves suspended before battlefield entry and their exact continuations.
    pub(crate) entries: crate::entry::Entries,
    /// Legend decisions waiting for their simultaneous SBA pass.
    pub(crate) sba_legend_decisions: Vec<crate::sba::LegendDecision>,
    /// Competing regeneration replacements awaiting the affected controller.
    pub(crate) regeneration_choices: crate::sba::RegenerationChoices,
    /// Shield-counter replacement choices awaiting the affected controller.
    pub(crate) shield_choices: crate::sba::ShieldChoices,
    /// Pending choices between damage clearing and other destruction replacements.
    pub(crate) damage_clearing_choices: crate::sba::DamageClearingChoices,
    /// Simultaneously staged battle protector decisions awaiting the SBA pass.
    pub(crate) battle_sba_decisions: Vec<crate::battle::sba::ProtectorDecision>,
    /// Counter-placement replacements awaiting their affected controller's order.
    pub(crate) counter_choices: crate::replacement::counter_order::CounterChoices,
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
    /// Exact-card play permissions, some lasting across several turns.
    pub play_permissions: Vec<PlayPermission>,
    /// Registered delayed triggers (suspend finishes, pact payments).
    pub delayed: Vec<DelayedTrigger>,
    /// Lands marked by a particular source incarnation; survives its departure.
    pub counter_links: Vec<crate::resolve::linked_counters::CounterLink>,
    /// Source identities immediately before battlefield departures.
    pub ltb_versions: Vec<(ObjectId, u32)>,
    /// Death-time information for damage-history triggers, until collection.
    pub(crate) damage_deaths: Vec<crate::damage_history::DamageDeath>,
    /// First-of-turn drawn cards awaiting a miracle offer (CR 702.94).
    pub pending_miracle: std::collections::VecDeque<(PlayerId, ObjectId)>,
    /// Draws Island Sanctuary may replace, waiting to be offered the skip
    /// (`ReplacementRule::MaySkipDrawStepDraw`): each entry is a player and
    /// how many cards an instruction told them to draw, in the order the
    /// draws would be made (CR 121.2, 121.2c). `draw_cards` puts a draw
    /// here instead of making it when the drawing player could skip it,
    /// and behind one already waiting so the order holds; the resolution
    /// (`resolve::run`) or, outside one, the machine asks each draw on its
    /// own (CR 121.2, 614.11a). Empty whenever nobody is being asked.
    pub draws_to_offer: std::collections::VecDeque<(PlayerId, u32)>,
    /// What the replacements have done so far to the draw at the front of
    /// [`Self::draws_to_offer`]: the optional ones its player declined and
    /// the Chains of Mephistopheles already applied to it (CR 614.5), and
    /// the cards an Aladdin's Lamp is looking at. Default between draws.
    pub draw_front: draw_offer::FrontDraw,
    /// How many waiting draws a replacement has modified, ever: an
    /// instruction that does something to "the card drawn" reads it to
    /// tell a plain draw from a replaced one (CR 614.11b).
    pub draws_replaced: u32,
    /// Library of Leng's answers for cards an effect is about to discard
    /// (`ReplacementRule::MayDiscardToLibraryTop`): `None` for the
    /// graveyard, `Some((rank, reveal))` for the library, rank 0 the top
    /// card. Asked before the discard and spent by it
    /// (`GameState::discard_card`), like `commander_redirect`.
    pub discard_answers: Vec<(ObjectId, Option<(usize, bool)>)>,
    /// The cards of the latest answer already put on top of a library, with
    /// their ranks, so the next one goes in under the ones it was ranked
    /// below. Cleared with each new answer.
    pub discards_on_top: Vec<(ObjectId, usize)>,
    /// Queued extra turns (CR 500.7), in the order they will be taken: the
    /// most recently created at the front. The front player takes the next
    /// turn instead of the normal successor.
    pub extra_turns: std::collections::VecDeque<PlayerId>,
    /// Players controlling other players during their next actual turn
    /// (CR 722.1), waiting for that turn or in it.
    pub turn_controls: Vec<crate::state::PlayerTurnControl>,
    /// While the current turn is an extra one, the player whose normal
    /// turn it was added after: "the turns directly after the specified
    /// turn" (CR 500.7), so the normal order resumes from that turn, not
    /// from the extra one. `None` during a normal turn, which the order
    /// goes on from itself.
    pub resume_after: Option<PlayerId>,
    /// Permanents a skipped turn left to untap (Time Vault). "Some effects
    /// cause a player to skip a step, phase, or turn, then take another
    /// action. That action is considered to be the first thing that happens
    /// during the next step, phase, or turn to actually occur" (CR 614.10b),
    /// so they wait here until a turn begins rather than untapping between
    /// turns. Each is the object and its version when the turn was skipped
    /// (CR 400.7): one that has left the battlefield since is not untapped.
    pub skip_followups: Vec<(ObjectId, u32)>,
    /// Auras whose enchant ability an effect has changed (Animate Dead):
    /// each binds one Aura incarnation and, once the card it returned has
    /// entered, that creature's incarnation (CR 303.4c, 400.7).
    pub(crate) reanimated_auras: Vec<crate::aura_bindings::ReanimatedAura>,
    /// Returns an Aura made that still owe their attachment: made once the
    /// returned card has entered, with its as-it-enters choices and statics
    /// in place, and before any state-based action looks at the board.
    pub(crate) reanimation_finishes: Vec<crate::aura_bindings::ReanimationFinish>,
    /// Each mana batch retains its restriction and its producing ability's choices.
    pub restriction_info: rustc_hash::FxHashMap<u32, crate::mana_provenance::ManaProvenance>,
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
    /// Who a permanent was in combat with the moment it left the
    /// battlefield: each partner as the object it was, and whether that
    /// partner was blocking it (`true`) or blocked by it (`false`). Written
    /// and cleared where [`Self::ltb_characteristics`] is;
    /// `Filter::BlockingSource` and `BlockedBySource` read it once the
    /// source is gone (Abu Ja'far).
    pub ltb_combat: Vec<(ObjectId, Vec<(baylee_core::ids::DamageSourceRef, bool)>)>,
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
    /// Announced damage shares for live spells and abilities. Exact targets
    /// prevent a later incarnation receiving the old object's damage.
    /// `SourceMemory` retains these decisions while a departed spell can be copied.
    pub divided: Vec<(ObjectId, Vec<(baylee_core::ids::TargetRef, u32)>)>,
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
    /// The monarch designation (CR 724), if any.
    pub monarch: Option<PlayerId>,
    /// The game as an effect ended it: "you win the game" (CR 104.2b) or
    /// "the game is a draw" (CR 104.4c). The game ends at once (CR 104.1):
    /// the engine reads this before anything else when it next looks.
    pub game_end: Option<crate::win::GameResult>,
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
    /// Five clauses share it because each is one ability's count of one
    /// thing it does in a turn: "this ability triggers only once each turn"
    /// (Jin-Gitaxias), "activate only once each turn" (Wall of Roots), "do
    /// this only once each turn" (The Reaper, King No More: set by the yes),
    /// "if this is the first time this ability has resolved this turn"
    /// (Omnath, Locus of Creation: its resolutions) and "if this ability has
    /// been activated four or more times this turn" (Dragon Whelp: its
    /// activations, as "activate only once each turn" counts them). No
    /// ability says two of them. The key is the exact object incarnation and
    /// ability index. A returning permanent starts over (CR 400.7), while
    /// waiting abilities retain the departed source's count (CR 608.2h–i).
    ///
    /// It is hashed into [`Self::loop_signature`], because what is left of a
    /// limit decides what is offered. `Engine::loyalty_used_this_turn` is
    /// the same kind of state and is **not** hashed, because it does not
    /// live here; that is a known hole and not this field's.
    pub ability_fires: rustc_hash::FxHashMap<(baylee_core::ids::DamageSourceRef, u32), u32>,
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
    /// Typed text substitutions on exact spell/permanent/ability incarnations.
    pub text_changes: crate::text_changes::TextChanges,
    /// A technical failure, consumed and rolled back at the Engine boundary.
    pub(crate) numeric_failure: Option<&'static str>,
    /// Exact mana obligations of nested controlled card plays.
    pub(crate) constrained_payments: Vec<crate::constrained_payment::ConstrainedPayment>,
    /// Frozen copiable characteristics of temporary copy effects.
    pub copy_snapshots: Vec<(baylee_core::ids::EffectId, Arc<Characteristics>)>,
    /// Sparse original rules and casting decisions restored when temporary copies expire.
    pub(crate) temporary_copy_origins: Vec<(baylee_core::ids::DamageSourceRef, Box<GameObject>)>,
    /// Text provenance that differs from an effect's ordinary source.
    pub effect_text_overrides: Vec<(baylee_core::ids::EffectId, crate::text_changes::TextOrigin)>,
    /// Registered replacement rules (Doubling Season, Panharmonicon, …).
    pub replacement_rules: Vec<ReplacementEntry>,
    /// Prevention shields resolved spells and abilities left behind
    /// (CR 615), in the order they were made; every one ends at the turn's
    /// cleanup (CR 514.2). See [`crate::prevention`].
    pub shields: crate::prevention::ShieldStore,
    /// Temporary special actions created by resolving effects.
    pub granted_actions: Vec<crate::granted::GrantedAction>,
    pub(crate) next_granted_action: u64,
    /// Identity for the next simultaneous damage event.
    pub(crate) next_damage_batch: u64,
    /// Source incarnations before leaving a public rules zone. Pending
    /// abilities and chosen-source shields can still name those incarnations
    /// after a second zone change; the ordinary last-move LKI cannot.
    pub(crate) damage_sources: Vec<Arc<GameObject>>,
    /// Exact references retained by live rules objects.
    pub(crate) source_memory: crate::sources::SourceMemory,
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
    /// Derived small index of physical cards declaring off-battlefield static abilities.
    /// Populated once at card creation; active zone and incarnation are read live.
    pub(crate) offboard_static_candidates: Vec<ObjectId>,
    /// Sparse candidates for pruning completed linked reanimations after trigger collection.
    pub(crate) reanimation_sources: Vec<baylee_core::ids::DamageSourceRef>,
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
    /// What [`Self::snapshot_hash`] last wrote for each arena chunk. A cache
    /// of the hash's own bytes, never rules state; a clone starts without
    /// one ([`SnapshotMemo`]).
    snapshot_memo: SnapshotMemo,
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
            format,
            arena,
            zones,
            graveyard_order,
            entries,
            sba_legend_decisions,
            regeneration_choices,
            shield_choices,
            damage_clearing_choices,
            battle_sba_decisions,
            counter_choices,
            players,
            turn,
            combat,
            per_turn,
            play_permissions,
            delayed,
            counter_links,
            ltb_versions,
            damage_deaths,
            pending_miracle,
            draws_to_offer,
            draw_front,
            draws_replaced,
            discard_answers,
            discards_on_top,
            extra_turns,
            turn_controls,
            resume_after,
            skip_followups,
            reanimated_auras,
            reanimation_finishes,
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
            ltb_combat,
            ceased,
            reflexive,
            discovered,
            divided,
            synthetic_copies,
            commanders,
            monarch,
            game_end,
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
            text_changes,
            effect_text_overrides,
            copy_snapshots,
            temporary_copy_origins,
            numeric_failure,
            constrained_payments,
            replacement_rules,
            shields,
            granted_actions,
            next_granted_action,
            next_damage_batch,
            damage_sources,
            source_memory,
            characteristics_generation,
            projection_ids,
            projected_cross_zone,
            token_cleanup,
            printed_pt_cda,
            offboard_static_candidates,
            reanimation_sources,
            // The snapshot hash's cache of its own bytes, not state.
            snapshot_memo: _,
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
            ("state.format", format!("{format:?}")),
            ("state.zones", format!("{zones:?}")),
            ("state.graveyard_order", format!("{graveyard_order:?}")),
            ("state.entries", format!("{entries:?}")),
            (
                "state.sba_legend_decisions",
                format!("{sba_legend_decisions:?}"),
            ),
            (
                "state.regeneration_choices",
                format!("{regeneration_choices:?}"),
            ),
            ("state.shield_choices", format!("{shield_choices:?}")),
            (
                "state.battle_sba_decisions",
                format!("{battle_sba_decisions:?}"),
            ),
            (
                "state.damage_clearing_choices",
                format!("{damage_clearing_choices:?}"),
            ),
            ("state.counter_choices", format!("{counter_choices:?}")),
            ("state.players", format!("{players:?}")),
            ("state.turn", format!("{turn:?}")),
            ("state.combat", format!("{combat:?}")),
            ("state.per_turn", format!("{per_turn:?}")),
            ("state.play_permissions", format!("{play_permissions:?}")),
            ("state.delayed", format!("{delayed:?}")),
            ("state.counter_links", format!("{counter_links:?}")),
            ("state.ltb_versions", format!("{ltb_versions:?}")),
            ("state.damage_deaths", format!("{damage_deaths:?}")),
            ("state.pending_miracle", format!("{pending_miracle:?}")),
            ("state.draws_to_offer", format!("{draws_to_offer:?}")),
            ("state.draw_front", format!("{draw_front:?}")),
            ("state.draws_replaced", format!("{draws_replaced:?}")),
            ("state.discard_answers", format!("{discard_answers:?}")),
            ("state.discards_on_top", format!("{discards_on_top:?}")),
            ("state.extra_turns", format!("{extra_turns:?}")),
            ("state.turn_controls", format!("{turn_controls:?}")),
            ("state.resume_after", format!("{resume_after:?}")),
            ("state.skip_followups", format!("{skip_followups:?}")),
            ("state.reanimated_auras", format!("{reanimated_auras:?}")),
            (
                "state.reanimation_finishes",
                format!("{reanimation_finishes:?}"),
            ),
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
            ("state.ltb_combat", format!("{ltb_combat:?}")),
            ("state.ceased", format!("{ceased:?}")),
            ("state.reflexive", format!("{reflexive:?}")),
            ("state.discovered", format!("{discovered:?}")),
            ("state.divided", format!("{divided:?}")),
            ("state.synthetic_copies", format!("{synthetic_copies:?}")),
            ("state.commanders", format!("{commanders:?}")),
            ("state.monarch", format!("{monarch:?}")),
            ("state.game_end", format!("{game_end:?}")),
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
            ("state.text_changes", format!("{text_changes:?}")),
            (
                "state.effect_text_overrides",
                format!("{effect_text_overrides:?}"),
            ),
            ("state.numeric_failure", format!("{numeric_failure:?}")),
            (
                "state.constrained_payments",
                format!("{constrained_payments:?}"),
            ),
            ("state.copy_snapshots", format!("{copy_snapshots:?}")),
            (
                "state.temporary_copy_origins",
                format!("{temporary_copy_origins:?}"),
            ),
            ("state.replacement_rules", format!("{replacement_rules:?}")),
            ("state.shields", format!("{shields:?}")),
            ("state.granted_actions", format!("{granted_actions:?}")),
            (
                "state.next_granted_action",
                format!("{next_granted_action:?}"),
            ),
            ("state.next_damage_batch", format!("{next_damage_batch:?}")),
            ("state.damage_sources", format!("{damage_sources:?}")),
            ("state.source_memory", format!("{source_memory:?}")),
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
            (
                "state.offboard_static_candidates",
                format!("{offboard_static_candidates:?}"),
            ),
            (
                "state.reanimation_sources",
                format!("{reanimation_sources:?}"),
            ),
        ]);
    }
}

/// The game as it stood before a payment began, to put back if the payment
/// cannot finish ([`GameState::checkpoint`]).
pub(crate) struct Checkpoint {
    state: Box<GameState>,
    journal: usize,
}

impl Checkpoint {
    /// How long the journal was when the game was kept.
    pub(crate) const fn journal_len(&self) -> usize {
        self.journal
    }
}

impl GameState {
    /// Distinct colors of the designated commander cards, wherever they are.
    /// Copy effects do not change a commander's color identity (CR 903.4).
    #[must_use]
    pub fn commander_colors(&self, player: PlayerId) -> baylee_core::color::ColorSet {
        self.commanders
            .get(usize::from(player.get()))
            .into_iter()
            .flatten()
            .filter_map(|commander| self.object(commander.object))
            .fold(baylee_core::color::ColorSet::EMPTY, |colors, object| {
                colors.union(
                    object
                        .original_base
                        .as_deref()
                        .unwrap_or(&object.base)
                        .color_identity,
                )
            })
    }

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
    /// [`Self::roll_back`] to a checkpoint that is kept, not spent: the
    /// game is copied out of it.
    pub(crate) fn roll_back_to(&mut self, to: &Checkpoint) {
        let mut journal = std::mem::take(&mut self.journal);
        journal.cancel_from(to.journal);
        *self = (*to.state).clone();
        self.journal = journal;
    }

    pub(crate) fn roll_back(&mut self, to: Checkpoint) {
        let mut journal = std::mem::take(&mut self.journal);
        journal.cancel_from(to.journal);
        *self = *to.state;
        self.journal = journal;
    }
}

impl GameState {
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
        // A resolving instruction continues after its controller leaves
        // (CR 608.2m), but the departed seat's retained information must
        // remain what it was immediately before leaving (CR 800.4i).
        if self.has_left(player)
            || self.life_total_cant_change(player)
            || (by < 0 && self.cant_lose_life(player))
        {
            return;
        }
        // Lich: the gain is replaced by that many draws (CR 614.1a), so no
        // life changes and no gain is recorded for a gain trigger to read.
        // One replacement however many effects say it (CR 614.5).
        if by > 0 && self.life_gain_draws_instead(player) {
            self.draw_cards(player, usize::try_from(by).unwrap_or(0));
            return;
        }
        let seat = player.get() as usize;
        let p = &mut self.players[seat];
        let old = p.life;
        let Some(new) = old.checked_add(by) else {
            self.numeric_failure = Some("life total exceeds i32");
            return;
        };
        p.life = new;
        if new < old {
            self.per_turn.life_lost[seat] = true;
            let lost = u32::try_from(i64::from(old) - i64::from(new)).unwrap_or(u32::MAX);
            self.per_turn.life_lost_amount[seat] =
                self.per_turn.life_lost_amount[seat].saturating_add(lost);
        }
        self.journal.record(GameEvent::LifeChanged {
            player,
            old,
            new,
            cause,
        });
        let active = self.turn.active;
        let index = usize::from(active.get());
        if new < old
            && self.is_opponent(active, player)
            && (1..4).contains(&self.players[index].speed)
            && !self.players[index].has_lost()
            && !self.per_turn.speed_triggered[index]
        {
            self.per_turn.speed_triggered[index] = true;
            self.journal
                .record(GameEvent::SpeedTriggered { player: active });
        }
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
        if !tapped
            && self.object(id).is_some_and(|o| {
                o.zone == Zone::Battlefield
                    && o.status.contains(crate::object::Status::TAPPED)
                    && o.counters.get(CounterKind::Stun) > 0
            })
        {
            crate::replacement::remove_counters(self, id, CounterKind::Stun, 1);
            return false;
        }
        self.restore_tapped(id, tapped)
    }

    /// Restores a reversed action without applying game-event replacements.
    pub(crate) fn restore_tapped(&mut self, id: ObjectId, tapped: bool) -> bool {
        if tapped
            && self
                .object(id)
                .is_some_and(|o| !o.status.contains(crate::object::Status::TAPPED))
        {
            self.reveal_masked(id);
        }
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
        if !tapped && self.effects.may_have_source_duration() {
            self.effects.remove_where(|effect| {
                effect.source == Some(id)
                    && matches!(
                        effect.duration,
                        baylee_cards_dsl::Duration::WhileSourceTapped { .. }
                    )
            });
        }
        self.board_state_changed();
        true
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
