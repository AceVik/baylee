//! The narrator: one decision, told in English, for a mind that reads.
//!
//! A language model is handed each real decision as one message (`llm-seat.md`
//! §5.1): the header (question, turn, step, whose turn, the time to answer),
//! the seats, the whole board, the hand and the stack, what happened since
//! the last decision, the text of cards the conversation has not shown yet,
//! and the question with its numbered options. The board is always whole;
//! only the log is a delta, because a model reconciling a delta board against
//! one two messages back attacks with creatures that died.
//!
//! # What the model may read
//!
//! Only what the seat's own [`PlayerView`] and [`Pending`] carry, and the
//! compiled English Oracle of cards the view names (`baylee_cards::oracle`,
//! `docs/legal.md` clause 9). A face-down card is "face-down", a hidden log
//! object is "a card", a library and another player's hand are counts. The
//! players' display names are the one text a person chose; they appear once
//! per conversation, in the prefix ([`prefix`]), inside «», and everywhere
//! else a player is `P1`…`Pn` (seat order, one-based), so a name never
//! stands where an instruction could.
//!
//! # Ids
//!
//! An object is `#` and its arena slot, which names one live object at a
//! time; a stale answer is refused, never misapplied, because an answer is
//! resolved against the question's own options ([`Menu::resolve`]).
//! Options are `a1`, `a2`, … and `p` for priority, `keep`/`mulligan`,
//! `y`/`n`, `m1`… for modes, colour letters and `P2`-style players, rebuilt
//! for every question.
//!
//! # A second path, for one stage
//!
//! The client labels the same options for a person (`abilities.rs`,
//! `choices.rs` in `baylee-client`); stage 1b moves those into client-core so
//! both read one describer. Until then these labels are the bridge's own.
//! What the menu cannot offer yet, and says so nowhere else: an activated
//! ability whose mana is not already floating (the engine lists an ability
//! only when the pool pays it, and a reader that planned taps for one would
//! also have to check its targets and timing, which the engine alone knows),
//! and a kicker or alternative cost reached by tapping (the engine asks
//! which way to cast once the card is cast).

mod board;
mod menu;
mod words;

pub use menu::{
    Act, Decision, GRAMMAR, Hint, Menu, React, Resolved, Step, Stop, Stops, Until, asked, tap,
};
pub use words::{KEYWORDS, color_name, step_id};

use crate::mind::{GameContext, Request};
use baylee_client_core::gamelog::{LogBook, Wording};
use baylee_client_core::i18n::Lang;
use baylee_core::ids::{CardIndex, ObjectId, PlayerId};
use baylee_view::{
    GameStatic, LogEntry, LogTail, PlayerView, PublicObject, RulesFace, SeatIdentity,
};
use std::collections::BTreeSet;
use std::fmt::Write as _;

/// The most log lines one wake carries; older ones are counted, not told.
pub const LOG_LINES: usize = 40;

/// A rough token count for English text: one token per three characters,
/// which over-counts English prose by about a quarter. For budgets and
/// tests; the provider's usage is the real number.
#[must_use]
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(3)
}

/// One decision, told.
#[derive(Clone, Debug)]
pub struct Wake {
    /// The message.
    pub text: String,
    /// What its options mean.
    pub menu: Menu,
    /// One line for a person watching: the question, the turn and the step.
    pub headline: String,
}

/// How a seat's messages are told, fixed for its game: what a model's way
/// of answering needs said, and nothing else.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    /// Whether each question ends with the shape of its answer even where
    /// the options say it all (`Answer: ask="q12", pick=[one option id]`):
    /// for a model whose answer no schema holds, which reads its fields
    /// only from its instructions. A tool's or a schema's model is told
    /// only the shapes the question itself carries (piles, blocks, how
    /// many ids).
    pub spell_answer: bool,
    /// How the deck is told at the head of a conversation.
    pub deck: DeckText,
}

/// How a seat's deck is told in a conversation's prefix: decided once a
/// game from how the mind is reached, so the prefix's bytes never change
/// within a game (a provider's cache keys on them).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DeckText {
    /// Every card's text, once, in the prefix: where the prefix is read
    /// back from a provider's cache at a tenth of the price (Anthropic, an
    /// endpoint that caches, an agent CLI).
    #[default]
    Full,
    /// The cards' names only; a card's text is told the first time the
    /// card is seen, in the "New cards" block: where every token of every
    /// call is read again at full cost (a model on this machine). Most of
    /// a Commander deck is never seen in a game.
    Names,
}

/// What one conversation has been told so far.
#[derive(Clone, Debug)]
pub struct Narrator {
    /// How it tells.
    style: Style,
    /// Card faces whose full text this conversation carries.
    shown: BTreeSet<(CardIndex, u8)>,
    /// Cards whose text the stable prefix carries: the seat's own deck.
    deck: BTreeSet<CardIndex>,
    /// Log entries the seat was told and no wake has narrated yet.
    unread: Vec<LogEntry>,
    /// The log index the next entry the seat is told will have.
    next: u64,
    /// Whether some lines never reached the narrator (a request dropped
    /// before its mind was asked).
    gap: bool,
}

impl Narrator {
    /// A narrator for a seat that brought the deck in `context`, telling
    /// in the default [`Style`].
    #[must_use]
    pub fn new(context: &GameContext) -> Self {
        Self::styled(context, Style::default())
    }

    /// A narrator for a seat that brought the deck in `context`, telling
    /// in `style`.
    #[must_use]
    pub fn styled(context: &GameContext, style: Style) -> Self {
        // Under [`DeckText::Names`] the prefix carries no card's text, so
        // every card is told on sight.
        let deck = match style.deck {
            DeckText::Full => context
                .deck
                .main
                .iter()
                .chain(&context.deck.commanders)
                .map(|entry| entry.card)
                .collect(),
            DeckText::Names => BTreeSet::new(),
        };
        Self {
            style,
            shown: BTreeSet::new(),
            deck,
            unread: Vec::new(),
            next: 0,
            gap: false,
        }
    }

    /// Takes the log lines a request carries. Every request is heard, the
    /// ones answered without a wake too, so the next wake tells what
    /// happened while the plan or the hints answered.
    pub fn hear(&mut self, log: &LogTail) {
        if log.entries.is_empty() {
            return;
        }
        let from = u64::from(log.from);
        if from > self.next {
            self.gap = true;
        }
        let skip = usize::try_from(self.next.saturating_sub(from)).unwrap_or(usize::MAX);
        self.unread.extend(log.entries.iter().skip(skip).cloned());
        let end = from + log.entries.len() as u64;
        self.next = self.next.max(end);
    }

    /// The stable first part of a conversation, as this narrator tells the
    /// deck ([`prefix`]).
    #[must_use]
    pub fn prefix(&self, context: &GameContext) -> String {
        prefix(context, self.style.deck)
    }

    /// The menu of `request`'s question, untold: what a plan's step is read
    /// against ([`Menu::plan_decision`]).
    #[must_use]
    pub fn menu(&self, request: &Request) -> Menu {
        let table = Table::new(&request.view, &request.context);
        menu::question(&table, request, self.style).1
    }

    /// Forgets which cards the conversation was shown: it was replaced by a
    /// fresh one, which carries only the deck in its prefix.
    pub fn forget_cards(&mut self) {
        self.shown.clear();
    }

    /// Tells `request` as one message. `notes` are the mind's own lines
    /// (a late answer, a plan that stopped), said under the header.
    #[must_use]
    pub fn wake(
        &mut self,
        request: &Request,
        notes: &[String],
        stops_summary: Option<&str>,
    ) -> Wake {
        let table = Table::new(&request.view, &request.context);
        let mut text = String::new();
        let headline = table.headline(request);
        let _ = writeln!(text, "{}", table.header(request, stops_summary));
        for note in notes {
            let _ = writeln!(text, "{note}");
        }
        text.push('\n');
        board::seats(&table, &mut text);
        text.push('\n');
        board::board(&table, &mut text);
        let log = self.take_log(&table);
        if !log.is_empty() {
            text.push_str("\nSince your last decision:\n");
            for line in log {
                let _ = writeln!(text, "  {line}");
            }
        }
        let cards = self.new_cards(&table);
        if !cards.is_empty() {
            text.push_str("\nNew cards (full text once; later by name):\n");
            for card in cards {
                text.push_str(&card);
            }
        }
        let (question, menu) = menu::question(&table, request, self.style);
        text.push('\n');
        text.push_str(&question);
        Wake {
            text,
            menu,
            headline,
        }
    }

    /// The lines not yet narrated, written for the seat.
    fn take_log(&mut self, table: &Table<'_>) -> Vec<String> {
        let entries = std::mem::take(&mut self.unread);
        let gap = std::mem::take(&mut self.gap);
        if entries.is_empty() && !gap {
            return Vec::new();
        }
        let mut book = LogBook::new();
        let whole = entries.len();
        book.append(&LogTail { from: 0, entries }, table.view);
        let statics = table.statics();
        let wording = Wording {
            lang: Lang::En,
            seat: table.me(),
            statics: Some(&statics),
            texts: &|_: CardIndex, _: u8| None,
        };
        // Lines that read the same once written are told once, counted:
        // four Treasures that left the battlefield have no handle left to
        // tell them apart by, and four lines would say nothing a count
        // does not.
        let mut folded: Vec<(String, u32)> = Vec::new();
        for line in book.lines(&wording) {
            let mut text = line.text.clone();
            // Name spans in reverse, so earlier ranges stay put.
            for span in line.names.iter().rev() {
                if table.visible(span.id) {
                    text.insert_str(span.range.end, &format!(" {}", tag(span.id)));
                }
            }
            let times = line.times.max(1);
            match folded.last_mut() {
                Some((last, n)) if *last == text => *n += times,
                _ => folded.push((text, times)),
            }
        }
        let mut lines: Vec<String> = folded
            .into_iter()
            .map(|(mut text, times)| {
                if times > 1 {
                    let _ = write!(text, " ({times} times)");
                }
                text.push('.');
                text
            })
            .collect();
        let mut out = Vec::new();
        if gap {
            out.push("(some earlier lines were lost on the way)".to_string());
        }
        if whole > LOG_LINES {
            let dropped = whole - LOG_LINES;
            lines.drain(..dropped);
            out.push(format!("({dropped} earlier lines not shown)"));
        }
        out.extend(lines);
        out
    }

    /// The full text of every card face the view names that this
    /// conversation has not been shown and the deck prefix does not carry.
    fn new_cards(&mut self, table: &Table<'_>) -> Vec<String> {
        let mut faces: Vec<RulesFace> = Vec::new();
        let view = table.view;
        let objects = view
            .battlefield
            .iter()
            .chain(&view.stack)
            .chain(view.graveyards.iter().flatten())
            .chain(view.exile.iter().flatten())
            .chain(view.command.iter().flatten())
            .chain(&view.looking_at)
            .chain(&view.library_tops)
            .chain(view.targeting.iter().map(|t| &t.source));
        for object in objects {
            faces.extend(object_faces(object));
        }
        for card in view
            .hand
            .iter()
            .chain(view.shared_hands.iter().flat_map(|h| &h.cards))
            .chain(view.controlled_hands.iter().flat_map(|h| &h.cards))
        {
            faces.push(RulesFace::from(card.card));
        }
        let mut out = Vec::new();
        for face in faces {
            if self.deck.contains(&face.card) || !self.shown.insert((face.card, face.face)) {
                continue;
            }
            if let Some(text) = card_text(face.card, usize::from(face.face)) {
                out.push(text);
            }
        }
        out
    }
}

/// The card faces an object shows text for: its rules face (a copy's is
/// the copied card's, CR 707.2), and on the stack the card an ability
/// comes from. None for a face-down object whose card the seat may not see.
fn object_faces(object: &PublicObject) -> Vec<RulesFace> {
    let mut faces = Vec::new();
    if let Some(rules) = object.rules {
        faces.push(rules);
    } else if let Some(card) = object.card {
        faces.push(RulesFace::from(card));
    }
    if let Some(baylee_view::StackItem::Ability {
        rules: Some(rules), ..
    }) = object.stack_item
    {
        faces.push(rules);
    }
    faces
}

/// One card face's full text, as the "New cards" block and the deck prefix
/// print it:
///
/// ```text
///   Serra Angel {3}{W}{W} · Creature — Angel · 4/4
///     Flying, vigilance
/// ```
#[must_use]
pub fn card_text(card: CardIndex, face: usize) -> Option<String> {
    let def = baylee_cards::by_index(card)?;
    let printed = def.faces.get(face)?;
    let mut out = format!("  {}", printed.name);
    let cost = printed.mana_cost.to_string();
    if !cost.is_empty() {
        let _ = write!(out, " {cost}");
    }
    let _ = write!(out, " · {}", baylee_cards::pool::type_line(printed));
    if let Some(stats) = baylee_cards::pool::stats(printed) {
        let _ = write!(out, " · {stats}");
    }
    out.push('\n');
    if let Some(oracle) = baylee_cards::oracle::face(card, face) {
        for line in oracle.lines().map(strip_reminder) {
            if !line.trim().is_empty() {
                let _ = writeln!(out, "    {line}");
            }
        }
    }
    Some(out)
}

/// One line of Oracle text without the reminder text a model knows by
/// heart (CR 207.2: reminder text is in parentheses): a parenthesis that is
/// the whole line (a basic land's `({T}: Add {G}.)`), one after nothing but
/// keywords the narrator names ([`KEYWORDS`]: `Vigilance (Attacking doesn't
/// cause …)`), or one after a sentence that ends in such a keyword
/// (`Creatures you control have trample. (…)`). A keyword with a parameter
/// keeps its reminder (`Suspend 4—{U} (…)`, `Ward {2} (…)`, `scry 1. (…)`),
/// and so does a parenthesis that does not close the line.
#[must_use]
pub fn strip_reminder(line: &str) -> String {
    let Some(open) = line.find('(') else {
        return line.to_string();
    };
    let is_keyword = |word: &str| {
        KEYWORDS
            .iter()
            .any(|(_, name)| name.eq_ignore_ascii_case(word.trim()))
    };
    let before = line[..open].trim();
    let keywords_only = !before.is_empty() && before.split(", ").all(is_keyword);
    let after_keyword = before.strip_suffix('.').is_some_and(|sentence| {
        KEYWORDS.iter().any(|(_, name)| {
            sentence.len() > name.len()
                && sentence.is_char_boundary(sentence.len() - name.len())
                && sentence[sentence.len() - name.len()..].eq_ignore_ascii_case(name)
                && sentence[..sentence.len() - name.len()].ends_with(' ')
        })
    });
    if !(before.is_empty() || keywords_only || after_keyword) {
        return line.to_string();
    }
    // The reminder is one parenthesis from `open` that closes the line.
    let rest = line[open..].trim_end();
    let mut depth = 0_i32;
    for (at, c) in rest.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && at + 1 != rest.len() {
                    return line.to_string();
                }
            }
            _ => {}
        }
    }
    if depth != 0 {
        return line.to_string();
    }
    line[..open].trim_end().to_string()
}

/// The stable first part of a conversation: the seat, the table and the
/// deck it brought, its full text or its names as `deck` says, which a
/// provider caches. A function of the game alone, so it is the same bytes
/// at every conversation of the game.
#[must_use]
pub fn prefix(context: &GameContext, deck_text: DeckText) -> String {
    let mut out = String::new();
    let me = usize::from(context.seat.get());
    let _ = writeln!(
        out,
        "THE GAME\nYou are P{} at a table of {} players ({}).",
        me + 1,
        context.seats,
        match context.format {
            baylee_core::preset::FormatId::Commander => "Commander: 40 life, commander tax",
            _ => "freeform",
        }
    );
    for (seat, name) in context.names.iter().enumerate() {
        if seat == me {
            continue;
        }
        let side = match (
            context.teams.get(me).copied().flatten(),
            context.teams.get(seat).copied().flatten(),
        ) {
            (Some(a), Some(b)) if a == b => "your teammate",
            _ => "opponent",
        };
        let _ = writeln!(out, "P{} is «{name}», {side}.", seat + 1);
    }
    if let Some(secs) = context.decision_secs {
        let _ = writeln!(out, "The table gives each decision {secs} seconds.");
    }
    let deck = &context.deck;
    let cards: u32 = deck
        .main
        .iter()
        .chain(&deck.commanders)
        .map(|entry| entry.count)
        .sum();
    let _ = match deck_text {
        DeckText::Full => writeln!(
            out,
            "\nYOUR DECK «{}» ({cards} cards). The full text of every card in it; later \
             messages name these cards without their text.",
            deck.name
        ),
        DeckText::Names => writeln!(
            out,
            "\nYOUR DECK «{}» ({cards} cards), by name. A card's full text is shown the \
             first time it is seen, under \"New cards\".",
            deck.name
        ),
    };
    for (label, entries) in [("Commander", &deck.commanders), ("Main deck", &deck.main)] {
        if entries.is_empty() {
            continue;
        }
        let _ = writeln!(out, "{label}:");
        for entry in entries {
            let Some(def) = baylee_cards::by_index(entry.card) else {
                let _ = writeln!(out, "  {}× (a card this build does not know)", entry.count);
                continue;
            };
            if deck_text == DeckText::Names {
                let _ = writeln!(out, "  {}× {}", entry.count, def.name());
                continue;
            }
            for face in 0..def.faces.len() {
                let Some(text) = card_text(entry.card, face) else {
                    continue;
                };
                if face == 0 {
                    let _ = write!(out, "  {}× {}", entry.count, text.trim_start());
                } else {
                    let _ = write!(out, "    other face: {}", text.trim_start());
                }
            }
        }
    }
    if !deck.sideboard.is_empty() {
        let names: Vec<String> = deck
            .sideboard
            .iter()
            .map(|entry| {
                baylee_cards::by_index(entry.card).map_or_else(
                    || "?".to_string(),
                    |def| format!("{}× {}", entry.count, def.name()),
                )
            })
            .collect();
        let _ = writeln!(
            out,
            "Sideboard (reached only by a wish): {}",
            names.join(", ")
        );
    }
    out
}

/// An object's handle as the model reads and writes it: `#` and its slot.
#[must_use]
pub fn tag(id: ObjectId) -> String {
    format!("#{}", id.slot())
}

/// The view and the table it was told at: every name the narrator writes.
pub(crate) struct Table<'a> {
    pub view: &'a PlayerView,
    pub context: &'a GameContext,
}

impl<'a> Table<'a> {
    pub const fn new(view: &'a PlayerView, context: &'a GameContext) -> Self {
        Self { view, context }
    }

    pub const fn me(&self) -> PlayerId {
        self.view.seat
    }

    /// A player as a subject: "you" or "P2".
    pub fn player(&self, p: PlayerId) -> String {
        if p == self.me() {
            "you".into()
        } else {
            format!("P{}", u16::from(p.get()) + 1)
        }
    }

    /// A player as an id the model may answer with: always "P2".
    pub fn player_id(p: PlayerId) -> String {
        format!("P{}", u16::from(p.get()) + 1)
    }

    /// A player's possessive: "your" or "P2's".
    pub fn whose(&self, p: PlayerId) -> String {
        if p == self.me() {
            "your".into()
        } else {
            format!("P{}'s", u16::from(p.get()) + 1)
        }
    }

    /// Whether `p` plays on this seat's side.
    pub fn ally(&self, p: PlayerId) -> bool {
        if p == self.me() {
            return true;
        }
        let side = |p: PlayerId| {
            self.context
                .teams
                .get(usize::from(p.get()))
                .copied()
                .flatten()
        };
        matches!((side(p), side(self.me())), (Some(a), Some(b)) if a == b)
    }

    /// An object's name as the view shows it, wherever it is.
    pub fn name(&self, id: ObjectId) -> Option<String> {
        if let Some(object) = self.view.object(id) {
            return Some(object_name(object));
        }
        self.view
            .hand
            .iter()
            .chain(self.view.shared_hands.iter().flat_map(|h| &h.cards))
            .chain(self.view.controlled_hands.iter().flat_map(|h| &h.cards))
            .find(|card| card.id == id)
            .map(|card| card.name.clone())
    }

    /// An object named with its handle: "Serra Angel #45", or "#45" when
    /// the view does not show it.
    pub fn named(&self, id: ObjectId) -> String {
        self.name(id)
            .map_or_else(|| tag(id), |name| format!("{name} {}", tag(id)))
    }

    /// Names only the entitled exact target snapshot, never a newer incarnation.
    pub fn named_target(&self, source: baylee_core::ids::DamageSourceRef) -> String {
        self.view.target_object(source).map_or_else(
            || "an earlier target whose identity is unavailable".to_string(),
            |target| {
                let state = if target.is_current {
                    "current"
                } else {
                    "earlier incarnation"
                };
                format!(
                    "{} {} ({state}, version {})",
                    target.name,
                    tag(source.object),
                    source.version
                )
            },
        )
    }

    /// Whether the view shows an object with this handle now.
    pub fn visible(&self, id: ObjectId) -> bool {
        self.name(id).is_some()
    }

    /// A roster for the log's writer that numbers every other seat `P2`…
    /// and never carries a display name into a sentence.
    pub fn statics(&self) -> GameStatic {
        GameStatic {
            view_version: baylee_view::VIEW_VERSION,
            game_id: self.context.game_id.clone(),
            your_seat: self.me(),
            seats: self
                .view
                .seats
                .iter()
                .map(|seat| SeatIdentity {
                    player: seat.player,
                    display_name: Self::player_id(seat.player),
                    is_ai: false,
                    away: false,
                    team: self
                        .context
                        .teams
                        .get(usize::from(seat.player.get()))
                        .copied()
                        .flatten(),
                })
                .collect(),
            prints: Vec::new(),
            decision_secs: self.context.decision_secs,
            reconnect_secs: None,
        }
    }

    /// "turn 7, yours" or "turn 7, P2's".
    fn turn_short(&self) -> String {
        let whose = if self.view.active == self.me() {
            "yours".to_string()
        } else {
            self.whose(self.view.active)
        };
        format!("turn {}, {whose}", self.view.turn)
    }

    /// "turn 7, your turn" or "turn 7, P2's turn".
    fn turn(&self) -> String {
        format!(
            "turn {}, {} turn",
            self.view.turn,
            self.whose(self.view.active)
        )
    }

    /// `DECISION q12 · turn 7, yours · main1 · 25 s`: the question, the
    /// turn and whose it is, the step by the name `stops` and `until` use,
    /// and the seconds the answer has.
    fn header(&self, request: &Request, stops_summary: Option<&str>) -> String {
        let mut text = format!(
            "DECISION q{} · {} · {} · {} s",
            request.question,
            self.turn_short(),
            words::step_id(self.view.phase, self.view.step),
            request.budget.as_secs(),
        );
        if let Some(stops) = stops_summary {
            text.push_str(" · ");
            text.push_str(stops);
        }
        text
    }

    fn headline(&self, request: &Request) -> String {
        format!(
            "q{} · {} · {} · {}",
            request.question,
            self.turn(),
            words::step_name(self.view.phase, self.view.step),
            crate::scripted::kind(&request.pending),
        )
    }
}

/// An object's name, with what a face-down object is instead of one.
fn object_name(object: &PublicObject) -> String {
    if object.status.is_face_down() {
        return match object.card {
            Some(card) => baylee_cards::by_index(card.index)
                .and_then(|def| def.faces.get(usize::from(card.face)))
                .map_or_else(
                    || "face-down card".to_string(),
                    |face| format!("face-down card (it is {})", face.name),
                ),
            None => "face-down card".into(),
        };
    }
    if object.name.is_empty() {
        "an unnamed object".into()
    } else {
        object.name.clone()
    }
}

#[cfg(test)]
pub(crate) mod tests;
