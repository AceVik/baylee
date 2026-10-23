//! The game log as a seat reads it (#262), with no renderer.
//!
//! The host sends a seat its log in pieces: every `StateDelta` carries a
//! [`LogTail`] beside its view, the lines the seat has not been sent yet.
//! [`LogBook`] keeps them in order, and writes each one in the reader's
//! language when it is read.
//!
//! The book keeps the host's [`LogEntry`]s, never sentences. A line is written
//! every time it is read, because two things it depends on change under it:
//! the language, which a player may switch mid-game, and card text, which
//! arrives from the catalog after the line that names the card.
//!
//! `docs/protocol.md` §"The game log (view version 33, #262)" is the host's
//! half of the contract, and `docs/client.md` §"The game log (#262)" this
//! half's.

use std::collections::BTreeMap;
use std::ops::Range;

use baylee_core::ids::{CardIndex, Defender, ObjectId, PlayerId};
use baylee_view::{
    CardIdentity, ClockAnswer, CounterKind, DayNight, GameStatic, LogAbility, LogEntry, LogEvent,
    LogFrom, LogObject, LogPlace, LogTail, LogTarget, LogZone, LossCause, Phase, PlayerView,
    RulesFace,
};

use crate::card_face::{CardText, shown_name};
use crate::i18n::{Lang, Phrase, seat_name};
use crate::interaction::loss_phrases;

/// Where the log finds the text of a card it names.
///
/// The renderer holds the catalog's text, in the served language or else in
/// English; client-core holds none, so the lookup is handed in. A closure
/// over the renderer's cache is one.
pub trait CardTextLookup {
    /// The text of face `face` of `card`, when the client has it.
    fn text(&self, card: CardIndex, face: u8) -> Option<CardText>;

    /// The words of mode `mode` (from 0) of the card `rules` names, as the
    /// card prints them, when the client can say them. A line about a modal
    /// spell or ability falls back to the mode's number without them.
    fn mode(&self, rules: RulesFace, mode: usize) -> Option<String> {
        let _ = (rules, mode);
        None
    }
}

impl<F> CardTextLookup for F
where
    F: Fn(CardIndex, u8) -> Option<CardText>,
{
    fn text(&self, card: CardIndex, face: u8) -> Option<CardText> {
        self(card, face)
    }
}

/// Who a line is written for, and where its names come from.
#[derive(Clone, Copy)]
pub struct Wording<'a> {
    /// The language to write in.
    pub lang: Lang,
    /// The reading seat. Every line about it is written in the second person.
    pub seat: PlayerId,
    /// The roster, which names every other seat ([`seat_name`]). `None`
    /// numbers them.
    pub statics: Option<&'a GameStatic>,
    /// Card text, for card names in the reader's language.
    pub texts: &'a dyn CardTextLookup,
}

/// One line of the log, written.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LogLine {
    /// Its place in the book, which never changes while the book holds it.
    pub index: usize,
    /// The turn it happened in.
    pub turn: u32,
    /// How many times to say it happened, at least 1.
    ///
    /// The host folds a run of lines that repeats the run before it into one
    /// line, and counts. A life total or counters changing the same way again
    /// fold as well, but into one line whose "was" already spans every
    /// change, so those say 1.
    pub times: u32,
    /// Whether it opens a turn, so a panel may draw it as a heading.
    pub header: bool,
    /// When the host wrote it, in milliseconds since the Unix epoch, for a
    /// panel that shows the time; 0 when the host was never told the time.
    pub at: u64,
    /// The sentence, with no full stop.
    pub text: String,
    /// Where `text` names an object, in order.
    pub names: Vec<NameSpan>,
    /// Where `text` names a player, in order (#300): another seat by its
    /// name wherever the sentence names it, and the reading seat where it is
    /// "you" as a subject or an object. "Your" names a thing and not the
    /// player, and has none. No span here overlaps a [`NameSpan`].
    pub players: Vec<PlayerSpan>,
    /// The ability an ability line names, when the seat may know it, for a
    /// panel that shows its printed sentence the way the stack does.
    pub ability: Option<LogAbility>,
    /// The seat the line is about, for a panel that marks it: the player the
    /// sentence names, as "you" or by name, or whose card moves between
    /// zones. `None` for a line about the table, which names no player or
    /// several: a spell countered or not resolving, counters, a block,
    /// damage to a permanent, a transform, day and night, a loop, the end of
    /// the game.
    pub subject: Option<PlayerId>,
    /// Whether the line crowns a monarch (CR 724), which a panel marks with
    /// the crown the monarch's plate and chip wear.
    pub crowned: bool,
    /// The phase of the turn it happened in, for a panel that groups a
    /// turn's lines by phase. `None` for a turn's heading and the opening
    /// mulligans.
    pub phase: Option<Phase>,
    /// Whether it carries on the line before it: the modes and targets of
    /// the spell cast or the ability put on the stack just above, written
    /// without naming it again.
    pub follows: bool,
}

impl LogLine {
    /// When it was written, as a clock on the wall says it: "14:05", `offset`
    /// seconds from UTC (#300). `None` for a line the host never dated.
    ///
    /// The offset is the caller's to find, because the zone is the device's
    /// and this crate reads no clock and no zone: the renderer asks the
    /// platform for the offset at [`Self::at`], so a line from before a change
    /// to summer time keeps the hour it was written at.
    #[must_use]
    pub fn clock(&self, offset: i32) -> Option<String> {
        if self.at == 0 {
            return None;
        }
        let secs = i64::try_from(self.at / 1000).ok()? + i64::from(offset);
        let day = secs.rem_euclid(86_400);
        Some(format!("{:02}:{:02}", day / 3600, day % 3600 / 60))
    }

    /// The sentence with how often it happened, for a reader that draws
    /// plain text.
    #[must_use]
    pub fn plain(&self, lang: Lang) -> String {
        if self.times > 1 {
            // Not `Phrase::fill`, which replaces one placeholder after the
            // other: a name that reads `{1}` would take the count.
            Piece::fill(
                Phrase::LogRepeated.text(lang),
                &[
                    Piece::plain(self.text.as_str()),
                    Piece::plain(self.times.to_string()),
                ],
            )
            .text
        } else {
            self.text.clone()
        }
    }
}

/// Where a line names an object.
///
/// Only an object the seat's view shows has one: a card the seat may not see
/// is "a card" and carries no handle, so a panel cannot point at it either.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct NameSpan {
    /// Byte range of the name in [`LogLine::text`].
    pub range: Range<usize>,
    /// The object, as the view names it.
    pub id: ObjectId,
    /// Its card, for a preview, as the view showed it when the line was
    /// written: [`CardIdentity::print`] indexes [`GameStatic::prints`], whose
    /// entry has the printing's finish. `None` for a token and a face-down
    /// card.
    pub card: Option<CardIdentity>,
    /// The registry token it is, for a preview of a token that has left the
    /// battlefield since ([`baylee_view::PublicObject::token`]).
    pub token: Option<u16>,
}

/// Where a line names a player.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlayerSpan {
    /// Byte range of the name, or of "you", in [`LogLine::text`].
    pub range: Range<usize>,
    /// The seat.
    pub player: PlayerId,
}

/// A seat's game log, as far as it has been told it.
#[derive(Clone, Debug, Default)]
pub struct LogBook {
    entries: Vec<LogEntry>,
    /// How many entries a reader has taken with [`Self::take_unread`].
    read: usize,
    gaps: u32,
    rewrites: u32,
    /// Every planeswalker a line says was attacked, as the view showed it
    /// when that line arrived. [`Defender`] names it by handle alone, and
    /// once it has died the view cannot name it any more.
    defenders: BTreeMap<ObjectId, Named>,
}

#[derive(Clone, Debug)]
struct Named {
    name: String,
    card: Option<CardIdentity>,
}

impl LogBook {
    /// An empty book, for a game not yet told any of its log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes the tail that arrived with `view`, and says how many entries
    /// were new.
    ///
    /// Call it with every `StateDelta`: a seat with many lines waiting is
    /// sent several frames with the same view and `seq`, each with a part of
    /// the log. Entries the book already holds are skipped, so a chunk sent
    /// twice, a snapshot that tells the log from its start and a reconnect
    /// that tells it again all add only what is missing. An empty tail
    /// changes nothing, wherever it says it starts: a question asked again
    /// carries one that starts at 0.
    ///
    /// A tail that starts past the book's end is refused whole and counted
    /// ([`Self::gaps`]): the lines between are lost to this socket, and the
    /// next tail that starts at 0 fills them. A tail that disagrees with what
    /// the book holds is another history, which the host never tells one
    /// seat (a line it has sent never changes), so it is a book kept across
    /// two games: the tail replaces the book from where they part, and the
    /// book counts it ([`Self::rewrites`]).
    pub fn append(&mut self, tail: &LogTail, view: &PlayerView) -> usize {
        if tail.entries.is_empty() {
            return 0;
        }
        let from = usize::try_from(tail.from).unwrap_or(usize::MAX);
        if from > self.entries.len() {
            self.gaps = self.gaps.saturating_add(1);
            return 0;
        }
        let held = &self.entries[from..];
        let agree = held
            .iter()
            .zip(&tail.entries)
            .take_while(|(held, told)| held == told)
            .count();
        if agree < held.len().min(tail.entries.len()) {
            self.entries.truncate(from + agree);
            self.read = self.read.min(self.entries.len());
            self.defenders.clear();
            self.rewrites = self.rewrites.saturating_add(1);
        }
        let skip = self.entries.len() - from;
        let new = tail.entries.get(skip..).unwrap_or_default();
        for entry in new {
            if let LogEvent::Attacked {
                defending: Defender::Planeswalker(id) | Defender::Battle(id),
                ..
            } = entry.event
                && let Some(object) = view.object(id)
            {
                self.defenders.insert(
                    id,
                    Named {
                        name: object.name.clone(),
                        card: object.card,
                    },
                );
            }
        }
        self.entries.extend_from_slice(new);
        new.len()
    }

    /// Every entry, oldest first.
    #[must_use]
    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    /// How many entries the book holds.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the book holds nothing yet.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many tails were refused because they started past the book's end.
    #[must_use]
    pub const fn gaps(&self) -> u32 {
        self.gaps
    }

    /// How many tails disagreed with the book and replaced part of it.
    #[must_use]
    pub const fn rewrites(&self) -> u32 {
        self.rewrites
    }

    /// Line `index`, written for `wording`.
    #[must_use]
    pub fn line(&self, index: usize, wording: &Wording<'_>) -> Option<LogLine> {
        let entry = self.entries.get(index)?;
        let before = index.checked_sub(1).and_then(|at| self.entries.get(at));
        Some(
            Writer {
                wording,
                defenders: &self.defenders,
            }
            .line(index, entry, before),
        )
    }

    /// The whole log, written for `wording`: what a game-over screen shows.
    #[must_use]
    pub fn lines(&self, wording: &Wording<'_>) -> Vec<LogLine> {
        self.lines_since(0, wording)
    }

    /// Every line from `index` on, for a reader that keeps its own place.
    #[must_use]
    pub fn lines_since(&self, index: usize, wording: &Wording<'_>) -> Vec<LogLine> {
        let writer = Writer {
            wording,
            defenders: &self.defenders,
        };
        self.entries
            .iter()
            .enumerate()
            .skip(index)
            .map(|(at, entry)| {
                let before = at.checked_sub(1).and_then(|at| self.entries.get(at));
                writer.line(at, entry, before)
            })
            .collect()
    }

    /// The whole log with every line a sentence of its own: no line carries
    /// on the line above it ([`LogLine::follows`] is never set), so each
    /// names its subject again. For a reader that sees the lines one by one
    /// and without the panel's indent, such as a language-model seat.
    #[must_use]
    pub fn lines_whole(&self, wording: &Wording<'_>) -> Vec<LogLine> {
        let writer = Writer {
            wording,
            defenders: &self.defenders,
        };
        self.entries
            .iter()
            .enumerate()
            .map(|(at, entry)| writer.line(at, entry, None))
            .collect()
    }

    /// How many lines arrived since [`Self::take_unread`] last took them.
    #[must_use]
    pub const fn unread(&self) -> usize {
        self.entries.len() - self.read
    }

    /// The lines that arrived since the last call, written for `wording`,
    /// which are then read.
    pub fn take_unread(&mut self, wording: &Wording<'_>) -> Vec<LogLine> {
        let lines = self.lines_since(self.read, wording);
        self.read = self.entries.len();
        lines
    }

    /// The modes the latest line announcing `id` says were chosen: the
    /// spell `id`, or with `ability` an ability whose source is `id`
    /// ([`LogEvent::Aimed`]). `None` where no line said any.
    ///
    /// Read back to the start of the turn only: nothing stays on the stack
    /// from one turn into the next, since a step ends only with the stack
    /// empty (CR 500.2).
    #[must_use]
    pub fn modes_of(&self, id: ObjectId, ability: bool) -> Option<(RulesFace, u8)> {
        let mut this_turn = self
            .entries
            .iter()
            .rev()
            .take_while(|entry| !matches!(entry.event, LogEvent::TurnStarted { .. }));
        this_turn.find_map(|entry| match &entry.event {
            LogEvent::Aimed {
                source,
                ability: of_ability,
                rules: Some(rules),
                modes,
                ..
            } if *of_ability == ability && *modes != 0 && handle(source) == Some(id) => {
                Some((*rules, *modes))
            }
            _ => None,
        })
    }

    /// Marks every line read without writing any.
    pub const fn mark_read(&mut self) {
        self.read = self.entries.len();
    }
}

/// Text with the places it names objects.
#[derive(Default)]
struct Piece {
    text: String,
    names: Vec<NameSpan>,
    players: Vec<PlayerSpan>,
    /// Whether the text opens with words of ours in lower case ("a card"),
    /// which a line that opens with them capitalizes. A name is never
    /// recased: a player may call their seat "bo".
    lower: bool,
    /// The seat a whole sentence is about ([`LogLine::subject`]), set where
    /// the sentence chooses between "you" and that seat's name.
    subject: Option<PlayerId>,
}

impl Piece {
    /// The same text and spans, as a new argument.
    fn copy(&self) -> Self {
        Self {
            text: self.text.clone(),
            names: self.names.clone(),
            players: self.players.clone(),
            lower: self.lower,
            subject: self.subject,
        }
    }

    fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            names: Vec::new(),
            players: Vec::new(),
            lower: false,
            subject: None,
        }
    }

    fn push(&mut self, piece: &Self) {
        if self.text.is_empty() {
            self.lower = piece.lower;
        }
        let offset = self.text.len();
        self.text.push_str(&piece.text);
        self.names.extend(piece.names.iter().map(|name| NameSpan {
            range: name.range.start + offset..name.range.end + offset,
            ..name.clone()
        }));
        self.players
            .extend(piece.players.iter().map(|span| PlayerSpan {
                range: span.range.start + offset..span.range.end + offset,
                player: span.player,
            }));
    }

    /// `template` with `{0}`, `{1}` … replaced by `args`, carrying their
    /// names along. A placeholder with no argument is left standing, as
    /// [`Phrase::fill`] leaves it.
    fn fill(template: &str, args: &[Self]) -> Self {
        let mut out = Self::default();
        let mut rest = template;
        while let Some(open) = rest.find('{') {
            out.text.push_str(&rest[..open]);
            let bytes = rest.as_bytes();
            let arg = bytes
                .get(open + 1)
                .filter(|digit| digit.is_ascii_digit() && bytes.get(open + 2) == Some(&b'}'))
                .and_then(|digit| args.get(usize::from(digit - b'0')));
            if let Some(arg) = arg {
                out.push(arg);
                rest = &rest[open + 3..];
            } else {
                out.text.push('{');
                rest = &rest[open + 1..];
            }
        }
        out.text.push_str(rest);
        out
    }
}

mod writer;
use writer::Writer;

/// Whether `entry` carries on `before`: an [`LogEvent::Aimed`] line right
/// after the cast of its spell or the line putting its ability on the
/// stack, or a player's life right after damage to that player. A folded
/// line never does: its count belongs to it alone.
fn follows(entry: &LogEntry, before: Option<&LogEntry>) -> bool {
    let Some(before) = before else {
        return false;
    };
    if before.turn != entry.turn || entry.repeat != 1 || before.repeat != 1 {
        return false;
    }
    // A player's life after the damage that took it there.
    if let (
        LogEvent::Life { player, old, new },
        LogEvent::Damage {
            target: LogTarget::Player(hurt),
            amount,
            ..
        },
    ) = (&entry.event, &before.event)
    {
        return player == hurt && i64::from(*old) - i64::from(*new) == i64::from(*amount);
    }
    let LogEvent::Aimed {
        source, ability, ..
    } = &entry.event
    else {
        return false;
    };
    let named = match (&before.event, ability) {
        (LogEvent::Cast { spell, .. }, false) => spell,
        (LogEvent::Ability { source, .. }, true) => source,
        _ => return false,
    };
    handle(named).is_some() && handle(named) == handle(source)
}

/// The handle a line names an object by, where it names one.
const fn handle(object: &LogObject) -> Option<ObjectId> {
    match object {
        LogObject::Known { id, .. } | LogObject::FaceDown { id } => Some(*id),
        LogObject::Hidden => None,
    }
}

/// Where [`Writer::phrase`] puts the reading seat: `{7}`, `{8}` and `{9}`.
const YOU_SLOT: usize = 7;

/// The log's own sentence for how the reading seat lost, where the end
/// screen's does not name it as "you" a panel can mark: "Your life fell to
/// 0" names the life, and has no "you" to mark.
const fn log_loss(cause: LossCause) -> Option<Phrase> {
    match cause {
        LossCause::Life => None,
        LossCause::EmptyDraw => Some(Phrase::LogLostEmptyDrawYou),
        LossCause::Poison => Some(Phrase::LogLostPoisonYou),
        LossCause::CommanderDamage => Some(Phrase::LogLostCommanderDamageYou),
        LossCause::Conceded => Some(Phrase::LogLostConcededYou),
        LossCause::Effect => Some(Phrase::LogLostEffectYou),
    }
}

/// `n` as an ordinal in `lang`: "2nd", "3rd", "11th" in English; the bare
/// number in German, whose sentence writes the full stop after it.
fn ordinal(lang: Lang, n: u32) -> String {
    match lang {
        Lang::En => {
            let suffix = match (n % 10, n % 100) {
                (_, 11..=13) => "th",
                (1, _) => "st",
                (2, _) => "nd",
                (3, _) => "rd",
                _ => "th",
            };
            format!("{n}{suffix}")
        }
        Lang::De => n.to_string(),
    }
}

#[cfg(test)]
mod tests;
