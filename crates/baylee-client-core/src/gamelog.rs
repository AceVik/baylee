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

/// Writes lines for one reader.
struct Writer<'a> {
    wording: &'a Wording<'a>,
    defenders: &'a BTreeMap<ObjectId, Named>,
}

impl Writer<'_> {
    fn line(&self, index: usize, entry: &LogEntry, before: Option<&LogEntry>) -> LogLine {
        let follows = follows(entry, before);
        let mut piece = if follows {
            self.short(entry)
        } else {
            self.sentence(entry)
        };
        if piece.lower {
            // ASCII only, so no span moves.
            if let Some(first) = piece.text.get_mut(..1) {
                first.make_ascii_uppercase();
            }
        }
        let times = match entry.event {
            LogEvent::Life { .. } | LogEvent::Counters { .. } | LogEvent::PlayerCounters { .. } => {
                1
            }
            _ => entry.repeat.max(1),
        };
        LogLine {
            index,
            turn: entry.turn,
            times,
            header: matches!(entry.event, LogEvent::TurnStarted { .. }),
            at: entry.at,
            text: piece.text,
            names: piece.names,
            players: piece.players,
            ability: match entry.event {
                LogEvent::Ability { ability, .. } => ability,
                _ => None,
            },
            subject: piece.subject,
            crowned: matches!(entry.event, LogEvent::BecameMonarch { .. }),
            phase: entry.phase,
            follows,
        }
    }

    /// What an [`LogEvent::Aimed`] line says after its spell or ability:
    /// "mode: …", "targeting …", or both, joined.
    fn aim_parts(&self, entry: &LogEntry) -> Piece {
        let LogEvent::Aimed {
            rules,
            modes,
            targets,
            ..
        } = &entry.event
        else {
            return Piece::default();
        };
        let mut parts = Vec::new();
        if let Some(rules) = rules.filter(|_| *modes != 0) {
            let chosen: Vec<String> = (0..8_usize)
                .filter(|i| modes & (1 << i) != 0)
                .map(|i| {
                    self.wording.texts.mode(rules, i).map_or_else(
                        || Phrase::CastModeNumber.fill(self.wording.lang, &[&(i + 1).to_string()]),
                        |words| Phrase::LogQuoted.fill(self.wording.lang, &[&words]),
                    )
                })
                .collect();
            let phrase = if chosen.len() > 1 {
                Phrase::LogAimedModes
            } else {
                Phrase::LogAimedMode
            };
            parts.push(self.phrase(
                phrase,
                &[Piece::default(), Piece::plain(chosen.join(" + "))],
            ));
        }
        if !targets.is_empty() {
            let named: Vec<Piece> = targets
                .iter()
                .map(|target| match target {
                    LogTarget::Player(player) if *player == self.wording.seat => {
                        self.you(Phrase::LogYouSubject)
                    }
                    LogTarget::Player(player) => self.seat(*player),
                    LogTarget::Object(object) => self.object(object),
                })
                .collect();
            let phrase = if named.len() > 1 {
                Phrase::LogAimedTargets
            } else {
                Phrase::LogAimedTarget
            };
            parts.push(self.phrase(phrase, &[Piece::default(), self.list(named)]));
        }
        let mut out = Piece::default();
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                out.text.push_str("; ");
            }
            out.push(part);
        }
        out.lower = false;
        out
    }

    /// A line under the line it carries on ([`LogLine::follows`]), without
    /// naming again what that line named: an [`LogEvent::Aimed`] line's
    /// parts, about the spell's or the ability's controller, or the life
    /// total that damage left a player at.
    fn short(&self, entry: &LogEntry) -> Piece {
        match entry.event {
            LogEvent::Aimed { controller, .. } => Piece {
                subject: Some(controller),
                ..self.aim_parts(entry)
            },
            LogEvent::Life { player, old, new } => {
                let n = usize::try_from(new).unwrap_or(0);
                Piece {
                    subject: Some(player),
                    ..self.counted(
                        Phrase::LogLifeNowPoints,
                        n,
                        &[Piece::default(), Self::count(&old), Self::count(&new)],
                    )
                }
            }
            _ => self.sentence(entry),
        }
    }

    /// `phrase` filled with `args` from `{0}` on, and with the reading seat
    /// as `{7}`, `{8}` and `{9}`: "you" as a subject, a direct object and an
    /// indirect object, which German says three ways.
    fn phrase(&self, phrase: Phrase, args: &[Piece]) -> Piece {
        self.filled(phrase.text(self.wording.lang), args)
    }

    /// [`Self::phrase`] for a counted phrase: `n` picks its form.
    fn counted(&self, phrase: Phrase, n: usize, args: &[Piece]) -> Piece {
        self.filled(&phrase.text_count(self.wording.lang, n), args)
    }

    fn filled(&self, text: &str, args: &[Piece]) -> Piece {
        let mut all: Vec<Piece> = args.iter().map(Piece::copy).collect();
        all.resize_with(YOU_SLOT, Piece::default);
        all.extend(
            [
                Phrase::LogYouSubject,
                Phrase::LogYouObject,
                Phrase::LogYouIndirect,
            ]
            .map(|you| self.you(you)),
        );
        Piece::fill(text, &all)
    }

    /// The reading seat as `you` says it, marked as that seat.
    fn you(&self, you: Phrase) -> Piece {
        let text = you.text(self.wording.lang);
        Piece {
            players: vec![PlayerSpan {
                range: 0..text.len(),
                player: self.wording.seat,
            }],
            lower: true,
            ..Piece::plain(text)
        }
    }

    fn count(n: &impl ToString) -> Piece {
        Piece::plain(n.to_string())
    }

    /// Another seat's name, or nothing for the reading seat, whose sentence
    /// says "you" and has no `{0}`.
    fn seat(&self, player: PlayerId) -> Piece {
        if player == self.wording.seat {
            Piece::default()
        } else {
            let name = seat_name(self.wording.lang, self.wording.statics, player);
            Piece {
                players: vec![PlayerSpan {
                    range: 0..name.len(),
                    player,
                }],
                ..Piece::plain(name)
            }
        }
    }

    /// A sentence about `player`: `you` for the reading seat, `other` with
    /// its name as `{0}` for any other. `rest` fills `{1}` on.
    fn about(&self, player: PlayerId, you: Phrase, other: Phrase, rest: Vec<Piece>) -> Piece {
        self.about_in(player, you, other, None, rest)
    }

    /// [`Self::about`] for a counted sentence: `n` picks its form.
    fn about_counted(
        &self,
        player: PlayerId,
        you: Phrase,
        other: Phrase,
        n: usize,
        rest: Vec<Piece>,
    ) -> Piece {
        self.about_in(player, you, other, Some(n), rest)
    }

    fn about_in(
        &self,
        player: PlayerId,
        you: Phrase,
        other: Phrase,
        n: Option<usize>,
        rest: Vec<Piece>,
    ) -> Piece {
        let phrase = if player == self.wording.seat {
            you
        } else {
            other
        };
        let mut args = vec![self.seat(player)];
        args.extend(rest);
        let said = match n {
            Some(n) => self.counted(phrase, n, &args),
            None => self.phrase(phrase, &args),
        };
        Piece {
            subject: Some(player),
            ..said
        }
    }

    /// A sentence with no player in it: `{0}` is empty.
    fn about_nobody(&self, phrase: Phrase, rest: Vec<Piece>) -> Piece {
        let mut args = vec![Piece::default()];
        args.extend(rest);
        self.phrase(phrase, &args)
    }

    fn neutral(&self, phrase: Phrase) -> Piece {
        Piece {
            lower: true,
            ..Piece::plain(phrase.text(self.wording.lang))
        }
    }

    /// An object the seat may know, by the name its card has in the reader's
    /// language ([`shown_name`]), else by the name the line carries.
    fn known(
        &self,
        id: ObjectId,
        card: Option<CardIdentity>,
        token: Option<u16>,
        name: &str,
    ) -> Piece {
        let text = card.and_then(|card| self.wording.texts.text(card.index, card.face));
        let shown = shown_name(name, text.as_ref());
        let shown = if shown.trim().is_empty() { name } else { shown };
        if shown.trim().is_empty() {
            return self.neutral(Phrase::LogACard);
        }
        Piece {
            text: shown.to_string(),
            names: vec![NameSpan {
                range: 0..shown.len(),
                id,
                card,
                token,
            }],
            players: Vec::new(),
            lower: false,
            subject: None,
        }
    }

    fn object(&self, object: &LogObject) -> Piece {
        match object {
            LogObject::Known {
                id,
                card,
                token,
                name,
            } => self.known(*id, *card, *token, name),
            LogObject::FaceDown { id } => {
                let mut piece = self.neutral(Phrase::LogAFaceDownCard);
                piece.names.push(NameSpan {
                    range: 0..piece.text.len(),
                    id: *id,
                    card: None,
                    token: None,
                });
                piece
            }
            LogObject::Hidden => self.neutral(Phrase::LogACard),
        }
    }

    /// Names joined as the language joins a list: "A, B and C".
    fn list(&self, items: Vec<Piece>) -> Piece {
        let mut items = items;
        let Some(last) = items.pop() else {
            return Piece::default();
        };
        if items.is_empty() {
            return last;
        }
        let mut head = Piece::default();
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                head.text.push_str(", ");
            }
            head.push(item);
        }
        self.phrase(Phrase::LogAnd, &[head, last])
    }

    fn objects(&self, objects: &[LogObject]) -> Piece {
        self.list(objects.iter().map(|o| self.object(o)).collect())
    }

    fn zone(&self, owner: PlayerId, zone: LogZone, into: bool) -> Piece {
        let yours = owner == self.wording.seat;
        let phrase = match (zone, into) {
            (LogZone::Library, false) if yours => Phrase::LogFromLibraryYou,
            (LogZone::Library, false) => Phrase::LogFromLibrary,
            (LogZone::Hand, false) if yours => Phrase::LogFromHandYou,
            (LogZone::Hand, false) => Phrase::LogFromHand,
            (LogZone::Graveyard, false) if yours => Phrase::LogFromGraveyardYou,
            (LogZone::Graveyard, false) => Phrase::LogFromGraveyard,
            (LogZone::Battlefield, false) => Phrase::LogFromBattlefield,
            (LogZone::Exile, false) => Phrase::LogFromExile,
            (LogZone::Command, false) => Phrase::LogFromCommand,
            (LogZone::Stack, false) => Phrase::LogFromStack,
            (LogZone::Library, true) if yours => Phrase::LogIntoLibraryYou,
            (LogZone::Library, true) => Phrase::LogIntoLibrary,
            (LogZone::Hand, true) if yours => Phrase::LogIntoHandYou,
            (LogZone::Hand, true) => Phrase::LogIntoHand,
            (LogZone::Graveyard, true) if yours => Phrase::LogIntoGraveyardYou,
            (LogZone::Graveyard, true) => Phrase::LogIntoGraveyard,
            (LogZone::Battlefield, true) => Phrase::LogIntoBattlefield,
            (LogZone::Exile, true) => Phrase::LogIntoExile,
            (LogZone::Command, true) => Phrase::LogIntoCommand,
            (LogZone::Stack, true) => Phrase::LogIntoStack,
        };
        self.phrase(phrase, &[self.seat(owner)])
    }

    /// Where in `owner`'s library a card went, or which library when the
    /// line cannot say where in it.
    fn library_place(&self, owner: PlayerId, place: Option<LogPlace>) -> Piece {
        let yours = owner == self.wording.seat;
        let (phrase, at) = match place {
            Some(LogPlace::Top) if yours => (Phrase::LogOntoLibraryTopYou, None),
            Some(LogPlace::Top) => (Phrase::LogOntoLibraryTop, None),
            Some(LogPlace::Bottom) if yours => (Phrase::LogOntoLibraryBottomYou, None),
            Some(LogPlace::Bottom) => (Phrase::LogOntoLibraryBottom, None),
            Some(LogPlace::FromTop(n)) if yours => (Phrase::LogIntoLibraryAtYou, Some(n)),
            Some(LogPlace::FromTop(n)) => (Phrase::LogIntoLibraryAt, Some(n)),
            Some(LogPlace::Shuffled) | None => return self.zone(owner, LogZone::Library, true),
        };
        let mut args = vec![self.seat(owner)];
        args.extend(at.map(|n| Piece::plain(ordinal(self.wording.lang, n))));
        self.phrase(phrase, &args)
    }

    /// Where `player` played or cast something from, with the space before
    /// it, or nothing when the line does not say.
    fn played_from(&self, player: PlayerId, from: Option<LogFrom>) -> Piece {
        let Some(LogFrom { zone, owner }) = from else {
            return Piece::default();
        };
        let own = match zone {
            LogZone::Hand if owner == player && owner != self.wording.seat => {
                Some(Phrase::LogFromOwnHand)
            }
            LogZone::Library if owner == player && owner != self.wording.seat => {
                Some(Phrase::LogFromOwnLibrary)
            }
            LogZone::Graveyard if owner == player && owner != self.wording.seat => {
                Some(Phrase::LogFromOwnGraveyard)
            }
            _ => None,
        };
        let mut out = Piece::plain(" ");
        out.push(&own.map_or_else(
            || self.zone(owner, zone, false),
            |own| self.phrase(own, &[]),
        ));
        out
    }

    fn counters(&self, kind: CounterKind, n: u16) -> Piece {
        let n = usize::from(n);
        let (phrase, sizes) = match kind {
            CounterKind::Plus { power, toughness } => {
                (Phrase::LogCountersPlus, Some((power, toughness)))
            }
            CounterKind::Minus { power, toughness } => {
                (Phrase::LogCountersMinus, Some((power, toughness)))
            }
            CounterKind::Loyalty => (Phrase::LogCountersLoyalty, None),
            CounterKind::Lore => (Phrase::LogCountersLore, None),
            CounterKind::Bore => (Phrase::LogCountersBore, None),
            CounterKind::Shield => (Phrase::LogCountersShield, None),
            CounterKind::Defense => (Phrase::LogCountersDefense, None),
            CounterKind::Time => (Phrase::LogCountersTime, None),
            CounterKind::Charge => (Phrase::LogCountersCharge, None),
            CounterKind::Poison => (Phrase::LogCountersPoison, None),
            CounterKind::Energy => (Phrase::LogCountersEnergy, None),
            CounterKind::Rad => (Phrase::LogCountersRad, None),
            CounterKind::Lifelink => (Phrase::LogCountersLifelink, None),
            CounterKind::Level => (Phrase::LogCountersLevel, None),
            CounterKind::Hone => (Phrase::LogCountersHone, None),
            CounterKind::Stun => (Phrase::LogCountersStun, None),
            CounterKind::Finality => (Phrase::LogCountersFinality, None),
            CounterKind::Flying => (Phrase::LogCountersFlying, None),
            CounterKind::Shadow => (Phrase::LogCountersShadow, None),
            CounterKind::Indestructible => (Phrase::LogCountersIndestructible, None),
            CounterKind::Custom(10) => (Phrase::LogCountersAge, None),
            CounterKind::Custom(_) => (Phrase::LogCountersOther, None),
        };
        let sizes = sizes.map_or_else(Vec::new, |(power, toughness)| {
            vec![Self::count(&power), Self::count(&toughness)]
        });
        self.counted(phrase, n, &sizes)
    }

    fn defender(&self, defending: Defender) -> (bool, Piece) {
        match defending {
            Defender::Player(player) => (player == self.wording.seat, self.seat(player)),
            Defender::Planeswalker(id) => (
                false,
                self.defenders.get(&id).map_or_else(
                    || self.neutral(Phrase::LogAPlaneswalker),
                    |named| self.known(id, named.card, None, &named.name),
                ),
            ),
            Defender::Battle(id) => (
                false,
                self.defenders.get(&id).map_or_else(
                    || self.neutral(Phrase::LogABattle),
                    |named| self.known(id, named.card, None, &named.name),
                ),
            ),
        }
    }

    #[allow(clippy::too_many_lines)] // one arm per event, and no `_`
    fn sentence(&self, entry: &LogEntry) -> Piece {
        match &entry.event {
            LogEvent::TurnStarted { active } => self.about(
                *active,
                Phrase::LogTurnYou,
                Phrase::LogTurn,
                vec![Self::count(&entry.turn)],
            ),
            LogEvent::Mulliganed { player } => self.about(
                *player,
                Phrase::LogMulliganYou,
                Phrase::LogMulligan,
                Vec::new(),
            ),
            LogEvent::Kept { player, cards } => {
                let n = usize::from(*cards);
                self.about_counted(
                    *player,
                    Phrase::LogKeptCardsYou,
                    Phrase::LogKeptCards,
                    n,
                    Vec::new(),
                )
            }
            LogEvent::TimedOut { player, answer } => {
                let (you, other) = match answer {
                    ClockAnswer::Passed => (Phrase::LogTimedPassedYou, Phrase::LogTimedPassed),
                    ClockAnswer::Kept => (Phrase::LogTimedKeptYou, Phrase::LogTimedKept),
                    ClockAnswer::NoAttackers => {
                        (Phrase::LogTimedNoAttackersYou, Phrase::LogTimedNoAttackers)
                    }
                    ClockAnswer::NoBlockers => {
                        (Phrase::LogTimedNoBlockersYou, Phrase::LogTimedNoBlockers)
                    }
                    ClockAnswer::Declined => {
                        (Phrase::LogTimedDeclinedYou, Phrase::LogTimedDeclined)
                    }
                    ClockAnswer::ChosenForThem => {
                        (Phrase::LogTimedChosenYou, Phrase::LogTimedChosen)
                    }
                };
                self.about(*player, you, other, Vec::new())
            }
            LogEvent::StandIn { player } => self.about(
                *player,
                Phrase::LogStandInYou,
                Phrase::LogStandIn,
                Vec::new(),
            ),
            LogEvent::ConnectionLost { player, wait_secs } => match wait_secs {
                Some(secs) => self.about(
                    *player,
                    Phrase::LogConnectionLostYou,
                    Phrase::LogConnectionLost,
                    vec![Piece::plain(format!("{}:{:02}", secs / 60, secs % 60))],
                ),
                None => self.about(
                    *player,
                    Phrase::LogConnectionLostPausedYou,
                    Phrase::LogConnectionLostPaused,
                    Vec::new(),
                ),
            },
            LogEvent::Returned { player } => self.about(
                *player,
                Phrase::LogReturnedYou,
                Phrase::LogReturned,
                Vec::new(),
            ),
            LogEvent::LandPlayed { player, land, from } => self.about(
                *player,
                Phrase::LogLandPlayedYou,
                Phrase::LogLandPlayed,
                vec![self.object(land), self.played_from(*player, *from)],
            ),
            LogEvent::Cast {
                player,
                spell,
                from,
            } => self.about(
                *player,
                Phrase::LogCastYou,
                Phrase::LogCast,
                vec![self.object(spell), self.played_from(*player, *from)],
            ),
            LogEvent::Ability {
                controller,
                source,
                activated,
                ..
            } => {
                let (you, other) = match activated {
                    Some(true) => (Phrase::LogActivatedYou, Phrase::LogActivated),
                    Some(false) => (Phrase::LogTriggeredYou, Phrase::LogTriggered),
                    None => (Phrase::LogAbilityYou, Phrase::LogAbility),
                };
                self.about(*controller, you, other, vec![self.object(source)])
            }
            LogEvent::Countered { spell } => {
                self.about_nobody(Phrase::LogCountered, vec![self.object(spell)])
            }
            LogEvent::DidNotResolve { object } => {
                self.about_nobody(Phrase::LogDidNotResolve, vec![self.object(object)])
            }
            LogEvent::Drew { player, cards } => {
                if cards.iter().all(|card| *card == LogObject::Hidden) {
                    let n = cards.len();
                    self.about_counted(
                        *player,
                        Phrase::LogDrewCardsYou,
                        Phrase::LogDrewCards,
                        n,
                        Vec::new(),
                    )
                } else {
                    self.about(
                        *player,
                        Phrase::LogDrewYou,
                        Phrase::LogDrew,
                        vec![self.objects(cards)],
                    )
                }
            }
            LogEvent::Discarded { player, card } => self.about(
                *player,
                Phrase::LogDiscardedYou,
                Phrase::LogDiscarded,
                vec![self.object(card)],
            ),
            LogEvent::Moved {
                object,
                owner,
                from,
                to,
                place,
            } => {
                let (phrase, into) = match (to, place) {
                    (LogZone::Library, Some(LogPlace::Shuffled)) => {
                        (Phrase::LogMovedShuffled, self.library_place(*owner, *place))
                    }
                    (LogZone::Library, _) => (Phrase::LogMoved, self.library_place(*owner, *place)),
                    _ => (Phrase::LogMoved, self.zone(*owner, *to, true)),
                };
                Piece {
                    subject: Some(*owner),
                    ..self.about_nobody(
                        phrase,
                        vec![self.object(object), self.zone(*owner, *from, false), into],
                    )
                }
            }
            LogEvent::Created { object, controller } => self.about(
                *controller,
                Phrase::LogCreatedYou,
                Phrase::LogCreated,
                vec![self.object(object)],
            ),
            LogEvent::Damage {
                source,
                target,
                amount,
                combat,
            } => {
                let (you, victim, subject) = match target {
                    LogTarget::Player(player) => (
                        *player == self.wording.seat,
                        self.seat(*player),
                        Some(*player),
                    ),
                    LogTarget::Object(object) => (false, self.object(object), None),
                };
                let (phrase, rest) = match (source, you, combat) {
                    (Some(source), true, true) => {
                        (Phrase::LogCombatDamageYou, Some(self.object(source)))
                    }
                    (Some(source), true, false) => {
                        (Phrase::LogDamageYou, Some(self.object(source)))
                    }
                    (Some(source), false, true) => {
                        (Phrase::LogCombatDamage, Some(self.object(source)))
                    }
                    (Some(source), false, false) => (Phrase::LogDamage, Some(self.object(source))),
                    (None, true, _) => (Phrase::LogDamageUnsourcedYou, None),
                    (None, false, _) => (Phrase::LogDamageUnsourced, None),
                };
                let mut args = vec![victim, Self::count(amount)];
                args.extend(rest);
                Piece {
                    subject,
                    ..self.phrase(phrase, &args)
                }
            }
            LogEvent::Life { player, old, new } => {
                let n = usize::try_from(*new).unwrap_or(0);
                self.about_counted(
                    *player,
                    Phrase::LogLifePointsYou,
                    Phrase::LogLifePoints,
                    n,
                    vec![Self::count(old), Self::count(new)],
                )
            }
            LogEvent::PlayerCounters {
                player,
                kind,
                old,
                new,
            } => self.about(
                *player,
                Phrase::LogPlayerCountersYou,
                Phrase::LogPlayerCounters,
                vec![
                    Self::count(new),
                    self.counters(*kind, *new),
                    Self::count(old),
                ],
            ),
            LogEvent::Counters {
                object,
                kind,
                old,
                new,
            } => self.about_nobody(
                // A permanent that had none, such as a planeswalker entering
                // with its loyalty, got them: "went to 4 (from 0)" says the
                // same thing as a sum.
                if *old == 0 {
                    Phrase::LogCountersGot
                } else {
                    Phrase::LogCounters
                },
                vec![
                    self.object(object),
                    Self::count(new),
                    self.counters(*kind, *new),
                    Self::count(old),
                ],
            ),
            LogEvent::Attacked {
                attacker,
                defending,
            } => {
                let (you, defender) = self.defender(*defending);
                let phrase = if you {
                    Phrase::LogAttackedYou
                } else {
                    Phrase::LogAttacked
                };
                Piece {
                    subject: match defending {
                        Defender::Player(player) => Some(*player),
                        Defender::Planeswalker(_) | Defender::Battle(_) => None,
                    },
                    ..self.phrase(phrase, &[defender, self.object(attacker)])
                }
            }
            LogEvent::Blocked { blocker, attacker } => self.about_nobody(
                Phrase::LogBlocked,
                vec![self.object(blocker), self.object(attacker)],
            ),
            LogEvent::Banded { attacker, with } => self.about_nobody(
                Phrase::LogBanded,
                vec![self.object(attacker), self.object(with)],
            ),
            LogEvent::ControlChanged { object, new, .. } => self.about(
                *new,
                Phrase::LogControlYou,
                Phrase::LogControl,
                vec![self.object(object)],
            ),
            LogEvent::TurnedFaceUp { object } => {
                self.about_nobody(Phrase::LogTurnedFaceUp, vec![self.object(object)])
            }
            LogEvent::Transformed { object } => {
                self.about_nobody(Phrase::LogTransformed, vec![self.object(object)])
            }
            LogEvent::CardsKept { player, cards } => self.about(
                *player,
                Phrase::LogCardsKeptYou,
                Phrase::LogCardsKept,
                vec![self.objects(cards)],
            ),
            LogEvent::Revealed { player, cards } => self.about(
                *player,
                Phrase::LogRevealedYou,
                Phrase::LogRevealed,
                vec![self.objects(cards)],
            ),
            LogEvent::Shuffled { player } => self.about(
                *player,
                Phrase::LogShuffledYou,
                Phrase::LogShuffled,
                Vec::new(),
            ),
            LogEvent::DiceRolled {
                player,
                sides,
                result,
            } => self.about(
                *player,
                Phrase::LogRolledYou,
                Phrase::LogRolled,
                vec![Self::count(sides), Self::count(result)],
            ),
            LogEvent::CoinFlipped { player, won } => self.about(
                *player,
                if *won {
                    Phrase::LogFlipWonYou
                } else {
                    Phrase::LogFlipLostYou
                },
                if *won {
                    Phrase::LogFlipWon
                } else {
                    Phrase::LogFlipLost
                },
                Vec::new(),
            ),
            LogEvent::Lost { player, cause } => {
                let (you, other) = loss_phrases(*cause);
                self.about(*player, log_loss(*cause).unwrap_or(you), other, Vec::new())
            }
            LogEvent::GameOver { winners } => {
                let seat = self.wording.seat;
                let mut names: Vec<Piece> = winners
                    .iter()
                    .filter(|player| *player != seat)
                    .map(|player| self.seat(player))
                    .collect();
                if winners.contains(seat) {
                    if names.is_empty() {
                        return self.phrase(Phrase::LogWonYou, &[]);
                    }
                    names.push(self.you(Phrase::LogYouInList));
                }
                match names.len() {
                    0 => self.neutral(Phrase::LogDrawn),
                    1 => self.phrase(Phrase::LogWonOne, &names),
                    _ => self.phrase(Phrase::LogWonMany, &[self.list(names)]),
                }
            }
            LogEvent::LoopDetected { broken } => self.neutral(if *broken {
                Phrase::LogLoopBroken
            } else {
                Phrase::LogLoop
            }),
            LogEvent::DayNight { now } => self.neutral(match now {
                DayNight::Day => Phrase::LogDay,
                DayNight::Night => Phrase::LogNight,
            }),
            LogEvent::BecameMonarch { player } => self.about(
                *player,
                Phrase::LogMonarchYou,
                Phrase::LogMonarch,
                Vec::new(),
            ),
            LogEvent::Aimed {
                controller,
                source,
                ability,
                ..
            } => {
                let phrase = if *ability {
                    Phrase::LogAimedAbility
                } else {
                    Phrase::LogAimedSpell
                };
                Piece {
                    subject: Some(*controller),
                    ..self.phrase(
                        phrase,
                        &[Piece::default(), self.object(source), self.aim_parts(entry)],
                    )
                }
            }
        }
    }
}

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
