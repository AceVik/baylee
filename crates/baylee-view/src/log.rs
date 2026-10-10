use crate::counters::CounterKind;
use crate::objects::{CardIdentity, RulesFace, StackText};
use crate::seats::LossCause;
use crate::turn::DayNight;
use baylee_core::ids::{AbilityRef, Defender, ObjectId, PlayerId, PrintRef, SeatSet};
use serde::{Deserialize, Serialize};

// ----------------------------------------------------------------------- log

/// The most entries one [`LogTail`] carries, so a long automated loop never
/// makes one giant frame. A host with more to send splits it over several
/// frames in one go.
pub const LOG_TAIL_CAP: usize = 256;

/// The part of a seat's game log it has not been sent yet (#262).
///
/// A log is a history, and [`PlayerView`] is a snapshot, so the two travel
/// side by side in one envelope and never inside each other: a full log in
/// every view would grow with the square of the game, and an agent answering
/// from a view has no business reading one.
///
/// `from` is the index in this seat's log of the first entry here. A client
/// appends when `from` is the length of what it holds, skips the overlap when
/// it is less, and marks a gap it cannot fill when it is more, so a line
/// received twice changes nothing. A socket's first tail starts at 0, so a
/// client that reconnects is sent the whole log again. A seat with more than
/// [`LOG_TAIL_CAP`] lines waiting is sent several frames with the same view
/// and `seq`, and reads the tail in every one of them.
///
/// [`PlayerView`]: crate::PlayerView
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct LogTail {
    /// Index of the first entry in this seat's log.
    pub from: u32,
    /// The entries, oldest first.
    pub entries: Vec<LogEntry>,
}

impl LogTail {
    /// Every printing these entries name, so a host can send the print table
    /// a seat needs before the log that points into it.
    pub fn prints(&self) -> impl Iterator<Item = PrintRef> + '_ {
        self.entries
            .iter()
            .flat_map(|entry| entry.event.objects())
            .filter_map(|object| match object {
                LogObject::Known {
                    card: Some(card), ..
                } => Some(card.print),
                _ => None,
            })
    }
}

/// One line of the game log.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    /// The turn it happened in. The opening mulligans are turn 1 too, before
    /// its [`LogEvent::TurnStarted`].
    pub turn: u32,
    /// How many times it happened, at least 1. A run of lines that says again
    /// what the run before it said folds into it, each line counting its
    /// repeats, so a long automated loop is a few lines and not thousands. A
    /// life total or counters changing the same way again fold into one line
    /// whose `old` and `new` span every change.
    pub repeat: u32,
    /// When the host wrote it, in milliseconds since the Unix epoch, as the
    /// host's caller last told it the time; 0 when nobody ever did. A folded
    /// line keeps the time of its first.
    pub at: u64,
    /// What happened.
    pub event: LogEvent,
}

/// An object as the log may name it to one seat: exactly as that seat's view
/// would have shown it when it happened, and never more.
///
/// The three shapes are the view's own. An object the view shows with its
/// card is `Known`. A face-down one the seat may not look at is `FaceDown`,
/// with the handle the view also shows, so a line can point at it on the
/// table. One the view does not show this seat at all, in a library or
/// another player's hand, is `Hidden` and carries **no handle**, so a card
/// cannot be followed from the draw that hid it to the cast that shows it.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum LogObject {
    /// The seat may know what it is.
    Known {
        /// Engine object handle, as the view names it.
        id: ObjectId,
        /// The card, when it is one. `None` for a token, which `name` names.
        card: Option<CardIdentity>,
        /// The registry token it is, as [`PublicObject::token`], so a line
        /// can show a token that has since left the battlefield.
        ///
        /// [`PublicObject::token`]: crate::PublicObject::token
        token: Option<u16>,
        /// Its name as it was then.
        name: String,
    },
    /// Face down, and the seat may not look (CR 708.5).
    FaceDown {
        /// Engine object handle, as the view names it.
        id: ObjectId,
    },
    /// A card the seat may not see: "a card". Also, to every seat, an object
    /// the host never saw at all because it was made and gone within one
    /// action.
    Hidden,
}

/// A zone, as a log line names where something went.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum LogZone {
    /// A library.
    Library,
    /// A hand.
    Hand,
    /// The battlefield.
    Battlefield,
    /// A graveyard.
    Graveyard,
    /// Exile.
    Exile,
    /// The stack.
    Stack,
    /// The command zone.
    Command,
}

/// Where a land was played or a spell cast from: a zone, and whose it is,
/// so a card cast out of another player's graveyard says so.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct LogFrom {
    /// The zone.
    pub zone: LogZone,
    /// Whose it is: the card's owner.
    pub owner: PlayerId,
}

/// Where in a library a card went. Every seat is told it, whoever may know
/// the card: where a card is put is public even when the card is not.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum LogPlace {
    /// On top.
    Top,
    /// On the bottom.
    Bottom,
    /// This many from the top, counting from 1 and never the top or the
    /// bottom card, which are [`Self::Top`] and [`Self::Bottom`].
    FromTop(u32),
    /// Into it, and the library was shuffled afterwards in the same action.
    Shuffled,
}

/// What a damage line dealt damage to.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum LogTarget {
    /// A player.
    Player(PlayerId),
    /// A permanent.
    Object(LogObject),
}

/// The ability a line names, when the seat may know its source.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct LogAbility {
    /// Which ability of which card, as [`StackItem::Ability::ability`].
    ///
    /// [`StackItem::Ability::ability`]: crate::StackItem::Ability::ability
    pub ability: Option<AbilityRef>,
    /// Where its printed sentence is, as [`StackItem::Ability::text`].
    ///
    /// [`StackItem::Ability::text`]: crate::StackItem::Ability::text
    pub text: Option<StackText>,
    /// The card it is printed on, as [`StackItem::Ability::rules`].
    ///
    /// [`StackItem::Ability::rules`]: crate::StackItem::Ability::rules
    pub rules: Option<RulesFace>,
}

/// One answer a seat's standing policy for one ability gave for it (#234).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct PolicyAct {
    /// This seat's policy answers counted from 1 over the whole game, so a
    /// client tells each one once, whichever frame carries it.
    pub number: u32,
    /// Which ability, named as a log line names one: nothing where the seat
    /// may not know the ability's source.
    pub ability: LogAbility,
    /// What the policy answered.
    pub answer: PolicyAnswer,
}

/// What a seat's standing policy for one ability answered for it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum PolicyAnswer {
    /// Passed priority over the ability on top of the stack.
    Passed,
    /// Said yes to the ability's optional question.
    Yes,
    /// Said no to it.
    No,
}

/// What a seat's decision clock answered when it ran out, which is the
/// answer that does nothing wherever there is one.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum ClockAnswer {
    /// Passed priority.
    Passed,
    /// Kept the opening hand.
    Kept,
    /// Declared no attackers.
    NoAttackers,
    /// Declared no blockers.
    NoBlockers,
    /// Declined an optional choice.
    Declined,
    /// A choice with no answer that does nothing, made by the house.
    ChosenForThem,
}

/// What happened, for one line of the log.
///
/// Players and cards only, never text: a client writes the sentence in its
/// own language and takes card names from its card text.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum LogEvent {
    /// A turn began.
    TurnStarted {
        /// Whose turn it is.
        active: PlayerId,
    },
    /// A seat took a mulligan.
    Mulliganed {
        /// The seat.
        player: PlayerId,
    },
    /// A seat kept its opening hand, of this many cards.
    Kept {
        /// The seat.
        player: PlayerId,
        /// How many cards it kept.
        cards: u8,
    },
    /// A seat's decision clock ran out, and this is what it answered.
    TimedOut {
        /// The seat.
        player: PlayerId,
        /// The answer.
        answer: ClockAnswer,
    },
    /// A seat's player is gone, and the house answers for them from here.
    StandIn {
        /// The seat.
        player: PlayerId,
    },
    /// A seat's player lost the connection (not a deliberate leave).
    ConnectionLost {
        /// The seat.
        player: PlayerId,
        /// How long the table waits before the house takes the chair,
        /// counted from this line's time; `None` when no other player is at
        /// the table and the game is paused until they come back.
        wait_secs: Option<u32>,
    },
    /// A seat's player is back in their chair.
    Returned {
        /// The seat.
        player: PlayerId,
    },
    /// A land was played.
    LandPlayed {
        /// Who played it.
        player: PlayerId,
        /// The land.
        land: LogObject,
        /// Where from.
        from: Option<LogFrom>,
    },
    /// A spell was cast.
    Cast {
        /// Who cast it.
        player: PlayerId,
        /// The spell.
        spell: LogObject,
        /// Where from. `None` for a spell that was never anywhere before the
        /// stack: a copy that is cast.
        from: Option<LogFrom>,
    },
    /// An activated or triggered ability was put on the stack.
    Ability {
        /// Who controls it.
        controller: PlayerId,
        /// What it came from.
        source: LogObject,
        /// Which ability, when the seat may know its source.
        ability: Option<LogAbility>,
    },
    /// A spell was countered.
    Countered {
        /// The spell.
        spell: LogObject,
    },
    /// A spell or ability left the stack without resolving.
    DidNotResolve {
        /// What it was.
        object: LogObject,
    },
    /// A player drew cards.
    Drew {
        /// Who drew.
        player: PlayerId,
        /// What they drew, one per card.
        cards: Vec<LogObject>,
    },
    /// A player discarded a card.
    Discarded {
        /// Who discarded it.
        player: PlayerId,
        /// The card.
        card: LogObject,
    },
    /// An object went from one zone to another.
    Moved {
        /// The object.
        object: LogObject,
        /// Whose zones: its owner.
        owner: PlayerId,
        /// Where from.
        from: LogZone,
        /// Where to.
        to: LogZone,
        /// Where in it, when `to` is a library; `None` otherwise.
        place: Option<LogPlace>,
    },
    /// A token was created.
    Created {
        /// The token.
        object: LogObject,
        /// Who controls it.
        controller: PlayerId,
    },
    /// Damage was dealt.
    Damage {
        /// What dealt it, when anything did.
        source: Option<LogObject>,
        /// What it was dealt to.
        target: LogTarget,
        /// How much.
        amount: u32,
        /// Whether it was combat damage.
        combat: bool,
    },
    /// A player's life total changed.
    Life {
        /// The player.
        player: PlayerId,
        /// Before.
        old: i32,
        /// After.
        new: i32,
    },
    /// Counters on an object changed.
    Counters {
        /// The object.
        object: LogObject,
        /// Which counters.
        kind: CounterKind,
        /// Before.
        old: u16,
        /// After.
        new: u16,
    },
    /// A creature attacked.
    Attacked {
        /// The creature.
        attacker: LogObject,
        /// What it attacks.
        defending: Defender,
    },
    /// An attacking creature joined the band of one with banding
    /// (CR 702.22c).
    Banded {
        /// The creature that joined.
        attacker: LogObject,
        /// The creature with banding whose band it joined.
        with: LogObject,
    },
    /// A creature blocked.
    Blocked {
        /// The blocker.
        blocker: LogObject,
        /// What it blocks.
        attacker: LogObject,
    },
    /// Control of a permanent changed.
    ControlChanged {
        /// The permanent.
        object: LogObject,
        /// Who controlled it.
        old: PlayerId,
        /// Who does now.
        new: PlayerId,
    },
    /// A permanent turned over (CR 701.27).
    /// A permanent was turned face up.
    TurnedFaceUp {
        /// The now-public object.
        object: LogObject,
    },
    /// A permanent changed to its other printed face.
    Transformed {
        /// The permanent, as it is now.
        object: LogObject,
    },
    /// Cards were shown to every player.
    Revealed {
        /// Who revealed them.
        player: PlayerId,
        /// The cards.
        cards: Vec<LogObject>,
    },
    /// Publicly chosen permanents to keep; face-down identities stay hidden.
    CardsKept {
        /// Who chose them.
        player: PlayerId,
        /// The kept permanents.
        cards: Vec<LogObject>,
    },
    /// A player's library was shuffled.
    Shuffled {
        /// Whose.
        player: PlayerId,
    },
    /// A die was rolled.
    DiceRolled {
        /// Who rolled it.
        player: PlayerId,
        /// Its sides.
        sides: u32,
        /// The result.
        result: u32,
    },
    /// A player lost the game.
    Lost {
        /// The player.
        player: PlayerId,
        /// Why.
        cause: LossCause,
    },
    /// The game ended.
    GameOver {
        /// The seats that won: one seat, a whole team, or none for a draw.
        winners: SeatSet,
    },
    /// A decision-free segment repeated itself (a house rule).
    LoopDetected {
        /// `true` when the loop was broken and play went on, `false` when
        /// the game ended in a draw (CR 104.4b).
        broken: bool,
    },
    /// It became day or night (CR 730.1).
    DayNight {
        /// Which it is now.
        now: DayNight,
    },
    /// A player became the monarch (CR 724.3). Who the monarch is now is
    /// [`crate::PlayerView::monarch`]; this line says when it changed.
    BecameMonarch {
        /// The new monarch.
        player: PlayerId,
    },
}

impl LogEvent {
    /// Every object the line names, in a fixed order.
    pub fn objects(&self) -> impl Iterator<Item = &LogObject> {
        let mut out: Vec<&LogObject> = Vec::new();
        match self {
            Self::TurnStarted { .. }
            | Self::Mulliganed { .. }
            | Self::Kept { .. }
            | Self::TimedOut { .. }
            | Self::StandIn { .. }
            | Self::ConnectionLost { .. }
            | Self::Returned { .. }
            | Self::Life { .. }
            | Self::Shuffled { .. }
            | Self::BecameMonarch { .. }
            | Self::DiceRolled { .. }
            | Self::Lost { .. }
            | Self::GameOver { .. }
            | Self::LoopDetected { .. }
            | Self::DayNight { .. } => {}
            Self::LandPlayed { land: o, .. }
            | Self::Cast { spell: o, .. }
            | Self::Ability { source: o, .. }
            | Self::Countered { spell: o }
            | Self::DidNotResolve { object: o }
            | Self::Discarded { card: o, .. }
            | Self::Moved { object: o, .. }
            | Self::Created { object: o, .. }
            | Self::Counters { object: o, .. }
            | Self::Attacked { attacker: o, .. }
            | Self::ControlChanged { object: o, .. }
            | Self::TurnedFaceUp { object: o }
            | Self::Transformed { object: o } => out.push(o),
            Self::Drew { cards, .. }
            | Self::Revealed { cards, .. }
            | Self::CardsKept { cards, .. } => out.extend(cards),
            Self::Damage { source, target, .. } => {
                out.extend(source);
                if let LogTarget::Object(o) = target {
                    out.push(o);
                }
            }
            Self::Blocked { blocker, attacker } => {
                out.push(blocker);
                out.push(attacker);
            }
            Self::Banded { attacker, with } => {
                out.push(attacker);
                out.push(with);
            }
        }
        out.into_iter()
    }

    /// Every object the line names, in the same order as [`Self::objects`].
    pub fn objects_mut(&mut self) -> impl Iterator<Item = &mut LogObject> {
        let mut out: Vec<&mut LogObject> = Vec::new();
        match self {
            Self::TurnStarted { .. }
            | Self::Mulliganed { .. }
            | Self::Kept { .. }
            | Self::TimedOut { .. }
            | Self::StandIn { .. }
            | Self::ConnectionLost { .. }
            | Self::Returned { .. }
            | Self::Life { .. }
            | Self::Shuffled { .. }
            | Self::BecameMonarch { .. }
            | Self::DiceRolled { .. }
            | Self::Lost { .. }
            | Self::GameOver { .. }
            | Self::LoopDetected { .. }
            | Self::DayNight { .. } => {}
            Self::LandPlayed { land: o, .. }
            | Self::Cast { spell: o, .. }
            | Self::Ability { source: o, .. }
            | Self::Countered { spell: o }
            | Self::DidNotResolve { object: o }
            | Self::Discarded { card: o, .. }
            | Self::Moved { object: o, .. }
            | Self::Created { object: o, .. }
            | Self::Counters { object: o, .. }
            | Self::Attacked { attacker: o, .. }
            | Self::ControlChanged { object: o, .. }
            | Self::TurnedFaceUp { object: o }
            | Self::Transformed { object: o } => out.push(o),
            Self::Drew { cards, .. }
            | Self::Revealed { cards, .. }
            | Self::CardsKept { cards, .. } => out.extend(cards),
            Self::Damage { source, target, .. } => {
                out.extend(source);
                if let LogTarget::Object(o) = target {
                    out.push(o);
                }
            }
            Self::Blocked { blocker, attacker } => {
                out.push(blocker);
                out.push(attacker);
            }
            Self::Banded { attacker, with } => {
                out.push(attacker);
                out.push(with);
            }
        }
        out.into_iter()
    }
}
