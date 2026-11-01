//! The game log (#262): what happened, told to each seat exactly as that
//! seat may know it.
//!
//! The engine's journal is the whole history of a game, and it names
//! everything. The session reads it after every action it applies, into
//! lines that remember, for each object they name, who may know what it is.
//! A seat is told its lines with every other name taken down to what its
//! view would have shown at the time ([`LogObject`]): the rule is one
//! sentence, and it is the view's. The session adds what only it knows: the
//! opening mulligans, what a decision clock answered, and a chair the house
//! stands in for.
//!
//! The journal is read *after* each action, so the state it is named from is
//! the one the action left. A card keeps its handle across zones, so a card
//! still in the game can always be named. An object gone from the state (a
//! token that has ceased to exist, a card that left the game with its owner)
//! is named from what this log remembers of it: everything it named, and
//! everything that stood in a public zone after any action. One it never saw
//! at all, made and gone within a single action, is told to every seat as
//! [`LogObject::Hidden`], with no handle.

use std::collections::{BTreeMap, BTreeSet};

use baylee_core::ids::{AbilityRef, ObjectId, PlayerId, SeatSet};
use baylee_engine::event::{Cause, DamageTarget, GameEvent, LibraryPlace};
use baylee_engine::object::{GameObject, ObjectKind};
use baylee_engine::state::GameState;
use baylee_engine::zone::{Zone, ZoneLocation};
use baylee_view::{
    CardIdentity, LogAbility, LogEntry, LogEvent, LogFrom, LogObject, LogPlace, LogTarget, LogZone,
    Phase, RulesFace,
};

use crate::view::{counter, day_night, loss_cause, may_know_card, phase, rules_face};

/// The longest run of lines a repeat is looked for in: a loop that blinks a
/// creature is three lines long (its trigger, the exile, the return).
const FOLD_SPAN: usize = 8;

/// Who may know what an object a line names is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Sees {
    /// Every seat: the object was face up in a public zone.
    Everyone,
    /// These seats. Every other seat is told [`LogObject::Hidden`].
    Only(SeatSet),
    /// These seats. Every other seat saw it face down, and is told
    /// [`LogObject::FaceDown`].
    FaceDown(SeatSet),
}

impl Sees {
    /// Nobody: a library, or outside the game.
    const NOBODY: Self = Self::Only(SeatSet::new());

    /// Who may know an object that was in two places, as it went from one to
    /// the other: whoever either place showed it to.
    fn and(self, other: Self) -> Self {
        let both = |a: SeatSet, b: SeatSet| a.iter().chain(b.iter()).collect();
        match (self, other) {
            (Self::Everyone, _) | (_, Self::Everyone) => Self::Everyone,
            (Self::Only(a), Self::Only(b)) => Self::Only(both(a, b)),
            (Self::FaceDown(a), Self::Only(b) | Self::FaceDown(b))
            | (Self::Only(b), Self::FaceDown(a)) => Self::FaceDown(both(a, b)),
        }
    }

    /// Whether `seat` is told the object by name.
    fn names_to(self, seat: PlayerId) -> bool {
        match self {
            Self::Everyone => true,
            Self::Only(seats) | Self::FaceDown(seats) => seats.contains(seat),
        }
    }

    /// Takes `object` down to what `seat` may know of it.
    fn tell(self, seat: PlayerId, object: &mut LogObject) {
        match self {
            Self::Everyone => {}
            Self::Only(seats) | Self::FaceDown(seats) if seats.contains(seat) => {}
            Self::Only(_) => *object = LogObject::Hidden,
            Self::FaceDown(_) => {
                if let LogObject::Known { id, .. } = object {
                    *object = LogObject::FaceDown { id: *id };
                }
            }
        }
    }
}

/// One line, as the host keeps it: every name in full, beside who may know
/// each one.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Line {
    turn: u32,
    repeat: u32,
    /// When it was written ([`LogEntry::at`]).
    at: u64,
    event: LogEvent,
    /// One per object the event names, in [`LogEvent::objects`] order.
    sees: Vec<Sees>,
    /// The phase it happened in ([`LogEntry::phase`]).
    phase: Option<Phase>,
}

impl Line {
    /// Whether every seat is told this line in full. Only such a line is
    /// folded into another: whether two lines fold must not depend on
    /// anything a seat may not know, or the fold itself would tell it.
    fn public(&self) -> bool {
        self.sees.iter().all(|sees| *sees == Sees::Everyone)
    }

    /// Whether `later` says the same thing again, so the two can be one line
    /// that happened twice. A change of life or of counters in the same
    /// direction, starting where this one ended, is the same thing again,
    /// and the folded line spans both.
    fn folds_with(&self, later: &Self) -> bool {
        if self.turn != later.turn || self.phase != later.phase || self.sees != later.sees {
            return false;
        }
        match (&self.event, &later.event) {
            (
                LogEvent::Life {
                    player: a,
                    old: a_old,
                    new: a_new,
                },
                LogEvent::Life {
                    player: b,
                    old: b_old,
                    new: b_new,
                },
            ) => a == b && a_new == b_old && a_new.cmp(a_old) == b_new.cmp(b_old),
            (
                LogEvent::Counters {
                    object: a,
                    kind: a_kind,
                    old: a_old,
                    new: a_new,
                },
                LogEvent::Counters {
                    object: b,
                    kind: b_kind,
                    old: b_old,
                    new: b_new,
                },
            ) => {
                a == b && a_kind == b_kind && a_new == b_old && a_new.cmp(a_old) == b_new.cmp(b_old)
            }
            (a, b) => a == b,
        }
    }

    /// Folds `later`, which [`Self::folds_with`] this line, into it.
    fn absorb(&mut self, later: Self) {
        self.repeat = self.repeat.saturating_add(later.repeat);
        match (&mut self.event, later.event) {
            (LogEvent::Life { new, .. }, LogEvent::Life { new: last, .. }) => *new = last,
            (LogEvent::Counters { new, .. }, LogEvent::Counters { new: last, .. }) => *new = last,
            _ => {}
        }
    }

    /// The line as `seat` is told it.
    fn told(&self, seat: PlayerId) -> LogEntry {
        let mut event = self.event.clone();
        let mut sees = self.sees.iter();
        for object in event.objects_mut() {
            // A name with no entry beside it is told to nobody: a line built
            // wrong fails closed rather than naming a card to everyone.
            sees.next()
                .copied()
                .unwrap_or(Sees::NOBODY)
                .tell(seat, object);
        }
        // Which ability it was says which card its source is, and so do
        // the modes chosen for it.
        match &mut event {
            LogEvent::Ability {
                source, ability, ..
            } if !matches!(source, LogObject::Known { .. }) => *ability = None,
            LogEvent::Aimed {
                source,
                rules,
                modes,
                ..
            } if !matches!(source, LogObject::Known { .. }) => {
                *rules = None;
                *modes = 0;
            }
            _ => {}
        }
        LogEntry {
            turn: self.turn,
            repeat: self.repeat,
            at: self.at,
            event,
            phase: self.phase,
        }
    }
}

/// The last this log saw of an object, for naming it once it is gone.
#[derive(Clone, Debug)]
struct Seen {
    object: LogObject,
    owner: PlayerId,
    controller: PlayerId,
    sees: Sees,
}

/// How well this log named what it named, counted.
///
/// A measurement, not a rule: the engine records no identity for an object
/// that has ceased to exist, and these say how often that costs a name.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Naming {
    /// Objects named, every time one was.
    pub references: u64,
    /// Of those, told as [`LogObject::Hidden`] to everyone: seen neither in
    /// the state nor before it was gone.
    pub unnamed: u64,
    /// Lines left out because they needed the owner or controller of such an
    /// object: a token made and gone within one action.
    pub dropped: u64,
    /// Abilities put on the stack.
    pub abilities: u64,
    /// Of those, named by their source alone, because the ability had left
    /// the stack before the log read it.
    pub by_source: u64,
}

/// Every seat's game log, kept once for the whole table.
#[derive(Clone, Debug)]
pub struct GameLog {
    /// Journal entries read so far.
    read: usize,
    lines: Vec<Line>,
    /// How many lines may have been sent to somebody. Those never change
    /// again, so every seat is told the same line at the same index.
    sealed: usize,
    /// Every object named so far, as it was when last named.
    seen: BTreeMap<ObjectId, Seen>,
    /// Whether the first turn has begun. Before it, cards moving between
    /// libraries and hands are the opening hands, which the mulligan lines
    /// already count.
    started: bool,
    turn: u32,
    /// The phase the journal is in, as far as it has been read.
    phase: Option<Phase>,
    /// Every spell and ability on the stack a line has named, with what it
    /// was last announced with ([`LogEvent::Aimed`]).
    aims: BTreeMap<ObjectId, Aim>,
    /// Lands played that had not moved yet when the play was recorded (an
    /// entry replacement, a shockland's life, asks first): the move onto the
    /// battlefield writes their play, with who played them.
    landing: BTreeMap<ObjectId, PlayerId>,
    /// The time as the caller last told it ([`Self::tell_time`]).
    now: u64,
    naming: Naming,
}

/// A spell or an ability on the stack, as the log last announced it.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Aim {
    /// Who controls it.
    controller: PlayerId,
    /// The spell itself, or the source of the ability.
    source: ObjectId,
    /// Whether it is an ability of `source`.
    ability: bool,
    /// Its chosen modes, as [`LogEvent::Aimed::modes`].
    modes: u8,
    /// Its object targets, in order.
    objects: Vec<ObjectId>,
    /// Its player targets.
    players: SeatSet,
}

impl Aim {
    fn says_nothing(&self) -> bool {
        self.modes == 0 && self.objects.is_empty() && self.players.is_empty()
    }
}

impl GameLog {
    /// A log of `state`'s game, with everything its journal already holds.
    #[must_use]
    pub fn new(state: &GameState) -> Self {
        let mut log = Self {
            read: 0,
            lines: Vec::new(),
            sealed: 0,
            seen: BTreeMap::new(),
            started: false,
            turn: state.turn.number,
            phase: None,
            aims: BTreeMap::new(),
            landing: BTreeMap::new(),
            now: 0,
            naming: Naming::default(),
        };
        log.consume(state);
        log
    }

    /// How many lines the log holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// How well this log has named what it named, so far.
    #[cfg(test)]
    #[must_use]
    pub const fn naming(&self) -> Naming {
        self.naming
    }

    /// Lines `from..to` as `seat` is told them.
    #[must_use]
    pub fn told(&self, seat: PlayerId, from: usize, to: usize) -> Vec<LogEntry> {
        self.lines
            .get(from..to)
            .unwrap_or_default()
            .iter()
            .map(|line| line.told(seat))
            .collect()
    }

    /// The time, in milliseconds since the Unix epoch, that every line from
    /// here on is stamped with ([`LogEntry::at`]) until the caller tells it
    /// another. The log never reads a clock: it is linked into a browser,
    /// where the standard library has none.
    pub fn tell_time(&mut self, unix_ms: u64) {
        self.now = unix_ms;
    }

    /// The first `upto` lines may have been sent: they never change again.
    pub fn seal(&mut self, upto: usize) {
        self.sealed = self.sealed.max(upto.min(self.lines.len()));
    }

    /// A line only the session knows, naming no object.
    pub fn note(&mut self, event: LogEvent) {
        self.push(event, Vec::new());
    }

    /// Reads what the journal recorded since the last call.
    pub fn consume(&mut self, state: &GameState) {
        let entries = state.journal.entries();
        let batch = self.lines.len();
        // Cards shown to everyone in this batch, until they go somewhere
        // hidden: the move that takes a revealed card to a hand is public.
        let mut shown: BTreeSet<ObjectId> = BTreeSet::new();
        // Cards whose discard is already a line of its own.
        let mut discarded: BTreeSet<ObjectId> = BTreeSet::new();
        // Where each object moved from last in this batch, for the play or
        // the cast that follows its move and says where it came from.
        let mut came: BTreeMap<ObjectId, Zone> = BTreeMap::new();
        for entry in entries.get(self.read..).unwrap_or_default() {
            if let GameEvent::ZoneChanged { object, from, .. } = &entry.event {
                came.insert(*object, *from);
            }
            self.read_event(
                state,
                &entry.event,
                batch,
                &mut shown,
                &mut discarded,
                &came,
            );
        }
        self.read = entries.len();
        self.reannounce(state);
        self.look_around(state);
    }

    /// What `id`, a spell or an ability on the stack, stands announced with
    /// now; `None` once it has left the stack.
    fn aim_of(
        state: &GameState,
        id: ObjectId,
        controller: PlayerId,
        source: ObjectId,
        ability: bool,
    ) -> Option<Aim> {
        let obj = state.object(id)?;
        if obj.zone != Zone::Stack {
            return None;
        }
        let modes = obj.modes | obj.mode_index.filter(|i| *i < 8).map_or(0, |i| 1_u8 << i);
        // A rule's own trigger (prowess, ward) carries the object it is about
        // as a target the card never printed: nothing was aimed.
        let synthetic = obj
            .ability
            .is_some_and(|loc| loc.index == AbilityRef::SYNTHETIC);
        Some(Aim {
            controller,
            source,
            ability,
            modes,
            objects: if synthetic {
                Vec::new()
            } else {
                obj.targets
                    .iter()
                    .chain(obj.second_targets().iter())
                    .copied()
                    .collect()
            },
            players: if synthetic {
                SeatSet::new()
            } else {
                baylee_engine::eval::first_target_players(obj)
                    .iter()
                    .chain(obj.second_target_players().iter())
                    .collect()
            },
        })
    }

    /// Whether `id` is still on the stack (`Some`) and whether its current
    /// modes and targets already match `held` (`Some(true)`), without
    /// allocating when nothing changed.
    fn aim_matches(state: &GameState, id: ObjectId, held: &Aim) -> Option<bool> {
        let obj = state.object(id)?;
        if obj.zone != Zone::Stack {
            return None;
        }
        let modes = obj.modes | obj.mode_index.filter(|i| *i < 8).map_or(0, |i| 1_u8 << i);
        if modes != held.modes {
            return Some(false);
        }
        let synthetic = obj
            .ability
            .is_some_and(|loc| loc.index == AbilityRef::SYNTHETIC);
        if synthetic {
            return Some(held.objects.is_empty() && held.players.is_empty());
        }
        let objects_match = obj
            .targets
            .iter()
            .chain(obj.second_targets().iter())
            .copied()
            .eq(held.objects.iter().copied());
        if !objects_match {
            return Some(false);
        }
        let players: SeatSet = baylee_engine::eval::first_target_players(obj)
            .iter()
            .chain(obj.second_target_players().iter())
            .collect();
        Some(players == held.players)
    }

    /// Writes what the spell or ability `id` was announced with, when it
    /// says anything and differs from what a line last said of it.
    fn announce(
        &mut self,
        state: &GameState,
        id: ObjectId,
        controller: PlayerId,
        source: ObjectId,
        ability: bool,
    ) {
        let Some(aim) = Self::aim_of(state, id, controller, source, ability) else {
            return;
        };
        let before = self.aims.insert(id, aim.clone());
        if aim.says_nothing() || before.as_ref() == Some(&aim) {
            return;
        }
        let rules = if aim.modes == 0 {
            None
        } else {
            state.object(id).and_then(|obj| {
                if ability {
                    log_ability(obj).and_then(|named| named.rules)
                } else {
                    obj.printed_face().map(rules_face)
                }
            })
        };
        self.aimed(state, &aim, rules);
    }

    /// Announces again every spell and ability on the stack whose modes or
    /// targets changed since a line last said them (a trigger's targets are
    /// asked after it is put on the stack, CR 603.3d), and forgets the ones
    /// that have left it.
    fn reannounce(&mut self, state: &GameState) {
        let mut changed = Vec::new();
        self.aims
            .retain(|&id, aim| match Self::aim_matches(state, id, aim) {
                None => false,
                Some(true) => true,
                Some(false) => {
                    changed.push((id, aim.controller, aim.source, aim.ability));
                    true
                }
            });
        for (id, controller, source, ability) in changed {
            self.announce(state, id, controller, source, ability);
        }
    }

    /// The line for `aim`.
    fn aimed(&mut self, state: &GameState, aim: &Aim, rules: Option<RulesFace>) {
        let (source, seen) = self.refer(state, aim.source);
        let mut sees = vec![seen];
        let mut targets = Vec::new();
        for id in &aim.objects {
            let (object, seen) = self.refer(state, *id);
            sees.push(seen);
            targets.push(LogTarget::Object(object));
        }
        targets.extend(aim.players.iter().map(LogTarget::Player));
        self.push(
            LogEvent::Aimed {
                controller: aim.controller,
                source,
                ability: aim.ability,
                rules,
                modes: if rules.is_some() { aim.modes } else { 0 },
                targets,
            },
            sees,
        );
    }

    /// Remembers every object in a public zone the log has not named yet.
    ///
    /// An object can leave the state before a line names it: a token that
    /// dies in a later action than the one that made it, a permanent whose
    /// owner leaves the game with it. Every seat has seen these, so the log
    /// may name them afterwards as they were.
    fn look_around(&mut self, state: &GameState) {
        let seats = (0..state.players.len()).map(|i| PlayerId::new(seat_byte(i)));
        let public = [ZoneLocation::Battlefield, ZoneLocation::Stack]
            .into_iter()
            .chain(seats.flat_map(|p| [ZoneLocation::Graveyard(p), ZoneLocation::Exile(p)]));
        for zone in public {
            for id in state.zones.list(zone) {
                if !self.seen.contains_key(id)
                    && let Some(obj) = state.object(*id)
                {
                    let sees = in_zone(state, obj, obj.zone, obj.owner);
                    self.remember(state, obj, sees);
                }
            }
        }
    }

    #[allow(clippy::too_many_lines)] // one arm per journal event
    fn read_event(
        &mut self,
        state: &GameState,
        event: &GameEvent,
        batch: usize,
        shown: &mut BTreeSet<ObjectId>,
        discarded: &mut BTreeSet<ObjectId>,
        came: &BTreeMap<ObjectId, Zone>,
    ) {
        match event {
            GameEvent::SpeedTriggered { .. }
            // Counter removal and the resulting triggered stack ability are
            // logged separately; this event freezes intrinsic Siege eligibility.
            | GameEvent::BattleDefeated { .. }
            | GameEvent::BecameTarget { .. }
            | GameEvent::PlayerBecameTarget { .. }
            | GameEvent::GameStarted { .. }
            // Stack announcements already log nonmana abilities; this is
            // the rules event marking completed cost payment.
            | GameEvent::AbilityActivated { .. }
            | GameEvent::ManaProduced { .. }
            | GameEvent::ObjectTapped { .. }
            | GameEvent::ObjectUntapped { .. }
            | GameEvent::PhaseChanged { .. }
            | GameEvent::StackObjectResolved { .. }
            | GameEvent::DevCommandApplied { .. }
            // The discard it paid is a line of its own, and the draw is the
            // ability's.
            | GameEvent::Cycled { .. }
            // What a seat's own policy answered for it is told to that seat
            // alone, in its view (#234). A line here would reach every seat,
            // or, kept to one, stop an automated loop from folding for all
            // of them.
            | GameEvent::AutoAnswered { .. }
            // The permanent's own label says which doors are open, and a
            // trigger the unlock set off has its own line.
            | GameEvent::DoorUnlocked { .. }
            // A cast taken back (CR 732.1) never left its zone, so no seat
            // but the caster ever saw which card it was: a line naming it
            // would reveal a card in a hidden hand, and a line naming none
            // says only that the caster thought for a while.
            | GameEvent::CastCancelled { .. }
            // The creating effect and resulting stack entries already identify copies.
            | GameEvent::SpellCopied { .. }
            | GameEvent::Connived { .. }
            | GameEvent::LibrarySearched { .. }
            // The board says which attackers nothing blocks, and an ability
            // it set off has its own line.
            | GameEvent::AttackerBlocked { .. }
            | GameEvent::CreatureMutated { .. }
            | GameEvent::AttackerUnblocked { .. }
            // The table as the preset laid it out is not something that
            // happened in the game.
            | GameEvent::ZoneChanged {
                cause: Cause::Setup,
                ..
            } => {}
            GameEvent::StepChanged { phase: now, .. } => self.phase = Some(phase(*now)),
            GameEvent::TurnStarted { number, active } => {
                self.started = true;
                self.turn = *number;
                // The turn's heading belongs to no phase: the untap step's
                // own change of step follows it.
                self.phase = None;
                self.push(LogEvent::TurnStarted { active: *active }, Vec::new());
            }
            GameEvent::CardsKept { player, cards } => {
                let (cards, sees) = cards.iter().map(|id| self.refer(state, *id)).unzip();
                self.push(LogEvent::CardsKept { player: *player, cards }, sees);
            }
            GameEvent::Revealed { player, cards } => {
                shown.extend(cards.iter().copied());
                let cards: Vec<LogObject> =
                    cards.iter().map(|id| self.refer(state, *id).0).collect();
                let sees = vec![Sees::Everyone; cards.len()];
                self.push(
                    LogEvent::Revealed {
                        player: *player,
                        cards,
                    },
                    sees,
                );
            }
            GameEvent::Shuffled { player, zone } => {
                if self.started
                    && *zone == Zone::Library
                    && !self.shuffled_in(*player, batch)
                {
                    self.push(LogEvent::Shuffled { player: *player }, Vec::new());
                }
            }
            GameEvent::ZoneChanged {
                object,
                from,
                to,
                place,
                ..
            } => self.moved(state, *object, (*from, *to), *place, shown, discarded, batch),
            GameEvent::CardsDrawn { player, count, .. } => {
                if self.started {
                    self.drew(*player, usize::from(*count), batch);
                }
            }
            GameEvent::CounterChanged {
                object,
                kind,
                old,
                new,
                ..
            } => {
                let (object, sees) = self.refer(state, *object);
                self.push(
                    LogEvent::Counters {
                        object,
                        kind: counter(*kind),
                        old: *old,
                        new: *new,
                    },
                    vec![sees],
                );
            }
            GameEvent::PlayerCountersChanged { player, kind, old, new } => {
                self.push(LogEvent::PlayerCounters { player: *player, kind: counter(*kind), old: *old, new: *new }, vec![]);
            }
            GameEvent::LifeChanged {
                player, old, new, ..
            } => self.push(
                LogEvent::Life {
                    player: *player,
                    old: *old,
                    new: *new,
                },
                Vec::new(),
            ),
            GameEvent::DamageDealt {
                source,
                target,
                amount,
                is_combat,
            } => {
                let mut sees = Vec::new();
                let source = source.map(|id| {
                    let (object, seen) = self.refer(state, id);
                    sees.push(seen);
                    object
                });
                let target = match target {
                    DamageTarget::Player(player) => LogTarget::Player(*player),
                    DamageTarget::Object(id) => {
                        let (object, seen) = self.refer(state, *id);
                        sees.push(seen);
                        LogTarget::Object(object)
                    }
                };
                self.push(
                    LogEvent::Damage {
                        source,
                        target,
                        amount: *amount,
                        combat: *is_combat,
                    },
                    sees,
                );
                self.cause_before_effect(batch);
            }
            GameEvent::CoinFlipped { player, won } => self.push(
                LogEvent::CoinFlipped {
                    player: *player,
                    won: *won,
                },
                Vec::new(),
            ),
            GameEvent::DiceRolled {
                player,
                sides,
                result,
            } => self.push(
                LogEvent::DiceRolled {
                    player: *player,
                    sides: *sides,
                    result: *result,
                },
                Vec::new(),
            ),
            GameEvent::LandPlayed { object, player } => {
                // A land whose entry is asked about first (it may enter
                // tapped, or its controller pays life) is recorded played
                // before it moves, perhaps actions before: the line waits
                // for the move, which says where it came from, and is the
                // play rather than a move of its own. Written now, it named
                // a card still in a hand: "Bo played a card".
                if !came.contains_key(object) {
                    self.landing.insert(*object, *player);
                    return;
                }
                // Otherwise the engine moved the land before it recorded the
                // play, so the move is already a line, and this one says it
                // again with where from.
                shown.remove(object);
                self.unwrite_move(*object, LogZone::Battlefield, batch);
                let from = came_from(state, *object, came);
                let (land, sees) = self.refer(state, *object);
                self.push(
                    LogEvent::LandPlayed {
                        player: *player,
                        land,
                        from,
                    },
                    vec![sees],
                );
            }
            GameEvent::SpellCast { object, player, .. } => {
                let from = came_from(state, *object, came);
                let (spell, sees) = self.refer(state, *object);
                self.push(
                    LogEvent::Cast {
                        player: *player,
                        spell,
                        from,
                    },
                    vec![sees],
                );
                self.announce(state, *object, *player, *object, false);
            }
            GameEvent::StackObjectDidNotResolve { object } => {
                let (object, sees) = self.refer(state, *object);
                self.push(LogEvent::DidNotResolve { object }, vec![sees]);
            }
            GameEvent::AbilityTriggered {
                object,
                source,
                controller,
                ..
            } => self.ability(state, *object, *source, *controller),
            GameEvent::SpellCountered { object } => {
                let (spell, sees) = self.refer(state, *object);
                self.push(LogEvent::Countered { spell }, vec![sees]);
            }
            GameEvent::Discarded { object, player, known, .. } => {
                // Into a graveyard, where everyone sees it (the move that
                // follows is this same line, and is not written twice). A
                // card Library of Leng put on top of a library instead was
                // not revealed (CR 701.9c): only its discarder is told what
                // it was.
                discarded.insert(*object);
                let (card, _) = self.refer(state, *object);
                let in_library = state
                    .object(*object)
                    .is_some_and(|o| o.zone == Zone::Library);
                let sees = if in_library && !known {
                    Sees::Only(std::iter::once(*player).collect())
                } else {
                    Sees::Everyone
                };
                self.push(
                    LogEvent::Discarded {
                        player: *player,
                        card,
                    },
                    vec![sees],
                );
            }
            GameEvent::BecameAttacker { object, defending } => {
                let (attacker, sees) = self.refer(state, *object);
                self.push(
                    LogEvent::Attacked {
                        attacker,
                        defending: *defending,
                    },
                    vec![sees],
                );
            }
            GameEvent::BecameBlocker { object, attacker } => {
                let (blocker, blocker_sees) = self.refer(state, *object);
                let (attacker, attacker_sees) = self.refer(state, *attacker);
                self.push(
                    LogEvent::Blocked { blocker, attacker },
                    vec![blocker_sees, attacker_sees],
                );
            }
            GameEvent::Banded { object, with } => {
                let (attacker, attacker_sees) = self.refer(state, *object);
                let (with, with_sees) = self.refer(state, *with);
                self.push(
                    LogEvent::Banded { attacker, with },
                    vec![attacker_sees, with_sees],
                );
            }
            GameEvent::PlayerLost { player, reason } => self.push(
                LogEvent::Lost {
                    player: *player,
                    cause: loss_cause(*reason),
                },
                Vec::new(),
            ),
            GameEvent::ControllerChanged { object, old, new } => {
                let (object, sees) = self.refer(state, *object);
                self.push(
                    LogEvent::ControlChanged {
                        object,
                        old: *old,
                        new: *new,
                    },
                    vec![sees],
                );
            }
            GameEvent::DayNightChanged { now } => {
                self.push(
                    LogEvent::DayNight {
                        now: day_night(*now),
                    },
                    Vec::new(),
                );
            }
            GameEvent::BecameMonarch { player } => {
                self.push(LogEvent::BecameMonarch { player: *player }, Vec::new());
            }
            GameEvent::TurnedFaceUp { object } => {
                let (object, sees) = self.refer(state, *object);
                self.push(LogEvent::TurnedFaceUp { object }, vec![sees]);
            }
            GameEvent::Transformed { object, .. } => {
                let (object, sees) = self.refer(state, *object);
                self.push(LogEvent::Transformed { object }, vec![sees]);
            }
            GameEvent::LoopDetected { broken, .. } => {
                self.push(LogEvent::LoopDetected { broken: *broken }, Vec::new());
            }
            GameEvent::GameWon { winner } => {
                // A seat, or every seat on the winning team; nobody for a
                // draw.
                let winners = match winner {
                    Some(victor) => state
                        .players
                        .iter()
                        .enumerate()
                        .map(|(i, player)| (PlayerId::new(seat_byte(i)), player.team))
                        .filter(|(seat, team)| victor.includes(*seat, *team))
                        .map(|(seat, _)| seat)
                        .collect(),
                    None => SeatSet::new(),
                };
                self.push(LogEvent::GameOver { winners }, Vec::new());
            }
        }
    }

    /// An object changed zones.
    #[allow(clippy::too_many_arguments)]
    fn moved(
        &mut self,
        state: &GameState,
        id: ObjectId,
        (from, to): (Zone, Zone),
        place: Option<LibraryPlace>,
        shown: &mut BTreeSet<ObjectId>,
        discarded: &mut BTreeSet<ObjectId>,
        batch: usize,
    ) {
        // Onto the stack is the cast or the ability; off it is the
        // resolution, or the counter that already has its line.
        if from == Zone::Stack || to == Zone::Stack {
            return;
        }
        let hidden = |zone: Zone| matches!(zone, Zone::Library | Zone::Hand);
        if !self.started && hidden(from) && hidden(to) {
            return;
        }
        if from == Zone::Hand && discarded.remove(&id) {
            return;
        }
        if to == Zone::Battlefield
            && let Some(player) = self.landing.remove(&id)
        {
            shown.remove(&id);
            self.unwrite_reveal(id, batch);
            let from = log_zone(from).map(|zone| LogFrom {
                zone,
                owner: state.object(id).map_or(player, |obj| obj.owner),
            });
            let (land, sees) = self.refer(state, id);
            self.push(LogEvent::LandPlayed { player, land, from }, vec![sees]);
            return;
        }
        let revealed = shown.contains(&id);
        if hidden(to) || to == Zone::OutsideGame {
            shown.remove(&id);
        }
        if from == Zone::OutsideGame && to == Zone::Battlefield {
            let Some((object, _, controller, sees)) = self.whole(state, id, to) else {
                return;
            };
            self.push(LogEvent::Created { object, controller }, vec![sees]);
            return;
        }
        // A wish, or a departed player's cards leaving the game: no zone of
        // the game's on one side.
        let (Some(from_zone), Some(to_zone)) = (log_zone(from), log_zone(to)) else {
            return;
        };
        let Some((object, owner, _, sees_to)) = self.whole(state, id, to) else {
            return;
        };
        let sees = if revealed {
            Sees::Everyone
        } else {
            let sees_from = state
                .object(id)
                .map_or(sees_to, |obj| in_zone(state, obj, from, obj.owner));
            sees_from.and(sees_to)
        };
        self.push(
            LogEvent::Moved {
                object,
                owner,
                from: from_zone,
                to: to_zone,
                place: place.map(|place| match place {
                    LibraryPlace::Top => LogPlace::Top,
                    LibraryPlace::Bottom => LogPlace::Bottom,
                    LibraryPlace::FromTop(n) => LogPlace::FromTop(n),
                }),
            },
            vec![sees],
        );
    }

    /// Puts the damage just written before the life it cost, where the
    /// journal recorded the loss first: "Bo dealt 2 damage to you", then
    /// "you went to 14 life (from 16)", which a reader takes in that order
    /// and a client writes as one (`LogLine::follows`). Only this batch's
    /// lines that nobody has been sent move, and only a pair that is the
    /// same amount for the same player.
    fn cause_before_effect(&mut self, batch: usize) {
        let first = batch.max(self.sealed);
        let n = self.lines.len();
        if n < first + 2 {
            return;
        }
        let (life, damage) = (&self.lines[n - 2], &self.lines[n - 1]);
        let pair = matches!(
            (&life.event, &damage.event),
            (
                LogEvent::Life { player, old, new },
                LogEvent::Damage {
                    target: LogTarget::Player(hurt),
                    amount,
                    ..
                },
            ) if player == hurt
                && life.repeat == 1
                && damage.repeat == 1
                && i64::from(*old) - i64::from(*new) == i64::from(*amount)
        );
        if pair {
            self.lines.swap(n - 2, n - 1);
        }
    }

    /// `player`'s library was shuffled: this batch's lines putting cards into
    /// it say they were shuffled in, and a line of its own would say it
    /// twice. Whether there were any.
    fn shuffled_in(&mut self, player: PlayerId, batch: usize) -> bool {
        let first = batch.max(self.sealed);
        let mut any = false;
        for line in self.lines.get_mut(first..).unwrap_or_default() {
            if let LogEvent::Moved {
                owner,
                to: LogZone::Library,
                place,
                ..
            } = &mut line.event
                && *owner == player
            {
                *place = Some(LogPlace::Shuffled);
                any = true;
            }
        }
        any
    }

    /// Takes back this batch's single-card `Revealed` line for `id` when the
    /// move or play that immediately follows already names `id` to the table.
    fn unwrite_reveal(&mut self, id: ObjectId, batch: usize) {
        let first = batch.max(self.sealed);
        if self.lines.len() > first
            && self.lines[self.lines.len() - 1].repeat == 1
            && matches!(
                &self.lines[self.lines.len() - 1].event,
                LogEvent::Revealed { cards, .. }
                    if matches!(cards.as_slice(), [LogObject::Known { id: shown, .. }] if *shown == id)
            )
        {
            self.lines.pop();
        }
    }

    /// Takes back this batch's line moving `id` to `to`, which a later line
    /// of the same action says better. Only a line nobody has been sent is
    /// taken back.
    fn unwrite_move(&mut self, id: ObjectId, to: LogZone, batch: usize) {
        let first = batch.max(self.sealed);
        let moved = self
            .lines
            .get(first..)
            .unwrap_or_default()
            .iter()
            .rposition(|line| {
                line.repeat == 1
                    && matches!(
                        &line.event,
                        LogEvent::Moved {
                            object: LogObject::Known { id: moved, .. },
                            to: now,
                            ..
                        } if *moved == id && *now == to
                    )
            });
        if let Some(at) = moved {
            self.lines.remove(first + at);
            if at > 0
                && self.lines[first + at - 1].repeat == 1
                && matches!(
                    &self.lines[first + at - 1].event,
                    LogEvent::Revealed { cards, .. }
                        if matches!(cards.as_slice(), [LogObject::Known { id: shown, .. }] if *shown == id)
                )
            {
                self.lines.remove(first + at - 1);
            }
        }
    }

    /// `count` cards were drawn: the moves that drew them, the batch's last
    /// lines, become one line. When the top of the library was revealed
    /// (`Courser of Kruphix`), `move_object` recorded a single-card
    /// `Revealed` before each `Moved` so every seat knows the card; those
    /// `Revealed` lines fold into `Drew` too (keeping `Sees::Everyone` on the
    /// drawn cards) instead of breaking the multi-card draw into interleaved
    /// `Revealed`/`Moved` lines and raising a reveal popup.
    fn drew(&mut self, player: PlayerId, count: usize, batch: usize) {
        let first = batch.max(self.sealed);
        let mut start = self.lines.len();
        let mut moved = 0;
        while start > first && moved < count && self.lines[start - 1].repeat == 1 {
            let LogEvent::Moved {
                object,
                owner,
                from: LogZone::Library,
                to: LogZone::Hand,
                ..
            } = &self.lines[start - 1].event
            else {
                break;
            };
            if *owner != player {
                break;
            }
            let drawn = object.clone();
            start -= 1;
            moved += 1;
            if start > first
                && self.lines[start - 1].repeat == 1
                && matches!(
                    &self.lines[start - 1].event,
                    LogEvent::Revealed {
                        player: who,
                        cards,
                    } if *who == player && cards.as_slice() == [drawn]
                )
            {
                start -= 1;
            }
        }
        let mut cards = Vec::new();
        let mut sees = Vec::new();
        for line in self.lines.drain(start..) {
            if let LogEvent::Moved { object, .. } = line.event {
                cards.push(object);
                sees.extend(line.sees);
            }
        }
        // How many is public. A card whose move could not be written (its
        // owner left the game with it in the same action) is still one.
        while cards.len() < count {
            cards.push(LogObject::Hidden);
            sees.push(Sees::NOBODY);
        }
        self.push(LogEvent::Drew { player, cards }, sees);
    }

    /// An ability was put on the stack.
    fn ability(
        &mut self,
        state: &GameState,
        object: ObjectId,
        source: ObjectId,
        controller: PlayerId,
    ) {
        // The monarch's two abilities have no source (CR 724.2), so there is
        // nothing to name one by; what it does has lines of its own.
        if source == ObjectId::NO_SOURCE {
            return;
        }
        let (named, sees) = self.refer(state, source);
        let ability = state.object(object).and_then(log_ability);
        self.naming.abilities += 1;
        if ability.is_none() {
            self.naming.by_source += 1;
        }
        // Once it has left the stack, the ability is named by its source.
        if let Some(seen) = self.seen.get(&source).cloned() {
            self.seen.insert(object, seen);
        }
        let activated = state.object(object).and_then(|obj| activated(state, obj));
        self.push(
            LogEvent::Ability {
                controller,
                source: named,
                ability,
                activated,
            },
            vec![sees],
        );
        self.announce(state, object, controller, source, true);
    }

    /// `id` as it is now, or as it was last seen: its name, and who may know
    /// it where it is.
    fn refer(&mut self, state: &GameState, id: ObjectId) -> (LogObject, Sees) {
        self.naming.references += 1;
        if let Some(obj) = state.object(id) {
            // An ability on the stack is named by its source.
            if obj.kind == ObjectKind::AbilityOnStack
                && let Some(loc) = obj.ability
                && loc.source != id
            {
                self.naming.references -= 1;
                return self.refer(state, loc.source);
            }
            let owner = obj.zone_owner.unwrap_or(obj.owner);
            let sees = in_zone(state, obj, obj.zone, owner);
            return (self.remember(state, obj, sees), sees);
        }
        if let Some(seen) = self.seen.get(&id) {
            return (seen.object.clone(), seen.sees);
        }
        // Never seen, and gone: a token or a copy made and gone within one
        // action, or a card that left the game with its owner from a zone
        // nobody else could see. Nobody is told what it was, or its handle.
        self.naming.unnamed += 1;
        (LogObject::Hidden, Sees::NOBODY)
    }

    /// `id` as it is now in `zone`, or as it was last seen, with its owner
    /// and controller; `None` when the log never saw it.
    fn whole(
        &mut self,
        state: &GameState,
        id: ObjectId,
        zone: Zone,
    ) -> Option<(LogObject, PlayerId, PlayerId, Sees)> {
        self.naming.references += 1;
        if let Some(obj) = state.object(id) {
            // Whose hand it went to is where it is now, if it is still there.
            let holder = if obj.zone == zone {
                obj.zone_owner.unwrap_or(obj.owner)
            } else {
                obj.owner
            };
            let sees = in_zone(state, obj, zone, holder);
            let object = self.remember(state, obj, sees);
            return Some((object, obj.owner, obj.controller, sees));
        }
        if let Some(seen) = self.seen.get(&id) {
            return Some((seen.object.clone(), seen.owner, seen.controller, seen.sees));
        }
        self.naming.dropped += 1;
        None
    }

    /// Names `obj` in full and remembers it.
    fn remember(&mut self, state: &GameState, obj: &GameObject, sees: Sees) -> LogObject {
        let object = LogObject::Known {
            id: obj.id,
            card: obj.card.map(|card| CardIdentity {
                index: card.index,
                print: card.print,
                face: obj.face_index,
            }),
            token: obj.token.map(baylee_cards::tokens::token_id),
            name: state.names.get(crate::view::known_name(obj)).to_string(),
        };
        self.seen.insert(
            obj.id,
            Seen {
                object: object.clone(),
                owner: obj.owner,
                controller: obj.controller,
                sees,
            },
        );
        object
    }

    /// Appends a line, or folds it into the ones before it when it says what
    /// they said again.
    fn push(&mut self, event: LogEvent, sees: Vec<Sees>) {
        let line = Line {
            turn: self.turn,
            repeat: 1,
            at: self.now,
            event,
            sees,
            phase: self.phase,
        };
        if !self.fold(&line) {
            self.lines.push(line);
        }
    }

    /// Folds `line` into the log when it completes a repeat of the lines
    /// before it: the last `k - 1` lines and this one say again what the `k`
    /// lines before them said. A real loop runs thousands of times before
    /// the engine calls it one, and this is what keeps it a few lines long.
    ///
    /// Only lines nobody has been sent fold, and only public ones.
    fn fold(&mut self, line: &Line) -> bool {
        let n = self.lines.len();
        for k in 1..=FOLD_SPAN {
            let Some(start) = (n + 1).checked_sub(2 * k) else {
                break;
            };
            if start < self.sealed {
                break;
            }
            let (first, second) = self.lines[start..].split_at(k);
            let again = first
                .iter()
                .zip(second.iter().chain(std::iter::once(line)))
                .all(|(a, b)| a.public() && b.public() && a.folds_with(b));
            if again {
                let later: Vec<Line> = self
                    .lines
                    .drain(start + k..)
                    .chain(std::iter::once(line.clone()))
                    .collect();
                for (earlier, later) in self.lines[start..].iter_mut().zip(later) {
                    earlier.absorb(later);
                }
                return true;
            }
        }
        false
    }
}

/// Who may know `obj` while it is in `zone`, whose owner is `owner`.
///
/// The view's own rules: a hand is its owner's, a library nobody's, and a
/// public zone everyone's unless the object is face down there, when it is
/// the seats [`may_know_card`] says.
fn in_zone(state: &GameState, obj: &GameObject, zone: Zone, owner: PlayerId) -> Sees {
    match zone {
        Zone::Hand => Sees::Only(std::iter::once(owner).collect()),
        Zone::Library | Zone::OutsideGame => Sees::NOBODY,
        Zone::Battlefield | Zone::Stack | Zone::Graveyard | Zone::Exile | Zone::Command => {
            let seats = state.players.len();
            let knows: SeatSet = (0..seats)
                .map(|i| PlayerId::new(seat_byte(i)))
                .filter(|seat| may_know_card(obj, *seat))
                .collect();
            if knows.len() == seats {
                Sees::Everyone
            } else {
                Sees::FaceDown(knows)
            }
        }
    }
}

/// Whether the ability `obj` is on the stack was activated (CR 602) or
/// triggered (CR 603), read off the definition it was put there from, as
/// resolution reads it; `None` for one the definition does not say.
fn activated(state: &GameState, obj: &GameObject) -> Option<bool> {
    use baylee_cards_dsl::AbilityDef;
    use baylee_engine::copiable_abilities::AbilityDefs;
    let loc = obj.ability?;
    // A rule's own trigger (prowess, ward), and an ability a permanent was
    // granted, which is always one its controller activates.
    if loc.index == AbilityRef::SYNTHETIC {
        return Some(false);
    }
    if baylee_engine::choice::granted_slot(loc.index).is_some() {
        return Some(true);
    }
    let abilities = obj.own_abilities.as_ref().map_or_else(
        || {
            state
                .object(loc.source)
                .map_or(AbilityDefs::EMPTY, |source| {
                    source.printed_abilities(&crate::session::RegistryLookup)
                })
        },
        AbilityDefs::from,
    );
    match abilities.get(usize::try_from(loc.index).ok()?)? {
        AbilityDef::Activated { .. }
        | AbilityDef::ActivatedConditional { .. }
        | AbilityDef::Loyalty { .. } => Some(true),
        AbilityDef::Triggered { .. }
        | AbilityDef::ModalTriggered { .. }
        | AbilityDef::SagaChapter { .. } => Some(false),
        _ => None,
    }
}

/// The ability `obj` is on the stack, as the view's stack entry names it.
fn log_ability(obj: &GameObject) -> Option<LogAbility> {
    let baylee_view::StackItem::Ability {
        ability,
        text,
        rules,
        ..
    } = crate::view::stack_item(obj)?
    else {
        return None;
    };
    Some(LogAbility {
        ability,
        text,
        rules,
    })
}

/// The ability a seat's own policy answered for it (#234), named to that
/// seat as a log line names one: in full while `object` is on the stack, by
/// its reference alone once it has left, and not at all where the seat may
/// not know its source.
///
/// A source that has left the state is not asked about: the ability was on
/// the stack, where every seat's view named it.
pub(crate) fn policy_ability(
    state: &GameState,
    object: Option<ObjectId>,
    ability: AbilityRef,
    seat: PlayerId,
) -> LogAbility {
    let by_ref = LogAbility {
        ability: Some(ability),
        text: None,
        rules: None,
    };
    let Some(obj) = object.and_then(|id| state.object(id)) else {
        return by_ref;
    };
    // A spell's own question has the spell as its source.
    let source = obj
        .ability
        .map_or(Some(obj), |loc| state.object(loc.source));
    if let Some(source) = source
        && !in_zone(
            state,
            source,
            source.zone,
            source.zone_owner.unwrap_or(source.owner),
        )
        .names_to(seat)
    {
        return LogAbility {
            ability: None,
            text: None,
            rules: None,
        };
    }
    log_ability(obj).map_or(by_ref, |full| LogAbility {
        ability: Some(ability),
        ..full
    })
}

/// Where `id` came from before this batch put it where it is, as a play or
/// a cast line names it.
fn came_from(state: &GameState, id: ObjectId, came: &BTreeMap<ObjectId, Zone>) -> Option<LogFrom> {
    let zone = log_zone(*came.get(&id)?)?;
    let owner = state.object(id)?.owner;
    Some(LogFrom { zone, owner })
}

/// The zone a log line names, where there is one.
const fn log_zone(zone: Zone) -> Option<LogZone> {
    match zone {
        Zone::Library => Some(LogZone::Library),
        Zone::Hand => Some(LogZone::Hand),
        Zone::Battlefield => Some(LogZone::Battlefield),
        Zone::Graveyard => Some(LogZone::Graveyard),
        Zone::Exile => Some(LogZone::Exile),
        Zone::Command => Some(LogZone::Command),
        Zone::Stack | Zone::OutsideGame => None,
    }
}

fn seat_byte(index: usize) -> u8 {
    u8::try_from(index).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_cards_dsl::{
        AbilityDef, CardDef, Coverage, Effect, FaceDef, Filter, PlayerRel, StepKind, TargetReq,
        TargetSpec, Trigger,
    };
    use baylee_core::ids::{CardIndex, PrintRef};
    use baylee_core::preset::{
        AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, LoopPolicy, PrintInfo,
        SeatController, SeatSpec,
    };
    use baylee_core::types::TypeSet;
    use baylee_engine::choice::{Pending, PlayerAction};
    use baylee_engine::engine::Engine;
    use baylee_engine::loops::LoopWatch;
    use baylee_engine::state::CardLookup;

    // The engine's own endless loop (`loop_tests.rs`): a creature whose
    // trigger blinks itself, and whose return triggers it again.
    const LOOPING: CardIndex = CardIndex::new(60_000);
    const FILLER: CardIndex = CardIndex::new(60_001);
    static SELF_ONLY: Filter = Filter::This;
    static BLINK_SELF: &[Effect] = &[Effect::Blink {
        target: TargetSpec::ThisObject,
        owner_control: true,
    }];
    static LOOPING_CARD: CardDef = CardDef {
        index: LOOPING,
        oracle_id: "test-looping",
        scryfall_id: "test-looping",
        faces: &[FaceDef {
            name: "Möbius Familiar",
            types: TypeSet::CREATURE,
            power: Some(1),
            toughness: Some(1),
            ..FaceDef::DEFAULT
        }],
        coverage: Coverage::Implemented,
        abilities: &[
            AbilityDef::Triggered {
                trigger: Trigger::StepBegin {
                    step: StepKind::Upkeep,
                    whose: PlayerRel::You,
                },
                effects: BLINK_SELF,
                targets: Some(TargetReq::one(TargetSpec::ThisObject)),
                second_targets: None,
                zone: baylee_cards_dsl::TriggerZone::Battlefield,
                once_per_turn: false,
                condition: None,
                resolves_if_targets_illegal: false,
            },
            AbilityDef::Triggered {
                trigger: Trigger::EntersBattlefield(&SELF_ONLY),
                effects: BLINK_SELF,
                targets: Some(TargetReq::one(TargetSpec::ThisObject)),
                second_targets: None,
                zone: baylee_cards_dsl::TriggerZone::Battlefield,
                once_per_turn: false,
                condition: None,
                resolves_if_targets_illegal: false,
            },
        ],
        ..CardDef::DEFAULT
    };
    static FILLER_CARD: CardDef = CardDef {
        index: FILLER,
        oracle_id: "test-filler",
        scryfall_id: "test-filler",
        faces: &[FaceDef {
            name: "Blank",
            types: TypeSet::LAND,
            ..FaceDef::DEFAULT
        }],
        coverage: Coverage::Implemented,
        ..CardDef::DEFAULT
    };

    struct TestPool;
    impl CardLookup for TestPool {
        fn card(&self, index: CardIndex) -> Option<&'static CardDef> {
            match index {
                LOOPING => Some(&LOOPING_CARD),
                FILLER => Some(&FILLER_CARD),
                other => baylee_cards::by_index(other),
            }
        }
    }

    fn looping_game() -> Engine<TestPool> {
        game(vec![LOOPING])
    }

    /// A duel with `battlefield` on seat 0's side and nothing on seat 1's.
    fn game(battlefield: Vec<CardIndex>) -> Engine<TestPool> {
        let entry = |card| DeckEntry {
            rule_print: None,
            choices: baylee_core::card_metadata::DeckChoices::default(),
            card,
            print: PrintRef::new(0),
        };
        let deck: Vec<DeckEntry> = (0..60).map(|_| entry(FILLER)).collect();
        let seat = |battlefield: Vec<DeckEntry>| SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: baylee_core::preset::SeatCapabilities::default(),
            deck: deck.clone(),
            sideboard: vec![],
            commanders: vec![],
            starting_life: None,
            starting_hand: Some(vec![]),
            starting_battlefield: battlefield,
            emblems: vec![],
            team: None,
        };
        let preset = GamePreset {
            rules: baylee_core::card_metadata::RulesMetadata::default(),
            format: FormatId::Freeform,
            seed: 7,
            house_rules: HouseRules {
                loop_policy: LoopPolicy::RunOnceThenBreak,
                ..HouseRules::default()
            },
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: Finish::Normal,
            }],
            seats: vec![
                seat(battlefield.into_iter().map(entry).collect()),
                seat(vec![]),
            ],
        };
        Engine::new(&preset, TestPool).expect("duel starts")
    }

    /// Answers whatever is pending in the most passive way, as the engine's
    /// loop test does, reading the log after every answer as a session does.
    fn play_passively(engine: &mut Engine<TestPool>, log: &mut GameLog, answers: u32) {
        for _ in 0..answers {
            let (player, action) = match engine.pending().clone() {
                Pending::Mulligan { player, .. } => (player, PlayerAction::MulliganKeep),
                Pending::Priority { player, .. } => (player, PlayerAction::PassPriority),
                Pending::ChooseAttackers { player, .. } => {
                    (player, PlayerAction::DeclareAttackers { attackers: vec![] })
                }
                Pending::ChooseBlockers { player, .. } => {
                    (player, PlayerAction::DeclareBlockers { blockers: vec![] })
                }
                Pending::ChooseTargets {
                    player,
                    options,
                    min,
                    ..
                } => (
                    player,
                    PlayerAction::ChooseObjects {
                        objects: options
                            .iter()
                            .take(usize::try_from(min).unwrap())
                            .copied()
                            .collect(),
                    },
                ),
                _ => return,
            };
            engine.apply(player, action).expect("a passive answer");
            log.consume(engine.state());
        }
    }

    /// A band is announced (CR 508.1e), so its line is told to every seat,
    /// the defending one included: the band decides what a block on one of
    /// its members blocks (CR 702.22h).
    #[test]
    fn a_band_is_a_line_every_seat_reads() {
        let card = |oracle| {
            baylee_cards::by_oracle_id(oracle)
                .expect("in the pool")
                .index
        };
        let hero = card("4c81cfb7-8765-4e28-ae33-4287fa9a86cc");
        let wolves = card("35d07ac9-b184-4b5f-8192-34b1db042f69");
        let mut engine = game(vec![hero, wolves]);
        let mut log = GameLog::new(engine.state());
        play_passively(&mut engine, &mut log, 2);
        for _ in 0..20 {
            match engine.pending().clone() {
                Pending::ChooseAttackers { .. } => break,
                Pending::Priority { player, .. } => {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
                other => panic!("unexpected: {other:?}"),
            }
        }
        let Pending::ChooseAttackers {
            player, attackers, ..
        } = engine.pending().clone()
        else {
            panic!("seat 0 was never asked to attack");
        };
        let them = baylee_core::ids::Defender::Player(PlayerId::new(1));
        engine
            .apply(
                player,
                PlayerAction::DeclareAttackers {
                    attackers: attackers.iter().map(|a| (*a, them)).collect(),
                },
            )
            .unwrap();
        let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
            panic!("expected the band question, got {:?}", engine.pending());
        };
        engine
            .apply(player, PlayerAction::ChooseObjects { objects: options })
            .unwrap();
        log.consume(engine.state());
        for seat in [PlayerId::new(0), PlayerId::new(1)] {
            let lines = log.told(seat, 0, log.len());
            assert_eq!(
                lines
                    .iter()
                    .filter(|l| matches!(l.event, LogEvent::Banded { .. }))
                    .count(),
                1,
                "seat {seat:?} reads one band line: {lines:?}"
            );
        }
    }

    /// A daybound card that enters at night enters with its back face up
    /// (CR 702.145b, CR 712.14a), and nothing was turned over, so the log
    /// does not tell the table that it transformed. The "transformed" line
    /// is read off the engine's `Transformed` entry, which the entry scan
    /// used to journal for it.
    ///
    /// The board is set directly: it is night, and the card is moved from
    /// the hand onto the battlefield, which journals the arrival the scan
    /// reads. Passing priority is only the nudge that runs the machine.
    #[test]
    fn a_werewolf_entering_at_night_is_not_logged_as_transformed() {
        let tavern_ruffian = baylee_cards::by_oracle_id("73a3b9a1-37a0-469a-9557-8c118a1ee78f")
            .expect("Tavern Ruffian is in the pool")
            .index;
        let entry = |card| DeckEntry {
            rule_print: None,
            choices: baylee_core::card_metadata::DeckChoices::default(),
            card,
            print: PrintRef::new(0),
        };
        let seat = |hand: Vec<DeckEntry>| SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: baylee_core::preset::SeatCapabilities {
                dev_commands: true,
                see_hidden: false,
            },
            deck: (0..60).map(|_| entry(FILLER)).collect(),
            sideboard: vec![],
            commanders: vec![],
            starting_life: None,
            starting_hand: Some(hand),
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        let preset = GamePreset {
            rules: baylee_core::card_metadata::RulesMetadata::default(),
            format: FormatId::Freeform,
            seed: 8,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: Finish::Normal,
            }],
            seats: vec![seat(vec![entry(tavern_ruffian)]), seat(vec![])],
        };
        let mut engine = Engine::new(&preset, TestPool).expect("duel starts");
        while let Pending::Mulligan { player, .. } = engine.pending().clone() {
            engine
                .apply(player, PlayerAction::MulliganKeep)
                .expect("a seat may keep");
        }
        let mut log = GameLog::new(engine.state());
        let from = log.len();
        let me = PlayerId::new(0);
        let card = engine.state().zones.list(ZoneLocation::Hand(me))[0];
        let state = engine
            .dev_state_mut(me)
            .expect("the harness may set boards up");
        state.day_night = Some(baylee_engine::turn::DayNight::Night);
        state
            .move_object(
                card,
                ZoneLocation::Battlefield,
                baylee_engine::zone::ZonePosition::Top,
                baylee_engine::event::Cause::Effect,
            )
            .expect("the harness moves a card");
        let Pending::Priority { player, .. } = engine.pending().clone() else {
            panic!("priority after the mulligans: {:?}", engine.pending())
        };
        engine
            .apply(player, PlayerAction::PassPriority)
            .expect("a seat may always pass");
        log.consume(engine.state());

        assert_eq!(
            engine.state().object(card).map(|o| o.face_index),
            Some(1),
            "it entered with its back face up"
        );
        let lines = log.told(me, from, log.len());
        assert!(
            lines
                .iter()
                .all(|line| !matches!(line.event, LogEvent::Transformed { .. })),
            "no line says it transformed: {lines:?}"
        );
    }

    /// A real endless loop runs thousands of times before the engine calls it
    /// one, and the log keeps it to a few lines that say how many times.
    #[test]
    fn an_endless_loop_is_a_few_lines_long() {
        let mut engine = looping_game();
        let mut log = GameLog::new(engine.state());
        let answers = (LoopWatch::WATCH_AFTER + LoopWatch::SAMPLE_EVERY * 24) as u32;
        play_passively(&mut engine, &mut log, answers);
        let lines = log.told(PlayerId::new(1), 0, log.len());
        let broken = lines
            .iter()
            .position(|line| matches!(line.event, LogEvent::LoopDetected { broken: true }))
            .expect("the loop was broken, and logged");
        let repeats: u32 = lines[..broken].iter().map(|line| line.repeat).sum();
        let journal = engine.state().journal.len();
        eprintln!(
            "journal {journal} entries; log {} lines, {repeats} before the break folded into {broken}",
            lines.len()
        );
        for line in &lines {
            eprintln!("{line:?}");
        }
        assert!(repeats > 1_000, "the loop ran: {repeats}");
        // The turn the loop ran and broke in. The answer budget reaches on
        // into a later turn of the looping player, where the same permanent
        // starts again, and how far it reaches moves with every priority
        // window before the loop: CR 103.8a's missing first draw step gave
        // it two answers more and a seventeenth line that is no part of the
        // loop being folded.
        let that_turn = lines[broken].turn;
        let in_that_turn = lines.iter().filter(|line| line.turn == that_turn).count();
        assert!(in_that_turn <= 16, "{in_that_turn} lines");
    }

    /// What a seat's policy answers for it is told to that seat alone, in
    /// its view (#234), and is no line: a line would reach every seat, and a
    /// line kept to one would stop the loop that seat yields to from folding
    /// for the whole table. The loop reads as it does when nobody yields.
    #[test]
    fn a_loop_a_seat_yields_to_still_folds() {
        let other = PlayerId::new(1);
        let first_loop = |yields: bool| {
            let mut engine = looping_game();
            let mut log = GameLog::new(engine.state());
            for index in (0..2).filter(|_| yields) {
                engine
                    .apply(
                        other,
                        PlayerAction::SetAbilityPolicy {
                            ability: AbilityRef::new(LOOPING, index),
                            pass: true,
                            answer: None,
                        },
                    )
                    .expect("a seat may set a policy at any time");
            }
            let answers = (LoopWatch::WATCH_AFTER + LoopWatch::SAMPLE_EVERY * 24) as u32;
            play_passively(&mut engine, &mut log, answers);
            let yielded = engine
                .state()
                .journal
                .entries()
                .iter()
                .filter(|entry| matches!(entry.event, GameEvent::AutoAnswered { .. }))
                .count();
            let mut lines = log.told(other, 0, log.len());
            let broken = lines
                .iter()
                .position(|line| matches!(line.event, LogEvent::LoopDetected { broken: true }))
                .expect("the loop was broken, and logged");
            lines.truncate(broken + 1);
            (lines, yielded)
        };
        let (plain, none) = first_loop(false);
        let (yielded_to, yielded) = first_loop(true);
        assert_eq!(none, 0);
        assert!(
            yielded > 1_000,
            "the other seat's policy passed {yielded} times"
        );
        let repeats: u32 = yielded_to.iter().map(|line| line.repeat).sum();
        assert!(repeats > 1_000, "the loop ran: {repeats}");
        // How often it ran, and where in its cycle the engine broke it,
        // depend on how many questions were asked; the fold does not.
        let folded = |lines: &[LogEntry]| -> Vec<LogEvent> {
            lines
                .iter()
                .filter(|line| line.repeat > 1)
                .map(|line| line.event.clone())
                .collect()
        };
        assert!(!folded(&plain).is_empty(), "the plain loop folds");
        assert_eq!(folded(&yielded_to), folded(&plain));
    }

    /// An ability a policy answered for is named by its handle alone once
    /// its object has left the stack (#234): the handle is the one the seat
    /// gave the policy.
    #[test]
    fn a_policy_ability_that_has_left_the_stack_is_named_by_its_handle() {
        let engine = looping_game();
        let ability = AbilityRef::new(LOOPING, 1);
        let by_handle = LogAbility {
            ability: Some(ability),
            text: None,
            rules: None,
        };
        for object in [None, Some(ObjectId::new(9_999, 0))] {
            assert_eq!(
                policy_ability(engine.state(), object, ability, PlayerId::new(1)),
                by_handle
            );
        }
    }

    fn known(id: u32) -> LogObject {
        LogObject::Known {
            id: ObjectId::new(id, 0),
            card: None,
            token: None,
            name: format!("object {id}"),
        }
    }

    fn line(event: LogEvent, sees: Vec<Sees>) -> Line {
        Line {
            turn: 1,
            repeat: 1,
            at: 0,
            event,
            sees,
            phase: None,
        }
    }

    /// A name a line holds no audience for is told to nobody: a line built
    /// wrong fails closed.
    #[test]
    fn a_name_with_no_audience_is_told_to_nobody() {
        let drew = line(
            LogEvent::Drew {
                player: PlayerId::new(0),
                cards: vec![known(1), known(2)],
            },
            vec![Sees::Everyone],
        );
        assert_eq!(
            drew.told(PlayerId::new(0)).event,
            LogEvent::Drew {
                player: PlayerId::new(0),
                cards: vec![known(1), LogObject::Hidden]
            }
        );
    }

    /// Which ability of a card is on the stack says which card it is, so a
    /// seat that may not know the source is not told the ability either.
    #[test]
    fn an_ability_whose_source_a_seat_may_not_know_is_not_named() {
        let ability = LogAbility {
            ability: Some(AbilityRef::new(CardIndex::new(7), 0)),
            text: None,
            rules: None,
        };
        let only_zero = line(
            LogEvent::Ability {
                controller: PlayerId::new(0),
                source: known(1),
                ability: Some(ability),
                activated: None,
            },
            vec![Sees::Only(std::iter::once(PlayerId::new(0)).collect())],
        );
        assert!(matches!(
            only_zero.told(PlayerId::new(0)).event,
            LogEvent::Ability {
                ability: Some(_),
                ..
            }
        ));
        assert_eq!(
            only_zero.told(PlayerId::new(1)).event,
            LogEvent::Ability {
                controller: PlayerId::new(0),
                source: LogObject::Hidden,
                ability: None,
                activated: None,
            }
        );
    }

    fn a_log() -> GameLog {
        GameLog {
            now: 0,
            read: 0,
            lines: Vec::new(),
            sealed: 0,
            seen: BTreeMap::new(),
            started: true,
            turn: 1,
            phase: None,
            aims: BTreeMap::new(),
            landing: BTreeMap::new(),
            naming: Naming::default(),
        }
    }

    /// Only lines every seat is told in full fold: whether two private lines
    /// were the same is itself something only their audience may know.
    /// A run folded into one line is dated by its first line: the time a
    /// panel shows is when it began.
    #[test]
    fn a_folded_line_keeps_the_time_it_began() {
        let mut log = a_log();
        let life = |old, new| LogEvent::Life {
            player: PlayerId::new(0),
            old,
            new,
        };
        log.tell_time(5);
        log.push(life(20, 18), Vec::new());
        log.tell_time(9);
        log.push(life(18, 16), Vec::new());
        let told = log.told(PlayerId::new(0), 0, log.len());
        assert_eq!(told.len(), 1, "{told:?}");
        assert_eq!((told[0].at, told[0].repeat), (5, 2));
    }

    #[test]
    fn a_line_some_seat_may_not_know_never_folds() {
        let mut log = a_log();
        let mine = vec![Sees::Only(std::iter::once(PlayerId::new(0)).collect())];
        let put_back = || LogEvent::Moved {
            object: known(1),
            owner: PlayerId::new(0),
            from: LogZone::Hand,
            to: LogZone::Library,
            place: Some(LogPlace::Top),
        };
        log.push(put_back(), mine.clone());
        log.push(put_back(), mine);
        assert_eq!(log.len(), 2);
        log.push(
            LogEvent::Shuffled {
                player: PlayerId::new(0),
            },
            Vec::new(),
        );
        log.push(
            LogEvent::Shuffled {
                player: PlayerId::new(0),
            },
            Vec::new(),
        );
        assert_eq!(log.len(), 3, "a public line folds");
        assert_eq!(log.lines[2].repeat, 2);
    }

    /// A line that may have been sent never changes, so every seat is told
    /// the same line at the same index.
    #[test]
    fn a_line_that_may_have_been_sent_never_changes() {
        let mut log = a_log();
        log.push(
            LogEvent::Shuffled {
                player: PlayerId::new(0),
            },
            Vec::new(),
        );
        log.seal(1);
        log.push(
            LogEvent::Shuffled {
                player: PlayerId::new(0),
            },
            Vec::new(),
        );
        assert_eq!(log.len(), 2);
        assert_eq!(log.lines[0].repeat, 1);
    }

    /// Life changing the same way again folds into one line spanning both;
    /// the other way does not.
    #[test]
    fn life_folds_only_the_same_way_and_end_to_end() {
        let mut log = a_log();
        let life = |old, new| LogEvent::Life {
            player: PlayerId::new(0),
            old,
            new,
        };
        log.push(life(20, 21), Vec::new());
        log.push(life(21, 22), Vec::new());
        log.push(life(22, 20), Vec::new());
        log.push(life(20, 25), Vec::new());
        let told: Vec<(u32, LogEvent)> = log
            .told(PlayerId::new(1), 0, log.len())
            .into_iter()
            .map(|entry| (entry.repeat, entry.event))
            .collect();
        assert_eq!(
            told,
            [(2, life(20, 22)), (1, life(22, 20)), (1, life(20, 25))]
        );
    }

    /// What `cause_before_effect` is asked about: the journal's order, a
    /// life change first and the damage after, as `(life, damage)` would
    /// read in the log.
    fn life_then_damage(
        life: (u8, i32, i32),
        damage: (u8, u32),
        sealed: bool,
    ) -> Vec<&'static str> {
        let mut log = a_log();
        log.push(
            LogEvent::Life {
                player: PlayerId::new(life.0),
                old: life.1,
                new: life.2,
            },
            Vec::new(),
        );
        if sealed {
            log.seal(1);
        }
        log.push(
            LogEvent::Damage {
                source: Some(known(1)),
                target: LogTarget::Player(PlayerId::new(damage.0)),
                amount: damage.1,
                combat: false,
            },
            Vec::new(),
        );
        log.cause_before_effect(0);
        log.lines
            .iter()
            .map(|l| match l.event {
                LogEvent::Life { .. } => "life",
                LogEvent::Damage { .. } => "damage",
                _ => "other",
            })
            .collect()
    }

    /// The loss of life the journal wrote before the damage that caused it is
    /// told after it: same player, same amount. A different amount (the
    /// damage was prevented in part, or the life change is another's) or
    /// another player's is not the same cause and keeps its place, and a
    /// line that may already have been sent never moves.
    #[test]
    fn damage_is_told_before_the_life_it_cost_and_only_for_that_life() {
        assert_eq!(
            life_then_damage((1, 20, 17), (1, 3), false),
            ["damage", "life"]
        );
        assert_eq!(
            life_then_damage((1, 20, 18), (1, 3), false),
            ["life", "damage"],
            "two lost against three dealt"
        );
        assert_eq!(
            life_then_damage((1, 20, 17), (0, 3), false),
            ["life", "damage"],
            "another player's life"
        );
        assert_eq!(
            life_then_damage((1, 20, 23), (1, 3), false),
            ["life", "damage"],
            "a gain of the same size is not a cost"
        );
        assert_eq!(
            life_then_damage((1, 20, 17), (1, 3), true),
            ["life", "damage"],
            "a line sent already stays where it was told"
        );
    }

    /// The modes chosen for a spell say which card it is, as the ability
    /// does: a seat that may not know the source is told neither the face
    /// nor the modes, but still the targets.
    #[test]
    fn an_aimed_line_whose_source_a_seat_may_not_know_names_no_modes() {
        let rules = RulesFace {
            card: CardIndex::new(7),
            face: 0,
        };
        let aimed = line(
            LogEvent::Aimed {
                controller: PlayerId::new(0),
                source: known(1),
                ability: false,
                rules: Some(rules),
                modes: 0b101,
                targets: vec![
                    LogTarget::Player(PlayerId::new(1)),
                    LogTarget::Object(known(2)),
                ],
            },
            vec![
                Sees::Only(std::iter::once(PlayerId::new(0)).collect()),
                Sees::Everyone,
            ],
        );
        assert!(matches!(
            aimed.told(PlayerId::new(0)).event,
            LogEvent::Aimed {
                rules: Some(_),
                modes: 0b101,
                ..
            }
        ));
        assert_eq!(
            aimed.told(PlayerId::new(1)).event,
            LogEvent::Aimed {
                controller: PlayerId::new(0),
                source: LogObject::Hidden,
                ability: false,
                rules: None,
                modes: 0,
                targets: vec![
                    LogTarget::Player(PlayerId::new(1)),
                    LogTarget::Object(known(2))
                ],
            }
        );
    }

    /// The same public line in two different phases is two lines: a reader
    /// that groups a turn by phase must not find one phase's line folded
    /// into another's. In the same phase it still folds.
    #[test]
    fn equal_lines_in_different_phases_do_not_fold() {
        let mut log = a_log();
        let shuffled = || LogEvent::Shuffled {
            player: PlayerId::new(0),
        };
        log.phase = Some(Phase::FirstMain);
        log.push(shuffled(), Vec::new());
        log.push(shuffled(), Vec::new());
        assert_eq!(log.len(), 1, "alike in one phase folds");
        log.phase = Some(Phase::Combat);
        log.push(shuffled(), Vec::new());
        assert_eq!(log.len(), 2, "the next phase starts a line of its own");
        let told = log.told(PlayerId::new(1), 0, log.len());
        assert_eq!(
            told.iter().map(|e| (e.repeat, e.phase)).collect::<Vec<_>>(),
            [(2, Some(Phase::FirstMain)), (1, Some(Phase::Combat))]
        );
    }
}
