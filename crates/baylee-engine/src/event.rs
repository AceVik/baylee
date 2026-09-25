//! Game events and the append-only journal.
//!
//! Every mutation of the game produces events; the journal is the ordered
//! record of all of them. It is the single source of truth for replays,
//! spectating, reconnect/resume, crash recovery, and golden tests.

use crate::object::CounterKind;
use crate::turn::{Phase, Step};
use crate::zone::Zone;
use baylee_core::ids::{AbilityRef, ObjectId, PlayerId};
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
    /// A player left the game, and what they controlled that did not leave
    /// with them is exiled (CR 800.4a), or is exiled once the last effect
    /// giving it to a player still in the game ends (CR 800.4c). Neither is
    /// a state-based action.
    PlayerLeft,
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
    /// Cards were shown to every player.
    ///
    /// A search that names anything narrower than "a card" reveals what it
    /// found on the way to a hidden zone — that reveal is the only thing
    /// that holds the searcher to the filter, which is exactly why the
    /// printed cards say it (Mystical Tutor, Cultivate) and why the ones
    /// that fetch to the battlefield do not.
    Revealed {
        /// Who revealed them.
        player: PlayerId,
        /// The cards, in the order they were found.
        cards: Vec<ObjectId>,
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
        /// Where in the library it went, when `to` is a library; `None`
        /// for every other zone (#300). The position is public, even when
        /// the card is not.
        #[serde(default)]
        place: Option<LibraryPlace>,
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
    /// A spell or ability left the stack **without resolving**.
    ///
    /// Two rules, through one door. A triggered ability whose
    /// intervening-`if` clause has stopped being true "is removed from the
    /// stack and does nothing" (CR 603.4); and a spell or ability all of
    /// whose targets have become illegal "doesn't resolve" and is removed
    /// the same way (CR 608.2b). The second one arrived after this comment
    /// said it had not, which is why it is worth saying that the event was
    /// already the right shape for it: the difference between the two is
    /// what left the stack and where it went, not how it is recorded.
    ///
    /// It is deliberately not [`GameEvent::SpellCountered`]: nothing
    /// countered this, and a card that cares about being countered would
    /// read one as the other.
    StackObjectDidNotResolve {
        /// The object.
        object: ObjectId,
    },
    /// A triggered/activated ability was put on the stack.
    AbilityTriggered {
        /// The ability object on the stack.
        object: ObjectId,
        /// Its source permanent/spell.
        source: ObjectId,
        /// Index into the source card's abilities.
        ability_index: u32,
        /// Controlling player.
        controller: PlayerId,
    },
    /// A spell was countered.
    SpellCountered {
        /// The countered spell object.
        object: ObjectId,
    },
    /// A card was discarded.
    Discarded {
        /// The card.
        object: ObjectId,
        /// The player.
        player: PlayerId,
    },
    /// Cards were drawn (drives "whenever you draw" triggers).
    CardsDrawn {
        /// The drawing player.
        player: PlayerId,
        /// How many.
        count: u16,
    },
    /// A creature was declared as attacker.
    BecameAttacker {
        /// The attacking creature.
        object: ObjectId,
        /// What it was declared against.
        defending: baylee_core::ids::Defender,
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
    /// Control of a permanent changed.
    ControllerChanged {
        /// The object.
        object: ObjectId,
        /// Previous controller.
        old: PlayerId,
        /// New controller.
        new: PlayerId,
    },
    /// A permanent phased out or in.
    PhaseChanged {
        /// The object.
        object: ObjectId,
        /// Whether it is now phased out.
        phased_out: bool,
    },
    /// The game became day or night (CR 730.1).
    ///
    /// One event for both directions, because CR 730.1a defines "night
    /// becomes day" as a single change — losing one designation and
    /// gaining the other — and the game's first designation is the same
    /// change from nothing.
    DayNightChanged {
        /// The designation the game now has.
        now: crate::turn::DayNight,
    },
    /// A permanent turned over (CR 701.27).
    ///
    /// Recorded by both doors that flip a face: the one a resolving effect
    /// queues and the one daybound/nightbound take immediately. Without it
    /// a client's `face_index` would be right while nothing that reads the
    /// journal — triggers, replays — knew a transform had happened.
    Transformed {
        /// The permanent.
        object: ObjectId,
        /// The face it now shows.
        face: u8,
    },
    /// A decision-free segment was found to repeat itself: a real endless
    /// loop rather than a large-but-finite pile of work (house rule, see
    /// [`crate::loops`]).
    LoopDetected {
        /// Length of the repeat, in sampled machine iterations.
        period: u64,
        /// `true` when the engine broke the loop and play continued;
        /// `false` when the game ended in a draw instead (CR 104.4b).
        broken: bool,
    },
    /// The game ended with a winner.
    GameWon {
        /// Who won (`None` = draw) — a seat, or a whole team.
        winner: Option<crate::win::Victor>,
    },
    /// A developer command was applied (dev games only).
    DevCommandApplied {
        /// Issuing seat.
        seat: PlayerId,
        /// Command text.
        command: String,
    },
    /// A seat's standing policy for one ability answered a question for it
    /// (#234): passed priority over that ability on top of the stack, or
    /// gave the seat's standing yes or no to that ability's optional
    /// question.
    ///
    /// Recorded only where the per-ability policy made the difference. A
    /// pass the seat's own priority hold would have made anyway is the
    /// hold's, which the seat already sees as held. The policy is the
    /// seat's own setting, so a host tells this to that seat alone; the game
    /// log leaves it out.
    AutoAnswered {
        /// The seat answered for.
        player: PlayerId,
        /// The ability the policy is filed under.
        ability: AbilityRef,
        /// The stack object the question was about: the ability passed
        /// over, or the spell or ability resolving when it asked. `None`
        /// when nothing on the stack asked.
        object: Option<ObjectId>,
        /// What the policy answered.
        answer: PolicyAnswer,
    },
}

/// Where in a library a card was put.
///
/// What the move asked for, not where the card happens to sit: a card put
/// on the bottom of an empty library is [`Self::Bottom`]. A card shuffled
/// in is put on top and then shuffled, and says so as a `Shuffled` of that
/// library right after it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum LibraryPlace {
    /// On top.
    Top,
    /// On the bottom.
    Bottom,
    /// Nth from the top, counting the top card as 1. Never 1 and never the
    /// bottom card: those are [`Self::Top`] and [`Self::Bottom`].
    FromTop(u32),
}

/// What a seat's standing policy for one ability answered for it (#234).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum PolicyAnswer {
    /// Passed priority over the ability on top of the stack.
    Passed,
    /// Said yes to the ability's optional question.
    Yes,
    /// Said no to it.
    No,
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
    /// Twenty-one combat damage from one commander (CR 903.10a).
    ///
    /// Its own reason and not [`Self::Life`] because it is not a life
    /// total: a player on forty life loses to it, and the log saying "life"
    /// there would describe a game nobody played.
    CommanderDamage,
    /// Concession (CR 104.3a).
    Conceded,
    /// An effect said so (CR 104.3e): a pact's "if you don't, you lose the
    /// game" and its kind.
    ///
    /// Its own reason for the same cause [`Self::CommanderDamage`] is: the
    /// seat that did not pay for its pact may be on twenty life, and "life"
    /// would name a loss that did not happen.
    Effect,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn started(seed: u64) -> GameEvent {
        GameEvent::GameStarted { seed, seats: 2 }
    }

    /// A sequence number is how everything else addresses an entry — a
    /// client's "what happened since" and the engine's own replay both take
    /// one and ask for what came after it. So it is 1-based and dense, and
    /// the number [`Journal::record`] hands back is the one the entry it
    /// just appended carries.
    #[test]
    fn a_journal_numbers_its_entries_from_one_and_densely() {
        let mut journal = Journal::default();
        assert!(journal.is_empty());
        assert_eq!(journal.len(), 0);
        assert_eq!(journal.last_seq(), 0, "0 is the empty journal's answer");

        let seqs: Vec<u64> = (0..3).map(|i| journal.record(started(i))).collect();
        assert_eq!(seqs, vec![1, 2, 3]);
        assert!(!journal.is_empty());
        assert_eq!(journal.len(), 3);

        for (i, entry) in journal.entries().iter().enumerate() {
            assert_eq!(entry.seq, seqs[i], "record answered with its own entry");
            assert!(
                matches!(entry.event, GameEvent::GameStarted { seed, .. } if seed == i as u64),
                "entries are in the order they were recorded"
            );
        }
    }

    /// The same number derived two ways, which is the only thing about this
    /// wrapper that can go wrong: [`Journal::last_seq`] counts the entries
    /// and the entry counts itself. They agree for as long as the journal
    /// is append-only, and a removal — a compaction, a rollback, a rewind
    /// that dropped what it undid — would part them silently, because every
    /// reader takes the cheap one.
    #[test]
    fn the_last_sequence_number_agrees_with_the_last_entry() {
        let mut journal = Journal::default();
        for i in 0..5 {
            let seq = journal.record(started(i));
            assert_eq!(journal.last_seq(), seq);
            assert_eq!(
                journal.last_seq(),
                journal.entries().last().expect("just recorded").seq
            );
        }
    }
}
