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
use baylee_core::mana::{ManaColor, ManaPool, ManaSymbol};
use baylee_core::preset::{FormatId, GamePreset, PresetError};
use rustc_hash::FxHashMap;

mod hash;
mod moves;
mod projection;
mod setup;
#[cfg(test)]
mod tests;

pub(crate) use hash::{
    SnapshotMemo, characteristics_fingerprint, mana_cost_fingerprint, structural_fingerprint,
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
    /// Every upkeep of the controller, for the rest of the game.
    EachUpkeep,
    /// At the beginning of the next upkeep, whoever's turn it is
    /// (Archangel Avacyn's delayed transform).
    NextUpkeepOfAnyone,
    /// At the controller's next first main phase (Mana Drain).
    NextFirstMain,
    /// At the beginning of the next end step (Venser +2).
    NextEndStep,
    /// At end of combat: as the next end of combat step begins, whoever's
    /// turn it is (CR 511.2; Cockatrice's "destroy that creature at end of
    /// combat").
    EndOfCombat,
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
    /// "When this Aura leaves the battlefield" (Animate Dead): the next time
    /// `card`, as the object it was at `version`, leaves the battlefield for
    /// any zone. Read off the journal like [`Self::DiesOrIsExiled`], after
    /// `after` only (CR 603.7a), and once (CR 603.7b); a leaves-the-
    /// battlefield ability looks back in time (CR 603.10a).
    LeavesBattlefield {
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
    /// Repeated cleanup tied to the incarnation that originally marked lands.
    LinkedCounterCleanup {
        /// Source object.
        source: ObjectId,
        /// Source incarnation.
        version: u32,
        /// The recurring instructions.
        effects: &'static [baylee_cards_dsl::Effect],
    },
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
        /// Exact incarnation when this delayed ability was created.
        version: u32,
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
        /// Exact incarnation when this delayed ability was created.
        version: u32,
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
        /// Exact incarnation when this delayed ability was created.
        source_version: u32,
        /// What it does.
        effects: &'static [baylee_cards_dsl::Effect],
        /// Wording frozen when the delayed ability was created.
        text: crate::text_changes::TextChangeMap,
    },
    /// [`Self::Trigger`] about one object (`Effect::AtNextEndStep`): its
    /// event object is `object` while that is still the object it was at
    /// `version`, and nothing once it has left its zone (CR 603.7c, 400.7).
    TriggerAbout {
        /// The source of the ability that created it.
        source: ObjectId,
        /// Exact incarnation when this delayed ability was created.
        source_version: u32,
        /// What it does.
        effects: &'static [baylee_cards_dsl::Effect],
        /// Wording frozen when the delayed ability was created.
        text: crate::text_changes::TextChangeMap,
        /// The object "that creature" names.
        object: ObjectId,
        /// Its identity when this was created.
        version: u32,
    },
}

/// Per-turn counters for conditional triggers (reset at every turn start).
#[derive(Clone, Hash, Debug)]
pub struct PerTurn {
    /// Untapped lands controlled by this turn's active player at its start,
    /// before untapping or phasing; independent of any source's presence.
    pub untapped_lands_at_start: u32,
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
    /// The damage dealt to each player this turn, per seat ("the damage
    /// dealt to you this turn", Simulacrum). Written by
    /// [`GameState::damage_player`] and nothing else.
    pub damage_dealt_to: Vec<u32>,
    /// Actual positive permanent damage, deduplicated by both incarnations.
    /// This historical fact survives cleanup; reset only as the turn ends.
    pub(crate) permanent_damage: Vec<crate::damage_history::DamageRecord>,
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
    /// The permanents no regeneration shield is applied to this turn, each
    /// with its version (`Effect::CantBeRegeneratedThisTurn`, CR 701.19c):
    /// read by `sba::destroy`.
    pub cant_regenerate: Vec<(ObjectId, u32)>,
    /// The creatures declared as attackers this turn, each with its
    /// version (`Filter::AttackedThisTurn`, CR 508.1): written by
    /// `Engine::declare_attackers` and nothing else, so a creature put onto
    /// the battlefield attacking is not in it (CR 508.4).
    pub attacked: Vec<(ObjectId, u32)>,
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
            untapped_lands_at_start: 0,
            noncreature_spells: vec![0; players],
            spells_cast: vec![0; players],
            no_more_spells: vec![false; players],
            life_lost: vec![false; players],
            damage_dealt_to: vec![0; players],
            permanent_damage: Vec::new(),
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
            cant_regenerate: Vec::new(),
            attacked: Vec::new(),
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
        self.untapped_lands_at_start = 0;
        self.noncreature_spells.iter_mut().for_each(|v| *v = 0);
        self.draws.iter_mut().for_each(|v| *v = 0);
        self.drew_in_draw_step = false;
        self.spells_cast.iter_mut().for_each(|v| *v = 0);
        self.no_more_spells.iter_mut().for_each(|v| *v = false);
        self.life_lost.iter_mut().for_each(|v| *v = false);
        self.damage_dealt_to.iter_mut().for_each(|v| *v = 0);
        self.permanent_damage.clear();
        self.creatures_died = 0;
        self.entered_battlefield.clear();
        self.drawn.clear();
        self.playable.clear();
        self.resolved.clear();
        self.graveyard_plays.clear();
        self.entered_graveyard.clear();
        self.exile_if_dies.clear();
        self.cant_regenerate.clear();
        self.attacked.clear();
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
    /// All game objects.
    pub arena: Arena<GameObject>,
    /// Ordered zone contents.
    pub zones: Zones,
    /// Choices owed for simultaneous graveyard arrivals.
    pub(crate) graveyard_order: crate::graveyard_order::Ordering,
    /// Legend decisions waiting for their simultaneous SBA pass.
    pub(crate) sba_legend_decisions: Vec<crate::sba::LegendDecision>,
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
    /// Lands marked by a particular source incarnation; survives its departure.
    pub counter_links: Vec<crate::resolve::linked_counters::CounterLink>,
    /// Source identities immediately before battlefield departures.
    pub ltb_versions: Vec<(ObjectId, u32)>,
    /// Death-time information for damage-history triggers, until collection.
    pub(crate) damage_deaths: Vec<crate::damage_history::DamageDeath>,
    /// First-of-turn drawn cards awaiting a miracle offer (CR 702.94).
    pub pending_miracle: std::collections::VecDeque<(PlayerId, ObjectId)>,
    /// Queued extra turns (CR 500.7); the front player takes the next
    /// turn instead of the normal successor.
    pub extra_turns: std::collections::VecDeque<PlayerId>,
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
    pub divided: Vec<(ObjectId, Vec<(baylee_core::ids::DamageSourceRef, u32)>)>,
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
            arena,
            zones,
            graveyard_order,
            sba_legend_decisions,
            players,
            turn,
            combat,
            per_turn,
            delayed,
            counter_links,
            ltb_versions,
            damage_deaths,
            pending_miracle,
            extra_turns,
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
            text_changes,
            effect_text_overrides,
            copy_snapshots,
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
            ("state.zones", format!("{zones:?}")),
            ("state.graveyard_order", format!("{graveyard_order:?}")),
            (
                "state.sba_legend_decisions",
                format!("{sba_legend_decisions:?}"),
            ),
            ("state.players", format!("{players:?}")),
            ("state.turn", format!("{turn:?}")),
            ("state.combat", format!("{combat:?}")),
            ("state.per_turn", format!("{per_turn:?}")),
            ("state.delayed", format!("{delayed:?}")),
            ("state.counter_links", format!("{counter_links:?}")),
            ("state.ltb_versions", format!("{ltb_versions:?}")),
            ("state.damage_deaths", format!("{damage_deaths:?}")),
            ("state.pending_miracle", format!("{pending_miracle:?}")),
            ("state.extra_turns", format!("{extra_turns:?}")),
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
    /// Whether an active effect removes this player's maximum hand size.
    #[must_use]
    pub fn no_max_hand_size(&self, player: PlayerId) -> bool {
        self.effects.iter().any(|fx| {
            matches!(fx.modifier, baylee_cards_dsl::Modifier::NoMaxHandSize)
                && fx.controller == player
        })
    }

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

    /// Whether an effect says `player` doesn't lose the game for having 0 or
    /// less life (`Modifier::NoLossForZeroLife`, Lich). Each effect's `who`
    /// is read from its own controller, so a Lich that changes hands takes
    /// the exception with it.
    #[must_use]
    pub fn no_loss_for_zero_life(&self, player: PlayerId) -> bool {
        self.effects.iter().any(|fx| {
            let baylee_cards_dsl::Modifier::NoLossForZeroLife { who } = fx.modifier else {
                return false;
            };
            crate::eval::players(who, self, fx.controller).is_some_and(|p| p.contains(&player))
        })
    }

    /// Whether `player`'s life gains are replaced by draws
    /// (`Modifier::LifeGainDrawsInstead`, Lich), read as
    /// [`Self::cant_lose_life`] reads its own.
    #[must_use]
    pub fn life_gain_draws_instead(&self, player: PlayerId) -> bool {
        self.effects.iter().any(|fx| {
            let baylee_cards_dsl::Modifier::LifeGainDrawsInstead { who } = fx.modifier else {
                return false;
            };
            crate::eval::players(who, self, fx.controller).is_some_and(|p| p.contains(&player))
        })
    }

    /// Whether an effect has `player` skip their untap steps
    /// (`Modifier::SkipUntapStep`, Stasis). Each effect's `who` is read from
    /// its own controller, as [`Self::cant_lose_life`] reads its own.
    #[must_use]
    pub fn skips_untap_step(&self, player: PlayerId) -> bool {
        self.effects.iter().any(|fx| {
            let baylee_cards_dsl::Modifier::SkipUntapStep { who } = fx.modifier else {
                return false;
            };
            crate::eval::players(who, self, fx.controller).is_some_and(|p| p.contains(&player))
        })
    }

    /// The game's one clock, monotonic: every object timestamp
    /// ([`GameObject::timestamp`], CR 613.7), every continuous effect's, and
    /// the moments [`GameObject::controlled_since`] and
    /// [`Player::turn_start_timestamp`] compare for summoning sickness
    /// (CR 302.6) are read off it.
    pub fn next_timestamp(&mut self) -> u64 {
        self.timestamp += 1;
        self.timestamp
    }

    /// CR 302.6: a creature must have been controlled continuously since
    /// its controller's most recent turn began, so *any* control change
    /// makes it summoning-sick again — including the one at end of turn
    /// that hands a stolen creature back.
    ///
    /// Only [`GameObject::controlled_since`] moves. A change of control is
    /// not one of the events CR 613.7 gives an object a new timestamp for,
    /// so the order its static abilities' effects apply in stays as it was.
    pub(crate) fn restart_summoning_sickness(&mut self, id: ObjectId) {
        let ts = self.next_timestamp();
        if let Some(obj) = self.object_mut(id) {
            obj.controlled_since = ts;
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
    ///
    /// The permanent stays the object it was, and every effect that applied
    /// to it goes on applying (CR 712.18). What its own static abilities and
    /// replacement effects did ends here, because those abilities were the
    /// face that turned away and the permanent no longer has them
    /// (CR 604.2); the next scan (`Engine::sync_static_effects`) registers
    /// what the new face prints, as [`Self::set_doors`] has it for a Room.
    /// Without this, Dowsing Dagger's "equipped creature gets +2/+1" went
    /// on applying from Lost Vale.
    ///
    /// The other face coming up without the game action is
    /// [`Self::turn_over`], which this is built on.
    pub fn transform(&mut self, id: ObjectId, def: &CardDef, face: usize) -> bool {
        let face = face.min(def.faces.len() - 1);
        if !self.turn_over(id, def, face) {
            return false;
        }
        // CR 613.7g: a new timestamp, which the new face's static abilities
        // take as the next scan registers them (CR 613.7a), so they apply
        // after every effect created while the other face was up. Who
        // controls it has not changed (CR 712.18), so `controlled_since`
        // stays and it is no more summoning-sick than it was (CR 302.6).
        let ts = self.next_timestamp();
        if let Some(obj) = self.object_mut(id) {
            obj.timestamp = ts;
        }
        self.journal.record(GameEvent::Transformed {
            object: id,
            face: face as u8,
        });
        true
    }

    /// Puts face `face` of a permanent up in place of the one it shows, and
    /// ends what the face turning away did: the continuous effects of its
    /// static abilities and its replacement effects, which the next scan
    /// (`Engine::sync_static_effects`) registers for the new face (CR 604.2).
    /// Returns whether the face changed.
    ///
    /// This is not the game action of transforming, and it records nothing
    /// and stamps nothing. Two things need it bare: [`Self::transform`],
    /// which adds the stamp and the journal entry, and a permanent that
    /// enters with its back face up (CR 712.14a, daybound at night,
    /// CR 702.145b). Such a permanent was never turned over (CR 701.27a
    /// transforms a *permanent*), so it has not transformed: a "transforms
    /// into" trigger (CR 701.27e) must not see it, the log must not say it
    /// did, and its timestamp is the one it took as it entered (CR 613.7d).
    /// It still has the front face's statics to lose, because the engine
    /// registers them for every arrival before the scan that turns it over.
    pub fn turn_over(&mut self, id: ObjectId, def: &CardDef, face: usize) -> bool {
        let face = face.min(def.faces.len() - 1);
        if self
            .object(id)
            .is_none_or(|o| o.face_index as usize == face)
        {
            return false;
        }
        self.effects.remove_where(|fx| {
            fx.source == Some(id) && fx.origin == crate::effects::EffectOrigin::Static
        });
        self.replacement_rules.retain(|r| r.source != id);
        self.switch_face(id, def, face);
        true
    }

    /// Attach an Aura or Equipment to a new permanent. A new attachment
    /// gives it and its static effects a new timestamp (CR 613.7a, 613.7e).
    /// Reattaching to the same object does nothing (CR 701.3b).
    pub(crate) fn attach(&mut self, source: ObjectId, host: ObjectId) {
        let Some(object) = self.object(source) else {
            return;
        };
        if object.attached_to == Some(host) {
            return;
        }
        let on_battlefield = object.zone == Zone::Battlefield;
        let timestamp = on_battlefield.then(|| self.next_timestamp());
        if let Some(object) = self.object_mut(source) {
            object.attached_to = Some(host);
            if let Some(timestamp) = timestamp {
                object.timestamp = timestamp;
            }
        }
        if let Some(timestamp) = timestamp {
            self.effects.retimestamp_statics(source, timestamp);
        }
        self.invalidate_projections();
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
        // A resolving instruction continues after its controller leaves
        // (CR 608.2m), but the departed seat's retained information must
        // remain what it was immediately before leaving (CR 800.4i).
        if self.has_left(player) || (by < 0 && self.cant_lose_life(player)) {
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

    /// Damage dealt to a player, after prevention: the life it costs
    /// (CR 120.3a), the turn's tally and the [`GameEvent::DamageDealt`]
    /// record. **The** door for damage to a player, as
    /// [`Self::change_life`] is for life: combat damage
    /// (`combat::deal_damage_to_player`) and an effect's
    /// (`resolve::life::deal_to_player`) both come through here, so "the
    /// damage dealt to you this turn" (Simulacrum) reads one tally that
    /// neither of them can forget to write.
    ///
    /// The tally counts damage dealt, not life lost, so it is not kept in
    /// `change_life`: a payment loses life and is no damage, and a player
    /// whose life can't change is still dealt the damage (`change_life`
    /// refuses the loss, this records the damage). Nothing is dealt below
    /// one point; the caller has already prevented what it prevents.
    pub fn damage_player(
        &mut self,
        source: ObjectId,
        player: PlayerId,
        amount: u32,
        is_combat: bool,
        cause: Cause,
    ) {
        if amount == 0 {
            return;
        }
        self.change_life(player, -i32::try_from(amount).unwrap_or(i32::MAX), cause);
        if let Some(tally) = self.per_turn.damage_dealt_to.get_mut(player.get() as usize) {
            *tally = tally.saturating_add(amount);
        }
        self.journal.record(GameEvent::DamageDealt {
            source: Some(source),
            target: crate::event::DamageTarget::Player(player),
            amount,
            is_combat,
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
        self.board_state_changed();
        true
    }

    /// Whether this incarnation still awaits its event-driven face-up replacement.
    #[must_use]
    pub(crate) fn awaits_masked_reveal(&self, id: ObjectId) -> bool {
        self.object(id).is_some_and(|object| {
            object.zone == Zone::Battlefield
                && object.status.contains(crate::object::Status::FACE_DOWN)
                && object.riders.contains(&crate::object::Rider::Masked)
        })
    }

    /// Turn the same permanent face up; this is not a zone change or entry.
    pub(crate) fn reveal_masked(&mut self, id: ObjectId) -> bool {
        if !self.awaits_masked_reveal(id) {
            return false;
        }
        let object = self.object_mut(id).expect("checked permanent");
        object.status.remove(crate::object::Status::FACE_DOWN);
        if let Some(original) = object.original_base.take() {
            object.base = original;
        }
        object.cache.clear();
        self.invalidate_projections();
        self.journal.record(GameEvent::TurnedFaceUp { object: id });
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
        if self.effects.iter().any(|fx| {
            matches!(fx.filter, crate::effects::EffectFilter::Dsl(f) if filter_reads_board_state(f))
                || modifier_reads_combat(fx.modifier)
        }) {
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

    /// Whether `player` has left the game (CR 800.4a). Nothing is created
    /// for them any more, and nothing they control triggers (CR 800.4d).
    #[must_use]
    pub fn has_left(&self, player: PlayerId) -> bool {
        self.players[usize::from(player.get())].has_lost()
    }

    /// Effective words of one ability, including token-defined quoted text.
    #[must_use]
    pub fn ability_text(&self, id: ObjectId, index: u32) -> crate::text_changes::TextChangeMap {
        let Some(object) = self.object(id) else {
            return crate::text_changes::TextChangeMap::IDENTITY;
        };
        let changed = self.text_changes.get(baylee_core::ids::DamageSourceRef {
            object: id,
            version: object.version,
        });
        if object.kind == ObjectKind::AbilityOnStack {
            return changed;
        }
        object
            .printed_ability_list(self.bases.as_ref())
            .base_text(index as usize)
            .then(changed)
    }

    /// The current copiable rules of an admitted object, without a second registry.
    #[must_use]
    pub fn printed_ability_list(&self, id: ObjectId) -> Option<crate::object::AbilityList> {
        self.object(id)
            .map(|object| object.printed_ability_list(self.bases.as_ref()))
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

    /// The exact incarnation dealing damage, current or retained as LKI.
    pub(crate) fn damage_source(&self, id: ObjectId, version: Option<u32>) -> Option<&GameObject> {
        self.object_or_departed(id)
            .filter(|obj| version.is_none_or(|v| v == obj.version))
            .or_else(|| {
                self.damage_sources
                    .iter()
                    .rev()
                    .find(|obj| obj.id == id && version == Some(obj.version))
                    .map(|obj| &**obj)
            })
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
        self.journal.record(GameEvent::BecameMonarch { player });
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
    /// left, so the leaver is never the heir, and neither is anybody in
    /// `leaving`, the players leaving at the same time who have yet to be
    /// marked: players who lose in one state-based check leave together, and
    /// the crown passed through one of them on its way to the heir, ending a
    /// Palace Jailer's exile on the way. Through [`Self::set_monarch`],
    /// so an exile that waited for an opponent to become the monarch (Palace
    /// Jailer) ends if the heir is such an opponent. The rule's "if there is
    /// no active player" never arises here, because the engine always has one
    /// (`TurnInfo::active`), and nothing in the engine keeps a player still
    /// in the game from becoming the monarch.
    pub(crate) fn monarch_leaves(&mut self, player: PlayerId, leaving: &[PlayerId]) {
        if self.monarch != Some(player) {
            return;
        }
        let seats = self.players.len();
        let active = usize::from(self.turn.active.get());
        let heir = (0..seats)
            .map(|offset| PlayerId::new(((active + offset) % seats) as u8))
            .find(|&p| !self.has_left(p) && !leaving.contains(&p));
        match heir {
            Some(heir) => self.set_monarch(heir),
            None => self.monarch = None,
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

    /// The semantic wording applicable to this continuous effect's definition.
    /// Layer-six grants follow their grantor, never the receiving object's text.
    #[must_use]
    pub fn effect_text(
        &self,
        effect: &crate::effects::ContinuousEffect,
    ) -> crate::text_changes::TextChangeMap {
        if let Some((_, origin)) = self
            .effect_text_overrides
            .iter()
            .find(|(id, _)| *id == effect.id)
        {
            return origin.resolve(&self.text_changes);
        }
        if effect.origin == crate::effects::EffectOrigin::Static
            && let Some(source) = effect
                .source
                .and_then(|source| self.source_identity(source))
        {
            return self.text_changes.get(source);
        }
        crate::text_changes::TextChangeMap::IDENTITY
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
///
/// `Filter::IsAttached` is not here for a plainer reason: every write of
/// `attached_to` invalidates the projection itself — the attach in
/// `resolve`, the unattach in `sba`, and the zone move that clears it.
pub(crate) fn filter_reads_board_state(filter: &baylee_cards_dsl::Filter) -> bool {
    use baylee_cards_dsl::Filter;
    match filter {
        Filter::Tapped
        | Filter::Untapped
        | Filter::Attacking
        | Filter::Blocking
        | Filter::Unblocked => true,
        Filter::And(parts) | Filter::Or(parts) => parts.iter().any(filter_reads_board_state),
        Filter::Not(f) => filter_reads_board_state(f),
        _ => false,
    }
}

/// Whether a modifier's number reads combat: a count of what the defending
/// player controls (`PtCount::DefendingPlayerControls`, CR 508.5), which
/// changes as an attack is declared and as combat ends while no effect
/// begins or ends — the input [`filter_reads_board_state`] announces for a
/// filter, here in the modifier.
fn modifier_reads_combat(modifier: baylee_cards_dsl::Modifier) -> bool {
    use baylee_cards_dsl::{Modifier, PtCount};
    let (Modifier::CharacteristicPT { count, .. }
    | Modifier::SetPTToCount(count)
    | Modifier::ModifyPTHalfCount(count)) = modifier
    else {
        return false;
    };
    matches!(count, PtCount::DefendingPlayerControls(_))
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
