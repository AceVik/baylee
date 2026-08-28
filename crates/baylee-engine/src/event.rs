//! Game events and the append-only journal.
//!
//! Every mutation of the game produces events; the journal is the ordered
//! record of all of them. It is the single source of truth for replays,
//! spectating, reconnect/resume, crash recovery, and golden tests.

use crate::object::CounterKind;
use crate::turn::{Phase, Step};
use crate::zone::Zone;
use baylee_core::ids::{ObjectId, PlayerId};
use serde::{Deserialize, Serialize};

/// Why an event happened.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Cause {
    /// Game setup.
    Setup,
    /// Casting or resolving a spell.
    Spell,
    /// An activated/triggered ability.
    Ability,
    /// A one-shot or continuous effect.
    Effect,
    /// Paying a cost.
    Cost,
    /// A turn-based action (CR 703).
    TurnBased,
    /// A state-based action (CR 704).
    StateBased,
    /// A developer-mode command (dev games only; journaled for honesty).
    DevCommand,
}

/// What damage was dealt to.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum DamageTarget {
    /// A player.
    Player(PlayerId),
    /// An object (creature/planeswalker/battle).
    Object(ObjectId),
}

/// A single game event.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum GameEvent {
    /// The game was set up from a preset.
    GameStarted {
        /// RNG seed.
        seed: u64,
        /// Number of seats.
        seats: u8,
    },
    /// A zone was shuffled.
    Shuffled {
        /// Whose zone.
        player: PlayerId,
        /// Which zone.
        zone: Zone,
    },
    /// An object changed zone (its `version` bumped, CR 400.7).
    ZoneChanged {
        /// The object.
        object: ObjectId,
        /// Origin zone.
        from: Zone,
        /// Destination zone.
        to: Zone,
        /// Why.
        cause: Cause,
    },
    /// Counters on an object changed.
    CounterChanged {
        /// The object.
        object: ObjectId,
        /// Counter kind.
        kind: CounterKind,
        /// Previous amount.
        old: u16,
        /// New amount.
        new: u16,
    },
    /// A player's life total changed.
    LifeChanged {
        /// The player.
        player: PlayerId,
        /// Previous life.
        old: i32,
        /// New life.
        new: i32,
        /// Why.
        cause: Cause,
    },
    /// A turn began.
    TurnStarted {
        /// Turn number (1-based).
        number: u32,
        /// Active player.
        active: PlayerId,
    },
    /// Phase/step changed.
    StepChanged {
        /// New phase.
        phase: Phase,
        /// New step.
        step: Step,
    },
    /// Damage was dealt.
    DamageDealt {
        /// Source object, if any.
        source: Option<ObjectId>,
        /// What was damaged.
        target: DamageTarget,
        /// Amount (after prevention).
        amount: u16,
        /// Whether it was combat damage.
        is_combat: bool,
    },
    /// A die was rolled (custom modes).
    DiceRolled {
        /// Rolling player.
        player: PlayerId,
        /// Die size.
        sides: u32,
        /// Result.
        result: u32,
    },
    /// Mana was produced.
    ManaProduced {
        /// The player whose pool received it.
        player: PlayerId,
        /// Color of the mana.
        color: baylee_core::mana::ManaColor,
        /// Amount.
        amount: u16,
        /// Producing object, if any.
        source: Option<ObjectId>,
    },
    /// An object became tapped.
    ObjectTapped {
        /// The object.
        object: ObjectId,
        /// Why.
        cause: Cause,
    },
    /// An object became untapped.
    ObjectUntapped {
        /// The object.
        object: ObjectId,
        /// Why.
        cause: Cause,
    },
    /// A land was played.
    LandPlayed {
        /// The land object.
        object: ObjectId,
        /// The playing player.
        player: PlayerId,
    },
    /// A spell was cast (moved to the stack, costs paid).
    SpellCast {
        /// The spell object.
        object: ObjectId,
        /// The casting player.
        player: PlayerId,
    },
    /// A spell or ability resolved and left the stack.
    StackObjectResolved {
        /// The object.
        object: ObjectId,
    },
    /// A card was discarded.
    Discarded {
        /// The card.
        object: ObjectId,
        /// The player.
        player: PlayerId,
    },
    /// A creature was declared as attacker.
    BecameAttacker {
        /// The attacking creature.
        object: ObjectId,
        /// The defending player.
        defending: PlayerId,
    },
    /// A creature was declared as blocker.
    BecameBlocker {
        /// The blocking creature.
        object: ObjectId,
        /// The attacker it blocks.
        attacker: ObjectId,
    },
    /// A player lost the game.
    PlayerLost {
        /// The player.
        player: PlayerId,
        /// Why.
        reason: LossReason,
    },
    /// The game ended with a winner.
    GameWon {
        /// The winner (`None` = draw).
        player: Option<PlayerId>,
    },
    /// A developer command was applied (dev games only).
    DevCommandApplied {
        /// Issuing seat.
        seat: PlayerId,
        /// Command text.
        command: String,
    },
}

/// Why a player lost the game.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum LossReason {
    /// Life total reached 0 or less (CR 104.3b).
    Life,
    /// Drew from an empty library (CR 104.3c).
    EmptyDraw,
    /// Ten or more poison counters (CR 104.3d).
    Poison,
    /// Concession (CR 104.3a).
    Conceded,
}

/// One journaled entry.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct JournalEntry {
    /// 1-based sequence number.
    pub seq: u64,
    /// The event.
    pub event: GameEvent,
}

/// The append-only event journal.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Journal {
    entries: Vec<JournalEntry>,
}

impl Journal {
    /// Appends an event, returning its sequence number.
    pub fn record(&mut self, event: GameEvent) -> u64 {
        let seq = self.entries.len() as u64 + 1;
        self.entries.push(JournalEntry { seq, event });
        seq
    }

    /// All entries in order.
    #[must_use]
    pub fn entries(&self) -> &[JournalEntry] {
        &self.entries
    }

    /// Number of entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the journal is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The latest sequence number (0 = empty).
    #[must_use]
    pub fn last_seq(&self) -> u64 {
        self.entries.len() as u64
    }
}
