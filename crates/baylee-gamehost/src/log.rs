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
};

use crate::view::{counter, day_night, loss_cause, may_know_card, rules_face, stack_text};

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
        if self.turn != later.turn || self.sees != later.sees {
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
        // Which ability it was says which card its source is.
        if let LogEvent::Ability {
            source, ability, ..
        } = &mut event
            && !matches!(source, LogObject::Known { .. })
        {
            *ability = None;
        }
        LogEntry {
            turn: self.turn,
            repeat: self.repeat,
            at: self.at,
            event,
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
    /// The time as the caller last told it ([`Self::tell_time`]).
    now: u64,
    naming: Naming,
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
        self.look_around(state);
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
            GameEvent::GameStarted { .. }
            | GameEvent::StepChanged { .. }
            | GameEvent::ManaProduced { .. }
            | GameEvent::ObjectTapped { .. }
            | GameEvent::ObjectUntapped { .. }
            | GameEvent::PhaseChanged { .. }
            | GameEvent::StackObjectResolved { .. }
            | GameEvent::DevCommandApplied { .. }
            // What a seat's own policy answered for it is told to that seat
            // alone, in its view (#234). A line here would reach every seat,
            // or, kept to one, stop an automated loop from folding for all
            // of them.
            | GameEvent::AutoAnswered { .. }
            // The table as the preset laid it out is not something that
            // happened in the game.
            | GameEvent::ZoneChanged {
                cause: Cause::Setup,
                ..
            } => {}
            GameEvent::TurnStarted { number, active } => {
                self.started = true;
                self.turn = *number;
                self.push(LogEvent::TurnStarted { active: *active }, Vec::new());
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
            } => self.moved(state, *object, (*from, *to), *place, shown, discarded),
            GameEvent::CardsDrawn { player, count } => {
                if self.started {
                    self.drew(*player, usize::from(*count), batch);
                }
            }
            GameEvent::CounterChanged {
                object,
                kind,
                old,
                new,
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
            }
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
                // The engine moves the land before it records the play, so
                // the move is already a line, and this one says it again
                // with where from.
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
            GameEvent::SpellCast { object, player } => {
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
            GameEvent::Discarded { object, player } => {
                // Into a graveyard, where everyone sees it (the move that
                // follows is this same line, and is not written twice).
                discarded.insert(*object);
                let (card, _) = self.refer(state, *object);
                self.push(
                    LogEvent::Discarded {
                        player: *player,
                        card,
                    },
                    vec![Sees::Everyone],
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
    fn moved(
        &mut self,
        state: &GameState,
        id: ObjectId,
        (from, to): (Zone, Zone),
        place: Option<LibraryPlace>,
        shown: &mut BTreeSet<ObjectId>,
        discarded: &mut BTreeSet<ObjectId>,
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
        }
    }

    /// `count` cards were drawn: the moves that drew them, the batch's last
    /// lines, become one line.
    fn drew(&mut self, player: PlayerId, count: usize, batch: usize) {
        let first = batch.max(self.sealed);
        let mut start = self.lines.len();
        while start > first
            && self.lines.len() - start < count
            && self.lines[start - 1].repeat == 1
            && matches!(
                &self.lines[start - 1].event,
                LogEvent::Moved {
                    owner,
                    from: LogZone::Library,
                    to: LogZone::Hand,
                    ..
                } if *owner == player
            )
        {
            start -= 1;
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
        self.push(
            LogEvent::Ability {
                controller,
                source: named,
                ability,
            },
            vec![sees],
        );
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
            name: state.names.get(obj.characteristics().name).to_string(),
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

/// The ability `obj` is on the stack, as the view's stack entry names it.
fn log_ability(obj: &GameObject) -> Option<LogAbility> {
    let loc = obj.ability?;
    Some(LogAbility {
        ability: loc.card.map(|card| AbilityRef::new(card, loc.index)),
        text: obj
            .printed_face()
            .and_then(|printed| stack_text(printed, loc.index)),
        rules: obj.printed_face().map(rules_face),
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
                once_per_turn: false,
                condition: None,
            },
            AbilityDef::Triggered {
                trigger: Trigger::EntersBattlefield(&SELF_ONLY),
                effects: BLINK_SELF,
                targets: Some(TargetReq::one(TargetSpec::ThisObject)),
                once_per_turn: false,
                condition: None,
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
        let entry = |card| DeckEntry {
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
            seats: vec![seat(vec![entry(LOOPING)]), seat(vec![])],
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
                        objects: options.iter().take(usize::from(min)).copied().collect(),
                    },
                ),
                _ => return,
            };
            engine.apply(player, action).expect("a passive answer");
            log.consume(engine.state());
        }
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
        assert!(lines.len() <= 16, "{} lines", lines.len());
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
                ability: None
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
}
