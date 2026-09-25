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
    LogFrom, LogObject, LogPlace, LogTail, LogTarget, LogZone, LossCause, PlayerView,
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
                defending: Defender::Planeswalker(id),
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
        Some(
            Writer {
                wording,
                defenders: &self.defenders,
            }
            .line(index, entry),
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
            .map(|(index, entry)| writer.line(index, entry))
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
    fn line(&self, index: usize, entry: &LogEntry) -> LogLine {
        let mut piece = self.sentence(entry);
        if piece.lower {
            // ASCII only, so no span moves.
            if let Some(first) = piece.text.get_mut(..1) {
                first.make_ascii_uppercase();
            }
        }
        let times = match entry.event {
            LogEvent::Life { .. } | LogEvent::Counters { .. } => 1,
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
        }
    }

    /// `phrase` filled with `args` from `{0}` on, and with the reading seat
    /// as `{7}`, `{8}` and `{9}`: "you" as a subject, a direct object and an
    /// indirect object, which German says three ways.
    fn phrase(&self, phrase: Phrase, args: &[Piece]) -> Piece {
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
        Piece::fill(phrase.text(self.wording.lang), &all)
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
        let phrase = if player == self.wording.seat {
            you
        } else {
            other
        };
        let mut args = vec![self.seat(player)];
        args.extend(rest);
        Piece {
            subject: Some(player),
            ..self.phrase(phrase, &args)
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
            (LogZone::Library, true) if yours => Phrase::LogIntoLibraryYou,
            (LogZone::Library, true) => Phrase::LogIntoLibrary,
            (LogZone::Hand, true) if yours => Phrase::LogIntoHandYou,
            (LogZone::Hand, true) => Phrase::LogIntoHand,
            (LogZone::Graveyard, true) if yours => Phrase::LogIntoGraveyardYou,
            (LogZone::Graveyard, true) => Phrase::LogIntoGraveyard,
            (LogZone::Battlefield, true) => Phrase::LogIntoBattlefield,
            (LogZone::Exile, true) => Phrase::LogIntoExile,
            (LogZone::Command, true) => Phrase::LogIntoCommand,
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
        let (one, many, sizes) = match kind {
            CounterKind::Plus { power, toughness } => (
                Phrase::LogCounterPlus,
                Phrase::LogCountersPlus,
                Some((power, toughness)),
            ),
            CounterKind::Minus { power, toughness } => (
                Phrase::LogCounterMinus,
                Phrase::LogCountersMinus,
                Some((power, toughness)),
            ),
            CounterKind::Loyalty => (Phrase::LogCounterLoyalty, Phrase::LogCountersLoyalty, None),
            CounterKind::Lore => (Phrase::LogCounterLore, Phrase::LogCountersLore, None),
            CounterKind::Time => (Phrase::LogCounterTime, Phrase::LogCountersTime, None),
            CounterKind::Charge => (Phrase::LogCounterCharge, Phrase::LogCountersCharge, None),
            CounterKind::Poison => (Phrase::LogCounterPoison, Phrase::LogCountersPoison, None),
            CounterKind::Energy => (Phrase::LogCounterEnergy, Phrase::LogCountersEnergy, None),
            CounterKind::Rad => (Phrase::LogCounterRad, Phrase::LogCountersRad, None),
            CounterKind::Lifelink => (
                Phrase::LogCounterLifelink,
                Phrase::LogCountersLifelink,
                None,
            ),
            CounterKind::Level => (Phrase::LogCounterLevel, Phrase::LogCountersLevel, None),
            CounterKind::Custom(_) => (Phrase::LogCounterOther, Phrase::LogCountersOther, None),
        };
        let sizes = sizes.map_or_else(Vec::new, |(power, toughness)| {
            vec![Self::count(&power), Self::count(&toughness)]
        });
        self.phrase(Phrase::counted(n, one, many), &sizes)
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
                self.about(
                    *player,
                    Phrase::counted(n, Phrase::LogKeptCardYou, Phrase::LogKeptCardsYou),
                    Phrase::counted(n, Phrase::LogKeptCard, Phrase::LogKeptCards),
                    vec![Self::count(&n)],
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
                controller, source, ..
            } => self.about(
                *controller,
                Phrase::LogAbilityYou,
                Phrase::LogAbility,
                vec![self.object(source)],
            ),
            LogEvent::Countered { spell } => {
                self.about_nobody(Phrase::LogCountered, vec![self.object(spell)])
            }
            LogEvent::DidNotResolve { object } => {
                self.about_nobody(Phrase::LogDidNotResolve, vec![self.object(object)])
            }
            LogEvent::Drew { player, cards } => {
                if cards.iter().all(|card| *card == LogObject::Hidden) {
                    let n = cards.len();
                    self.about(
                        *player,
                        Phrase::counted(n, Phrase::LogDrewCardYou, Phrase::LogDrewCardsYou),
                        Phrase::counted(n, Phrase::LogDrewCard, Phrase::LogDrewCards),
                        vec![Self::count(&n)],
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
                self.about(
                    *player,
                    Phrase::counted(n, Phrase::LogLifePointYou, Phrase::LogLifePointsYou),
                    Phrase::counted(n, Phrase::LogLifePoint, Phrase::LogLifePoints),
                    vec![Self::count(old), Self::count(new)],
                )
            }
            LogEvent::Counters {
                object,
                kind,
                old,
                new,
            } => self.about_nobody(
                Phrase::LogCounters,
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
                        Defender::Planeswalker(_) => None,
                    },
                    ..self.phrase(phrase, &[defender, self.object(attacker)])
                }
            }
            LogEvent::Blocked { blocker, attacker } => self.about_nobody(
                Phrase::LogBlocked,
                vec![self.object(blocker), self.object(attacker)],
            ),
            LogEvent::ControlChanged { object, new, .. } => self.about(
                *new,
                Phrase::LogControlYou,
                Phrase::LogControl,
                vec![self.object(object)],
            ),
            LogEvent::Transformed { object } => {
                self.about_nobody(Phrase::LogTransformed, vec![self.object(object)])
            }
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
        }
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
mod tests {
    use super::*;
    use crate::test_support::{ViewBuilder, printed, statics};
    use baylee_core::ids::{PrintRef, SeatSet};
    use baylee_view::{LossCause, SeatIdentity};

    const ME: PlayerId = PlayerId::new(0);
    const BO: PlayerId = PlayerId::new(1);
    const BOLT: CardIndex = CardIndex::new(7);

    fn entry(turn: u32, event: LogEvent) -> LogEntry {
        LogEntry {
            at: 0,
            turn,
            repeat: 1,
            event,
        }
    }

    /// `n` entries that all differ, numbered from `first`.
    fn entries(first: u32, n: u32) -> Vec<LogEntry> {
        (first..first + n)
            .map(|turn| entry(turn, LogEvent::TurnStarted { active: ME }))
            .collect()
    }

    fn tail(from: u32, entries: &[LogEntry]) -> LogTail {
        LogTail {
            from,
            entries: entries.to_vec(),
        }
    }

    fn view() -> PlayerView {
        ViewBuilder::new(2).build()
    }

    fn book(entries: &[LogEntry]) -> LogBook {
        let mut book = LogBook::new();
        assert_eq!(book.append(&tail(0, entries), &view()), entries.len());
        book
    }

    /// Seat 0 is Ada, who reads; seat 1 is Bo; seat 2 is Cy.
    fn roster() -> GameStatic {
        let mut roster = statics(0);
        roster.seats = ["Ada", "Bo", "Cy"]
            .iter()
            .zip(0..)
            .map(|(name, seat)| SeatIdentity {
                player: PlayerId::new(seat),
                display_name: (*name).to_string(),
                is_ai: false,
                away: false,
                team: None,
            })
            .collect();
        roster
    }

    fn card_text(name: &str, english: &str) -> CardText {
        CardText {
            lang: String::new(),
            name: name.to_string(),
            type_line: String::new(),
            oracle_text: String::new(),
            mana_cost: String::new(),
            english_name: english.to_string(),
        }
    }

    /// The catalog's text for Lightning Bolt, in the language it serves.
    struct Catalog(Lang);

    impl CardTextLookup for Catalog {
        fn text(&self, card: CardIndex, face: u8) -> Option<CardText> {
            let name = match self.0 {
                Lang::En => "Lightning Bolt",
                Lang::De => "Blitzschlag",
            };
            (card == BOLT && face == 0).then(|| card_text(name, "Lightning Bolt"))
        }
    }

    fn no_text(_: CardIndex, _: u8) -> Option<CardText> {
        None
    }

    fn bolt(slot: u32) -> LogObject {
        LogObject::Known {
            token: None,
            id: ObjectId::new(slot, 0),
            card: Some(CardIdentity {
                index: BOLT,
                print: PrintRef::new(0),
                face: 0,
            }),
            name: "Lightning Bolt".to_string(),
        }
    }

    fn token(slot: u32, name: &str) -> LogObject {
        LogObject::Known {
            token: None,
            id: ObjectId::new(slot, 0),
            card: None,
            name: name.to_string(),
        }
    }

    /// Line 0 of `book`, as Ada reads it in `lang` with the catalog's text.
    fn read(book: &LogBook, lang: Lang) -> LogLine {
        let roster = roster();
        let catalog = Catalog(lang);
        let wording = Wording {
            lang,
            seat: ME,
            statics: Some(&roster),
            texts: &catalog,
        };
        book.line(0, &wording).expect("the book holds line 0")
    }

    /// `event` as Ada reads it in `lang`.
    fn say(lang: Lang, event: LogEvent) -> String {
        read(&book(&[entry(1, event)]), lang).text
    }

    // ---- the book ----------------------------------------------------------

    /// Tails that follow each other are appended in order, each once.
    #[test]
    fn tails_that_follow_each_other_are_appended_in_order() {
        let all = entries(1, 5);
        let mut book = LogBook::new();
        assert_eq!(book.append(&tail(0, &all[..3]), &view()), 3);
        assert_eq!(book.append(&tail(3, &all[3..]), &view()), 2);
        assert_eq!(book.entries(), &all[..]);
        assert_eq!((book.gaps(), book.rewrites()), (0, 0));
    }

    /// A chunk sent twice, as several frames with one `seq` may repeat it,
    /// adds nothing the second time.
    #[test]
    fn a_chunk_sent_twice_adds_nothing() {
        let all = entries(1, 5);
        let mut book = book(&all[..3]);
        assert_eq!(book.append(&tail(3, &all[3..]), &view()), 2);
        assert_eq!(book.append(&tail(3, &all[3..]), &view()), 0);
        assert_eq!(book.append(&tail(1, &all[1..4]), &view()), 0);
        assert_eq!(book.entries(), &all[..]);
    }

    /// A snapshot tells the log from its start: the book keeps what it holds
    /// and adds only the lines it was missing, each exactly once.
    #[test]
    fn a_snapshot_from_the_start_adds_only_what_was_missing() {
        let all = entries(1, 6);
        let mut book = book(&all[..2]);
        assert_eq!(book.append(&tail(0, &all), &view()), 4);
        assert_eq!(book.entries(), &all[..]);
        // And a reconnect tells it all again.
        assert_eq!(book.append(&tail(0, &all), &view()), 0);
        assert_eq!(book.entries(), &all[..]);
        assert_eq!(book.rewrites(), 0);
    }

    /// An empty tail changes nothing, although a question asked again
    /// carries one that says it starts at 0.
    #[test]
    fn an_empty_tail_from_the_start_empties_nothing() {
        let all = entries(1, 3);
        let mut book = book(&all);
        assert_eq!(book.append(&LogTail::default(), &view()), 0);
        assert_eq!(book.append(&tail(2, &[]), &view()), 0);
        assert_eq!(book.append(&tail(9, &[]), &view()), 0);
        assert_eq!(book.entries(), &all[..]);
        assert_eq!(book.gaps(), 0, "saying nothing is not a gap");
    }

    /// A tail that starts past the book's end is refused whole and counted,
    /// and the next tail from the start fills the gap.
    #[test]
    fn a_tail_past_the_end_is_refused_and_counted() {
        let all = entries(1, 6);
        let mut book = book(&all[..2]);
        assert_eq!(book.append(&tail(4, &all[4..]), &view()), 0);
        assert_eq!(book.entries(), &all[..2], "nothing from past the gap");
        assert_eq!(book.gaps(), 1);
        assert_eq!(book.append(&tail(0, &all), &view()), 4);
        assert_eq!(book.entries(), &all[..]);
    }

    /// A resend from the start that is shorter than the book, and agrees
    /// with it, takes nothing away.
    #[test]
    fn a_shorter_resend_that_agrees_takes_nothing_away() {
        let all = entries(1, 5);
        let mut book = book(&all);
        assert_eq!(book.append(&tail(0, &all[..2]), &view()), 0);
        assert_eq!(book.append(&tail(1, &all[1..3]), &view()), 0);
        assert_eq!(book.entries(), &all[..]);
        assert_eq!(book.rewrites(), 0);
    }

    /// A tail that disagrees with the book is another game's log: it replaces
    /// the book from where the two part, once.
    #[test]
    fn a_tail_that_disagrees_replaces_the_book_from_where_they_part() {
        let old = entries(1, 5);
        let mut new = old[..2].to_vec();
        new.extend(entries(20, 1));
        let mut book = book(&old);
        assert_eq!(book.append(&tail(0, &new), &view()), 1);
        assert_eq!(book.entries(), &new[..]);
        assert_eq!(book.rewrites(), 1);
    }

    /// The panel's cursor: new lines are unread until taken, and neither a
    /// repeated chunk nor a refused tail moves it.
    #[test]
    fn lines_are_unread_until_taken() {
        let all = entries(1, 6);
        let roster = roster();
        let wording = Wording {
            lang: Lang::En,
            seat: ME,
            statics: Some(&roster),
            texts: &no_text,
        };
        let mut book = book(&all[..3]);
        assert_eq!(book.unread(), 3);
        book.append(&tail(0, &all[..3]), &view());
        assert_eq!(book.unread(), 3, "a repeated chunk is not news");
        book.append(&tail(5, &all[5..]), &view());
        assert_eq!(book.unread(), 3, "a refused tail reads nothing");
        let taken = book.take_unread(&wording);
        assert_eq!(taken.iter().map(|l| l.index).collect::<Vec<_>>(), [0, 1, 2]);
        assert_eq!(book.unread(), 0);
        assert!(book.take_unread(&wording).is_empty());

        book.append(&tail(3, &all[3..]), &view());
        assert_eq!(book.unread(), 3);
        assert_eq!(book.take_unread(&wording)[0].index, 3);

        // The game-over screen reads the whole log whatever the panel took.
        assert_eq!(book.lines(&wording).len(), 6);
        assert_eq!(book.lines_since(4, &wording).len(), 2);
        book.append(&tail(6, &entries(7, 1)), &view());
        book.mark_read();
        assert_eq!(book.unread(), 0);
    }

    /// Lines replaced by another game's log are unread again.
    #[test]
    fn a_rewrite_below_the_cursor_makes_the_new_lines_unread() {
        let mut book = book(&entries(1, 4));
        book.mark_read();
        book.append(&tail(0, &entries(30, 2)), &view());
        assert_eq!(book.unread(), 2);
    }

    // ---- names -------------------------------------------------------------

    /// A card's name is its catalog name in the reader's language, from text
    /// that may arrive after the line did, and in whichever language the
    /// reader picks.
    #[test]
    fn a_card_is_named_from_card_text_that_arrives_later() {
        let book = book(&[entry(
            1,
            LogEvent::Cast {
                from: None,
                player: BO,
                spell: bolt(3),
            },
        )]);
        let roster = roster();
        let before = Wording {
            lang: Lang::De,
            seat: ME,
            statics: Some(&roster),
            texts: &no_text,
        };
        assert_eq!(book.lines(&before)[0].text, "Bo hat Lightning Bolt gewirkt");
        assert_eq!(read(&book, Lang::De).text, "Bo hat Blitzschlag gewirkt");
        assert_eq!(read(&book, Lang::En).text, "Bo cast Lightning Bolt");
    }

    /// A copy is named by the name it has, not by its card's: the catalog
    /// text of the card under it would name the wrong card.
    #[test]
    fn a_copy_keeps_the_name_it_shows() {
        let LogObject::Known { id, card, .. } = bolt(3) else {
            unreachable!()
        };
        let copy = LogObject::Known {
            token: None,
            id,
            card,
            name: "Shock".to_string(),
        };
        assert_eq!(
            say(
                Lang::De,
                LogEvent::Cast {
                    from: None,
                    player: BO,
                    spell: copy
                }
            ),
            "Bo hat Shock gewirkt"
        );
    }

    /// A card the seat may not see is "a card", a face-down one "a
    /// face-down card", and neither line says which object it is.
    #[test]
    fn a_hidden_or_face_down_card_is_named_neutrally() {
        let face_down = ObjectId::new(77, 3);
        let line = read(
            &book(&[entry(
                1,
                LogEvent::Blocked {
                    blocker: LogObject::FaceDown { id: face_down },
                    attacker: LogObject::Hidden,
                },
            )]),
            Lang::En,
        );
        assert_eq!(line.text, "A face-down card blocked a card");
        assert!(!line.text.contains("77"), "{}", line.text);
        assert_eq!(
            say(
                Lang::De,
                LogEvent::Discarded {
                    player: BO,
                    card: LogObject::Hidden
                }
            ),
            "Bo hat eine Karte abgeworfen"
        );
        // The face-down card is on the table for everyone to point at; the
        // hidden one has no handle to point with.
        assert_eq!(line.names.len(), 1);
        assert_eq!(line.names[0].id, face_down);
        assert_eq!(line.names[0].card, None);
    }

    /// No name is ever empty: an empty catalog name falls back to the name
    /// the line carries, and an object with no name at all is "a card".
    #[test]
    fn a_name_is_never_empty() {
        let blank = |_: CardIndex, _: u8| Some(card_text("", "Lightning Bolt"));
        let roster = roster();
        let wording = Wording {
            lang: Lang::En,
            seat: ME,
            statics: Some(&roster),
            texts: &blank,
        };
        let book = book(&[
            entry(
                1,
                LogEvent::Cast {
                    from: None,
                    player: BO,
                    spell: bolt(3),
                },
            ),
            entry(
                1,
                LogEvent::Created {
                    object: token(4, " "),
                    controller: BO,
                },
            ),
        ]);
        let lines = book.lines(&wording);
        assert_eq!(lines[0].text, "Bo cast Lightning Bolt");
        assert_eq!(lines[1].text, "Bo created a card");
    }

    /// Every name span covers exactly the name, wherever the language puts it.
    #[test]
    fn a_name_span_covers_its_name() {
        let line = read(
            &book(&[entry(
                1,
                LogEvent::Damage {
                    source: Some(bolt(3)),
                    target: LogTarget::Object(token(4, "Soldier")),
                    amount: 3,
                    combat: false,
                },
            )]),
            Lang::De,
        );
        assert_eq!(
            line.text,
            "Soldier hat 3 Schaden durch Blitzschlag erlitten"
        );
        let named: Vec<(&str, ObjectId)> = line
            .names
            .iter()
            .map(|span| (&line.text[span.range.clone()], span.id))
            .collect();
        assert_eq!(
            named,
            [
                ("Soldier", ObjectId::new(4, 0)),
                ("Blitzschlag", ObjectId::new(3, 0))
            ]
        );
        assert_eq!(line.names[1].card.map(|c| c.index), Some(BOLT));

        let list = read(
            &book(&[entry(
                1,
                LogEvent::Revealed {
                    player: BO,
                    cards: vec![token(5, "Clue"), bolt(6), token(7, "Food")],
                },
            )]),
            Lang::En,
        );
        assert_eq!(list.text, "Bo revealed Clue, Lightning Bolt and Food");
        let spans: Vec<&str> = list
            .names
            .iter()
            .map(|span| &list.text[span.range.clone()])
            .collect();
        assert_eq!(spans, ["Clue", "Lightning Bolt", "Food"]);
    }

    /// A name that reads like a placeholder is written as it is, in the line
    /// and in its plain form.
    #[test]
    fn a_name_that_looks_like_a_placeholder_stays_a_name() {
        let mut roster = roster();
        roster.seats[1].display_name = "{1}".to_string();
        let catalog = Catalog(Lang::En);
        let wording = Wording {
            lang: Lang::En,
            seat: ME,
            statics: Some(&roster),
            texts: &catalog,
        };
        let book = book(&[
            LogEntry {
                at: 0,
                turn: 1,
                repeat: 3,
                event: LogEvent::Cast {
                    from: None,
                    player: BO,
                    spell: bolt(3),
                },
            },
            entry(
                1,
                LogEvent::Blocked {
                    blocker: token(4, "{2}"),
                    attacker: bolt(3),
                },
            ),
        ]);
        let lines = book.lines(&wording);
        assert_eq!(lines[0].plain(Lang::En), "{1} cast Lightning Bolt (×3)");
        assert_eq!(lines[1].text, "{2} blocked Lightning Bolt");
    }

    /// A planeswalker an attack names is called what the view showed when
    /// the attack arrived, also after it has left the battlefield, and "a
    /// planeswalker" when the view never showed it.
    #[test]
    fn an_attacked_planeswalker_is_named_as_the_view_showed_it() {
        let walker = printed(40, 1, "Jace Beleren", 3);
        let attack = entry(
            2,
            LogEvent::Attacked {
                attacker: bolt(3),
                defending: Defender::Planeswalker(walker.id),
            },
        );
        let shown = ViewBuilder::new(2)
            .with_battlefield(1, [walker.clone()])
            .build();
        let mut book = LogBook::new();
        book.append(&tail(0, std::slice::from_ref(&attack)), &shown);
        book.append(&tail(1, &entries(3, 1)), &view());
        let line = read(&book, Lang::En);
        assert_eq!(line.text, "Lightning Bolt attacked Jace Beleren");
        assert_eq!(line.names[1].id, walker.id);
        assert_eq!(line.names[1].card, walker.card);

        let fresh = book_with_view(&[attack], &view());
        assert_eq!(
            read(&fresh, Lang::De).text,
            "Blitzschlag hat einen Planeswalker angegriffen"
        );
    }

    fn book_with_view(entries: &[LogEntry], view: &PlayerView) -> LogBook {
        let mut book = LogBook::new();
        book.append(&tail(0, entries), view);
        book
    }

    // ---- sentences ---------------------------------------------------------

    /// The reading seat is "you", in the verb's form for it; any other seat
    /// is named.
    #[test]
    fn the_reading_seat_is_you_and_the_others_are_named() {
        let cast = |player| LogEvent::Cast {
            from: None,
            player,
            spell: bolt(3),
        };
        assert_eq!(say(Lang::En, cast(ME)), "You cast Lightning Bolt");
        assert_eq!(say(Lang::De, cast(ME)), "Du hast Blitzschlag gewirkt");
        assert_eq!(say(Lang::En, cast(BO)), "Bo cast Lightning Bolt");
        let moved = |owner| LogEvent::Moved {
            place: None,
            object: bolt(3),
            owner,
            from: LogZone::Hand,
            to: LogZone::Graveyard,
        };
        assert_eq!(
            say(Lang::En, moved(ME)),
            "Lightning Bolt moved from your hand into your graveyard"
        );
        assert_eq!(
            say(Lang::De, moved(BO)),
            "Blitzschlag ist aus der Hand von Bo in den Friedhof von Bo gelangt"
        );
        let hit = |player, combat| LogEvent::Damage {
            source: Some(bolt(3)),
            target: LogTarget::Player(player),
            amount: 3,
            combat,
        };
        assert_eq!(
            say(Lang::En, hit(ME, false)),
            "Lightning Bolt dealt 3 damage to you"
        );
        assert_eq!(
            say(Lang::De, hit(ME, true)),
            "Blitzschlag hat dir 3 Kampfschaden zugefügt"
        );
        assert_eq!(
            say(Lang::En, hit(BO, true)),
            "Lightning Bolt dealt 3 combat damage to Bo"
        );
        let attack = |player| LogEvent::Attacked {
            attacker: token(4, "Soldier"),
            defending: Defender::Player(player),
        };
        assert_eq!(say(Lang::De, attack(ME)), "Soldier hat dich angegriffen");
        assert_eq!(say(Lang::En, attack(BO)), "Soldier attacked Bo");
        assert_eq!(
            say(
                Lang::En,
                LogEvent::ControlChanged {
                    object: token(4, "Soldier"),
                    old: BO,
                    new: ME
                }
            ),
            "You gained control of Soldier"
        );
    }

    /// A count picks the singular or the plural sentence.
    #[test]
    fn a_count_picks_its_sentence() {
        let drew = |player, n| LogEvent::Drew {
            player,
            cards: vec![LogObject::Hidden; n],
        };
        assert_eq!(say(Lang::En, drew(ME, 2)), "You drew 2 cards");
        assert_eq!(say(Lang::De, drew(BO, 1)), "Bo hat 1 Karte gezogen");
        let kept = |player, cards| LogEvent::Kept { player, cards };
        assert_eq!(say(Lang::En, kept(ME, 1)), "You kept 1 card");
        assert_eq!(say(Lang::De, kept(BO, 7)), "Bo hat 7 Karten behalten");
        let life = |player, old, new| LogEvent::Life { player, old, new };
        assert_eq!(
            say(Lang::En, life(BO, 20, 17)),
            "Bo went to 17 life (from 20)"
        );
        assert_eq!(
            say(Lang::De, life(ME, 2, 1)),
            "Du hattest danach 1 Lebenspunkt (vorher 2)"
        );
        assert_eq!(
            say(Lang::De, life(ME, 1, -2)),
            "Du hattest danach -2 Lebenspunkte (vorher 1)"
        );
        let counters = |kind, old, new| LogEvent::Counters {
            object: token(4, "Soldier"),
            kind,
            old,
            new,
        };
        assert_eq!(
            say(Lang::En, counters(CounterKind::PLUS_ONE, 1, 2)),
            "Soldier went to 2 +1/+1 counters (from 1)"
        );
        assert_eq!(
            say(Lang::De, counters(CounterKind::Loyalty, 3, 1)),
            "Soldier hatte danach 1 Loyalitätsmarke (vorher 3)"
        );
    }

    /// A draw the seat may partly see names what it may and counts nothing.
    #[test]
    fn a_draw_names_what_the_seat_may_see() {
        assert_eq!(
            say(
                Lang::En,
                LogEvent::Drew {
                    player: ME,
                    cards: vec![bolt(3), LogObject::Hidden]
                }
            ),
            "You drew Lightning Bolt and a card"
        );
    }

    /// Who won, with the reading seat last among a team, and a draw.
    #[test]
    fn the_end_of_the_game_names_its_winners() {
        let over = |seats: &[PlayerId]| {
            let mut winners = SeatSet::new();
            for seat in seats {
                winners.insert(*seat);
            }
            LogEvent::GameOver { winners }
        };
        assert_eq!(say(Lang::En, over(&[ME])), "You won the game");
        assert_eq!(say(Lang::De, over(&[BO])), "Bo hat das Spiel gewonnen");
        assert_eq!(say(Lang::En, over(&[ME, BO])), "Bo and you won the game");
        assert_eq!(
            say(Lang::De, over(&[ME, BO])),
            "Bo und du haben das Spiel gewonnen"
        );
        assert_eq!(
            say(Lang::En, over(&[BO, PlayerId::new(2)])),
            "Bo and Cy won the game"
        );
        assert_eq!(say(Lang::De, over(&[])), "Das Spiel endete unentschieden");
    }

    /// A loss is told in the end screen's words.
    #[test]
    fn a_loss_is_told_in_the_end_screens_words() {
        let lost = |player| LogEvent::Lost {
            player,
            cause: LossCause::Conceded,
        };
        assert_eq!(
            say(Lang::En, lost(BO)),
            Phrase::LostConcededOther.fill(Lang::En, &["Bo"])
        );
        assert_eq!(
            say(Lang::De, lost(ME)),
            Phrase::LostConcededYou.text(Lang::De)
        );
    }

    /// A turn opens with a heading, and only a turn does.
    #[test]
    fn a_turn_opens_with_a_heading() {
        let book = book(&[
            entry(3, LogEvent::TurnStarted { active: BO }),
            entry(3, LogEvent::Shuffled { player: BO }),
            entry(4, LogEvent::TurnStarted { active: ME }),
        ]);
        let roster = roster();
        let wording = Wording {
            lang: Lang::De,
            seat: ME,
            statics: Some(&roster),
            texts: &no_text,
        };
        let lines = book.lines(&wording);
        let seen: Vec<(bool, &str)> = lines.iter().map(|l| (l.header, l.text.as_str())).collect();
        assert_eq!(
            seen,
            [
                (true, "Zug 3 · Bo"),
                (false, "Bo hat die eigene Bibliothek gemischt"),
                (true, "Zug 4 · dein Zug"),
            ]
        );
    }

    /// A folded line says how often it happened, except a life total or
    /// counters, whose "was" already spans every change folded into it.
    #[test]
    fn a_folded_line_says_how_often_unless_it_spans() {
        let folded = |event| LogEntry {
            at: 0,
            turn: 1,
            repeat: 5,
            event,
        };
        let book = book(&[
            folded(LogEvent::Cast {
                from: None,
                player: BO,
                spell: bolt(3),
            }),
            folded(LogEvent::Life {
                player: BO,
                old: 20,
                new: 15,
            }),
            folded(LogEvent::Counters {
                object: token(4, "Soldier"),
                kind: CounterKind::PLUS_ONE,
                old: 0,
                new: 5,
            }),
        ]);
        let roster = roster();
        let wording = Wording {
            lang: Lang::En,
            seat: ME,
            statics: Some(&roster),
            texts: &no_text,
        };
        let lines = book.lines(&wording);
        assert_eq!(lines.iter().map(|l| l.times).collect::<Vec<_>>(), [5, 1, 1]);
        assert_eq!(lines[0].plain(Lang::En), "Bo cast Lightning Bolt (×5)");
        assert_eq!(lines[1].plain(Lang::En), "Bo went to 15 life (from 20)");
    }

    /// An ability line carries the ability, for a panel to show its text.
    #[test]
    fn an_ability_line_carries_its_ability() {
        let ability = LogAbility {
            ability: None,
            text: None,
            rules: None,
        };
        let line = read(
            &book(&[entry(
                1,
                LogEvent::Ability {
                    controller: BO,
                    source: token(4, "Soldier"),
                    ability: Some(ability),
                },
            )]),
            Lang::En,
        );
        assert_eq!(line.text, "Bo put an ability of Soldier on the stack");
        assert_eq!(line.ability, Some(ability));
    }

    /// A line that opens with "a card" opens with a capital, and a seat's
    /// name keeps the case its player gave it.
    #[test]
    fn a_line_capitalizes_our_words_and_never_a_name() {
        let unresolved = LogEvent::DidNotResolve {
            object: LogObject::Hidden,
        };
        assert_eq!(say(Lang::En, unresolved.clone()), "A card did not resolve");
        assert_eq!(
            say(Lang::De, unresolved),
            "Eine Karte wurde nicht verrechnet"
        );
        let mut roster = roster();
        roster.seats[1].display_name = "bo".to_string();
        let wording = Wording {
            lang: Lang::En,
            seat: ME,
            statics: Some(&roster),
            texts: &no_text,
        };
        let book = book(&[entry(1, LogEvent::Shuffled { player: BO })]);
        assert_eq!(book.lines(&wording)[0].text, "bo shuffled their library");
    }

    /// Which variant an event is, so the table below can show it has them
    /// all. No `_` arm: a new event is a compile error here.
    fn variant(event: &LogEvent) -> usize {
        match event {
            LogEvent::TurnStarted { .. } => 0,
            LogEvent::Mulliganed { .. } => 1,
            LogEvent::Kept { .. } => 2,
            LogEvent::TimedOut { .. } => 3,
            LogEvent::StandIn { .. } => 4,
            LogEvent::Returned { .. } => 5,
            LogEvent::LandPlayed { .. } => 6,
            LogEvent::Cast { .. } => 7,
            LogEvent::Ability { .. } => 8,
            LogEvent::Countered { .. } => 9,
            LogEvent::DidNotResolve { .. } => 10,
            LogEvent::Drew { .. } => 11,
            LogEvent::Discarded { .. } => 12,
            LogEvent::Moved { .. } => 13,
            LogEvent::Created { .. } => 14,
            LogEvent::Damage { .. } => 15,
            LogEvent::Life { .. } => 16,
            LogEvent::Counters { .. } => 17,
            LogEvent::Attacked { .. } => 18,
            LogEvent::Blocked { .. } => 19,
            LogEvent::ControlChanged { .. } => 20,
            LogEvent::Transformed { .. } => 21,
            LogEvent::Revealed { .. } => 22,
            LogEvent::Shuffled { .. } => 23,
            LogEvent::DiceRolled { .. } => 24,
            LogEvent::Lost { .. } => 25,
            LogEvent::GameOver { .. } => 26,
            LogEvent::LoopDetected { .. } => 27,
            LogEvent::DayNight { .. } => 28,
        }
    }

    const VARIANTS: usize = 29;

    /// Every kind of line, about `player`, with every answer, cause, zone and
    /// counter a line can carry.
    #[allow(clippy::too_many_lines)]
    fn every_line_about(player: PlayerId) -> Vec<LogEvent> {
        let card = bolt(3);
        let mut out = vec![
            LogEvent::TurnStarted { active: player },
            LogEvent::Mulliganed { player },
            LogEvent::Kept { player, cards: 1 },
            LogEvent::Kept { player, cards: 6 },
            LogEvent::StandIn { player },
            LogEvent::Returned { player },
            LogEvent::LandPlayed {
                from: None,
                player,
                land: token(5, "Island"),
            },
            LogEvent::Cast {
                from: None,
                player,
                spell: card.clone(),
            },
            LogEvent::Ability {
                controller: player,
                source: LogObject::FaceDown {
                    id: ObjectId::new(9, 1),
                },
                ability: None,
            },
            LogEvent::Countered {
                spell: card.clone(),
            },
            LogEvent::DidNotResolve {
                object: LogObject::Hidden,
            },
            LogEvent::Drew {
                player,
                cards: vec![LogObject::Hidden],
            },
            LogEvent::Drew {
                player,
                cards: vec![LogObject::Hidden; 3],
            },
            LogEvent::Drew {
                player,
                cards: vec![card.clone(), token(6, "Clue")],
            },
            LogEvent::Discarded {
                player,
                card: card.clone(),
            },
            LogEvent::Created {
                object: token(6, "Clue"),
                controller: player,
            },
            LogEvent::Life {
                player,
                old: 2,
                new: 1,
            },
            LogEvent::Life {
                player,
                old: 20,
                new: 17,
            },
            LogEvent::Attacked {
                attacker: token(4, "Soldier"),
                defending: Defender::Player(player),
            },
            LogEvent::Attacked {
                attacker: token(4, "Soldier"),
                defending: Defender::Planeswalker(ObjectId::new(40, 0)),
            },
            LogEvent::Blocked {
                blocker: token(4, "Soldier"),
                attacker: card.clone(),
            },
            LogEvent::ControlChanged {
                object: card.clone(),
                old: PlayerId::new(2),
                new: player,
            },
            LogEvent::Transformed {
                object: card.clone(),
            },
            LogEvent::Revealed {
                player,
                cards: vec![card.clone()],
            },
            LogEvent::Shuffled { player },
            LogEvent::DiceRolled {
                player,
                sides: 20,
                result: 17,
            },
            LogEvent::GameOver {
                winners: SeatSet::new(),
            },
            LogEvent::LoopDetected { broken: true },
            LogEvent::LoopDetected { broken: false },
            LogEvent::DayNight { now: DayNight::Day },
            LogEvent::DayNight {
                now: DayNight::Night,
            },
        ];
        for answer in [
            ClockAnswer::Passed,
            ClockAnswer::Kept,
            ClockAnswer::NoAttackers,
            ClockAnswer::NoBlockers,
            ClockAnswer::Declined,
            ClockAnswer::ChosenForThem,
        ] {
            out.push(LogEvent::TimedOut { player, answer });
        }
        for cause in [
            LossCause::Life,
            LossCause::EmptyDraw,
            LossCause::Poison,
            LossCause::CommanderDamage,
            LossCause::Conceded,
            LossCause::Effect,
        ] {
            out.push(LogEvent::Lost { player, cause });
        }
        let zones = [
            LogZone::Library,
            LogZone::Hand,
            LogZone::Battlefield,
            LogZone::Graveyard,
            LogZone::Exile,
            LogZone::Command,
        ];
        for from in zones {
            for to in zones {
                out.push(LogEvent::Moved {
                    place: None,
                    object: card.clone(),
                    owner: player,
                    from,
                    to,
                });
            }
        }
        for place in [
            LogPlace::Top,
            LogPlace::Bottom,
            LogPlace::FromTop(2),
            LogPlace::FromTop(11),
            LogPlace::FromTop(23),
            LogPlace::Shuffled,
        ] {
            out.push(LogEvent::Moved {
                place: Some(place),
                object: card.clone(),
                owner: player,
                from: LogZone::Hand,
                to: LogZone::Library,
            });
        }
        for (zone, owner) in [
            (LogZone::Hand, player),
            (LogZone::Library, player),
            (LogZone::Graveyard, player),
            (LogZone::Graveyard, PlayerId::new(2)),
            (LogZone::Exile, PlayerId::new(2)),
            (LogZone::Command, player),
        ] {
            let from = Some(LogFrom { zone, owner });
            out.push(LogEvent::LandPlayed {
                player,
                land: token(5, "Island"),
                from,
            });
            out.push(LogEvent::Cast {
                player,
                spell: card.clone(),
                from,
            });
        }
        for (source, combat) in [
            (Some(card.clone()), true),
            (Some(card.clone()), false),
            (None, false),
        ] {
            for target in [
                LogTarget::Player(player),
                LogTarget::Object(token(4, "Soldier")),
            ] {
                out.push(LogEvent::Damage {
                    source: source.clone(),
                    target,
                    amount: 2,
                    combat,
                });
            }
        }
        for kind in [
            CounterKind::PLUS_ONE,
            CounterKind::Minus {
                power: 2,
                toughness: 1,
            },
            CounterKind::Loyalty,
            CounterKind::Lore,
            CounterKind::Time,
            CounterKind::Charge,
            CounterKind::Poison,
            CounterKind::Energy,
            CounterKind::Rad,
            CounterKind::Lifelink,
            CounterKind::Level,
            CounterKind::Custom(99),
        ] {
            for new in [1, 4] {
                out.push(LogEvent::Counters {
                    object: token(4, "Soldier"),
                    kind,
                    old: 2,
                    new,
                });
            }
        }
        for winners in [&[player][..], &[player, PlayerId::new(2)]] {
            let mut set = SeatSet::new();
            for seat in winners {
                set.insert(*seat);
            }
            out.push(LogEvent::GameOver { winners: set });
        }
        out
    }

    /// Every kind of line, about the reading seat and about another, in every
    /// language, is a whole sentence: no placeholder left, no gap where a
    /// name went, no full stop, and the reading seat never by its name.
    #[test]
    fn every_line_is_a_whole_sentence_in_every_language() {
        let mut seen = [false; VARIANTS];
        for player in [ME, BO] {
            let events = every_line_about(player);
            for event in &events {
                seen[variant(event)] = true;
            }
            let entries: Vec<LogEntry> = events.into_iter().map(|e| entry(1, e)).collect();
            let book = book(&entries);
            for lang in Lang::ALL {
                let roster = roster();
                let catalog = Catalog(lang);
                let wording = Wording {
                    lang,
                    seat: ME,
                    statics: Some(&roster),
                    texts: &catalog,
                };
                for (line, entry) in book.lines(&wording).iter().zip(&entries) {
                    let text = &line.text;
                    let about = format!("{:?} in {lang:?}: {text:?}", entry.event);
                    assert!(!text.trim().is_empty(), "{about}");
                    assert!(!text.contains(['{', '}']), "{about}");
                    assert!(!text.contains("  "), "{about}");
                    assert_eq!(text.trim(), text, "{about}");
                    assert!(!text.ends_with('.'), "{about}");
                    assert!(!text.starts_with(char::is_lowercase), "{about}");
                    assert!(!text.contains("Ada"), "the reader by name: {about}");
                    for span in &line.names {
                        assert!(text.get(span.range.clone()).is_some(), "{about}");
                    }
                }
            }
        }
        let missing: Vec<usize> = (0..VARIANTS).filter(|&i| !seen[i]).collect();
        assert!(missing.is_empty(), "no line of variants {missing:?}");
    }

    /// The seat each kind of line is about: the player it names, or whose
    /// card moved; nobody for a line about the table. No `_` arm: a new event
    /// has to say.
    fn about_whom(event: &LogEvent) -> Option<PlayerId> {
        match event {
            LogEvent::TurnStarted { active: player }
            | LogEvent::Mulliganed { player }
            | LogEvent::Kept { player, .. }
            | LogEvent::TimedOut { player, .. }
            | LogEvent::StandIn { player }
            | LogEvent::Returned { player }
            | LogEvent::LandPlayed { player, .. }
            | LogEvent::Cast { player, .. }
            | LogEvent::Ability {
                controller: player, ..
            }
            | LogEvent::Drew { player, .. }
            | LogEvent::Discarded { player, .. }
            | LogEvent::Moved { owner: player, .. }
            | LogEvent::Created {
                controller: player, ..
            }
            | LogEvent::Damage {
                target: LogTarget::Player(player),
                ..
            }
            | LogEvent::Life { player, .. }
            | LogEvent::Attacked {
                defending: Defender::Player(player),
                ..
            }
            | LogEvent::ControlChanged { new: player, .. }
            | LogEvent::Revealed { player, .. }
            | LogEvent::Shuffled { player }
            | LogEvent::DiceRolled { player, .. }
            | LogEvent::Lost { player, .. } => Some(*player),
            LogEvent::Countered { .. }
            | LogEvent::DidNotResolve { .. }
            | LogEvent::Damage {
                target: LogTarget::Object(_),
                ..
            }
            | LogEvent::Counters { .. }
            | LogEvent::Attacked {
                defending: Defender::Planeswalker(_),
                ..
            }
            | LogEvent::Blocked { .. }
            | LogEvent::Transformed { .. }
            | LogEvent::GameOver { .. }
            | LogEvent::LoopDetected { .. }
            | LogEvent::DayNight { .. } => None,
        }
    }

    /// Every line says which seat it is about, the reading seat as much as
    /// any other, so a panel can mark it; a line about the table says
    /// nobody, even the end of a game one seat won.
    #[test]
    fn every_line_says_which_seat_it_is_about() {
        let cast = LogEvent::Cast {
            from: None,
            player: BO,
            spell: bolt(3),
        };
        assert_eq!(read(&book(&[entry(1, cast)]), Lang::En).subject, Some(BO));
        let day = LogEvent::DayNight { now: DayNight::Day };
        assert_eq!(read(&book(&[entry(1, day)]), Lang::En).subject, None);

        let roster = roster();
        let wording = Wording {
            lang: Lang::En,
            seat: ME,
            statics: Some(&roster),
            texts: &no_text,
        };
        for player in [ME, BO] {
            let entries: Vec<LogEntry> = every_line_about(player)
                .into_iter()
                .map(|event| entry(1, event))
                .collect();
            let lines = book(&entries).lines(&wording);
            for (line, entry) in lines.iter().zip(&entries) {
                assert_eq!(
                    line.subject,
                    about_whom(&entry.event),
                    "{:?}: {:?}",
                    entry.event,
                    line.text
                );
            }
            let about = lines.iter().filter(|line| line.subject == Some(player));
            assert!(about.count() > 40, "the walk names {player:?}");
        }
    }

    /// Every player a line names is marked where it names them (#300):
    /// another seat by its name wherever the sentence puts it, and the
    /// reading seat wherever the sentence says "you" to it, and nowhere
    /// else: not "your", and never inside a card's name.
    #[test]
    fn every_player_a_line_names_is_marked() {
        let roster = roster();
        let you: fn(Lang) -> &'static [&'static str] = |lang| match lang {
            Lang::En => &["you", "You"],
            Lang::De => &["du", "Du", "dich", "dir"],
        };
        let mut marked_you = 0;
        for lang in Lang::ALL {
            let wording = Wording {
                lang,
                seat: ME,
                statics: Some(&roster),
                texts: &no_text,
            };
            for player in [ME, BO] {
                let entries: Vec<LogEntry> = every_line_about(player)
                    .into_iter()
                    .map(|event| entry(1, event))
                    .collect();
                for line in book(&entries).lines(&wording) {
                    let in_a_name = |at: usize| line.names.iter().any(|n| n.range.contains(&at));
                    let mut words = Vec::new();
                    let mut start = None;
                    for (at, c) in line.text.char_indices().chain([(line.text.len(), ' ')]) {
                        match (c.is_alphanumeric(), start) {
                            (true, None) => start = Some(at),
                            (false, Some(from)) => {
                                words.push(from..at);
                                start = None;
                            }
                            _ => {}
                        }
                    }
                    let naming: Vec<Range<usize>> = words
                        .into_iter()
                        .filter(|word| !in_a_name(word.start))
                        .filter(|word| {
                            let text = &line.text[word.clone()];
                            you(lang).contains(&text) || text == "Bo" || text == "Cy"
                        })
                        .collect();
                    let marks: Vec<Range<usize>> =
                        line.players.iter().map(|span| span.range.clone()).collect();
                    assert_eq!(marks, naming, "{lang:?}: {}", line.text);
                    for span in &line.players {
                        let text = &line.text[span.range.clone()];
                        if span.player == ME {
                            assert!(you(lang).contains(&text), "{}", line.text);
                            marked_you += 1;
                        } else {
                            assert_eq!(text, seat_name(lang, Some(&roster), span.player));
                        }
                    }
                }
            }
        }
        assert!(marked_you > 80, "the walk says you {marked_you} times");
    }

    /// A card put into a library says where in it (#300), and a card
    /// shuffled in says that instead.
    #[test]
    fn a_card_put_into_a_library_says_where() {
        let moved = |owner, place| LogEvent::Moved {
            object: bolt(3),
            owner,
            from: LogZone::Hand,
            to: LogZone::Library,
            place: Some(place),
        };
        assert_eq!(
            say(Lang::En, moved(ME, LogPlace::Top)),
            "Lightning Bolt moved from your hand on top of your library"
        );
        assert_eq!(
            say(Lang::En, moved(BO, LogPlace::Bottom)),
            "Lightning Bolt moved from Bo's hand on the bottom of Bo's library"
        );
        for (n, nth) in [
            (2, "2nd"),
            (3, "3rd"),
            (4, "4th"),
            (11, "11th"),
            (12, "12th"),
            (21, "21st"),
            (113, "113th"),
        ] {
            assert_eq!(
                say(Lang::En, moved(ME, LogPlace::FromTop(n))),
                format!("Lightning Bolt moved from your hand {nth} from the top of your library")
            );
        }
        assert_eq!(
            say(Lang::De, moved(BO, LogPlace::FromTop(3))),
            "Blitzschlag ist aus der Hand von Bo als 3. Karte von oben in die Bibliothek von Bo gelangt"
        );
        assert_eq!(
            say(Lang::En, moved(ME, LogPlace::Shuffled)),
            "Lightning Bolt was shuffled from your hand into your library"
        );
        assert_eq!(
            say(Lang::De, moved(ME, LogPlace::Shuffled)),
            "Blitzschlag wurde aus deiner Hand in deine Bibliothek gemischt"
        );
    }

    /// A land played and a spell cast say where from (#300): "their" when the
    /// zone is the player's own, the owner's name when it is not, and
    /// nothing when the host could not say.
    #[test]
    fn a_play_and_a_cast_say_where_from() {
        let cast = |player, from| LogEvent::Cast {
            player,
            spell: bolt(3),
            from,
        };
        let from = |zone, owner| Some(LogFrom { zone, owner });
        assert_eq!(
            say(
                Lang::En,
                LogEvent::LandPlayed {
                    player: ME,
                    land: token(5, "Island"),
                    from: from(LogZone::Hand, ME),
                }
            ),
            "You played Island from your hand"
        );
        assert_eq!(
            say(Lang::En, cast(BO, from(LogZone::Graveyard, BO))),
            "Bo cast Lightning Bolt from their graveyard"
        );
        assert_eq!(
            say(Lang::De, cast(BO, from(LogZone::Graveyard, BO))),
            "Bo hat Blitzschlag aus dem eigenen Friedhof gewirkt"
        );
        assert_eq!(
            say(Lang::En, cast(BO, from(LogZone::Graveyard, ME))),
            "Bo cast Lightning Bolt from your graveyard"
        );
        assert_eq!(
            say(Lang::En, cast(ME, from(LogZone::Graveyard, BO))),
            "You cast Lightning Bolt from Bo's graveyard"
        );
        assert_eq!(
            say(Lang::De, cast(ME, from(LogZone::Exile, BO))),
            "Du hast Blitzschlag aus dem Exil gewirkt"
        );
        assert_eq!(say(Lang::En, cast(BO, None)), "Bo cast Lightning Bolt");
    }

    /// A line's time is said as the wall clock says it, wherever the device
    /// is: the offset moves it across midnight either way, and a half-hour
    /// zone keeps its minutes (#300).
    #[test]
    fn a_line_says_its_time_on_the_wall_clock() {
        let mut line = LogLine {
            index: 0,
            turn: 1,
            times: 1,
            header: false,
            at: 0,
            text: String::new(),
            names: Vec::new(),
            players: Vec::new(),
            ability: None,
            subject: None,
        };
        assert_eq!(line.clock(0), None, "a line the host never dated");
        // 2026-09-25 12:34:56.789 UTC.
        line.at = 1_790_339_696_789;
        assert_eq!(line.clock(0).as_deref(), Some("12:34"));
        assert_eq!(line.clock(2 * 3600).as_deref(), Some("14:34"));
        // West of Greenwich across midnight, and east across it.
        assert_eq!(line.clock(-13 * 3600).as_deref(), Some("23:34"));
        assert_eq!(line.clock(12 * 3600).as_deref(), Some("00:34"));
        assert_eq!(line.clock(5 * 3600 + 1800).as_deref(), Some("18:04"));
    }

    /// A line carries the time the host wrote it, and a token's registry
    /// entry, so a panel can show both after the token is gone (#300).
    #[test]
    fn a_line_carries_its_time_and_its_token() {
        let treasure = LogObject::Known {
            id: ObjectId::new(12, 0),
            card: None,
            token: Some(3),
            name: "Treasure".to_string(),
        };
        let entry = LogEntry {
            at: 1_727_000_000_000,
            ..entry(
                2,
                LogEvent::Created {
                    object: treasure,
                    controller: BO,
                },
            )
        };
        let line = read(&book(&[entry]), Lang::En);
        assert_eq!(line.at, 1_727_000_000_000);
        assert_eq!(line.names.len(), 1);
        assert_eq!(line.names[0].token, Some(3));
        assert_eq!(&line.text[line.names[0].range.clone()], "Treasure");
    }

    /// Every log phrase has a line that says it: a phrase nothing writes is a
    /// sentence a translator keeps for nobody.
    #[test]
    fn every_log_phrase_is_written_by_this_module() {
        let source = include_str!("gamelog.rs");
        let (code, _) = source
            .split_once("#[cfg(test)]")
            .expect("the tests follow the code");
        let unused: Vec<String> = Phrase::ALL
            .iter()
            .map(|phrase| format!("{phrase:?}"))
            .filter(|name| name.starts_with("Log"))
            .filter(|name| !code.contains(&format!("Phrase::{name}")))
            .collect();
        assert!(unused.is_empty(), "never written: {unused:?}");
    }
}
