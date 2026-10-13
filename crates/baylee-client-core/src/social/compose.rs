//! The composer (§10.4, §12.1): the chat box's text, its `#` and `@`
//! suggestions (the report sheet's [`refs`] reused), the lines sent this
//! session, its target chip, a draft per conversation, and the card a
//! right-click links.
//!
//! While it is open it holds every key; [`Composer::key`] answers each with
//! a [`ComposerOutcome`] for the shell.

use std::collections::{BTreeMap, VecDeque};

use baylee_core::ids::CardIndex;
use baylee_view::SeenIn;

use crate::bugreport::refs::{self, Candidates, CardRef, RefPrint, Sigil, Span, Trigger};
use crate::textbuf::{Dir, Step, TextBuffer};

use super::commands::Target;
use super::convs::{Body, BodyLink, BodyMention, BodyPrint, COMPOSE_LIMIT, ConvId};
use super::text::MAX_LINE_BREAKS;
use super::words::ChatPhrase;

/// How many sent lines `↑` walks back through (this session only).
pub const SENT_KEPT: usize = 32;

/// A key the composer was given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposerKey {
    /// Take the suggestion, send, or close an empty box.
    Enter,
    /// A line break (at most six).
    ShiftEnter,
    /// Put the suggestions away, else close (keeping the draft).
    Esc,
    /// In the suggestions, between lines, or back through what was sent.
    Up,
    /// The other way.
    Down,
    /// Take the suggestion, complete a name, or the next target.
    Tab,
    /// Take the suggestion, or the previous target.
    ShiftTab,
    /// Scroll the history up.
    PageUp,
    /// Scroll the history down.
    PageDown,
    /// To the start of the text.
    Home,
    /// To the end of the text.
    End,
    /// One character left.
    Left,
    /// One character right.
    Right,
    /// Delete the character before the caret, or the selection.
    Backspace,
    /// Delete the character after the caret, or the selection.
    Delete,
    /// Select everything typed.
    SelectAll,
    /// Copy the selection.
    Copy,
    /// Copy the selection and delete it.
    Cut,
    /// Insert text from the clipboard.
    Paste(String),
    /// One word left (`Alt`/`Option`+`←`).
    WordLeft,
    /// One word right.
    WordRight,
    /// Delete the word before the caret (`Ctrl`/`Cmd`+`⌫`).
    WordBackspace,
    /// `Alt`+digit: switch to tab N.
    AltDigit(u8),
}

/// What a key did, for the shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposerOutcome {
    /// Nothing changed.
    Nothing,
    /// The text, caret, suggestions or target changed: redraw.
    Changed,
    /// Send what is typed ([`super::Social::submit`] reads it).
    Submit,
    /// Close the box.
    Close,
    /// Scroll the history by pages (negative is up).
    ScrollHistory(i32),
    /// Show tab N (1-based).
    SwitchTab(u8),
    /// Put this on the clipboard.
    Copy(String),
}

/// What the composer is told about where it is, for Tab.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComposeContext {
    /// The targets Tab cycles through, in order: Table, Team, the open
    /// whisper partners by recency, then groups by recency.
    pub targets: Vec<Target>,
    /// The names `/w` completes: friends, recent whisper partners and the
    /// roster's handles.
    pub names: Vec<String>,
}

/// One suggestion in the popover.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SuggestItem {
    /// A card, by its index in the composer's candidates.
    Card(usize),
    /// A player, by its index in the composer's candidates.
    Player(usize),
}

/// The popover above the caret.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggest {
    /// What is being typed.
    pub trigger: Trigger,
    /// The suggestions, best first.
    pub items: Vec<SuggestItem>,
    /// The highlighted one.
    pub highlight: usize,
}

/// A card a right-press offers to link (§11.9): every place that shows a
/// card sends one, and only an open composer reads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkedCard {
    /// The registry index.
    pub card: CardIndex,
    /// The face showing.
    pub face: u8,
    /// The printing, when there is one.
    pub print: Option<RefPrint>,
    /// Where the view showed it; `None` outside a game (the deck builder).
    pub seen: Option<SeenIn>,
    /// Drawn face down: its identity is never shared.
    pub face_down: bool,
    /// Its name as the player sees it.
    pub name: String,
    /// Its English name.
    pub english: String,
}

/// The chat box.
#[derive(Clone, Debug, Default)]
pub struct Composer {
    buffer: TextBuffer,
    candidates: Candidates,
    picked: Vec<CardRef>,
    sent: VecDeque<String>,
    /// Where `↑` stands in `sent`, and the text that was typed before.
    recall: Option<(usize, String)>,
    target: Target,
    suggest: Option<Suggest>,
    draft: BTreeMap<ConvId, String>,
    own_card_hinted: bool,
    /// Tab's last completion: the prefix typed and the index into the names.
    completing: Option<(String, usize)>,
}

impl Composer {
    /// An empty composer.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The text typed.
    #[must_use]
    pub fn text(&self) -> &str {
        self.buffer.text()
    }

    /// The text field, for drawing its caret and selection.
    #[must_use]
    pub fn buffer(&self) -> &TextBuffer {
        &self.buffer
    }

    /// The target chip.
    #[must_use]
    pub fn target(&self) -> &Target {
        &self.target
    }

    /// Sets the target chip.
    pub fn set_target(&mut self, target: Target) {
        self.target = target;
    }

    /// What `#` and `@` may name here.
    pub fn set_candidates(&mut self, candidates: Candidates) {
        self.candidates = candidates;
        self.refresh();
    }

    /// The candidates `#` and `@` offer.
    #[must_use]
    pub fn candidates(&self) -> &Candidates {
        &self.candidates
    }

    /// The popover, when one is up.
    #[must_use]
    pub fn suggestions(&self) -> Option<&Suggest> {
        self.suggest.as_ref()
    }

    /// A suggestion's words, for drawing the popover.
    #[must_use]
    pub fn suggestion_label(&self, item: &SuggestItem) -> Option<&str> {
        match item {
            SuggestItem::Card(i) => self.candidates.cards.get(*i).map(|c| c.name.as_str()),
            SuggestItem::Player(i) => self.candidates.players.get(*i).map(|p| p.name.as_str()),
        }
    }

    /// Takes the suggestion at `index` (a press on its row in the popover);
    /// `false` when there is none there.
    pub fn choose(&mut self, index: usize) -> bool {
        match self.suggest.as_mut() {
            Some(suggest) if index < suggest.items.len() => {
                suggest.highlight = index;
                self.take_suggestion()
            }
            _ => false,
        }
    }

    /// Characters typed, for the count drawn from 900 of 1000.
    #[must_use]
    pub fn chars(&self) -> usize {
        self.buffer.text().chars().count()
    }

    /// Opens with `prefix` typed: `/` (the table's `/` key) or
    /// `/r Bob#0012 ` (its `R`). An empty prefix keeps the draft.
    pub fn open_with(&mut self, prefix: &str) {
        if !prefix.is_empty() {
            self.buffer = TextBuffer::new(prefix);
            self.picked.clear();
        }
        self.recall = None;
        self.refresh();
    }

    /// Keeps what is typed as `conv`'s draft and empties the box.
    pub fn stash_draft(&mut self, conv: &ConvId) {
        let text = self.buffer.text().to_string();
        if text.is_empty() {
            self.draft.remove(conv);
        } else {
            self.draft.insert(conv.clone(), text);
        }
        self.buffer.clear();
        self.suggest = None;
    }

    /// Puts `conv`'s draft back, if it has one.
    pub fn restore_draft(&mut self, conv: &ConvId) {
        if let Some(text) = self.draft.remove(conv) {
            self.buffer = TextBuffer::new(&text);
            self.refresh();
        }
    }

    /// Text typed (after each change, as the field reports it: a browser or
    /// an IME types for the client).
    pub fn typed(&mut self, text: &str) {
        let room = COMPOSE_LIMIT.saturating_sub(self.chars());
        // Hidden characters (bidi controls, zero-width, BOM) are dropped as
        // a paste drops them: a receiver would clean them out and lose the
        // message's links with them.
        let clean: String = text
            .chars()
            .filter(|c| !c.is_control() && !super::text::is_hidden(*c))
            .take(room)
            .collect();
        if clean.is_empty() {
            return;
        }
        self.buffer.insert(&clean);
        self.after_edit();
    }

    /// Answers a key.
    pub fn key(&mut self, key: ComposerKey, ctx: &ComposeContext) -> ComposerOutcome {
        if !matches!(key, ComposerKey::Tab | ComposerKey::ShiftTab) {
            self.completing = None;
        }
        match key {
            ComposerKey::Enter => {
                if self.take_suggestion() {
                    ComposerOutcome::Changed
                } else if self.buffer.text().trim().is_empty() {
                    self.buffer.clear();
                    ComposerOutcome::Close
                } else {
                    ComposerOutcome::Submit
                }
            }
            ComposerKey::ShiftEnter => {
                let breaks = self.buffer.text().matches('\n').count();
                if breaks >= MAX_LINE_BREAKS || self.chars() >= COMPOSE_LIMIT {
                    return ComposerOutcome::Nothing;
                }
                self.buffer.replace_selection("\n");
                self.after_edit();
                ComposerOutcome::Changed
            }
            ComposerKey::Esc => {
                if self.suggest.take().is_some() {
                    ComposerOutcome::Changed
                } else {
                    ComposerOutcome::Close
                }
            }
            ComposerKey::Up => self.vertical(Dir::Left),
            ComposerKey::Down => self.vertical(Dir::Right),
            ComposerKey::Tab => self.tab(ctx, true),
            ComposerKey::ShiftTab => self.tab(ctx, false),
            ComposerKey::PageUp => ComposerOutcome::ScrollHistory(-1),
            ComposerKey::PageDown => ComposerOutcome::ScrollHistory(1),
            ComposerKey::Home => self.caret(Step::Line, Dir::Left),
            ComposerKey::End => self.caret(Step::Line, Dir::Right),
            ComposerKey::Left => self.caret(Step::Char, Dir::Left),
            ComposerKey::Right => self.caret(Step::Char, Dir::Right),
            ComposerKey::WordLeft => self.caret(Step::Word, Dir::Left),
            ComposerKey::WordRight => self.caret(Step::Word, Dir::Right),
            ComposerKey::Backspace => self.edit(TextBuffer::delete_back),
            ComposerKey::Delete => self.edit(TextBuffer::delete_forward),
            ComposerKey::WordBackspace => self.edit(|b| {
                if b.selection().is_none() {
                    b.move_caret(Step::Word, Dir::Left, true);
                }
                b.delete_back();
            }),
            ComposerKey::SelectAll => {
                self.buffer.select_all();
                ComposerOutcome::Changed
            }
            ComposerKey::Copy => self
                .selected()
                .map_or(ComposerOutcome::Nothing, ComposerOutcome::Copy),
            ComposerKey::Cut => match self.selected() {
                Some(text) => {
                    self.buffer.delete_back();
                    self.after_edit();
                    ComposerOutcome::Copy(text)
                }
                None => ComposerOutcome::Nothing,
            },
            ComposerKey::Paste(text) => {
                self.paste(&text);
                ComposerOutcome::Changed
            }
            ComposerKey::AltDigit(n) => ComposerOutcome::SwitchTab(n),
        }
    }

    /// Links a right-pressed card at the caret (§11.9). A face-down card, a
    /// teammate's shared hand or a hand seen through control — anything
    /// [`refs::zone_of`] would not offer — inserts nothing and answers
    /// [`ChatPhrase::PrivateCard`]. A card only this player can see (its
    /// own hand, a card it is looking at) is linked, with
    /// [`ChatPhrase::OwnCardHint`] the first time.
    pub fn insert_card(&mut self, card: LinkedCard) -> Option<ChatPhrase> {
        let zone = match card.seen {
            Some(seen) => match refs::zone_of(seen) {
                Some(zone) => Some(zone),
                None => return Some(ChatPhrase::PrivateCard),
            },
            None => None,
        };
        if card.face_down {
            return Some(ChatPhrase::PrivateCard);
        }
        let link = format!("[{}] ", card.name);
        if self.chars() + link.chars().count() > COMPOSE_LIMIT {
            return None;
        }
        self.buffer.replace_selection(&link);
        self.picked.push(CardRef {
            name: card.name,
            english: card.english,
            card: card.card,
            face: card.face,
            art: None,
            print: card.print,
            object: None,
            zone,
            owner: None,
            kind: None,
        });
        self.after_edit();
        let private = matches!(card.seen, Some(SeenIn::Hand | SeenIn::LookingAt));
        if private && !self.own_card_hinted {
            self.own_card_hinted = true;
            return Some(ChatPhrase::OwnCardHint);
        }
        None
    }

    /// What is typed, as the chip's target and a body; `None` when nothing
    /// is.
    #[must_use]
    pub fn body(&self) -> Option<(Target, Body)> {
        let text = self.buffer.text().trim();
        (!text.is_empty()).then(|| (self.target.clone(), self.body_of(text, false)))
    }

    /// `text` as a body, its `[Name]` and `[@Name#tag]` brackets made links
    /// and mentions where they name a candidate (spans in characters).
    #[must_use]
    pub fn body_of(&self, text: &str, emote: bool) -> Body {
        let chars = |byte: usize| text[..byte].chars().count();
        let mut body = Body {
            text: text.to_string(),
            links: Vec::new(),
            mentions: Vec::new(),
            emote,
        };
        for Span { at, target } in refs::parse(text, &self.candidates, &self.picked) {
            let span = [chars(at.start), chars(at.end)];
            match target {
                refs::Target::Card(card) => body.links.push(BodyLink {
                    at: span,
                    card: card.card.get(),
                    face: card.face,
                    print: card.print.map(|p| BodyPrint {
                        scryfall_id: p.scryfall_id,
                        lang: p.lang,
                    }),
                }),
                refs::Target::Player(player) => {
                    if let Some(tag) = player.tag {
                        body.mentions.push(BodyMention { at: span, tag });
                    }
                }
            }
        }
        body
    }

    /// The box after a send: empty, the line remembered for `↑`.
    pub fn sent(&mut self) {
        let text = self.buffer.text().trim().to_string();
        if !text.is_empty() && self.sent.back() != Some(&text) {
            self.sent.push_back(text);
            if self.sent.len() > SENT_KEPT {
                self.sent.pop_front();
            }
        }
        self.buffer.clear();
        self.picked.clear();
        self.suggest = None;
        self.recall = None;
    }

    /// Empties the box without remembering it (a command that ran).
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.picked.clear();
        self.suggest = None;
        self.recall = None;
    }

    fn selected(&self) -> Option<String> {
        self.buffer
            .selection()
            .map(|r| self.buffer.text()[r].to_string())
            .filter(|s| !s.is_empty())
    }

    fn paste(&mut self, text: &str) {
        let breaks = self.buffer.text().matches('\n').count();
        let mut room = COMPOSE_LIMIT.saturating_sub(self.chars());
        let mut left = MAX_LINE_BREAKS.saturating_sub(breaks);
        let mut clean = String::new();
        for c in text.chars() {
            if room == 0 {
                break;
            }
            match c {
                '\r' => continue,
                '\n' if left > 0 => left -= 1,
                '\n' => {
                    clean.push(' ');
                    room -= 1;
                    continue;
                }
                c if c.is_control() || super::text::is_hidden(c) => continue,
                _ => {}
            }
            clean.push(c);
            room -= 1;
        }
        self.buffer.replace_selection(&clean);
        self.after_edit();
    }

    fn caret(&mut self, step: Step, dir: Dir) -> ComposerOutcome {
        self.buffer.move_caret(step, dir, false);
        self.refresh();
        ComposerOutcome::Changed
    }

    fn edit(&mut self, change: impl FnOnce(&mut TextBuffer)) -> ComposerOutcome {
        let before = self.buffer.text().to_string();
        change(&mut self.buffer);
        if self.buffer.text() == before {
            return ComposerOutcome::Nothing;
        }
        self.after_edit();
        ComposerOutcome::Changed
    }

    fn after_edit(&mut self) {
        self.recall = None;
        self.completing = None;
        // A picked card no bracket names any more is forgotten.
        let text = self.buffer.text().to_string();
        self.picked
            .retain(|c| text.contains(&format!("[{}]", c.name)));
        self.refresh();
    }

    /// Re-reads the trigger at the caret and the suggestions for it.
    fn refresh(&mut self) {
        let Some(trigger) = refs::trigger(self.buffer.text(), self.buffer.cursor()) else {
            self.suggest = None;
            return;
        };
        let items: Vec<SuggestItem> = match trigger.sigil {
            Sigil::Card => refs::best(&self.candidates.cards, &trigger.query)
                .into_iter()
                .map(SuggestItem::Card)
                .collect(),
            Sigil::Player => refs::best_players(&self.candidates.players, &trigger.query)
                .into_iter()
                .map(SuggestItem::Player)
                .collect(),
        };
        if items.is_empty() {
            self.suggest = None;
            return;
        }
        let highlight = self
            .suggest
            .as_ref()
            .filter(|s| s.trigger.at.start == trigger.at.start)
            .map_or(0, |s| s.highlight.min(items.len() - 1));
        self.suggest = Some(Suggest {
            trigger,
            items,
            highlight,
        });
    }

    /// Takes the highlighted suggestion; `false` when none is up.
    fn take_suggestion(&mut self) -> bool {
        let Some(suggest) = self.suggest.take() else {
            return false;
        };
        let Some(item) = suggest.items.get(suggest.highlight) else {
            return false;
        };
        let (sigil, name) = match item {
            SuggestItem::Card(i) => {
                let Some(card) = self.candidates.cards.get(*i).cloned() else {
                    return false;
                };
                let name = card.name.clone();
                self.picked.push(card);
                (Sigil::Card, name)
            }
            SuggestItem::Player(i) => {
                let Some(player) = self.candidates.players.get(*i) else {
                    return false;
                };
                (Sigil::Player, player.name.clone())
            }
        };
        let text = self.buffer.text().to_string();
        let at = suggest.trigger.at;
        let written = refs::taken(sigil, &name);
        let mut next = String::with_capacity(text.len() + written.len());
        next.push_str(&text[..at.start]);
        next.push_str(&written);
        next.push_str(&text[at.end..]);
        if next.chars().count() > COMPOSE_LIMIT {
            return false;
        }
        let caret = at.start + written.len();
        self.buffer.set(&next, caret, None);
        self.recall = None;
        self.refresh();
        true
    }

    fn vertical(&mut self, dir: Dir) -> ComposerOutcome {
        if let Some(suggest) = &mut self.suggest {
            let n = suggest.items.len();
            suggest.highlight = match dir {
                Dir::Left => (suggest.highlight + n - 1) % n,
                Dir::Right => (suggest.highlight + 1) % n,
            };
            return ComposerOutcome::Changed;
        }
        let text = self.buffer.text();
        let caret = self.buffer.cursor();
        let line_start = text[..caret].rfind('\n').map_or(0, |i| i + 1);
        let line_end = text[caret..].find('\n').map_or(text.len(), |i| caret + i);
        let moved = match dir {
            Dir::Left if line_start > 0 => {
                let prev_start = text[..line_start - 1].rfind('\n').map_or(0, |i| i + 1);
                Some(column_in(
                    text,
                    prev_start,
                    line_start - 1,
                    caret - line_start,
                ))
            }
            Dir::Right if line_end < text.len() => {
                let next_end = text[line_end + 1..]
                    .find('\n')
                    .map_or(text.len(), |i| line_end + 1 + i);
                Some(column_in(text, line_end + 1, next_end, caret - line_start))
            }
            _ => None,
        };
        if let Some(at) = moved {
            self.buffer.place(at, None);
            return ComposerOutcome::Changed;
        }
        self.walk_sent(dir)
    }

    /// `↑`/`↓` through the lines sent this session, stopping at the ends;
    /// past the newest, what was being typed comes back.
    fn walk_sent(&mut self, dir: Dir) -> ComposerOutcome {
        if self.sent.is_empty() {
            return ComposerOutcome::Nothing;
        }
        let last = self.sent.len() - 1;
        let next = match (&self.recall, dir) {
            (None, Dir::Left) => Some(last),
            (None, Dir::Right) | (Some((0, _)), Dir::Left) => return ComposerOutcome::Nothing,
            (Some((i, _)), Dir::Left) => Some(i - 1),
            (Some((i, _)), Dir::Right) if *i < last => Some(i + 1),
            (Some(_), Dir::Right) => None,
        };
        let typed = self
            .recall
            .take()
            .map_or_else(|| self.buffer.text().to_string(), |(_, t)| t);
        match next {
            Some(i) => {
                self.buffer = TextBuffer::new(&self.sent[i]);
                self.recall = Some((i, typed));
            }
            None => self.buffer = TextBuffer::new(&typed),
        }
        self.suggest = None;
        ComposerOutcome::Changed
    }

    fn tab(&mut self, ctx: &ComposeContext, forward: bool) -> ComposerOutcome {
        if self.take_suggestion() {
            return ComposerOutcome::Changed;
        }
        if forward && self.complete_name(ctx) {
            return ComposerOutcome::Changed;
        }
        if ctx.targets.is_empty() {
            return ComposerOutcome::Nothing;
        }
        let n = ctx.targets.len();
        let at = ctx.targets.iter().position(|t| *t == self.target);
        let next = match (at, forward) {
            (None, true) => 0,
            (None, false) => n - 1,
            (Some(i), true) => (i + 1) % n,
            (Some(i), false) => (i + n - 1) % n,
        };
        self.target = ctx.targets[next].clone();
        ComposerOutcome::Changed
    }

    /// In `/w name` (the caret at the end of the name), Tab completes the
    /// name from `ctx.names`, cycling on repeated presses.
    fn complete_name(&mut self, ctx: &ComposeContext) -> bool {
        let text = self.buffer.text().to_string();
        if self.buffer.cursor() != text.len() {
            return false;
        }
        let Some(rest) = text.strip_prefix('/') else {
            return false;
        };
        let Some((command, arg)) = rest.split_once(' ') else {
            return false;
        };
        let command = command.to_lowercase();
        let names_a_player = [
            "w", "whisper", "tell", "msg", "invite", "inv", "watch", "spectate", "friend", "f",
            "add", "unfriend", "remove", "block", "unblock", "ignore", "unignore", "nick",
        ];
        if !names_a_player.contains(&command.as_str()) || arg.contains(' ') {
            return false;
        }
        let (prefix, from) = match &self.completing {
            Some((prefix, i)) => (prefix.clone(), i + 1),
            None => (arg.to_string(), 0),
        };
        let lower = prefix.to_lowercase();
        let matching: Vec<&String> = ctx
            .names
            .iter()
            .filter(|n| n.to_lowercase().starts_with(&lower))
            .collect();
        if matching.is_empty() {
            return false;
        }
        let i = from % matching.len();
        let line = format!("/{command} {}", matching[i]);
        let len = line.len();
        self.buffer.set(&line, len, None);
        self.completing = Some((prefix, i));
        true
    }
}

/// The byte in `text[start..end]` at `column` bytes from `start`, or its
/// end, on a character boundary.
fn column_in(text: &str, start: usize, end: usize, column: usize) -> usize {
    let mut at = (start + column).min(end);
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}
